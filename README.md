# bzt-rs (TLDR)

**bzt-rs** is a high-performance, Rust-based load generator that executes Taurus YAML and native shorthand configurations using the Goose engine.

## Quick Start

1. **Build**: `cargo build --release`
2. **Run**: `./target/release/bzt-rs test.yaml`

### Minimal Example (`test.yaml`)
```yaml
execution: [{concurrency: 5, ramp-up: 10s, hold-for: 1m, scenario: simple}]
scenarios:
  simple:
    requests: ["http://localhost:8080/"]
```

## Key Capabilities
- **Formats**: YAML, JSON, TOML (Shorthand supported).
- **Features**: State extraction, variable interpolation, TDD-first logic.
- **Scale**: Native distributed Manager/Worker mode.
- **Observability**: Prometheus & OpenTelemetry ready.

## Full Documentation
For detailed guides, examples, and advanced features, see the [Sphinx Documentation](docs/sphinx/source/index.rst).

## License
Apache License 2.0.
