# CLI Contract: Dry-Run and Mock Server

## New Flags

### `--dry-run`
- **Short**: `-d` (Optional)
- **Description**: Validate the configuration file and dependencies without executing the test.
- **Exit Codes**:
  - `0`: Valid configuration.
  - `1`: Syntax error, missing file, or invalid configuration.
- **Output**: Detailed list of validation steps and their status.

### `--mock`
- **Short**: `-m` (Optional)
- **Description**: Start an internal HTTP mock server based on the provided configuration.
- **Behavior**: Starts the mock server on a random available port and prints the address. The process will remain active until manually terminated.
- **Output**: `Mock server started at http://localhost:<port>`

### `--mock-run`
- **Short**: (None)
- **Description**: Start the mock server AND automatically run the load test against it.
- **Behavior**:
  - Starts the mock server.
  - Overrides the `host` in all execution plans to the mock server's address.
  - Runs the Goose attack.
  - Gracefully stops the mock server after the attack finishes.

## Output Formats

### Dry-Run Success
```text
[OK] Parsing test.yaml
[OK] Normalizing configuration
[OK] Verifying data-sources: users.csv
[OK] Translating to Goose Attack
Validation Successful.
```

### Dry-Run Failure
```text
[OK] Parsing test.yaml
[OK] Normalizing configuration
[ERROR] Missing data-source: data/users.csv
Validation Failed.
```
