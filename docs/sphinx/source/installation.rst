Installation
============

bzt-rs is designed as a standalone binary with zero runtime dependencies. 
This makes it incredibly easy to distribute across different environments.

System Prerequisites
--------------------
Before building bzt-rs, ensure your system meets the following requirements:
* **Operating System**: Linux (primary support), macOS, or Windows.
* **Rust Toolchain**: Rust 2024 Edition or later is required. 
  Install it via `rustup <https://rustup.rs/>`_.
* **Build Tools**: A standard C/C++ toolchain (e.g., `build-essential` 
  on Ubuntu) for compiling dependencies like OpenSSL (if enabled).

Building from Source
--------------------
To build bzt-rs from source, follow these steps:

1. **Clone the Repository**:
   .. code-block:: bash

      git clone https://github.com/v/bzt-rs.git
      cd bzt-rs

2. **Run a Full Build**:
   .. code-block:: bash

      cargo build --release

3. **Verify the Installation**:
   The binary is generated at `target/release/bzt-rs`. Check the version:
   .. code-block:: bash

      ./target/release/bzt-rs --version

Optimizing the Build
--------------------
For maximum performance during load generation, use the following build options:
* **LTO (Link Time Optimization)**: Enable this in `Cargo.toml` for 
  further runtime performance gains.
* **Target-Specific Optimization**:
  .. code-block:: bash

     RUSTFLAGS="-C target-cpu=native" cargo build --release

Docker Installation
-------------------
If you prefer to run bzt-rs in a containerized environment:

.. code-block:: bash

   docker build -t bzt-rs .
   docker run --rm bzt-rs --help

Common Build Issues
-------------------
* **SSL/TLS Dependencies**: If the build fails on `openssl-sys`, ensure 
  `pkg-config` and `libssl-dev` (or equivalent) are installed on your 
  system.
* **Toolchain Version**: If you encounter errors related to Rust features, 
  update your toolchain: `rustup update stable`.
