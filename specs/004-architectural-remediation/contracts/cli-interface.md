# CLI Contract: pummel

## Usage

```
pummel [OPTIONS] <CONFIG>
```

## Arguments

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | `CONFIG` | Yes | Path to configuration file (.yaml, .yml, .json, .toml) |

## Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-d`, `--dry-run` | bool | false | Validate configuration and dependencies without executing the test |
| `-m`, `--mock` | bool | false | Start an internal HTTP mock server based on the configuration |
| `--mock-run` | bool | false | Start the mock server and run the configuration against it |
| `-h`, `--help` | — | — | Print help information |
| `-V`, `--version` | — | — | Print version information |

### Removed Flags (from previous version)

The following flags were documented as "not yet implemented" and have been removed to avoid false expectations:

| Removed Flag | Reason |
|---|---|
| `--manager` | Distributed execution not yet implemented |
| `--worker` | Distributed execution not yet implemented |
| `--expect-workers` | Distributed execution not yet implemented |
| `--manager-host` | Distributed execution not yet implemented |
| `--manager-port` | Distributed execution not yet implemented |
| `--metrics` | Prometheus metrics not yet implemented |
| `--metrics-port` | Prometheus metrics not yet implemented |
| `--otel` | OpenTelemetry tracing not yet implemented |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (test passed or dry-ran valid) |
| 1 | Validation error, test failure, or SLA breach |

## Output Protocol

- **stdout**: CLI summary table after test execution; validation report for dry-run; mock server address
- **stderr**: Warnings, errors, tracing/log output
