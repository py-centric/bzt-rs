# Tasks: Architectural Remediation

**Status**: The codebase has a solid pipeline skeleton with several genuinely implemented components (parsers, normalizer, translator, assertions, extraction, mock server, validation). However, critical gaps exist between spec claims and actual implementation. No-op modules marked complete, misleading CLI flags, and stringly-typed errors need remediation.

**Theme**: `arch-remediation`

---

## Phase 1: Implement No-Op Modules (P0 - Credibility)

**Purpose**: Replace empty placeholder files with real implementations or honest removal.

- [ ] T101 [env] Implement `EnvironmentLoader` in `src/engine/env.rs`
  - Read `.env` files from `config/` directory
  - Structured variable resolution: CLI args > env vars > config defaults
  - Support `${env.DEBUG}` syntax expected by `sample_test.yaml`
  - Add secret/sensitive variable masking for log safety
  - Tests: env var resolution, priority chaining, missing var handling

- [ ] T102 [pacing] Implement `PacingEngine` in `src/engine/pacing.rs`
  - Closed-loop pacing (think-time based) and open-loop (arrival-rate based)
  - Config model: extend `ExecutionPlan` with optional `PacingConfig { rate, per, randomize }`
  - Integrate with existing `ExecutionPlan.throughput` field
  - Tests: rate limiting accuracy, randomization distribution, edge cases (0 rate)

- [ ] T103 [sla] Implement `SlaEngine` in `src/engine/sla.rs`
  - Move inline SLA logic from `src/engine/goose.rs:28-47` into `SlaEngine::evaluate()`
  - Support rich criteria: `avg-rt`, `fail`, `p90`, `p99`, `throughput`
  - Config model: extend `ReportingDefinition` with `criteria: Vec<SlaCriterion>`
  - Support actions per criterion: `stop`, `warn`, `continue`
  - Structured `SlaViolation` result type instead of `Err(String)`
  - Tests: pass/fail evaluation for each criterion type

---

## Phase 2: Fix Error Handling (P0 - Quality)

**Purpose**: Replace stringly-typed errors with the defined `PummelError` enum.

- [ ] T201 [errors] Populate `PummelError` with proper variants in `src/engine/mod.rs`
  - Add variants: `Io`, `Serde`, `Goose`, `Parse`, `Validation`, `Network`, `SlaViolation`
  - Use `#[from]` for automatic conversion from `std::io::Error`, `serde::Error`, etc.
  - Each variant carries structured context data, not just strings

- [ ] T202 [errors] Migrate `src/parser/` from `Result<_, String>` to `Result<_, PummelError>`
  - All three parsers: `yaml.rs`, `json.rs`, `toml.rs`
  - Callsites in `src/main.rs:73-88`

- [ ] T203 [errors] Migrate `src/translator/mod.rs` from `Result<_, String>` to `Result<_, PummelError>`
  - `StateTranslator::translate`, `evaluate_condition`, duration parsers

- [ ] T204 [errors] Migrate `src/engine/` modules from `Result<_, String>` to `Result<_, PummelError>`
  - `goose.rs`, `control_flow.rs`, `data_sources.rs`, `extraction.rs`, `mock.rs`, `validation.rs`

- [ ] T205 [errors] Remove unused `#[allow(clippy::missing_errors_doc)]` annotations after migration

---

## Phase 3: Fix Misleading CLI Flags (P0 - Trust)

**Purpose**: Either implement or strip flags that set false expectations.

### Option A (Recommended: Strip for now):
- [ ] T301 [cli] Remove `--manager`, `--worker`, `--expect-workers`, `--manager-host`, `--manager-port` from `Args` in `src/main.rs:19-37`
- [ ] T302 [cli] Remove Goose env var setting for manager/worker in `src/main.rs:94-109`
- [ ] T303 [cli] Remove `--metrics`, `--metrics-port` from `Args` and env var scaffolding in `src/main.rs:115-135`
- [ ] T304 [cli] Remove `--otel` flag and env var in `src/main.rs:127-134`
- [ ] T305 [cli] Remove unused dependencies from `Cargo.toml`: `prometheus`, `opentelemetry`
- [ ] T306 [tests] Update `tests/distributed_integration.rs` to reflect removed flags (or remove file)

### Option B (Implement properly):
- [ ] T310 [distributed] Implement Manager process: worker registration, liveness, phase coordination, result aggregation in `src/engine/distributed.rs`
- [ ] T311 [distributed] Implement Worker process: connect to manager, receive config, report status in `src/engine/distributed.rs`
- [ ] T312 [distributed] Add network protocol: heartbeat, worker discovery, partition tolerance
- [ ] T313 [observability] Implement Prometheus `/metrics` endpoint using `prometheus` crate in `src/engine/metrics.rs`
- [ ] T314 [observability] Initialize OpenTelemetry tracer and inject `traceparent` into HTTP requests in `src/translator/mod.rs`

