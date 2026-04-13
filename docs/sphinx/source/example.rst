Detailed Example
================

This example demonstrates a complete user journey including authentication, 
state management, and control flow.

Scenario Definition
-------------------

.. code-block:: yaml

   execution:
   - concurrency: 5
     ramp-up: 10s
     hold-for: 1m
     scenario: complex-journey

   scenarios:
     complex-journey:
       data-sources:
       - users.csv
       requests:
       # Login and extract token
       - url: http://api.example.com/login
         method: POST
         body: '{"username": "${username}", "password": "${password}"}'
         extract-jsonpath:
           token: $.access_token
         assert:
         - contains: [200]
           subject: http-code

       # Access profile using token
       - url: http://api.example.com/profile
         headers:
           Authorization: Bearer ${token}
         extract-jsonpath:
           userId: $.id

       # Conditional check
       - url: http://api.example.com/check/${userId}
         if: userId != ""
         
       # Polling loop
       - url: http://api.example.com/status/${userId}
         loop: status == "processing"
         extract-jsonpath:
           status: $.status

   reporting:
   - module: junit-xml
     filename: results.xml
     failed-threshold: 0.05
