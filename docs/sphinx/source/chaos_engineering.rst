Chaos Engineering
=================

bzt-rs natively supports chaos engineering workflows by combining lifecycle
hooks, real-time SLA breach actions, and a dynamic HTTP control API. Together,
these features let you inject faults into a running system, monitor the impact
against service-level objectives, and automatically trigger rollback or
remediation logic — all from a single YAML configuration file.

Architecture Overview
---------------------
The following diagram illustrates how the chaos engineering subsystems
interact during a test run:

.. code-block:: text

   ┌──────────────────────────────────────────────────────────────────┐
   │  CLI (bzt-rs config.yml)                                        │
   │                                                                  │
   │   1. Parse config & resolve env vars                             │
   │   2. Execute PREPARE hooks (provision proxies, start sidecars)   │
   │           │                                                      │
   │           ▼                                                      │
   │   3. Execute STARTUP hooks (inject faults / toxics)              │
   │           │                                                      │
   │           ▼                                                      │
   │   4. Launch Goose Engine ───────────────────────────┐            │
   │      (virtual users execute scenarios)              │            │
   │           │                                         │            │
   │           ▼                                         ▼            │
   │   5. Background SLA Checker              6. Control API          │
   │      (evaluates rules every 1s)             GET /metrics         │
   │      breach → exec: command                 POST /control/stop   │
   │           │                                         │            │
   │           ▼                                         │            │
   │   7. Execute SHUTDOWN hooks (tear down proxies) ◄───┘            │
   │      (guaranteed even on crash / abort)                          │
   └──────────────────────────────────────────────────────────────────┘

Shell Hook Services
-------------------
The ``services`` configuration block lets you run arbitrary shell commands
at well-defined lifecycle phases of the test run:

* **prepare**: Runs *before* the load engine starts. Use this to provision
  infrastructure such as proxy instances, chaos agents, or database fixtures.

* **startup**: Runs *after* prepare completes but *before* virtual users begin
  executing scenarios. Use this to inject faults, add toxic rules, or flip
  feature flags.

* **shutdown**: Runs *after* the test completes or is aborted. Shutdown hooks
  are **guaranteed to execute** even when the test is interrupted by a signal
  (``SIGINT`` / ``SIGTERM``), an SLA ``stop`` action, or an unexpected crash.
  Use this to tear down proxies, remove toxics, and clean up resources.

.. code-block:: yaml

   services:
   - module: shell
     prepare:
     - toxiproxy-cli create db_proxy -l 0.0.0.0:13306 -u db-server:3306
     startup:
     - toxiproxy-cli toxic add -t latency -a latency=500 db_proxy
     shutdown:
     - toxiproxy-cli delete db_proxy

Each command is executed sequentially within its phase. A non-zero exit code
in **prepare** or **startup** will abort the test and immediately trigger the
**shutdown** phase.

Real-Time SLA Breach Actions
-----------------------------
A background thread evaluates SLA rules **every 1 second** against live metrics
collected from the running test. When a threshold is breached, the configured
action fires immediately.

Actions can be one of the built-in keywords or a custom shell command:

* **stop**: Halt the test immediately and proceed to shutdown hooks.
* **warn**: Log a warning but continue the test.
* **continue**: Record the breach in the report but take no runtime action.
* **exec:<command>**: Execute an arbitrary shell command. The command string
  after the ``exec:`` prefix is passed to the system shell.

Supported metrics:

* **fail-rate**: Ratio of failed requests to total (0.0 to 1.0).
* **avg-response-time**: Average response time in milliseconds.
* **p90-response-time**: 90th percentile response time (ms).
* **p95-response-time**: 95th percentile response time (ms).
* **p99-response-time**: 99th percentile response time (ms).
* **throughput**: Requests per second.

.. code-block:: yaml

   reporting:
   - module: junit-xml
     sla:
     - metric: avg-response-time
       threshold: 500.0
       action: "exec:curl -X POST http://rollback-service/api/rollback"
     - metric: fail-rate
       threshold: 0.10
       action: "exec:sh scripts/disable-chaos.sh"

When an ``exec:`` action is triggered, the command inherits the environment of
the bzt-rs process and has access to all exported environment variables.

Dynamic Control API
-------------------
bzt-rs can expose an HTTP control API during the test run. Enable it in your
configuration:

.. code-block:: yaml

   api:
     enabled: true
     port: 8000

The API binds to ``127.0.0.1`` by default for security. Two endpoints are
available:

**GET /metrics**

Returns a JSON object containing per-endpoint statistics:

.. code-block:: bash

   curl http://127.0.0.1:8000/metrics