---

## Phase 4: Add CLI Reporter (P1 - UX)

**Purpose**: Users currently get no terminal output after test execution.

- [ ] T401 [reporting] Implement `CliReporter` in `src/engine/reporting.rs`
  - After `attack.execute().await` in `src/engine/goose.rs:18`, call `CliReporter::summary(&stats)`
  - Output table: total requests, success/fail counts, avg/p95/p99 latency, RPS
  - Duration, users, scenario breakdown

- [ ] T402 [reporting] Add `-v`/`--verbose` flag to `Args` in `src/main.rs` for detailed CLI output
  - Aligns with `quickstart.md` which documents this flag (currently absent)

---

## Phase 5: Security Hardening (P1 - Safety)

**Purpose**: Address path traversal, OOM risk, injection, and silent failures.

- [ ] T501 [validation] Add `#[serde(deny_unknown_fields)]` to all config structs in `src/models/config.rs`
  - `Configuration`, `ExecutionPlan`, `ScenarioDefinition`, `DetailedRequest`, `ReportingDefinition`

- [ ] T502 [validation] Add path canonicalization and prefix check in `src/engine/data_sources.rs:12`
  - Reject paths containing `..` that escape allowed directories
  - Use `std::fs::canonicalize` and verify prefix

- [ ] T503 [validation] Add max file size limit (100MB) for CSV loading in `src/engine/data_sources.rs`
  - Check `File::metadata().len()` before reading into memory

- [ ] T504 [security] Change mock server bind address from `0.0.0.0` to `127.0.0.1` in `src/engine/mock.rs:103`

- [ ] T505 [security] Add env var allowlist/blocklist in `src/engine/macros.rs:50`
  - Block sensitive patterns (`AWS_SECRET`, `DATABASE_URL`, `PRIVATE_KEY`, etc.) from config-level access
  - Or restrict environment variable access to explicitly declared vars only

---

## Phase 6: Config Model Completeness (P1 - Compatibility)

**Purpose**: Support standard Taurus YAML fields that are currently silently dropped.

- [ ] T601 [model] Add scenario-level `headers: HashMap<String, String>` to `ScenarioDefinition` in `src/models/config.rs`
  - Apply as default headers for all requests in the scenario

- [ ] T602 [model] Add `label: Option<String>` to `DetailedRequest` in `src/models/config.rs`
  - Custom label for request metrics (used in `sample_test.yaml:17`)

- [ ] T603 [model] Add `timeout: Option<String>` to `DetailedRequest` in `src/models/config.rs`
- [ ] T604 [model] Add `body_file: Option<String>` to `DetailedRequest` (Taurus-standard `body-file`)
- [ ] T605 [model] Add `port: Option<u16>` to `ReportingDefinition` for `module: prometheus`
- [ ] T606 [model] Add structured `DataSourceDefinition` replacing `Vec<String>`:
  ```rust
  pub struct DataSourceDefinition {
      pub path: String,
      pub delimiter: Option<char>,
      pub quoted: Option<bool>,
      pub loop_data: Option<bool>,
      pub random_order: Option<bool>,
  }
  ```

- [ ] T607 [model] Add `PacingConfig` block to `ExecutionPlan`: `rate: usize`, `per: String`, `randomize: bool`

---

## Phase 7: Flesh Out Protocol Stubs (P2 - Honesty)

**Purpose**: WebSocket and gRPC are `println!` only. Either implement or clearly document as not-yet-supported.

- [ ] T701 [ws] Add `tokio-tungstenite` dependency to `Cargo.toml`
- [ ] T702 [ws] Implement `WebSocketEngine` in `src/engine/websocket.rs`
  - Connection pooling, frame send/receive, reconnection
  - Integration with extraction/interpolation/assertion pipeline

- [ ] T703 [grpc] Add `tonic` dependency to `Cargo.toml`
- [ ] T704 [grpc] Implement `GrpcEngine` in `src/engine/grpc.rs`
  - Protobuf deserialization, method invocation, streaming support

- [ ] T705 [protocol] Refactor protocol dispatch in `src/translator/mod.rs:149-168`
  - Route to dedicated engines instead of `println!`
  - Handle protocol-specific errors gracefully

---

## Phase 8: Complete Missing Unit Tests (P2 - Coverage)

**Purpose**: Fill critical testing gaps where modules have zero tests.

- [ ] T801 [tests] Add unit tests for `CsvDataSource` in `src/engine/data_sources.rs`
  - Normal CSV, empty CSV, CSV with missing fields, CSV with headers mismatch

