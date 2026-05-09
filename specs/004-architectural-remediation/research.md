# Research: Architectural Remediation

**Branch**: `004-architectural-remediation` | **Date**: 2026-05-09 | **Spec**: [spec.md](spec.md)

## Researched Decisions

### 1. BztError Variant Design

**Decision**: Extend `BztError` enum with specific variants covering all error paths.

**Rationale**: The existing `BztError` has 3 variants all carrying `String` — too generic. Each error path in the codebase maps to a distinct variant with structured context.

| Error Source | Current | Proposed Variant | Context Fields |
|---|---|---|---|
| File I/O (data_sources, reporting) | `String` (manual) | `BztError::Io(std::io::Error)` | source file, operation |
| Serde parsing (yaml/json/toml) | `String` (serde error) | `BztError::Serde(String)` | raw error, file path |
| Goose engine | `String` (map_err) | `BztError::Goose(Box<dyn std::error::Error>)` | wrapped GooseError |
| Config validation | `BztError::Validation(String)` | `BztError::Validation { field: String, reason: String }` | field name, reason |
| Mock server | `BztError::Mock(String)` | `BztError::Mock { addr: String, reason: String }` | address, reason |
| Network/connection | `String` | `BztError::Network(String)` | host, operation |
| SLA violation | `String` (inline format!) | `BztError::SlaViolation { actual: f32, threshold: f32, metric: String }` | actual, threshold, metric |
| Internal/unexpected | `BztError::Internal(String)` | unchanged | message |
| Environment resolution | `String` | `BztError::Environment { var: String, reason: String }` | variable name, reason |

**Alternatives considered**: Using `Box<dyn std::error::Error>` as the sole error type (loses structured matching), or a custom error crate (overkill for current scope).

---

### 2. CLI Flags — Strip vs. Implement

**Decision**: Strip unimplemented CLI flags (`--manager`, `--worker`, `--metrics`, `--otel`) and mark as "planned for future release" in README.

**Rationale**: Proper distributed execution and observability instrumentation require significant infrastructure work (network protocol, /metrics HTTP server, OTel SDK integration). Stripping now eliminates false user expectations. Re-implement later when these features are genuinely ready.

**Alternatives considered**: 
- Keep flags with "not implemented" error messages — creates ongoing maintenance burden of flags that do nothing
- Full implementation — scope too large for current remediation sprint (would require distributed consensus, metrics HTTP server, OTel trace propagation)

**Affected files**: `src/main.rs` (remove flags + env var scaffolding), `Cargo.toml` (remove prometheus + opentelemetry deps), `README.md` (update feature list)

---

### 3. Environment Module Design

**Decision**: Implement `EnvironmentLoader` in `env.rs` with priority chain and allowlist.

**Priority chain** (highest to lowest):
1. CLI argument overrides
2. System environment variables
3. `.env` file from project root / `config/settings.env`
4. Default values in config

**Allowlist approach**: Blocklist of sensitive patterns (`AWS_*`, `SECRET*`, `KEY`, `TOKEN`, `PASSWORD`, `PRIVATE*`, `DATABASE_URL`, `*_SECRET`, `*_KEY`) rather than explicit allowlist. Blocked patterns are replaced with `[REDACTED]` in logs but still resolved in the actual value (only log output is masked).

**Alternatives considered**: 
- Explicit allowlist (too restrictive, requires updating for each new env var)
- No protection (current behavior — all env vars exposed)

---

### 4. Pacing Engine Design

**Decision**: Implement `PacingEngine` as a pre-request delay calculator integrated with Goose's existing `ThrottleRequests` and think-time.

**Strategy**: The scheduler tracks request timing and inserts calculated delays before each request to maintain the target rate. Two modes:
- **Fixed rate**: Uniform delay = period / rate (e.g., 100ms delay for 10 req/s)
- **Randomized rate**: Exponential distribution around the target rate (more realistic traffic patterns)

The existing `ExecutionPlan.throughput` feeds into this engine. Additional config fields: `pacing.randomize: bool`.

**Alternatives considered**:
- Pure Goose throttle (already exists but no pacing/randomization)
- Token bucket algorithm (more complex, unnecessary for current requirements)

