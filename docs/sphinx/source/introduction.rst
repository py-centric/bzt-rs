Introduction
============

bzt-rs is designed to bring the ease of Taurus load testing to the Rust ecosystem. 
By leveraging the high-performance Goose load engine, bzt-rs can generate massive 
amounts of traffic with minimal resource overhead.

Purpose
-------
The purpose of bzt-rs is to provide a comprehensive execution engine that interprets 
standard performance testing configurations and executes them at scale.

Architecture
------------
The system consists of several primary components:
* **Multi-Format Parser**: Handles YAML, JSON, and TOML.
* **Normalizer**: Unifies different input formats into a single internal AST.
* **Translator**: Converts the AST into Goose tasks and scenarios.
* **Goose Engine**: The underlying executor for load generation.
