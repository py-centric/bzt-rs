# Data Model: bzt-rs Core

## Internal AST Entities

### `Configuration` (Root)
The top-level object containing execution parameters and scenarios.
- `execution`: List of `ExecutionPlan` objects.
- `scenarios`: Map of scenario name to `ScenarioDefinition`.
- `services`: (Optional) External services like Prometheus or Monitoring.

### `ExecutionPlan`
Maps to Taurus `execution` block.
- `concurrency`: Number of simulated users (Goose `users`).
- `ramp_up`: Time to reach full concurrency.
- `hold_for`: Duration of the test (Goose `duration`).
- `scenario`: Name of the scenario to execute.

### `ScenarioDefinition`
The blueprint for a user's actions.
- `requests`: List of `HTTPRequestDefinition`.
- `think_time`: (Optional) Delay between requests.
- `headers`: Map of global headers for the scenario.

### `HTTPRequestDefinition`
A single HTTP operation.
- `url`: Target URL (supports interpolation).
- `method`: GET, POST, PUT, DELETE, etc.
- `headers`: (Optional) Request-specific headers.
- `body`: (Optional) Request body.

---

## State Transitions

1. **Raw Source** (YAML/JSON/TOML) 
   - *Action*: `serde` deserialization
2. **Intermediate Representation** (Shorthand/Taurus Mixed)
   - *Action*: `SchemaNormalizer` unifies into `Configuration` AST
3. **Goose Configuration**
   - *Action*: `StateTranslator` builds `GooseAttack` and `GooseScenario` objects
4. **Execution Engine**
   - *Action*: Runtime execution of tasks