---

### 5. CLI Reporter Design

**Decision**: Implement `CliReporter` using `GooseMetrics` data structure, outputting an ASCII table to stdout.

**Data sources from GooseMetrics**:
- `stats.requests` — HashMap of `GooseRequestMetricAggregate` (method, path, success_count, fail_count, raw_data with counter/total_time/minimum_time/maximum_time)
- `stats.duration` — total test duration in seconds
- `stats.maximum_users` / `stats.total_users`
- `stats.errors` — error aggregates

**Output format**:
```
===== Test Results =====
Duration: 60.0s    Users: 50
Total Requests: 12,345    Failed: 45 (0.36%)
Avg Latency: 142ms    p95: 412ms    p99: 891ms

  Method  Path              Count    Failed    Avg(ms)    p95(ms)
  ------  ----              -----    ------    -------    -------
  GET     /api/status       5000      0         120        350
  POST    /api/users        2345     12        180        450
...
```

**p95/p99 calculation**: Sort the response time distribution (available in `raw_data.times` as `BTreeMap<usize, usize>`), then compute percentile positions.

**Alternatives considered**: 
- JSON-only output (less human-friendly)
- Delegate to Goose's built-in Display (already shows tables but only on stderr with specific flags)

---

### 6. Config Model Extensions

**Decision**: Add the following fields to the existing model (not a new config schema):

| Field | Type | Location | Default |
|---|---|---|---|
| `label` | `Option<String>` | `DetailedRequest` | None (use URL) |
| `headers` (scenario) | `Option<HashMap<String, String>>` | `ScenarioDefinition` | None |
| `timeout` | `Option<String>` | `DetailedRequest` | None (no timeout) |
| `body-file` | `Option<PathBuf>` | `DetailedRequest` | None |
| `pacing` | `Option<PacingConfig>` | `ExecutionPlan` | None |
| DataSourceDefinition | struct | `ScenarioDefinition` replacing `Vec<String>` | — |

**Serde safety**: All new fields use `#[serde(default)]` to maintain backward compatibility with existing configs.

---

### 7. Testing Baseline

**Current integration test status**: Verified by examining test files.

| Test File | Status | Action |
|---|---|---|
| skeleton_integration.rs | ✅ Passing | No change |
| state_integration.rs | ✅ Passing | No change |
| data_integration.rs | ❌ Broken (missing users.csv) | Create users.csv fixture in tests/common/ |
| distributed_integration.rs | ⚠️ Weak (env vars only) | Remove or rewrite to test flag behavior |
| dry_run_integration.rs | ✅ Passing | No change |
| mock_integration.rs | ⚠️ Race condition (5s sleep) | Replace with retry/await pattern |
| sla_integration.rs | ⚠️ Shallow (no threshold) | Add threshold, verify breach |
| shorthand_integration.rs | ✅ Passing | No change |
| branching_integration.rs | ✅ Passing | No change |

**Modules missing unit tests**: `data_sources.rs`, `reporting.rs`, `goose.rs`, `interpolation.rs`

---

### 8. HTTP Method Support

**Decision**: Support all standard methods by using Goose's generic request infrastructure.

**Goose API**: `GooseUser` has `request_builder()` method that accepts `GooseMethod` enum. All standard methods (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS, CONNECT, TRACE) are available via the `reqwest` backend.

**Implementation**: Replace `user.post()` and `user.get()` with a match on method string → `user.request_builder(goose_method, url)`.

---

### 9. Security Hardening Details

| Issue | Implementation |
|---|---|
| Path traversal | `std::fs::canonicalize()` + `Path::starts_with()` check against allowed root |
| File size limit | `std::fs::metadata().len()` check before reading (100MB limit) |
| Unknown config fields | `#[serde(deny_unknown_fields)]` on all config structs |
| Mock server bind | Change `0.0.0.0` to `127.0.0.1` in `mock.rs` |
| Env var exposure | Blocklist approach (see Decision 3) |
| Regex ReDoS | Add `regex.RegexBuilder` with size limit; wrap in try/error instead of `unwrap()` |
