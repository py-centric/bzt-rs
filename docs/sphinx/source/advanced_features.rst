Advanced Features
=================

bzt-rs includes several advanced features for realistic load testing.

Variable Extraction
-------------------
Extract values from responses using JSONPath or Regular Expressions.

.. code-block:: yaml

   scenarios:
     login:
       requests:
       - url: /api/token
         extract-jsonpath:
           authToken: $.token

Interpolation
-------------
Use extracted variables in subsequent requests.

.. code-block:: yaml

   scenarios:
     authenticated:
       requests:
       - url: /api/user/${authToken}

Control Flow
------------
Support for conditional execution and loops.

.. code-block:: yaml

   requests:
   - url: /api/poll
     loop: status == "pending"
