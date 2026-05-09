# bzt-rs

**bzt-rs** is a high-performance, Rust-native load generator that bridges the simplicity of [Taurus](https://gettaurus.org/) YAML configurations with the extreme performance of the [Goose](https://goose.rs/) attack engine.

Built for modern SRE and DevOps workflows, `bzt-rs` allows you to define complex load tests in human-readable formats and execute them with minimal resource overhead.

## Key Features

- **Multi-Format Support**: Native parsing for **YAML**, **JSON**, and **TOML**.
- **Simplified Shorthand**: Use a ultra-concise TOML-based shorthand for quick tests.
- **Smart Validation**: Use `--dry-run` to verify configurations and file dependencies before execution.
- **Integrated Mocking**: Spin up an assertion-based HTTP **Mock Server** on `127.0.0.1` for functional verification.
- **Dynamic Data**: Environment variable injection (with sensitive var masking), CSV data sources, and macro-based fake data generation.
- **Advanced Control Flow**: Weighted branching, conditional execution, polling loops, and setup/teardown tasks.
- **Structured Errors**: Typed `BztError` with `[CATEGORY]` format, file paths, and source chaining for actionable diagnostics.
- **Pacing & Rate Limits**: Fixed-rate or randomized throughput enforcement.
- **SLA Evaluation**: Pass/fail criteria (fail-rate, response time percentiles, throughput) with Stop/Warn/Continue actions.
- **Rich Reporting**: Post-test CLI summary table (per-endpoint metrics with p95/p99), JUnit XML for CI/CD.
- **Security Hardening**: Path traversal prevention, 100MB file size limits, sensitive env var masking (`[REDACTED]`), `deny_unknown_fields` on config structs, localhost-only mock bind.

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

> **Tip:** Errors use a structured `[CATEGORY]` format (e.g., `[CONFIG]` file not found, `[NETWORK]` connection refused). Config typos are caught — 7 config structs use `#[serde(deny_unknown_fields)]`, so misspelled YAML/JSON/TOML keys cause a deserialization error with the file path and offending key shown.

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

> **Note:** Run `--help` to see all CLI flags. Only 4 exist: `<CONFIG>`, `-d`/`--dry-run`, `-m`/`--mock`, and `--mock-run`. Flags removed from Taurus (`--manager`, `--worker`, `--metrics`, `--otel`) are **rejected** with an "unrecognized option" error — they are not silently ignored.

### CLI Arguments Summary

| Argument | Description |
|----------|-------------|
| `<CONFIG>` | Path to `.yaml`, `.json`, or `.toml` configuration. |
| `-d`, `--dry-run` | Validate configuration and dependencies without execution. |
| `-m`, `--mock` | Start an internal HTTP mock server based on the config. |
| `--mock-run` | Run the test configuration against an internal mock server. |

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

Sensitive vars (`AWS_*`, `SECRET*`, `*_TOKEN`, `*_PASSWORD`, `DATABASE_URL`) are blocked and masked as `[REDACTED]` in logs.

Additional security: file paths are canonicalized and prefix-checked to prevent traversal attacks, data source files are capped at 100 MB, and the mock server binds to `127.0.0.1` only. Security events are logged at WARN level with a `[SECURITY]` prefix.

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

## Architecture

`bzt-rs` follows a modular pipeline:

1. **Parser**: Deserializes multi-format configs into a core AST.
2. **Normalizer**: Unifies different schemas (Taurus/Shorthand).
3. **Validator**: Dry-run checks for configuration and file dependencies.
4. **Security Checks**: Path traversal prevention, file size limits, env var masking.
5. **Environment Loader**: `${env.VAR}` resolution with priority chain.
6. **Pacing Engine**: Fixed-rate and randomized throughput enforcement.
7. **Translator**: Maps the AST to dynamic Goose scenarios and tasks. Supports all standard HTTP methods (GET, POST, PUT, DELETE, PATCH, HEAD).
8. **Goose Engine**: Executes the high-concurrency attack.
9. **SLA Engine**: Evaluates pass/fail criteria with action dispatch.
10. **CLI Reporter**: Post-test ASCII summary table with per-endpoint metrics.
11. **JUnit Reporter**: XML report generation for CI/CD integration.

## Next Steps

- [Architecture](docs/architecture.md) — detailed pipeline diagram and component design
- [Sphinx Docs](docs/sphinx/source/index.rst) — full config reference, advanced features


## License

Apache License 2.0.
