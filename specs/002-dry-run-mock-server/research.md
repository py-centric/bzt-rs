# Research: Dry-Run and Mock Server

## Technical Decisions

### Decision 1: Dry-Run Implementation
- **Choice**: Execute the full pipeline (Parser -> Normalizer -> Translator) and stop before `GooseAttack::execute`.
- **Rationale**: This validates the syntax, configuration normalization, and file existence (CSV, etc.) which are handled in the translation phase.
- **Alternatives considered**: Only parsing. Rejected because translation catches configuration logic errors and missing resources.

### Decision 2: Mock Server Framework
- **Choice**: `axum`.
- **Rationale**: Already a dependency for metrics, high-performance, and very flexible for dynamic route generation.
- **Alternatives considered**: `warp`, `rocket`. Rejected to minimize additional dependencies.

### Decision 3: Dynamic Route Generation
- **Choice**: Build a single `axum` handler that matches any path and returns a simulated response based on the `Configuration` object.
- **Rationale**: More flexible than static route registration when paths contain interpolation or variables.
- **Implementation**: The mock server will receive the `Configuration` object and use the requested path/method to look up the corresponding `HTTPRequestDefinition`.

### Decision 4: Assertion-Based Mock Responses
- **Choice**: Inspect `AssertionDefinition.contains` list.
- **Rationale**: Simple and satisfies FR-008 (Q1: B). The mock server will concatenate all "contains" strings for the matching request into a single response body.
- **Alternatives considered**: Fully configurable mock responses. Rejected to keep initial scope manageable (per spec).

### Decision 5: Automatic Host Overriding
- **Choice**: Set `GOOSE_HOST` environment variable or override via `GooseAttack.set_default(GooseDefault::Host, ...)` when `--mock-run` is used.
- **Rationale**: Seamless for the user.

## Best Practices
- **Mock Server Port**: Use `0.0.0.0:0` to let the OS assign a random available port, preventing port collisions (Edge Case 1).
- **Graceful Shutdown**: Ensure the mock server task can be terminated after the load test completes.
