Distributed Mode
================

bzt-rs supports distributed load generation using a Manager/Worker architecture.

Manager
-------
Start the manager process:

.. code-block:: bash

   ./bzt-rs --manager --expect-workers 2 test.yaml

Worker
------
Start worker processes:

.. code-block:: bash

   ./bzt-rs --worker --manager-host <MANAGER_IP> test.yaml