.. code-block:: json

   {
     "uptime_secs": 42,
     "total_requests": 1520,
     "total_failures": 3,
     "endpoints": [
       {
         "method": "GET",
         "path": "/api/status",
         "count": 760,
         "failures": 1,
         "avg_time_ms": 45.2,
         "p95": 120.0,
         "p99": 250.0
       },
       {
         "method": "POST",
         "path": "/api/orders",
         "count": 760,
         "failures": 2,
         "avg_time_ms": 88.7,
         "p95": 210.0,
         "p99": 480.0
       }
     ]
   }

**POST /control/stop**

Gracefully aborts the running test. The engine completes in-flight requests,
runs shutdown hooks, and exits with the appropriate status code:

.. code-block:: bash

   curl -X POST http://127.0.0.1:8000/control/stop

Use cases for the control API include:

* **CI/CD pipeline integration**: A pipeline step polls ``/metrics`` and
  calls ``/control/stop`` when a deployment gate condition is met.
* **Grafana alerting webhooks**: Configure a Grafana alert contact point to
  POST to ``/control/stop`` when an external SLO dashboard breaches.
* **Manual operator control**: An on-call engineer can abort a scheduled
  chaos experiment from any machine with network access.

Complete Chaos Engineering Example
-----------------------------------
The following configuration demonstrates all three features working together
in a realistic scenario: Toxiproxy injects 500 ms of database latency while
bzt-rs drives traffic against the application, monitors response times, and
automatically rolls back the fault if the p95 exceeds the safety threshold.

.. code-block:: yaml

   execution:
   - concurrency: 50
     ramp-up: 30s
     hold-for: 5m
     scenario: order-flow

   scenarios:
     order-flow:
       default-address: http://app-server:8080
       requests:
       - url: /api/health
         method: GET
         assert:
         - contains: ["ok"]
       - url: /api/orders
         method: POST
         body: '{"item": "widget-${uuid}", "qty": 1}'
         extract-jsonpath:
           order_id: $.id
       - url: /api/orders/${order_id}
         method: GET
         assert:
         - jsonpath: $.status
           expected: "confirmed"

   services:
   - module: shell
     prepare:
     - toxiproxy-cli create db_proxy -l 0.0.0.0:13306 -u db-server:3306
     - echo "Proxy created, ready to inject chaos"
     startup:
     - toxiproxy-cli toxic add -t latency -a latency=500 -a jitter=100 db_proxy
     shutdown:
     - toxiproxy-cli delete db_proxy
     - echo "Chaos cleanup complete"

   reporting:
   - module: junit-xml
     filename: chaos-results.xml
     sla:
     - metric: p95-response-time
       threshold: 2000.0
       action: "exec:toxiproxy-cli toxic remove -n latency_downstream db_proxy"
     - metric: fail-rate
       threshold: 0.10
       action: stop
     - metric: avg-response-time
       threshold: 3000.0
       action: warn

   api:
     enabled: true
     port: 8000

Integration with CI/CD
-----------------------
The control API and shell hooks make it straightforward to wire chaos tests
into your CI/CD pipeline. Below are examples for GitHub Actions and Jenkins.

**GitHub Actions**

.. code-block:: yaml

   # .github/workflows/chaos-test.yml
   name: Chaos Test
   on:
     schedule:
     - cron: "0 3 * * 1"  # Every Monday at 03:00 UTC

   jobs:
     chaos:
       runs-on: ubuntu-latest
       services:
         toxiproxy:
           image: ghcr.io/shopify/toxiproxy:latest
           ports:
           - 8474:8474
           - 13306:13306
       steps:
       - uses: actions/checkout@v4

       - name: Install bzt-rs
         run: cargo install bzt-rs

       - name: Run chaos test
         run: |
           bzt-rs chaos-config.yml &
           BZT_PID=$!
           sleep 10

           # Poll metrics until p95 exceeds threshold or test ends
           while kill -0 $BZT_PID 2>/dev/null; do
             P95=$(curl -s http://127.0.0.1:8000/metrics | jq '.endpoints[0].p95')
             echo "Current p95: $P95 ms"
             if (( $(echo "$P95 > 5000" | bc -l) )); then
               echo "Safety threshold exceeded, stopping test"
               curl -X POST http://127.0.0.1:8000/control/stop
               break
             fi
             sleep 5
           done
           wait $BZT_PID || true

       - name: Upload results
         uses: actions/upload-artifact@v4
         with:
           name: chaos-results
           path: chaos-results.xml

**Jenkins Pipeline**

.. code-block:: groovy

   pipeline {
       agent any
       stages {
           stage('Chaos Test') {
               steps {
                   sh 'toxiproxy-server &'
                   sh 'bzt-rs chaos-config.yml &'
                   sh '''
                       sleep 15
                       STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
                                http://127.0.0.1:8000/metrics)
                       if [ "$STATUS" = "200" ]; then
                           echo "Control API is healthy"
                       fi
                   '''
               }
               post {
                   always {
                       sh 'curl -X POST http://127.0.0.1:8000/control/stop || true'
                       junit 'chaos-results.xml'
                   }
               }
           }
       }
   }
