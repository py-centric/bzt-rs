API Reference
=============

CLI Arguments
-------------

.. code-block:: text

   Usage: bzt-rs [OPTIONS] <CONFIG>

   Arguments:
     <CONFIG>  Path to the configuration file (.yaml, .json, .toml)

   Options:
         --manager                          Start in Manager mode
         --worker                           Start in Worker mode
         --expect-workers <EXPECT_WORKERS>  Number of workers to expect (Manager mode only)
         --manager-host <MANAGER_HOST>      Host the Manager is listening on (Worker mode only)
         --manager-port <MANAGER_PORT>      Port the Manager is listening on
         --metrics                          Enable Prometheus metrics exporter
         --metrics-port <METRICS_PORT>      Port for the Prometheus metrics exporter [default: 8080]
         --otel                             Enable OpenTelemetry tracing injection
     -h, --help                             Print help
     -V, --version                          Print version
