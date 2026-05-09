# bzt-rs Quickstart

From zero to first load test in under 5 minutes.

## Prerequisites

**Rust 2024+ toolchain** — [rustup](https://rustup.rs/)

## Installation

```bash
git clone https://github.com/py-centric/bzt-rs.git
cd bzt-rs
cargo build --release
```

Binary: `./target/release/bzt-rs`.

## Your First Test

Save as `first-test.yaml`:

```yaml
execution:
  - concurrency: 10
    ramp-up: 5s
    hold-for: 30s
    scenario: smoke

scenarios:
  smoke:
    requests:
      - https://jsonplaceholder.typicode.com/posts/1
```

Run it:

```bash
./target/release/bzt-rs first-test.yaml
```

Output:

```
==========================================================================================
  BZT-RS LOAD TEST SUMMARY
==========================================================================================
  Duration: 30s  |  Max Users: 10  |  Total Requests: 1452  |  Failures: 0 (0.0%)
------------------------------------------------------------------------------------------
  Method   Path                                         Requests    Fails    Avg(ms)    p95(ms)    p99(ms)
------------------------------------------------------------------------------------------
  GET      https://jsonplaceholder.typicode.com/posts…      1452        0      187.2      412.0      589.0
------------------------------------------------------------------------------------------
```

Shows per-endpoint **Method**, **Path**, **Requests**, **Failures**, and latency at **Avg/p95/p99**.

## Validation: `--dry-run`

Check config without hitting the target:

```bash
./target/release/bzt-rs first-test.yaml --dry-run
```

Validates parsing, file dependencies (CSV, body files), and Goose translation.

## Mock Server

Start mock on `127.0.0.1`:

```bash
./target/release/bzt-rs test.yaml --mock
```

Start + run in one shot:

```bash
./target/release/bzt-rs test.yaml --mock-run
```

Serves responses from `assert:` criteria — verify assertions in isolation.

## Common Patterns

### Shorthand TOML

```toml
[execution]
concurrency = 5
ramp-up = "0s"
hold-for = "10s"
scenario = "quick"

[scenarios.quick]
requests = [
    "http://localhost:8080/api/health",
    "http://localhost:8080/api/status"
]
```

### POST with Body

```yaml
requests:
  - url: https://api.example.com/login
    method: POST
    label: user_login
    headers:
      Content-Type: application/json
    body: '{"username": "admin", "password": "hunter2"}'
```

Use `body-file: payload.json` to load from disk.

### CSV Data Parameterization

```yaml
scenarios:
  smoke:
    data-sources:
      - users.csv       # columns: id,name,email
    requests:
      - url: "https://api.example.com/user/${id}?name=${name}"
        method: GET
        label: get_user
```

Fields like `${id}`, `${name}` interpolate from CSV rows per-request.

### Environment Variables

```yaml
url: "https://${env.API_HOST}/v1/items"
```

Sensitive vars (`AWS_*`, `*_SECRET*`, `*_PASSWORD`) are blocked and masked as `***`.

### Pacing / Rate Limits

```yaml
pacing:
  rate: 10        # 10 requests
  per: 1s         # per second
  randomize: true # exponential jitter
```

### SLA Thresholds

```yaml
reporting:
  - module: junit-xml
    filename: results.xml
    sla:
      - metric: avg-response-time
        threshold: 500.0
        action: warn
      - metric: fail-rate
        threshold: 0.05
        action: stop
```

Metrics: `fail-rate`, `avg-response-time`, `p90/p95/p99-response-time`, `throughput`. Actions: `stop`, `warn`, `continue`.

### Labels and Timeouts

```yaml
requests:
  - url: https://slow-api.example.com/data
    label: slow_endpoint
    timeout: 30s
```

Labels appear in the CLI summary. Timeouts default to Goose's internal default.

## Next Steps

- [Architecture](architecture.md) — pipeline flow and component design
- [README](../README.md) — CLI reference
- [Sphinx Docs](sphinx/source/index.rst) — full config reference, advanced features
