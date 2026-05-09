Distributed Mode
================

.. warning::

   Distributed mode is **not yet implemented**.

   The ``--manager``, ``--worker``, ``--expect-workers``, ``--manager-host``,
   and ``--manager-port`` CLI flags have been removed from the argument parser
   to avoid silent no-op behavior. They will be re-implemented in a future
   release.

Current Capabilities
--------------------

For the current release, bzt-rs focuses on single-node load generation.
To generate load from a single machine:

.. code-block:: bash

   ./bzt-rs test.yaml

See the :doc:`quickstart` for common usage patterns and the
:doc:`configuration` reference for available options.
