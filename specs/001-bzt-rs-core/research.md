# Research: bzt-rs Core Architecture

## Decisions & Rationale

### 1. Multi-Format Parsing (YAML/JSON/TOML)
- **Decision**: Use `serde` as the core serialization framework, combined with `serde_yaml`, `serde_json`, and `toml-rs`.
- **Rationale**: Rust's ecosystem is heavily built around `serde`, making it the most robust choice for cross-format support with minimal boilerplate. This allows us to deserialize all three formats into a single internal AST or "Normal Form" effortlessly.
- **Alternatives considered**: Manual parsing using `nom` or similar (rejected as too complex for configuration schemas), using language-specific wrappers like `pyo3` (rejected for initial core to keep binary size small).

### 2. Taurus vs. Shorthand Unification
- **Decision**: Define a "Normal Form" (AST) that is a superset of the core Taurus configuration needed by Goose.
- **Rationale**: By normalizing all inputs (YAML/TOML/JSON/Shorthand) into a single internal representation, we decouple the parser from the execution engine. This ensures that the engine only needs to know how to translate the AST into Goose tasks.
- **Alternatives considered**: Direct translation from each format to Goose (rejected as non-scalable).

### 3. Dynamic Task Generation in Goose
- **Decision**: Use Goose's programmatic API to dynamically register scenarios and tasks at runtime based on the AST.
- **Rationale**: This allows `bzt-rs` to be a generic runner that doesn't need to be recompiled for different tests. We will map Taurus `execution` blocks to Goose `GooseAttack` parameters and `scenario` blocks to `GooseScenario` builders.
- **Alternatives considered**: Codegen (rejected as non-dynamic).

### 4. Metrics & Observability
- **Decision**: Initial output will leverage Goose's native HTML and CLI reporting.
- **Rationale**: Aligns with Principle XI (Simplicity/YAGNI). We'll build on top of Goose's solid base before adding custom Prometheus/BlazeMeter exporters in later phases.
