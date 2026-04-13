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
* **Metrics Exporter**: Pushes data to Prometheus, BlazeMeter, and JUnit XML.
* **Cluster Coordinator**: Manages distributed load generation.

Indices and tables
==================

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`
