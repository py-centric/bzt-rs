Introduction
============

pummel is a high-performance, Rust-based load testing runner that interprets
industry-standard Taurus configurations and a simplified native shorthand,
executing them using the Goose load engine. It is designed to bring the ease
of declarative performance testing to the Rust ecosystem with minimal
resource overhead and maximum reliability.

Core Vision
-----------
The vision for pummel is to provide a zero-dependency, single-binary execution
engine that scales from simple local tests to complex CI/CD pipelines.
By combining the user-friendly YAML/JSON/TOML syntax of Taurus with the
unparalleled performance of Rust and the Goose engine, pummel offers a
modern alternative for performance engineering.

Design Principles
-----------------

* **Library-First Architecture**: The core engine and parsers are implemented
  as self-contained modules, allowing for future embedding in other tools.

* **Declarative Configuration**: Tests are defined in human-readable YAML,
  JSON, or TOML, focusing on "what" to test rather than "how" to implement it.

* **High Performance**: Leveraging Rust's memory safety and zero-cost
  abstractions, pummel maintains a runtime overhead of < 5% compared to native
  Goose implementations.

* **Reliability-First**: Robust error handling with structured error types,
  path traversal prevention, file size limits, and sensitive environment
  variable masking ensure safe operation.

* **Observability-Centric**: Post-test CLI summary reports with per-endpoint
  latency percentiles, SLA evaluation, and JUnit XML output for CI/CD.

System Architecture
-------------------
The pummel system is built around a modular pipeline that processes
configuration files into an executable load test:

1. **Multi-Format Parser**: Deserializes input files (YAML, JSON, TOML) into
   format-specific internal structures.

2. **Schema Normalizer**: Unifies different input formats (including Taurus
   and Shorthand) into a single, canonical Abstract Syntax Tree (AST).

3. **Validation Engine**: Performs "dry-run" checks to verify configuration
   syntax and ensure all external dependencies (e.g., CSV files) are present.

4. **Security Checks**: Validates file paths (traversal prevention), file
   sizes (100MB limit), and environment variable exposure (blocklist masking).

5. **Mock Server**: An assertion-aware HTTP server binding to ``127.0.0.1``
   that dynamically simulates backend responses for isolated functional
   testing of scenarios.

6. **Environment Loader**: Resolves ``${env.VAR}`` references with a priority
   chain (CLI > system env > ``.env`` file > defaults).

7. **Pacing Engine**: Enforces fixed-rate or randomized throughput limits
   between requests.

8. **State Translator**: Maps the internal AST into Goose-native execution
   tasks, scenarios, and transaction sets. Supports all standard HTTP methods
   (GET, POST, PUT, DELETE, PATCH, HEAD).

9. **Goose Engine**: The high-concurrency execution wrapper that performs
   the actual load generation and metric collection.

10. **SLA Engine**: Evaluates pass/fail criteria (fail-rate, response time
    percentiles, throughput) and dispatches Stop/Warn/Continue actions.

11. **CLI Reporter**: Produces a post-test ASCII summary table with per-endpoint
    metrics (requests, failures, average latency, p95/p99).

12. **JUnit Reporter**: Generates XML test reports for CI/CD integration.
