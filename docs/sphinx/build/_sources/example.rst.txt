Detailed Example: E-Commerce User Journey
=========================================

This example demonstrates a realistic load test scenario for an e-commerce
platform, covering authentication, data parameterization, stateful
extraction, and complex control flow.

The Scenario
------------
A virtual user will:

1. Load credentials from a CSV file.

2. Authenticate and extract a session token.

3. Search for a product using a randomized search term.

4. Add the product to their cart if it exists.

5. Poll the order status until it is confirmed.

Full Configuration (``ecommerce_test.yaml``)
--------------------------------------------

.. code-block:: yaml

   execution:
   - concurrency: 50           # Scale to 50 concurrent users
     ramp-up: 1m               # Reach target concurrency over 1 minute
     hold-for: 5m              # Maintain load for 5 minutes
     scenario: checkout-flow

   scenarios:
     checkout-flow:
       data-sources:
       - users.csv             # CSV file containing 'username' and 'password'
       headers:
         Accept: application/json

       requests:
       # Step 1: Authentication
       - url: http://api.shop.com/v1/login
         method: POST
         label: login_request
         body:
           user: "${username}"
           pass: "${password}"
         extract-jsonpath:
           authToken: $.session.token
         assert:
         - contains: [200]
           subject: http-code

       # Step 2: Product Search with Macro
       - url: http://api.shop.com/v1/search?q=${faker.word}
         method: GET
         label: search_products
         headers:
           Authorization: "Bearer ${authToken}"
         extract-jsonpath:
           productId: $.results[0].id

       # Step 3: Conditional Add to Cart
       - url: http://api.shop.com/v1/cart/add
         method: POST
         label: add_to_cart
         if: "${productId}" != ""  # Only add to cart if a product was found
         headers:
           Authorization: "Bearer ${authToken}"
         body:
           product_id: "${productId}"
           quantity: 1

       # Step 4: Polling for Order Status
       - url: http://api.shop.com/v1/orders/status
         method: GET
         label: poll_order_status
         headers:
           Authorization: "Bearer ${authToken}"
         loop: "${orderStatus}" == "processing"
         extract-jsonpath:
           orderStatus: $.status

   reporting:
   - module: junit-xml
     filename: test_results.xml
     failed-threshold: 0.05
     sla:
     - metric: fail-rate
       threshold: 0.05
       action: warn
     - metric: avg-response-time
       threshold: 5000.0
       action: warn

Step-by-Step Breakdown
----------------------

### 1. Execution Settings
We configure ``50`` concurrent users with a ``1m`` ramp-up. This ensures that
load is added gradually to avoid overwhelming the system at the very
beginning of the test.

### 2. Data Sources
The ``users.csv`` file provides unique credentials for each virtual user,
preventing duplicate login attempts and ensuring a realistic distribution
of accounts.

### 3. Scenario-Level Headers
The ``headers`` block under the scenario applies the ``Accept`` header to
every request, avoiding repetition in individual request definitions.

### 4. State Management
We use ``extract-jsonpath`` to capture the ``authToken`` from the login response
and reuse it in the ``Authorization`` header for all subsequent requests.
This maintains the stateful nature of a real user session.

### 5. Dynamic Data
The ``${faker.word}`` macro generates a random search term for each request,
preventing the backend from serving cached results and ensuring a
comprehensive test of the search index.

### 6. Control Flow
The ``if`` condition handles cases where a search might return no results,
while the ``loop`` ensures the user waits for their order to process before
finishing the scenario, mirroring real user patience.

### 7. SLA Criteria
The reporting section defines two SLA criteria: a fail-rate threshold of
5% (warn if exceeded) and an average response time threshold of 5000ms
(warn). These are evaluated after the test completes and reported in
console output alongside the JUnit XML report.
