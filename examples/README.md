# bzt-rs Examples

A collection of runnable example configurations for bzt-rs, organized by
feature. All examples work with the built-in mock server.

## Quick Start

Run any example against the mock server:

```bash
cargo run -- examples/01-http-methods.yaml --mock-run
```

Or against a real target:

```bash
cargo run -- examples/00-basic-get.yaml
```

## Examples Index

| #  | File | Features |
|----|------|----------|
| 00 | `00-basic-get.yaml` | Minimal smoke test GET |
| 01 | `01-http-methods.yaml` | GET, POST, PUT, DELETE, PATCH, HEAD |
| 02 | `02-post-with-body.yaml` | POST with inline JSON body, per-request headers |
| 03 | `03-body-file.yaml` | POST with body loaded from `payloads/` |
| 04 | `04-headers.yaml` | Scenario-level + per-request headers |
| 05 | `05-csv-parameterization.yaml` | CSV data sources with `${var}` interpolation |
| 06 | `06-pacing.yaml` | Rate limiting (5 req/s, randomized) |
| 07 | `07-sla.yaml` | SLA thresholds (fail-rate, avg/p95 latency) |
| 08 | `08-labels-timeouts.yaml` | Custom labels and per-request timeouts |
| 09 | `09-env-vars.yaml` | `${env.VAR}` environment variable injection |
| 10 | `10-extraction.yaml` | JSONPath + regex extraction, chained variables |
| 11 | `11-assertions.yaml` | HTTP status, body content, regex, negative assertions |
| 12 | `12-control-flow.yaml` | Conditional execution (`if`) + polling loops (`loop`) |
| 13 | `13-weighted-scenarios.yaml` | Weighted traffic split (80/20) |
| 14 | `14-think-time.yaml` | Think time with fixed + range delays |
| 15 | `15-shorthand.toml` | Ultra-concise TOML shorthand format |
| 16 | `16-multiple-executions.yaml` | Two execution blocks with different scenarios |
| 17 | `17-comprehensive.yaml` | All features combined — full e2e journey |

## Fixture Files

- `examples/payloads/login.json` — sample login request body
- `examples/payloads/order.json` — sample order request body
- `examples/data/users.csv` — 4 users (username,password,role)
- `examples/data/products.csv` — 5 products (id,product,price,in_stock)

## Tips

- Paths in configs are relative to the project root (where `cargo run` is executed).
- Use `--dry-run` to validate any config before running:
  ```bash
  cargo run -- examples/05-csv-parameterization.yaml --dry-run
  ```
- Mock server logs requests and responses at INFO level:
  ```
  [MOCK] Request: POST /api/login
  [MOCK] Response: POST /api/login -> 200 body="token session"
  ```
- The mock server binds to `127.0.0.1` only.
- Unknown config fields are rejected at deserialization (typos → errors).
- Sensitive env vars (`AWS_*`, `SECRET*`, `*_PASSWORD`, etc.) are masked
  as `[REDACTED]` in logs.
