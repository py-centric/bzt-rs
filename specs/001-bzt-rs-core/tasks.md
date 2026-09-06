# Tasks: pummel Core Architecture & Roadmap

**Input**: Design documents from `/specs/001-pummel-core/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md

**Tests**: As per Constitution Principle III, TDD is MANDATORY. Tests MUST be written and fail before implementation.

**Commits**: As per Constitution Principle VI, an ATOMIC COMMIT must be created immediately after the completion of each discrete task. Use conventional commit messages.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Rust project**: `src/`, `tests/` at repository root.
- Use `src/lib.rs` or `src/main.rs` as entry points.
- Submodules in `src/[module]/mod.rs` or `src/[module].rs`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [x] T001 Create project structure: `src/parser/`, `src/normalizer/`, `src/translator/`, `src/engine/`, `src/models/`
- [x] T002 Initialize Rust project with `goose`, `tokio`, `serde`, `serde_yaml`, `serde_json`, `toml`, `regex`, `fake`, `uuid` in `Cargo.toml`
- [x] T003 [P] Configure `cargo fmt`, `clippy`, and `pre-commit` hooks (including 80% coverage enforcement) for local enforcement

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T004 Setup data models for `Configuration`, `ExecutionPlan`, `ScenarioDefinition` in `src/models/config.rs`
- [x] T005 [P] Implement core traits for `Parser` and `Translator` in `src/parser/mod.rs` and `src/translator/mod.rs`
- [x] T006 [P] Setup logging using `tracing` and `tracing-subscriber` in `src/engine/mod.rs`
- [x] T007 Setup environment configuration and secret management in `config/settings.env` and ensure `config/secrets.env` is in `.gitignore`
- [x] T008 [P] Configure GitHub Actions CI pipeline to run `cargo check`, `clippy`, `test`, and `audit`

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Basic Load Test Execution (YAML Core) (Priority: P1) 🎯 MVP

**Goal**: Execute a basic load test using standard Taurus YAML.

**Independent Test**: Provide a valid `test.yaml` and verify the tool initiates a Goose attack with matching parameters and outputs a report.

### Tests for User Story 1 (MANDATORY - TDD) ⚠️

- [x] T010 [P] [US1] Unit tests for YAML deserialization in `src/parser/yaml.rs`
- [x] T011 [P] [US1] Integration test for "Walking Skeleton" run in `tests/skeleton_integration.rs`

### Implementation for User Story 1

- [x] T012 [P] [US1] Implement YAML parser using `serde_yaml` in `src/parser/yaml.rs`
- [x] T013 [P] [US1] Implement basic `ExecutionPlan` to `GooseAttack` mapping in `src/translator/mod.rs`
- [x] T014 [US1] Implement `Goose` engine wrapper to start attacks in `src/engine/goose.rs`
- [x] T015 [US1] Implement CLI entry point in `src/main.rs` to parse args and run test
- [x] T016 [US1] Add pedantic logging for YAML parsing and attack initiation

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently.

---

## Phase 4: User Story 2 - Multi-Format & Shorthand (Priority: P2)

**Goal**: Support JSON, TOML, and simplified Shorthand syntax.

**Independent Test**: Provide identical tests in JSON/TOML/Shorthand and verify identical execution.

### Tests for User Story 2 (MANDATORY - TDD) ⚠️

- [x] T017 [P] [US2] Unit tests for JSON and TOML deserialization in `src/parser/json.rs` and `src/parser/toml.rs`
- [x] T018 [P] [US2] Integration test for Shorthand syntax normalization in `tests/shorthand_integration.rs`

### Implementation for User Story 2

- [x] T019 [P] [US2] Implement JSON parser in `src/parser/json.rs` and TOML parser in `src/parser/toml.rs`
- [x] T020 [US2] Implement `SchemaNormalizer` to unify Shorthand and Taurus schemas in `src/normalizer/mod.rs`
- [x] T021 [US2] Integrate multi-format detection in `src/main.rs`

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently.

---

## Phase 5: User Story 3 - Hierarchies & Scenario Branching (Priority: P3)

**Goal**: Support nested scenarios and weighted traffic distribution.

**Independent Test**: Run a test with a setup parent scenario and 80/20 weighted child scenarios; verify metrics.

### Tests for User Story 3 (MANDATORY - TDD) ⚠️

- [x] T022 [P] [US3] Unit tests for hierarchical scenario resolution in `src/normalizer/mod.rs`
- [x] T023 [P] [US3] Integration test for weighted scenario distribution in `tests/branching_integration.rs`

### Implementation for User Story 3

- [x] T024 [US3] Update `SchemaNormalizer` to support dot-notation and physical nesting in `src/normalizer/mod.rs`
- [x] T025 [US3] Implement `Goose` scenario weighting in `src/translator/mod.rs`
- [x] T026 [US3] Ensure parent scenarios map to `Goose` setup tasks to run once per session

**Checkpoint**: User Stories 1, 2, and 3 are functional.

---

## Phase 6: User Story 4 - Realistic Traffic Generation (Priority: P4)

**Goal**: Inject env vars, macros (faker/uuid), and CSV data.

**Independent Test**: Run a test using `${ENV}`, `${faker.email}`, and a CSV file; verify request payloads.

### Tests for User Story 4 (MANDATORY - TDD) ⚠️

- [x] T027 [P] [US4] Unit tests for macro evaluation in `src/engine/macros.rs`
- [x] T028 [P] [US4] Integration test for CSV parameterization in `tests/data_integration.rs`

### Implementation for User Story 4

- [x] T029 [P] [US4] Implement environment variable injection logic in `src/engine/macros.rs`
- [x] T030 [P] [US4] Implement dynamic macro evaluator (faker/uuid) in `src/engine/macros.rs`
- [x] T031 [US4] Implement CSV data source reader in `src/engine/data_sources.rs`
- [x] T032 [US4] Implement fixed and randomized think-time in `src/engine/goose.rs` via `src/translator/mod.rs`

---

## Phase 7: User Story 5 - State & Control Flow (Priority: P5)

**Goal**: Support extraction, interpolation, assertions, and polling loops.

**Independent Test**: Run a test with JSONPath/Regex extraction and variable interpolation; verify logic works.

### Tests for User Story 5 (MANDATORY - TDD) ⚠️

- [x] T039 [P] [US5] Unit tests for JSONPath/Regex extraction in `src/engine/extraction.rs`
- [x] T040 [P] [US5] Integration test for variable interpolation and polling loops in `tests/state_integration.rs`

### Implementation for User Story 5

- [x] T041 [US5] Implement extraction engine (JSONPath/Regex) in `src/engine/extraction.rs` (FR-011)
- [x] T042 [US5] Implement session-variable interpolation in `src/engine/interpolation.rs` (FR-011)
- [x] T043 [US5] Implement assertions and control flow logic (if/loops) in `src/engine/control_flow.rs` (FR-012, FR-013)

---

## Phase 8: User Story 6 - CI/CD & SLAs (Priority: P6)

**Goal**: Support JUnit XML, SLA thresholds, and strict throughput pacing.

**Independent Test**: Verify JUnit reports and SLA breach behavior in CLI.

### Tests for User Story 6 (MANDATORY - TDD) ⚠️

- [x] T044 [P] [US6] Unit tests for JUnit XML generation and SLA threshold checks
- [x] T045 [P] [US6] Integration test for pass/fail criteria and throughput pacing

### Implementation for User Story 6

- [x] T046 [US6] Implement JUnit XML exporter in `src/engine/reporting.rs` (FR-014)
- [x] T047 [US6] Implement SLA/PassFail enforcement logic in `src/engine/sla.rs` (FR-015)
- [x] T048 [US6] Implement strict throughput pacing/throttling in `src/engine/pacing.rs` (FR-016, SC-005)

---

## Phase 9: User Story 7 - Distributed & Multi-Protocol (Priority: P7)

**Goal**: Support cluster coordination, WebSockets, and gRPC.

**Independent Test**: Run a test in distributed mode and verify multi-protocol support.

- [x] T049 [US7] Implement Manager/Worker CLI modes and network coordination (FR-017)
- [x] T050 [US7] Add support for WebSockets and gRPC protocol blocks (FR-017)

---

## Phase 10: User Story 8 - Observability (Priority: P8)

**Goal**: Support Prometheus metrics and OpenTelemetry tracing.

**Independent Test**: Verify metrics at /metrics and traceparent injection.

- [x] T051 [US8] Implement Prometheus metrics exporter and `/metrics` endpoint (FR-018)
- [x] T052 [US8] Implement OpenTelemetry traceparent injection logic (FR-018)

---

## Phase N: Polish, Quality Gates & Documentation

**Purpose**: Final verification and exhaustive documentation

- [x] T033 [P] Run `cargo fmt`, `clippy`, and `audit` for final quality pass
- [x] T034 [P] Verify code coverage >= 80% using `cargo-llvm-cov` or `tarpaulin`
- [x] T053 [P] Conduct performance benchmarks to verify startup-only parsing (SC-001) and <5% overhead (SC-002)
- [x] T035 [P] Generate `rustdoc` for all public modules and document all public APIs
- [x] T036 [P] Create architecture diagrams (Mermaid) in `docs/architecture.md`
- [x] T037 [P] Finalize deployment guide and license documentation in `README.md`
- [x] T038 Run `quickstart.md` validation on the final binary


---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 & 2**: MUST be completed first.
- **Phase 3 (MVP)**: Can start after Phase 2.
- **Phases 4, 5, 6, 7, 8, 9, 10**: Can run in parallel after Phase 3 is validated, or sequentially.

### Implementation Strategy

- **MVP First**: Complete through Phase 3 to have a working YAML runner.
- **Incremental Delivery**: Add multi-format support (Phase 4), then complex scenarios (Phase 5), then traffic realism (Phase 6), state management (Phase 7), CI/CD (Phase 8), distribution (Phase 9), and observability (Phase 10).

---

## Parallel Opportunities

- T003 (Hooks) can run alongside T001/T002.
- T005, T006, T008 (Traits, Logging, CI) can run in parallel in Phase 2.
- Unit tests for different parsers in Phase 4 (T017) can be written in parallel.
- Environment variable (T029) and Macro (T030) logic in Phase 6 can be developed in parallel.
