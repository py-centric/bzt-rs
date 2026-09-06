# Library API Contract: PummelError

## Public Types

### `PummelError` (enum)

The unified error type for all pummel operations. All public API functions return `Result<T, PummelError>`.

```rust
pub enum PummelError {
    Io { source: std::io::Error, context: String },
    Serde { message: String, file_path: Option<String> },
    Goose { source: Box<dyn std::error::Error + Send> },
    Validation { field: String, reason: String },
    Mock { reason: String },
    Network { host: String, operation: String, details: String },
    SlaViolation { actual: f32, threshold: f32, metric: String },
    Environment { var: String, reason: String },
    NotImplemented { feature: String },
}
```

### Error Display Format

Each variant displays in the format: `[CATEGORY] message`

Examples:
- `[IO] Failed to read data source 'users.csv': No such file or directory`
- `[VALIDATION] Field 'concurrency' must be > 0`
- `[SLA] Fail rate 0.12 exceeds threshold 0.05 for metric 'fail-rate'`
- `[NOT IMPLEMENTED] Distributed mode (--manager) is not yet implemented`

## `#[from]` Implementations

- `std::io::Error` → `PummelError::Io`
- `serde_json::Error` → `PummelError::Serde`
- `serde_yaml::Error` → `PummelError::Serde`
- `toml::de::Error` → `PummelError::Serde`