- [ ] T802 [tests] Add unit tests for `JUnitReporter` in `src/engine/reporting.rs`
  - Valid XML output, failure representation, empty stats, zero counter edge case

- [ ] T803 [tests] Add unit tests for `Interpolator` in `src/engine/interpolation.rs`
  - Single var, multiple vars, missing var (leave placeholder), empty input

- [ ] T804 [tests] Add unit tests for `run_attack` in `src/engine/goose.rs`
  - Mock GooseMetrics, verify JUnit call, verify SLA enforcement

- [ ] T805 [tests] Add unit tests for mock server edge cases in `src/engine/mock.rs`
  - Method mismatch, 404 fallback, multiple scenarios with same path

- [ ] T806 [tests] Fix `data_integration.rs` — create or mock `users.csv` so test doesn't fail with file not found

- [ ] T807 [tests] Fix `distributed_integration.rs` — either test actual distributed behavior or remove

- [ ] T808 [tests] Fix `mock_integration.rs` — replace 5s race condition sleep with proper retry/await pattern, add assertions

- [ ] T809 [tests] Strengthen `sla_integration.rs` — set a `failed_threshold` and verify breach detection

---

## Phase 9: Spec Alignment & Documentation (P2 - Integrity)

**Purpose**: Make the codebase honest about what it delivers.

- [ ] T901 [spec] Update `specs/001-pummel-core/tasks.md` — unmark tasks that are incomplete:
  - T047 (sla): file is no-op → uncheck
  - T048 (pacing): file is no-op → uncheck
  - T049 (distributed): only env vars → uncheck
  - T050 (websocket/grpc): only println stubs → uncheck
  - T051 (prometheus): no `/metrics` endpoint → uncheck
  - T052 (otel): no instrumentation → uncheck

- [ ] T902 [spec] Update `README.md` feature claims to reflect actual state
  - "Distributed Scale" → mark as planned, not implemented
  - "Observability" → mark as scaffolding only
  - "Advanced Control Flow" → mark setup/teardown and protocol support as partial

- [ ] T903 [doc] Update `docs/architecture.md` diagrams to reflect actual component states
  - Mark distributed, observability, pacing, SLA as "planned" with dashed borders

---

## Phase 10: Additional HTTP Method Support (P3)

**Purpose**: Currently only `POST` and `GET` have real implementations.

- [ ] T1001 [translator] Add PUT, DELETE, PATCH, HEAD, OPTIONS support in `src/translator/mod.rs:184-188`
  - Use Goose's generic request builder instead of `user.post`/`user.get`
  - Map method string to correct Goose request call

---

## Dependencies & Execution Order

- **Phase 1** (No-Op Modules): Foundation — must be completed first
- **Phase 2** (Error Handling): Can start after Phase 1 conceptually, independent files
- **Phase 3** (CLI Flags): Can run in parallel with Phase 1
- **Phase 4** (CLI Reporter): Depends on understanding GooseMetrics shape (look at existing usage in goose.rs/reporting.rs)
- **Phase 5** (Security): Independent, can run in any order
- **Phase 6** (Config Model): Depends on Phase 2 (new types need proper errors)
- **Phase 7** (Protocols): Depends on Phase 6 (model changes)
- **Phase 8** (Tests): Can run in parallel with all other phases
- **Phase 9** (Spec Alignment): Final phase — only after all other changes are validated

### Recommended Priority Order

1. **P0 — Phase 1 (T101-T103)**: Implement or remove no-op modules. This is a credibility blocker.
2. **P0 — Phase 3 (T301-T306 or T310-T314)**: Fix misleading CLI flags. Users should not see flags that don't work.
3. **P1 — Phase 2 (T201-T205)**: Migrate to `PummelError`. Touches every module, repays debugging debt.
4. **P1 — Phase 4 (T401-T402)**: Add CLI reporter. Users need terminal output from every test run.
5. **P1 — Phase 5 (T501-T505)**: Security hardening. Always time-sensitive.
6. **P1 — Phase 6 (T601-T607)**: Config model completeness. Ensures Taurus compatibility.
7. **P2 — Phase 8 (T801-T809)**: Test coverage. Prevent regression.
8. **P2 — Phase 7 (T701-T705)**: Protocol stubs. Low urgency since these are new features.
9. **P2 — Phase 9 (T901-T903)**: Spec alignment. Important for maintainers but not runtime.
10. **P3 — Phase 10 (T1001)**: Additional HTTP methods. Quick win, low effort.

## Parallel Opportunities

- T103 (SLA) depends on T102 (Pacing) architecturally but the file contents are independent
- T201-T205 (Error handling) can be done per-file in parallel across different contributors
- T401 (CLI reporter) and T402 (verbose flag) are independent
- All T8xx (tests) can be written in parallel
