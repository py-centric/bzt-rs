# Quickstart: Dry-Run and Mock Server

## Validating Your Configuration
Before running a large-scale load test, use the `--dry-run` flag to ensure your configuration is valid and all required files exist:

```bash
bzt-rs --dry-run test.yaml
```

## Running Local Functional Tests
You can verify your scenario logic (extraction, interpolation, assertions) against a built-in mock server. This is useful for testing in isolated environments or CI pipelines.

### Option 1: Start the Mock Server only
Useful if you want to manually point another tool at the mock:

```bash
bzt-rs --mock test.yaml
```

### Option 2: Integrated Mock Run
Automatically starts the mock, points `bzt-rs` at it, and runs the test:

```bash
bzt-rs --mock-run test.yaml
```

## Mock Server Behavior
The mock server generates responses based on your configuration's assertions:

```yaml
scenarios:
  sample:
    requests:
    - url: /api/login
      assert:
      - contains: ["Success", "token: 123"]
```

When you call `/api/login` on the mock server, it will return:
- **Status**: `200 OK`
- **Body**: `Success token: 123`
- **Headers**: `Content-Type: text/plain`

This allows your extraction rules (JSONPath/Regex) to correctly process the mock data.
