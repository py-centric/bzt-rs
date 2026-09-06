pummel: High-Performance Taurus Runner in Rust
==============================================

.. toctree::
   :maxdepth: 2
   :caption: Contents:

   introduction
   installation
   configuration
   advanced_features
   chaos_engineering
   distributed_mode
   example
   api_reference

Introduction
------------
pummel is a comprehensive execution engine that interprets industry-standard Taurus
configurations and a simplified native shorthand, executing them using the
high-performance, Rust-based Goose framework.

Key Features
------------

* **Multi-Protocol Support**: Native support for HTTP/S, WebSockets, and dynamic gRPC.

* **Dynamic gRPC Reflection**: Service discovery at runtime without pre-compiled protos.

* **Real-time Observability**: Metrics shipping to InfluxDB with worker-level synchronization.

* **Multi-Format Parsing**: Deserializes configuration files (.yaml, .json, .toml).

* **Schema Normalizer**: Unifies Shorthand and Taurus schemas.

* **State Translator**: Maps execution bounds to Goose configurations.

* **Dynamic Task Engine**: Evaluates dynamic data, macros, and control flow.

* **Advanced Extraction**: Captured variables using JSONPath, Regex, and full XPath 2.0.

* **Structured Error Handling**: Typed ``PummelError`` with context, file paths,
  and source chaining for actionable diagnostics.

* **Security Hardening**: Path traversal prevention, 100MB file size limits,
  sensitive env var masking, ``deny_unknown_fields`` on config structs.

* **Environment & Pacing**: ``${env.VAR}`` resolution with priority chain,
  fixed-rate and randomized throughput enforcement.

* **SLA Evaluation**: Pass/fail criteria with Stop/Warn/Continue actions
  (fail-rate, average response time, p90/p95/p99, throughput).

* **Chaos Engineering Support**: Native lifecycle hooks (``services`` block)
  for chaos injection/cleanup with guaranteed shutdown execution.

* **Real-Time SLA Actions**: Background SLA evaluation every 1s with custom
  ``exec:`` shell commands on breach for automated rollback.

* **Dynamic Control API**: Optional HTTP API (Axum) exposing live metrics
  via ``GET /metrics`` and graceful abort via ``POST /control/stop``.

* **CLI Reporter**: Post-test ASCII summary table with per-endpoint metrics.

* **Reporting**: JUnit XML, HTML, and Real-time InfluxDB output.

Indices and tables
==================


* :ref:`genindex`

* :ref:`modindex`

* :ref:`search`
