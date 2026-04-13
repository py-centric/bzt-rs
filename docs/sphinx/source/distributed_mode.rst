Distributed Mode
================

bzt-rs provides native support for distributed load generation using a 
highly-scalable Manager/Worker architecture. This allows you to generate 
massive amounts of traffic by coordinating multiple machines from a single 
point of control.

Manager/Worker Architecture
---------------------------
* **Manager**: The central controller that parses the configuration, 
  distributes it to workers, and aggregates metrics. The manager does not 
  generate load itself.
* **Worker**: Independent agents that receive instructions from the manager, 
  generate the actual load, and report metrics back to the manager.

Setting Up the Manager
----------------------
Launch the manager on a machine that all workers can reach:

.. code-block:: bash

   ./bzt-rs --manager --expect-workers 5 --manager-port 5115 test.yaml

* **--expect-workers**: The number of workers the manager should wait for 
  before starting the test.
* **--manager-port**: The port the manager will listen on for worker 
  connections (default: 5115).

Setting Up the Workers
----------------------
Launch workers on each machine that will generate load:

.. code-block:: bash

   ./bzt-rs --worker --manager-host <MANAGER_IP> --manager-port 5115

* **--manager-host**: The IP address or hostname of the machine running the 
  manager.
* **--manager-port**: The same port specified on the manager.

Network Requirements
--------------------
Ensure the following ports are open between the manager and workers:
* **Manager Port (default: 5115)**: Used for coordination and control.
* **Metrics Port (optional)**: If Prometheus metrics are enabled on the 
  manager, ensure the metrics port (default: 8080) is accessible for 
  scraping.

Communication Protocol
----------------------
The manager and workers communicate over a high-performance, low-latency 
gRPC-based protocol to ensure synchronization and real-time metric updates 
with minimal overhead.
