Configuration
=============

bzt-rs supports standard Taurus YAML and a simplified Shorthand syntax in TOML or JSON.

Taurus YAML
-----------
Standard Taurus configurations are fully supported.

.. code-block:: yaml

   execution:
   - concurrency: 10
     ramp-up: 1m
     hold-for: 5m
     scenario: my-scenario

   scenarios:
     my-scenario:
       requests:
       - http://example.com/

Shorthand Syntax
----------------
For rapid test creation, a simplified syntax is available.

.. code-block:: toml

   [execution]
   concurrency = 10
   ramp-up = "1m"
   hold-for = "5m"
   scenario = "my-scenario"

   [scenarios.my-scenario]
   requests = ["http://example.com/"]
