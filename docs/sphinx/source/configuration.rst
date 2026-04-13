Configuration
=============

bzt-rs provides a flexible configuration system that supports standard 
Taurus YAML and a simplified native Shorthand syntax available in YAML, 
JSON, or TOML.

Format Support
--------------
* **YAML**: Primary format for Taurus-compatible configurations (.yaml, .yml).
* **JSON**: Full support for both standard and shorthand schemas (.json).
* **TOML**: Native shorthand syntax optimized for readability (.toml).

Schema Normalization
--------------------
Regardless of the input format, bzt-rs unifies all configurations into a 
canonical internal representation. This means you can mix and match 
standard Taurus structures with simplified shorthand where appropriate.

Core Configuration Parameters
-----------------------------
The configuration consists of two primary sections: `execution` and `scenarios`.

### The `execution` Block
Defines "how" the load should be generated:
* **concurrency**: The number of concurrent users (threads) to launch.
* **ramp-up**: Time to reach target concurrency (e.g., `10s`, `1m`).
* **hold-for**: Total duration of the test (e.g., `5m`, `1h`).
* **iterations**: Number of times each user should execute the scenario.
* **scenario**: Name of the scenario to execute from the `scenarios` block.

### The `scenarios` Block
Defines "what" the load should do:
* **requests**: A list of URLs or request objects.
* **data-sources**: CSV files for parameterization.
* **variables**: Static or dynamic context for the scenario.
* **think-time**: Delay between requests.

Taurus YAML (Standard)
----------------------
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
         body: '{"username": "admin"}'
       - url: http://example.com/api/profile

Native Shorthand (TOML)
-----------------------
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
assertions, and control flow.
