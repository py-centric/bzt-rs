Introduction
============

bzt-rs is a high-performance, Rust-based load testing runner that interprets
industry-standard Taurus configurations and a simplified native shorthand,
executing them using the Goose load engine. It is designed to bring the ease
of declarative performance testing to the Rust ecosystem with minimal
resource overhead and maximum reliability.

Core Vision
-----------
The vision for bzt-rs is to provide a zero-dependency, single-binary execution
engine that can scale from simple local tests to massive distributed clusters.
By combining the user-friendly YAML/JSON/TOML syntax of Taurus with the
unparalleled performance of Rust and the Goose engine, bzt-rs offers a
modern alternative for performance engineering.

Design Principles
-----------------

* **Library-First Architecture**: The core engine and parsers are implemented
  as self-contained modules, allowing for future embedding in other tools.

* **Declarative Configuration**: Tests are defined in human-readable YAML,
  JSON, or TOML, focusing on "what" to test rather than "how" to implement it.

* **High Performance**: Leveraging Rust's memory safety and zero-cost
  abstractions, bzt-rs maintains a runtime overhead of < 5% compared to native
  Goose implementations.

* **Scalability by Default**: Built-in support for distributed Manager/Worker
  modes for large-scale load generation.

* **Observability-Centric**: Deep integration with Prometheus and
  OpenTelemetry for real-time monitoring and tracing.

System Architecture
-------------------
The bzt-rs system is built around a modular pipeline that processes
configuration files into an executable load test:


1. **Multi-Format Parser**: Deserializes input files (YAML, JSON, TOML) into
   format-specific internal structures.

2. **Schema Normalizer**: Unifies different input formats (including Taurus
   and Shorthand) into a single, canonical Abstract Syntax Tree (AST).

3. **Validation Engine**: Performs "dry-run" checks to verify configuration
   syntax and ensure all external dependencies (e.g., CSV files) are present.

4. **Mock Server**: An assertion-aware HTTP server that dynamically simulates
   backend responses for isolated functional testing of scenarios.

5. **State Translator**: Maps the internal AST into Goose-native execution
   tasks, scenarios, and transaction sets.

6. **Goose Engine**: The high-concurrency execution wrapper that performs
   the actual load generation and metric collection.

7. **Reporting & Observability**: Post-processes metrics into various
   formats including JUnit XML, Prometheus metrics, and console summaries.
