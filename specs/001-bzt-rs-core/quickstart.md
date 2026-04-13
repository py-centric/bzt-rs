# Quickstart: bzt-rs Core

## Running Your First Load Test

`bzt-rs` allows you to run performance tests using standard Taurus configuration files or a simplified shorthand syntax.

### 1. Basic Taurus YAML (`test.yaml`)

Create a file named `test.yaml`:

```yaml
execution:
- concurrency: 5
  ramp-up: 10s
  hold-for: 1m
  scenario: sample

scenarios:
  sample:
    requests:
    - http://localhost:8080/
    - http://localhost:8080/api/status
```

Run it:
```bash
./bzt-rs test.yaml
```

### 2. Shorthand TOML (`test.toml`)

`bzt-rs` also supports a minimal "Shorthand" format in TOML:

```toml
[execution]
concurrency = 5
ramp-up = "10s"
hold-for = "1m"
scenario = "sample"

[scenarios.sample]
requests = [
    "http://localhost:8080/",
    "http://localhost:8080/api/status"
]
```

Run it:
```bash
./bzt-rs test.toml
```

### 3. CLI Options

- `-v`, `--verbose`: Enable detailed logging (INFO level).
- `-vv`, `--debug`: Enable pedantic logging (DEBUG level).
- `--report`: Path to the output HTML report (e.g., `--report my_report.html`).
- `--help`: Show all available options.

### 4. Configuration Detection

`bzt-rs` automatically detects the configuration format based on the file extension (`.yaml`, `.yml`, `.json`, `.toml`).
