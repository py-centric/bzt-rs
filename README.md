# bzt-rs

**bzt-rs** is a high-performance, Rust-native load generator that bridges the simplicity of [Taurus](https://gettaurus.org/) YAML configurations with the extreme performance of the [Goose](https://goose.rs/) attack engine.

Built for modern SRE and DevOps workflows, `bzt-rs` allows you to define complex load tests in human-readable formats and execute them with minimal resource overhead.

---

## Key Features

- **Multi-Format Support**: Native parsing for **YAML**, **JSON**, and **TOML**.
- **Simplified Shorthand**: Use a ultra-concise TOML-based shorthand for quick tests.
- **Smart Validation**: Use `--dry-run` to verify configurations and file dependencies before execution.
- **Integrated Mocking**: Spin up an assertion-based HTTP **Mock Server** based on your test configuration for functional verification.
- **Dynamic Data**: Support for environment variable injection, CSV data sources, and macro-based fake data generation.
- **Advanced Control Flow**: Support for hierarchical scenarios, weighted branching, and setup/teardown tasks.
- **Rich Reporting**: Generates CLI summaries, interactive HTML reports, and JUnit-compatible XML.

---

## Installation

Ensure you have [Rust 2024+](https://www.rust-lang.org/tools/install) installed.

```bash
# Clone the repository
git clone https://github.com/py-centric/bzt-rs.git
cd bzt-rs

# Build the release binary
cargo build --release
```

The binary will be available at `./target/release/bzt-rs`.

---

## Usage

### Basic Execution
Run a standard Taurus YAML file:
```bash
./target/release/bzt-rs test.yaml
```

### Validation & Mocking
**Dry Run** (validate config only):
```bash
./target/release/bzt-rs test.yaml --dry-run
```

**Start Mock Server**:
```bash
./target/release/bzt-rs test.yaml --mock
```

**Run against Mock Server**:
```bash
./target/release/bzt-rs test.yaml --mock-run
```

### CLI Arguments Summary

| Argument | Description |
|----------|-------------|
| `<CONFIG>` | Path to `.yaml`, `.json`, or `.toml` configuration. |
| `-d`, `--dry-run` | Validate configuration and dependencies without execution. |
| `-m`, `--mock` | Start an internal HTTP mock server based on the config. |
| `--mock-run` | Run the test configuration against an internal mock server. |

---

## Configuration Examples

### Taurus YAML (`test.yaml`)
```yaml
execution:
  - concurrency: 50
    ramp-up: 30s
    hold-for: 5m
    scenario: web-app

scenarios:
  web-app:
    requests:
      - http://api.example.com/v1/status
      - url: http://api.example.com/v1/user
        method: POST
        body: '{"name": "${USER_NAME}"}'
```

### Shorthand TOML (`quick.toml`)
```toml
concurrency = 10
hold-for = "1m"
url = "http://localhost:8080/"
```

---

## Architecture

`bzt-rs` follows a modular pipeline:
1. **Parser**: Deserializes multi-format configs into a core AST.
2. **Normalizer**: Unifies different schemas (Taurus/Shorthand).
3. **Translator**: Maps the AST to dynamic Goose scenarios and tasks.
4. **Goose Engine**: Executes the high-concurrency attack.

---

## Documentation

For detailed guides, API references, and advanced configuration examples, please see the [Sphinx Documentation](docs/sphinx/source/index.rst).

## License

Apache License 2.0.
