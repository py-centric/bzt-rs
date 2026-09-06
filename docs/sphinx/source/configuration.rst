Configuration
=============

pummel provides a flexible configuration system that supports standard
Taurus YAML and a simplified native Shorthand syntax available in YAML,
JSON, or TOML.

Format Support
--------------

* **YAML**: Primary format for Taurus-compatible configurations (.yaml, .yml).

* **JSON**: Full support for both standard and shorthand schemas (.json).

* **TOML**: Native shorthand syntax optimized for readability (.toml).

Schema Normalization
--------------------
Regardless of the input format, pummel unifies all configurations into a
canonical internal representation. This means you can mix and match
standard Taurus structures with simplified shorthand where appropriate.

Core Configuration Parameters
-----------------------------
The configuration consists of two primary sections: ``execution`` and ``scenarios``.

### The ``execution`` Block
Defines "how" the load should be generated:

* **concurrency**: The number of concurrent users (threads) to launch.

* **ramp-up**: Time to reach target concurrency (e.g., ``10s``, ``1m``).

* **hold-for**: Total duration of the test (e.g., ``5m``, ``1h``).

* **scenario**: Name of the scenario to execute from the ``scenarios`` block.

* **throughput**: Target requests per second (applied as a Goose throttle).

* **pacing**: Structured pacing configuration (see Pacing below).

### The ``scenarios`` Block
Defines "what" the load should do:

* **requests**: A list of URLs or detailed request objects.

* **data-sources**: CSV files or structured data source definitions for
  parameterization.

* **variables**: Static or dynamic context for the scenario.

* **think-time**: Delay between requests (e.g., ``100ms``, ``1s-3s``).

* **headers**: Scenario-level HTTP headers applied to all requests.

### Pacing Configuration
Control the rate at which requests are sent:

.. code-block:: yaml

   execution:
   - concurrency: 10
     scenario: paced-test
     pacing:
       rate: 50           # Target requests per period
       per: 1s            # Time period (default: 1s)
       randomize: true    # Randomize inter-request timing

* **rate** (required): Target number of requests per time period.
* **per** (optional): Time period string (e.g., ``1s``, ``1m``). Default: ``1s``.
* **randomize** (optional): When ``true``, uses exponential distribution for
  more realistic traffic patterns. Default: ``false``.

### Data Source Definitions
Data sources can be specified as a simple path string or a structured object:

.. code-block:: yaml

   scenarios:
     data-test:
       data-sources:
         - users.csv               # Simple path (comma-delimited, quoted)
          - path: products.csv      # Structured definition
            delimiter: ";"
            quoted: true
            loop_data: true
            ordered: false

Structured fields:

* **path** (required): File path to the CSV data source.
* **delimiter** (optional): CSV delimiter character. Default: ``,``.
* **quoted** (optional): Whether fields are quoted. Default: ``true``.
* **loop_data** (optional): Loop back to start when reaching end. Default: ``true``.
* **ordered** (optional): Select records in order (false = by user index). Default: ``false``.

### Taurus YAML (Standard)
A complete, standard Taurus YAML configuration example:

.. code-block:: yaml

   execution:
   - concurrency: 10
     ramp-up: 30s
     hold-for: 5m
     scenario: standard-test

   scenarios:
     standard-test:
       requests:
       - url: http://example.com/api/login
         method: POST
         label: login
         body: '{"username": "admin"}'
         timeout: 30s
       - url: http://example.com/api/profile

### Detailed Request Fields
Each request object supports these fields:

.. code-block:: yaml

   - url: /api/endpoint
     method: GET                  # GET, POST, PUT, DELETE, PATCH, HEAD
     label: my-request            # Human-readable name for metrics
     headers:                     # Per-request headers
       X-Custom: value
     body: '{"key": "value"}'     # Inline request body
     body-file: payload.json      # Read body from file
     timeout: 30s                 # Request timeout (e.g., 30s, 5000ms)
     think-time: 100ms            # Delay after this request
     if: "${var}" != ""           # Conditional execution
     loop: "${status}" == "ok"    # Polling loop condition

### Native Shorthand (TOML)
A more compact shorthand syntax using TOML:

.. code-block:: toml

   [execution]
   concurrency = 10
   ramp-up = "30s"
   hold-for = "5m"
   scenario = "shorthand-test"

   [scenarios.shorthand-test]
   requests = [
       "http://example.com/api/login",
       "http://example.com/api/profile"
   ]

Advanced Configuration
----------------------
See the :doc:`advanced_features` section for details on extraction,
assertions, control flow, SLA criteria, and security features.
