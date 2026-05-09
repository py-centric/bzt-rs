# Quickstart: Architectural Remediation

This is an internal remediation feature — no user-facing workflow changes.

## Verification

1. **Run existing tests** to confirm no regressions:
   ```bash
   cargo test
   ```

2. **Verify CLI flags** that were misleading now behave correctly:
   ```bash
   # Option A (stripped): --manager is no longer accepted
   cargo run -- test.yaml --manager
   # Expected: "error: unexpected argument '--manager' found"

   # Option B (kept with error): --manager prints clear message
   cargo run -- test.yaml --manager
   # Expected: "[NOT IMPLEMENTED] Distributed mode (--manager) is not yet implemented"
   ```

3. **Verify environment variable resolution**:
   ```bash
   DEBUG=true cargo run -- sample_test.yaml --dry-run
   # Expected: `${env.DEBUG}` resolves to "true" in conditional evaluation
   ```

4. **Verify security hardening**:
   ```bash
   # Path traversal rejected
   cargo run -- test.yaml --dry-run
   # (config referencing ../../etc/passwd as data-source)
   # Expected: "SecurityError: Path '../../etc/passwd' traverses outside allowed directory"
   ```

5. **Verify CLI output** after test:
   ```bash
   cargo run -- test.yaml
   # Expected: ASCII summary table printed to stdout
   ```

6. **Verify unknown fields produce warnings**:
   ```bash
   # Config with typo "concurrncy" instead of "concurrency"
   cargo run -- test.yaml --dry-run
   # Expected: "Warning: Unknown field 'concurrncy' in execution[0]"
   ```

7. **Verify HTTP method support**:
   ```bash
   # Config with PUT, DELETE, PATCH methods
   cargo run -- test.yaml --mock-run
   # Expected: Requests are made with correct HTTP methods
   ```
