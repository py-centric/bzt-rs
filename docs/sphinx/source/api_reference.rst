API Reference
=============

CLI Arguments
-------------

.. code-block:: text

   Usage: pummel [OPTIONS] <CONFIG>

   Arguments:
     <CONFIG>  Path to the configuration file (.yaml, .json, .toml)

   Options:
     -d, --dry-run   Validate the configuration and dependencies without
                     executing the test
     -m, --mock      Start an internal HTTP mock server based on the
                     configuration
         --mock-run  Start the mock server and run the configuration against it
     -h, --help      Print help
     -V, --version   Print version

The following Taurus CLI flags are **not yet implemented** and have been
removed from the argument parser: ``--manager``, ``--worker``,
``--expect-workers``, ``--manager-host``, ``--manager-port``, ``--metrics``,
``--metrics-port``, ``--otel``.
