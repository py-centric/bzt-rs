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
* **${env.MY_VAR}**: Injects the value of the `MY_VAR` environment variable.

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

Variable Extraction & Interpolation
-----------------------------------
Build stateful user journeys by extracting data from responses:
* **JSONPath Extraction**:
  .. code-block:: yaml

     extract-jsonpath:
       token: $.access_token

* **Regex Extraction**:
  .. code-block:: yaml

     extract-regexp:
       id: "id=([0-9]+)"

Use extracted variables in any subsequent request field:
.. code-block:: yaml

   requests:
   - url: /api/profile/${token}
     headers:
       X-User-ID: ${id}

Control Flow: Conditions & Loops
--------------------------------
* **Conditional Execution (`if`)**:
  Execute a request only if a condition is met.
  .. code-block:: yaml

     requests:
     - url: /api/special
       if: "${id}" != ""

* **Polling Loops (`loop`)**:
  Repeatedly execute a request until a condition is satisfied.
  .. code-block:: yaml

     requests:
     - url: /api/status
       loop: "${status}" == "processing"

SLA & Pass/Fail Criteria
------------------------
Define success thresholds for your load test.

.. code-block:: yaml

   reporting:
   - module: passfail
     criteria:
     - avg-rt of scenario > 500ms for 10s, continue as failed
     - fail of scenario > 1% for 30s, stop as failed
