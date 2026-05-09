bzt-rs: High-Performance Taurus Runner in Rust
==============================================

.. toctree::
   :maxdepth: 2
   :caption: Contents:

   introduction
   installation
   configuration
   advanced_features
   distributed_mode
   example
   api_reference

Introduction
------------
bzt-rs is a comprehensive execution engine that interprets industry-standard Taurus
configurations and a simplified native shorthand, executing them using the
high-performance, Rust-based Goose framework.

Key Features
------------

* **Multi-Format Parsing**: Deserializes configuration files (.yaml, .json, .toml).

* **Schema Normalizer**: Unifies Shorthand and Taurus schemas.

* **State Translator**: Maps execution bounds to Goose configurations.

* **Dynamic Task Engine**: Evaluates dynamic data, macros, and control flow.

* **Structured Error Handling**: Typed ``BztError`` with context, file paths,
  and source chaining for actionable diagnostics.

* **Security Hardening**: Path traversal prevention, 100MB file size limits,
  sensitive env var masking, ``deny_unknown_fields`` on config structs.

* **Environment & Pacing**: ``${env.VAR}`` resolution with priority chain,
  fixed-rate and randomized throughput enforcement.

* **SLA Evaluation**: Pass/fail criteria with Stop/Warn/Continue actions
  (fail-rate, average response time, p90/p95/p99, throughput).

* **CLI Reporter**: Post-test ASCII summary table with per-endpoint metrics.

* **Reporting**: JUnit XML output for CI/CD integration.

Indices and tables
==================


* :ref:`genindex`

* :ref:`modindex`

* :ref:`search`
