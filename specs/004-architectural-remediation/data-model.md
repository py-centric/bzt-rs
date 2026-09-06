# Data Model: Architectural Remediation

**Branch**: `004-architectural-remediation` | **Date**: 2026-05-09 | **Spec**: [spec.md](spec.md) | **Research**: [research.md](research.md)

## Entity Summary

| Entity | Type | Description |
|--------|------|-------------|
| PummelError | Enum | Structured error type for all error paths |
| PacingConfig | Struct | Throttling/pacing configuration for an execution plan |
| DataSourceDefinition | Struct | Structured data source configuration |
| SlaCriterion | Struct | A single SLA pass/fail condition |
| SlaResult | Struct | Result of evaluating an SLA criterion |
| CliSummary | Struct | Reporter output for terminal display |

---

## PummelError (Engine Error Enum)

**File**: `src/engine/mod.rs`

The unified error type for all pummel operations. Replaces `Result<(), String>` throughout the codebase.

| Variant | Context Fields | Source |
|---|---|---|
| `Io` | `source: std::io::Error`, `context: String` | File operations |
| `Serde` | `message: String`, `file_path: Option<String>` | Config deserialization |
| `Goose` | `source: Box<dyn std::error::Error + Send>` | Goose engine errors |
| `Validation` | `field: String`, `reason: String` | Config validation failures |
| `Mock` | `reason: String` | Mock server errors |
| `Network` | `host: String`, `operation: String`, `details: String` | Connection/IO failures |
| `SlaViolation` | `actual: f32`, `threshold: f32`, `metric: String` | SLA threshold breach |
| `Environment` | `var: String`, `reason: String` | Env var resolution failures |
| `NotImplemented` | `feature: String` | CLI flags for unimplemented features |

**Validation Rules**:
- ALL error returns must use PummelError, never raw String
- `Io` variants must preserve original `std::io::Error` via `#[from]`
- `Goose` variant must preserve original error via `source` field

---

## PacingConfig (Execution Plan Pacing)

**File**: `src/models/config.rs` (new struct)

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `rate` | `usize` | Yes | — | Target request rate |
| `per` | `String` | No | `"1s"` | Time period for the rate (e.g., "1s", "1m") |
| `randomize` | `bool` | No | `false` | Whether to randomize inter-request timing |

**Validation Rules**:
- `rate` must be > 0 (rate of 0 means no pacing)
- `per` must parse as a valid duration (e.g., "1s", "30s", "1m")
- `randomize` when true uses exponential distribution; when false uses uniform timing

**Relationships**: Embedded in `ExecutionPlan` as `pacing: Option<PacingConfig>`

---

## DataSourceDefinition (Structured Data Source)

**File**: `src/models/config.rs` (new struct, replaces raw `Vec<String>`)

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `path` | `String` | Yes | — | File path to the data source |
| `delimiter` | `char` | No | `','` | CSV delimiter character |
| `quoted` | `bool` | No | `true` | Whether fields are quoted |
| `loop_data` | `bool` | No | `true` | Whether to loop back to start when reaching end |
| `random_order` | `bool` | No | `false` | Whether to select records in random order |

**Validation Rules**:
- `path` must not contain `../` traversal (checked at validation time)
- `path` file size must not exceed 100MB
- `delimiter` must be a single ASCII character

**Relationships**: `ScenarioDefinition.data_sources` changes from `Option<Vec<String>>` to `Option<Vec<DataSourceDefinition>>`

---

## SlaCriterion (Pass/Fail Condition)

**File**: `src/models/config.rs` (new struct)

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `metric` | `SlaMetric` | Yes | — | Which metric to evaluate |
| `threshold` | `f32` | Yes | — | Threshold value for the metric |
| `subject` | `Option<String>` | No | `None` | Specific request/scenario to apply to (None = aggregate) |
| `duration` | `Option<String>` | No | `None` | Window over which to evaluate |
| `action` | `SlaAction` | No | `stop` | Action to take on breach |

### SlaMetric Enum

| Variant | Description |
|---|---|
| `FailRate` | Ratio of failed requests to total |
| `AvgResponseTime` | Average response time (ms) |
| `P90ResponseTime` | 90th percentile response time (ms) |
| `P95ResponseTime` | 95th percentile response time (ms) |
| `P99ResponseTime` | 99th percentile response time (ms) |
| `Throughput` | Requests per second |

### SlaAction Enum

| Variant | Description |
|---|---|
| `Stop` | Halt the test immediately |
| `Warn` | Log warning but continue |
| `Continue` | Record breach but continue |

**Validation Rules**:
- `threshold` must be > 0.0 for all metrics except FailRate (0.0–1.0)
- `duration` must parse as valid duration or be None
- Unknown metrics produce a validation error

---

## SlaResult (SLA Evaluation Result)

| Field | Type | Description |
|---|---|---|
| `metric` | `SlaMetric` | Which criterion was evaluated |
| `actual` | `f32` | Actual value observed |
| `threshold` | `f32` | Configured threshold |
| `passed` | `bool` | Whether criterion passed |
| `action` | `SlaAction` | Configured action |
| `subject` | `Option<String>` | Subject if scoped |

---

## CliSummary (Terminal Reporter Output)

**File**: `src/engine/reporting.rs` (internal struct)

| Field | Type | Source |
|---|---|---|
| `duration_secs` | `f64` | `GooseMetrics.duration` |
| `total_users` | `usize` | `GooseMetrics.maximum_users` |
| `total_requests` | `usize` | Sum of `request.success_count + request.fail_count` |
| `total_failures` | `usize` | Sum of `request.fail_count` |
| `avg_latency_ms` | `f64` | Weighted average of `raw_data.total_time / raw_data.counter` |
| `p95_latency_ms` | `usize` | 95th percentile from `raw_data.times` BTreeMap |
| `p99_latency_ms` | `usize` | 99th percentile from `raw_data.times` BTreeMap |
| `requests_per_second` | `f64` | `total_requests / duration_secs` |
| `per_request` | `Vec<RequestRow>` | Per-endpoint breakdown |

---

## State Transitions

### Error Flow
```
Error occurs → PummelError constructed with context → 
  ├── Propagated up via `?` operator
  ├── Logged at ERROR level with full context
  └── Displayed to user as actionable message
```

### Pacing Flow
```
ExecutionPlan.throughput / pacing config
  → PacingEngine calculates delay
  → Inter-request delay inserted
  → Goose ThrottleRequests as secondary limiter
  → Metrics collected
```

### SLA Evaluation Flow
```
Test completes → GooseMetrics available
  → SlaEngine evaluates each SlaCriterion
  → SlaResult per criterion
  → If any action == Stop && breached → halt with SlaViolation error
  → If action == Warn && breached → log warning
  → If action == Continue → record in report
```
