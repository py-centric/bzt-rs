# bzt-rs Architecture

```mermaid
graph TD
    CLI[CLI main.rs] --> Parser[Multi-Format Parser]
    Parser --> AST[Configuration AST]
    AST --> Normalizer[Schema Normalizer]
    Normalizer --> UnifiedAST[Unified Configuration]
    UnifiedAST --> Translator[State Translator]
    Translator --> Goose[Goose Load Engine]
    Goose --> Metrics[Prometheus/Metrics]
```

## Components

1. **Multi-Format Parser**: Deserializes YAML, JSON, and TOML.
2. **Schema Normalizer**: Bridges "Shorthand" and Taurus schemas.
3. **State Translator**: Maps AST to dynamic Goose scenarios and tasks.
4. **Goose Load Engine**: High-performance Rust-based execution.
