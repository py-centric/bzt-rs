Advanced Features
=================

bzt-rs provides robust mechanisms for building complex, realistic
performance tests that mirror real-world user behavior.

Dynamic Data & Macros
---------------------
Inject dynamic, randomized data into your requests using built-in macros:

* **${faker.email}**: Generates a random email address.

* **${faker.name}**: Generates a random full name.

* **${uuid}**: Generates a unique UUID v4.

* **${env.MY_VAR}**: Injects the value of the ``MY_VAR`` environment variable.
  Sensitive variable patterns (``AWS_*``, ``SECRET*``, ``KEY``, ``TOKEN``,
  ``PASSWORD``, ``PRIVATE*``, ``DATABASE_URL``, ``DB_*``) are automatically
  masked in log output as ``[REDACTED]``.

Environment Variable Resolution
--------------------------------
Environment variables are resolved with the following priority chain
(highest to lowest):

1. CLI argument overrides
2. System environment variables
3. ``.env`` file from project root or ``config/settings.env``
4. Default values in configuration

Before test execution, all ``${env.VAR}`` references in the configuration
are scanned for sensitive patterns. A warning is emitted for each match,
but the actual value is still resolved (only log output is masked).

Data Sources (CSV)
------------------
Iterate through external data files for parameterization.

.. code-block:: yaml

   scenarios:
     csv-test:
       data-sources:
       - users.csv
       requests:
       - url: /api/login
         method: POST
         body: '{"user": "${username}", "pass": "${password}"}'

Data sources can be specified as a simple path or a structured definition
with custom delimiter, quoting, and looping behavior (see :doc:`configuration`).

Variable Extraction & Interpolation
------------------------------------
Build stateful user journeys by extracting data from responses:

* **JSONPath Extraction**:

  .. code-block:: yaml

     extract-jsonpath:
       token: $.access_token


* **Regex Extraction**:

  .. code-block:: yaml

     extract-regexp:
       id: "id=([0-9]+)"

* **XPath 2.0 Extraction**:

  .. code-block:: yaml

     extract-xpath:
       item_count: "//inventory/item/@count"

Use extracted variables in any subsequent request field:

.. code-block:: yaml

   requests:
   - url: /api/profile/${token}
     headers:
       X-User-ID: ${id}

Dynamic gRPC (Reflection & Streaming)
-------------------------------------
bzt-rs provides advanced, dynamic support for gRPC without requiring pre-compiled
proto files. By leveraging gRPC Reflection, the engine discovers service schemas
at runtime.

* **Unary gRPC**: Standard request/response calls.
* **Server-side Streaming**: Processes a stream of response messages, executing
  assertions and extraction rules against *every* frame.

.. code-block:: yaml

   scenarios:
     grpc-test:
       requests:
       - url: grpc://localhost:50051
         protocol: grpc
         method-name: my.pkg.Service/GetStream
         grpc-mode: server-streaming
         body: '{"id": "123"}'
         assert:
         - contains: ["STATUS_OK"]

WebSocket Testing
-----------------
Full support for bidirectional text-based WebSockets. You can initiate connections,
send messages, and verify asynchronous responses.

.. code-block:: yaml

   scenarios:
     ws-test:
       requests:
       - url: ws://localhost:8080/chat
         protocol: websocket
         message: "Hello from bzt-rs"
         assert:
         - contains: ["Welcome"]

Real-time InfluxDB Observability
--------------------------------
Ship metrics to InfluxDB in real-time during the test run. Each worker node in
a distributed test is uniquely identified via a ``worker_id`` tag, ensuring
accurate data aggregation in Grafana dashboards.

.. code-block:: yaml

   reporting:
   - module: influxdb
     url: http://influxdb:8086
     bucket: test_metrics
     token: ${env.INFLUX_TOKEN}
     interval: 10s  # Push metrics every 10 seconds

Control Flow: Conditions & Loops
---------------------------------
...
* **Polling Loops (``loop``)**:
  Repeatedly execute a request until a condition is satisfied. Supports numeric
  comparisons and boolean logic.

  .. code-block:: yaml

     requests:
     - url: /api/status
       loop: "${status}" == "processing" || "${count}" < 5

SLA & Pass/Fail Criteria
-------------------------
Define structured success thresholds for your load test:

.. code-block:: yaml

   reporting:
   - module: junit-xml
     filename: results.xml
     failed-threshold: 0.05
     sla:
     - metric: fail-rate
       threshold: 0.05
       action: warn
     - metric: avg-response-time
       threshold: 5000.0
       action: stop
     - metric: p95-response-time
       threshold: 2000.0
       subject: /api/login
       duration: 30s
       action: warn

Supported metrics:

* **fail-rate**: Ratio of failed requests to total (0.0 to 1.0).
* **avg-response-time**: Average response time in milliseconds.
* **p90-response-time**: 90th percentile response time (ms).
* **p95-response-time**: 95th percentile response time (ms).
* **p99-response-time**: 99th percentile response time (ms).
* **throughput**: Requests per second.

Each SLA criterion supports:

* **metric** (required): Which metric to evaluate.
* **threshold** (required): Threshold value for the metric.
* **subject** (optional): Specific request or scenario to scope the criterion.
* **duration** (optional): Evaluation window duration string.
* **action** (optional): Action on breach — ``stop`` (halt test), ``warn``
  (log warning), ``continue`` (record and continue). Default: ``stop``.

Security Features
-----------------
bzt-rs includes several security hardening measures:

* **Path Traversal Prevention**: All file paths (data sources, body files)
  are canonicalized and checked against the project root. Paths containing
  ``../`` or equivalent traversal sequences are rejected with a warning.

* **File Size Limits**: Data source files are checked against a 100MB
  maximum before reading. Files exceeding this limit are rejected.

* **Unknown Field Rejection**: Configuration structs use
  ``#[serde(deny_unknown_fields)]`` to reject unrecognized YAML/JSON/TOML
  fields at deserialization time, preventing silent misconfiguration.

* **Localhost Mock Binding**: The built-in mock server binds to
  ``127.0.0.1`` instead of ``0.0.0.0`` to prevent external access.

* **Sensitive Env Var Masking**: Environment variable values matching
  known sensitive patterns (``AWS_*``, ``SECRET*``, ``KEY``, ``TOKEN``,
  ``PASSWORD``, ``PRIVATE*``, ``DATABASE_URL``, ``DB_*``) are masked with
  ``[REDACTED]`` in all log output.

CLI Reporter
------------
After every test run, bzt-rs outputs a terminal summary table:

.. code-block:: text

   ===== Test Results =====
   Duration: 60.0s    Users: 10
   Total Requests: 600    Failed: 0 (0.00%)
   Avg Latency: 45ms    p95: 120ms    p99: 250ms

     Method  Path              Count    Failed    Avg(ms)    p95(ms)
     ------  ----              -----    ------    -------    -------
     GET     /api/status       300      0         32         85
     POST    /api/login        300      0         58         150

The report includes per-endpoint breakdowns of request count, failure count,
average latency, and p95/p99 latency percentiles.
