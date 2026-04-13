# Data Model: Dry-Run and Mock Server

## Feature Specific Entities

### `MockResponse`
Simulated response generated from assertions.
- `status`: Defaulting to 200 OK.
- `body`: Concatenated list of all "contains" strings from assertions.
- `headers`: (Optional) Standard JSON or HTML content-type headers.

### `ValidationSummary` (Internal)
The result of the dry-run execution.
- `file_existence`: Map of path to status (FOUND/MISSING).
- `syntax_valid`: Boolean.
- `normalization_successful`: Boolean.
- `translation_successful`: Boolean.

---

## Behavior Model

### Dry-Run Workflow
1. Input File -> `Parser`
2. Configuration -> `Normalizer` (Checks file existence of `data-sources`)
3. `Configuration` AST -> `StateTranslator` (Validates configuration logic and mappings)
4. IF successful, print success message and exit.

### Mock Server Workflow
1. Start `axum` server task.
2. The server receives the `Configuration` object.
3. On request:
   - Identify the scenario being mocked.
   - Match the requested `url` (normalized) and `method` against `ScenarioDefinition.requests`.
   - If match found, return a `MockResponse` derived from assertions.
   - If no match, return 404.
4. If `--mock-run`:
   - Override the `ExecutionPlan.host` to point to the local mock server.
   - Initiate the Goose attack.
   - Stop the mock server after the attack finishes.
