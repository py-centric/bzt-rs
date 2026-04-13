# Feature Specification: bzt-rs Core Architecture & Roadmap

**Feature Branch**: `001-bzt-rs-core`  
**Created**: April 12, 2026  
**Status**: Draft  
**Input**: User description: Software Requirements Specification (SRS) for bzt-rs (Goose-Taurus Runner) Version 9.2.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Basic Load Test Execution (YAML Core) (Priority: P1)

As a Performance Engineer, I want to run a basic load test using standard Taurus YAML configurations so that I can validate my core system architecture without learning a new syntax.

**Why this priority**: Proves the foundational capability of the tool and provides immediate utility for existing Taurus users.

**Independent Test**: Can be fully tested by providing a basic YAML file with concurrency and static HTTP requests, and verifying standard metrics are output.

**Acceptance Scenarios**:

1. **Given** a valid Taurus YAML file with concurrency set to 50 and hold-for set to 1m, **When** the tool executes the file, **Then** it successfully initiates a load test matching those exact bounds.
2. **Given** a scenario with standard GET and POST requests, **When** the test runs, **Then** the tool correctly constructs and sends the HTTP requests to the target server.
3. **Given** a completed load test, **When** execution finishes, **Then** an HTML report file and a CLI metrics summary table are successfully generated.

---

### User Story 2 - Frictionless Test Creation (Multi-Format & Shorthand) (Priority: P2)

As a Developer, I want to define my load tests using JSON, TOML, or a simplified Shorthand syntax so that I can use the format that best fits my existing project structure and reduce boilerplate.

**Why this priority**: Expands tool adoption by supporting developer-preferred formats and streamlining test creation.

**Independent Test**: Provide identical test definitions in YAML, JSON, and TOML (Shorthand) and verify the execution output is identical.

**Acceptance Scenarios**:

1. **Given** identical load test configurations written in JSON and TOML, **When** executed, **Then** the tool parses and runs them with the exact same behavior as the YAML equivalent.
2. **Given** a minimal "Shorthand" TOML configuration, **When** parsed, **Then** the configuration is correctly translated into the internal AST without dropping execution parameters.

---

### User Story 3 - Complex Test Scenarios (Hierarchies & Branching) (Priority: P3)

As a QA Engineer, I want to define hierarchical scenarios and branch execution paths with specific weights so that I can model complex user behaviors like "authenticate once, then perform various actions proportionally."

**Why this priority**: Essential for modeling realistic user journeys rather than just hammering a single endpoint.

**Independent Test**: Configure a test with a parent "login" step and weighted child steps, verifying the login happens once and child steps distribute traffic accurately.

**Acceptance Scenarios**:

1. **Given** a nested scenario configuration, **When** executed, **Then** the parent request executes exactly once per simulated user session before child requests.
2. **Given** branched scenarios with assigned weights (e.g., 80/20), **When** executed, **Then** the resulting traffic metrics reflect the approximate requested distribution.

---

### User Story 4 - Realistic Traffic Generation (Priority: P4)

As a Security Tester, I want to inject environment variables, dynamic data (faker/UUID), and read from CSV files so that I can generate realistic, unique traffic that bypasses caching and database uniqueness constraints.

**Why this priority**: Required for testing robust environments where static data would be rejected or cached.

**Independent Test**: Provide a configuration with environment variables, macro placeholders, and a think-time range, verifying the outgoing requests contain substituted values and occur with variable delays.

**Acceptance Scenarios**:

1. **Given** a target URL containing an environment variable placeholder, **When** parsed, **Then** the tool successfully substitutes the OS environment variable value at runtime.
2. **Given** a request body containing dynamic macros (e.g., email, UUID), **When** executed, **Then** each concurrent request generates and sends unique, valid data.
3. **Given** a think-time configuration of 1s-3s, **When** executed, **Then** the engine pauses for a random duration between 1 and 3 seconds between sequential requests.

---

### Edge Cases

- What happens when an environment variable referenced in the configuration is missing from the host OS?
- How does the system handle poorly formatted CSV data for parameterization?
- What occurs if a nested scenario references a parent scenario that fails its initial request (e.g., authentication fails)?
- How does the tool behave if the specified output directory for reports is read-only or out of disk space?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST parse configurations natively in YAML, JSON, and TOML.
- **FR-002**: System MUST map basic execution blocks (concurrency, hold-for, ramp-up) dynamically.
- **FR-003**: System MUST execute static HTTP requests with static headers and bodies.
- **FR-004**: System MUST output HTML reports and CLI metrics summaries.
- **FR-005**: System MUST support hierarchical scenario definitions and ensure parent/setup tasks execute only once per session.
- **FR-006**: System MUST support numeric execution weights for branched scenarios.
- **FR-007**: System MUST inject host OS environment variables into configurations at runtime.
- **FR-008**: System MUST evaluate embedded macros (e.g., fake data generation) at runtime per-request.
- **FR-009**: System MUST read external CSV files to parameterize test runs.
- **FR-010**: System MUST support fixed and randomized delays (think-time) between requests.
- **FR-011**: System MUST extract variables from responses via JSONPath/Regex and interpolate them into subsequent requests.
- **FR-012**: System MUST enforce HTTP status code and response body text assertions.
- **FR-013**: System MUST support conditional execution and polling loops directly within the configuration.
- **FR-014**: System MUST generate JUnit-compatible XML test reports.
- **FR-015**: System MUST halt execution and return non-zero exit codes if defined SLA thresholds are breached.
- **FR-016**: System MUST support strict RPS limits to throttle traffic.
- **FR-017**: System MUST support distributed execution across a network cluster.
- **FR-018**: System MUST expose real-time metrics (e.g., Prometheus format) and support distributed tracing injection.

### Key Entities

- **Test Configuration**: The user-provided definition of the load test, including execution bounds, scenarios, and environments.
- **User Session**: The stateful context for a single simulated concurrent user, holding extracted variables, connection state, and cookies.
- **Execution Metric**: The atomic record of a single request/action (latency, status, size, endpoint) pushed to the reporting pipeline.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Configuration normalization and parsing must occur exclusively at startup and not block test execution.
- **SC-002**: Runtime overhead during execution must not degrade native engine request-per-second capability by more than 5%.
- **SC-003**: The tool must be packaged and distributed as a single, standalone executable with zero external dependencies.
- **SC-004**: Users can successfully write and execute a complex, multi-step API journey (create, verify, poll, delete) using purely configuration text, with no custom code.
- **SC-005**: System accurately throttles traffic to exact RPS targets even when concurrency limits exceed the necessary load.

## Assumptions

- Target users are technical professionals (Developers, QA, SREs) comfortable with CLI tools and configuration files.
- Tests will primarily target HTTP/S APIs, with subsequent phases introducing other protocols.
- The underlying engine (Goose) provides the necessary low-level async performance; bzt-rs acts as an orchestration, configuration, and state management layer on top of it.
