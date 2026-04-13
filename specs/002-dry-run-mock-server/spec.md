# Feature Specification: Dry-Run and Mock Server

**Feature Branch**: `002-dry-run-mock-server`  
**Created**: April 13, 2026  
**Status**: Draft  
**Input**: User description: "implement a dry-run feature, also implement a feature to start the mock server and run config against mock server"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Configuration Validation (Dry-Run) (Priority: P1)

As a Performance Engineer, I want to perform a "dry-run" of my load test configuration without actually generating traffic to the target servers, so that I can verify that my configuration (YAML/JSON/TOML) is syntactically correct and all referenced files (CSV, etc.) exist.

**Why this priority**: High priority as it provides immediate feedback to users during the test development phase, preventing wasted time on broken runs.

**Independent Test**: Run the tool with the `--dry-run` flag against a valid and an invalid configuration. Verify that it reports success for valid configs and specific errors for missing files or invalid syntax without initiating a Goose attack.

**Acceptance Scenarios**:

1. **Given** a valid configuration file, **When** executed with `--dry-run`, **Then** the system outputs "Validation Successful" and exits with code 0 without sending any network requests.
2. **Given** a configuration referencing a non-existent CSV file, **When** executed with `--dry-run`, **Then** the system outputs a clear error message indicating the missing file and exits with a non-zero code.

---

### User Story 2 - Local Functional Testing (Mock Server) (Priority: P2)

As a Developer, I want to start a local mock server based on my test configuration and run the load test against it, so that I can verify my scenario logic (extraction, interpolation, control flow) in a safe, isolated environment.

**Why this priority**: Medium priority as it bridges the gap between configuration validation and full-scale load testing, ensuring scenario logic is sound.

**Independent Test**: Start the tool with a `--mock` flag. Verify that a local server starts and that the load test automatically targets this local server, succeeding only if the scenario logic correctly matches the mock server's behavior.

**Acceptance Scenarios**:

1. **Given** a configuration with a simple GET request, **When** started with `--mock`, **Then** the system starts a local HTTP server, targets it, and the test completes successfully.
2. **Given** a configuration with an assertion, **When** started with `--mock`, **Then** the mock server returns a compatible response, and the assertion passes.

---

### User Story 3 - Integrated Mock Execution (Priority: P3)

As a CI/CD Engineer, I want to run a complete load test scenario against an automatically managed mock server in a single command, so that I can include functional performance tests in my pipeline without external dependencies.

**Why this priority**: Lower priority than basic mock support but provides high value for automation and reliability.

**Independent Test**: Execute a single command that both starts the mock and runs the test, verifying the entire process is self-contained.

**Acceptance Scenarios**:

1. **Given** a complex scenario with state extraction, **When** executed with `--mock-run`, **Then** the system orchestrates both the mock server and the test run, verifying the end-to-end logic without manual intervention.

---

### Edge Cases

- What happens if the mock server port is already in use?
- How does the system handle a dry-run for a configuration that uses environment variables that are not set?
- What occurs if the mock server receives a request that wasn't defined in the scenario (e.g., unexpected URL)?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a `--dry-run` CLI flag to validate configuration syntax and file existence.
- **FR-002**: Dry-run MUST verify that all `data-sources` (CSVs) are readable.
- **FR-003**: System MUST provide a `--mock` CLI flag to start an internal HTTP mock server.
- **FR-004**: The mock server MUST be generated dynamically based on the `requests` defined in the scenario.
- **FR-005**: The mock server MUST return a default status of 200 OK for matched requests without specific assertions. This default MUST be configurable via a global `mock-default-status` setting in the configuration file.
- **FR-009**: Unmatched requests (unexpected URLs) MUST return a 404 Not Found response to distinguish between configuration gaps and functional failures.
- **FR-006**: System MUST allow running a configuration specifically against the internal mock server (e.g., `--mock-run`).
- **FR-007**: When running against a mock server, the `host` in the execution plan MUST be automatically overridden to the local mock server address.
- **FR-008**: System MUST support assertion-based response simulation. The mock server MUST parse defined assertions (e.g., `contains`) and generate a response body that satisfies those rules to enable functional testing of downstream extraction and logic.

### Key Entities *(include if feature involves data)*

- **MockServer**: An internal component that listens for HTTP requests and provides simulated responses based on the test configuration.
- **ValidationReport**: The output of a dry-run, listing any syntax errors or missing dependencies.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Dry-run validation of a 100-request configuration completes in under 500ms.
- **SC-002**: Mock server startup time is less than 1 second.
- **SC-003**: 100% of standard Taurus HTTP GET/POST requests can be mocked without manual mock configuration.
- **SC-004**: Users can successfully verify a complex scenario (extract-then-interpolate) against a mock server without external dependencies.

## Assumptions

- Target users are comfortable with CLI flags.
- The mock server will use a random available port or a standard default (e.g., 8080) if not specified.
- Dry-run does not require network access.
- Mock server only supports HTTP/S protocols in the initial implementation.
- Complex stateful mocking (e.g., specific response based on previous request data) is out of scope for the initial version.
