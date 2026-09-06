# Feature Specification: Architectural Remediation

**Feature Branch**: `004-architectural-remediation`
**Created**: 2026-05-09
**Status**: Draft
**Input**: Architecture analysis report identifying gaps between spec claims and actual implementation in pummel load testing tool

## User Scenarios & Testing

### User Story 1 - CLI Flags Work Reliably (Priority: P1)

A DevOps engineer runs `pummel test.yaml --manager` expecting to start a distributed load test. Instead of silently doing nothing, the tool either works correctly or clearly tells them the feature is unavailable.

**Why this priority**: Engineers lose trust in a tool when documented flags silently do nothing. This is a credibility issue that undermines all other features.

**Independent Test**: Run `pummel test.yaml --manager --expect-workers 3` and verify the tool either starts in distributed mode or exits with a clear "not implemented" error message.

**Acceptance Scenarios**:

1. **Given** the tool is invoked with `--manager`, **When** the command runs, **Then** it either starts a manager process or exits with a clear message that distributed mode is not available
2. **Given** the tool is invoked with `--metrics`, **When** the command runs, **Then** a metrics endpoint is accessible on the configured port or a clear message explains it is not available
3. **Given** the tool is invoked with `--otel`, **When** the command runs, **Then** trace context is injected into requests or a clear message explains it is not available
4. **Given** the tool is invoked with `--worker`, **When** the command runs, **Then** it either connects to a manager or exits with a clear message

---

### User Story 2 - All Specified Features Have Real Implementation (Priority: P1)

An SRE reads the README and configures environment-based conditional logic, throughput pacing, and failure thresholds. All three work as documented rather than having no effect.

**Why this priority**: Empty module files ("No-op engine modules") marked as complete in specs erode trust in the entire project. Users should be able to rely on the feature set being honest.

**Independent Test**: Create a config that uses `${env.DEBUG}`, sets a `throughput` limit with pacing, and configures an SLA failure threshold. All three should produce measurable effects.

**Acceptance Scenarios**:

1. **Given** a config referencing `${env.VARIABLE}` syntax, **When** the tool runs, **Then** the variable is resolved from the environment and injected into the request
2. **Given** a config with a throughput limit set, **When** the tool executes requests, **Then** the request rate does not exceed the configured limit
3. **Given** a config with an SLA failure threshold, **When** the test exceeds that threshold, **Then** the test halts with a clear SLA violation message

---

### User Story 3 - Clear and Actionable Error Messages (Priority: P2)

A developer makes a typo in a config file or a CSV data source is missing. Instead of a generic "Validation failed" string, they get a specific error message identifying the exact file, field, and issue.

**Why this priority**: Debugging load tests is time-consuming. Stringly-typed errors lose context and force users to manually trace failures.

**Independent Test**: Provide a config referencing a non-existent CSV file and verify the error message includes the exact file path and reason.

**Acceptance Scenarios**:

1. **Given** a config references a missing data source file, **When** the tool validates or runs, **Then** the error message includes the file path and the specific issue
2. **Given** a CSV file with a malformed row, **When** the tool reads the file, **Then** the error message identifies the exact row and column with the problem
3. **Given** a Goose engine execution failure, **When** the error propagates, **Then** the root cause is preserved and displayed (not just "Internal error")

---

### User Story 4 - Safe Usage Without Security Risks (Priority: P2)

A team runs load tests in a CI/CD pipeline. Config files reference CSV data sources and use environment variables. No configuration can accidentally read files outside the project directory or leak secrets in logs.

**Why this priority**: Load testing tools process user-supplied configuration that may reference external files. Without proper validation, a malicious or malformed config can lead to information disclosure or resource exhaustion.

**Independent Test**: Create a config with `../` path traversal in a data-source reference and verify the tool rejects it with a clear security warning.

**Acceptance Scenarios**:

1. **Given** a data-source path contains `../` traversal, **When** the tool processes the config, **Then** it rejects the path with a security warning
2. **Given** a CSV data source larger than 100MB, **When** the tool tries to load it, **Then** it rejects the file with a size limit message instead of crashing
3. **Given** a config with unknown or misspelled fields, **When** the tool parses it, **Then** it warns about the unrecognized fields instead of silently ignoring them
4. **Given** the mock server is started, **When** it listens for requests, **Then** it binds to localhost only, not all network interfaces

---

### User Story 5 - Terminal Output After Test Execution (Priority: P3)

An engineer runs a 5-minute load test. After completion, they see a summary table showing total requests, success rate, average latency, and requests per second — without needing to open a file or enable verbose logging.

**Why this priority**: Every test run should provide immediate feedback. Currently users get no terminal output after a test completes, making it impossible to quickly assess results.

**Independent Test**: Run a 10-second load test and verify the terminal displays a summary table with at least total requests, pass/fail counts, and average response time.

**Acceptance Scenarios**:

1. **Given** a completed load test run, **When** the tool finishes, **Then** a summary table is printed showing total requests, success/failure counts, and request rate
2. **Given** a test with multiple scenarios, **When** the tool finishes, **Then** the summary includes per-scenario breakdown
3. **Given** a test with latency data available, **When** the tool finishes, **Then** the summary includes average, p95, and p99 latency

---

### User Story 6 - Standard Taurus YAML Fields Are Recognized (Priority: P3)

A user migrates an existing Taurus YAML test suite to pummel. Fields like `label`, `headers`, `timeout`, and `body-file` are accepted and have effect, rather than being silently dropped.

**Why this priority**: Silent field dropping leads to subtle configuration bugs. Users expect Taurus YAML compatibility as advertised.

**Independent Test**: Create a Taurus YAML with `label`, `headers`, `timeout`, and `body-file` fields and verify all are recognized and applied.

**Acceptance Scenarios**:

1. **Given** a config with `label` on a request definition, **When** the tool runs, **Then** the label appears in metrics and reports instead of the raw URL
2. **Given** a config with scenario-level `headers`, **When** requests are made, **Then** the specified headers are included on every request in that scenario
3. **Given** a config with `timeout` on a request, **When** the request exceeds the timeout, **Then** it fails with a timeout error rather than hanging indefinitely
4. **Given** a config referencing a `body-file`, **When** the request is made, **Then** the file contents are sent as the request body

---

### Edge Cases

- What happens when ALL CLI flags that are not implemented are invoked together?
- How does the tool handle a config with both unknown fields AND path traversal simultaneously?
- What happens when a CSV file contains a BOM or non-UTF-8 encoding?
- How does the CLI summary handle a test with zero requests (no scenarios matched)?
- What happens when environment variable resolution encounters a circular reference?
- How does the tool behave when the SLA threshold is exactly 0.0 or 1.0?

## Requirements

### Functional Requirements

- **FR-001**: CLI flags that enable unimplemented features MUST either work correctly or produce a clear error message explaining the feature is not available
- **FR-002**: The `env.rs` module MUST provide structured environment variable resolution supporting `${env.VAR}` syntax with consistent priority ordering
- **FR-003**: The `sla.rs` module MUST provide configurable pass/fail criteria including fail rate, average response time, and percentile-based thresholds
- **FR-004**: The `pacing.rs` module MUST enforce request rate limits with configurable pacing strategies (fixed rate and randomized)
- **FR-005**: All errors MUST carry structured context including the source file, line/field, and root cause chain
- **FR-006**: All errors MUST use a structured error type that preserves root cause context, source location, and error classification, rather than using plain strings
- **FR-007**: Data source file paths MUST be validated to prevent directory traversal outside allowed directories
- **FR-008**: CSV data source loading MUST enforce a maximum file size limit
- **FR-009**: All configuration structs MUST reject unknown fields at deserialization time with clear warnings
- **FR-010**: The mock server MUST bind to localhost only by default
- **FR-011**: After test execution completes, the tool MUST display a summary table in the terminal showing total requests, success/failure counts, request rate, and latency percentiles
- **FR-012**: WebSocket and gRPC protocol stubs MUST produce clear "not implemented" error messages instead of silent `println!` output
- **FR-013**: The configuration model MUST support standard Taurus fields: `label`, `headers` (scenario-level), `timeout`, `body-file`, structured data-source definitions, and pacing configuration
- **FR-014**: All existing integration tests MUST pass without requiring manually created external files
- **FR-015**: Environment variable access from config files MUST be restricted to an allowlist or exclude sensitive patterns

- **FR-016**: The tool MUST support all standard HTTP methods: GET, POST, PUT, DELETE, PATCH, HEAD, and OPTIONS

### Key Entities

- **Configuration**: The parsed test plan containing execution settings, scenario definitions, and reporting configuration
- **Execution Plan**: A single test run configuration specifying concurrency, timing, scenario reference, and pacing
- **Scenario Definition**: A named set of HTTP/WS/gRPC requests with optional data sources, headers, and control flow
- **Validation Summary**: The result of a dry-run validation including file existence checks and translation verification
- **SLA Criterion**: A pass/fail condition with metric type, threshold value, duration window, and action on breach
- **Pacing Config**: Throttling parameters specifying request rate, time period, and randomization strategy
- **DataSource Definition**: Structured configuration for external data sources including file path, delimiter, quoting, looping, and ordering behavior
- **Testing Report**: Output from a test run including per-endpoint metrics, latency distributions, and pass/fail status per SLA criterion

## Success Criteria

### Measurable Outcomes

- **SC-001**: No CLI flag produces silent no-op behavior — every flag either works or clearly explains why it is unavailable
- **SC-002**: Environment variables referenced in config files as `${env.VAR}` are resolved correctly in all supported formats
- **SC-003**: Request rate limits are enforced within 5% of the configured value under steady-state conditions
- **SC-004**: All error messages include at least 3 contextual pieces of information (what failed, where, and why)
- **SC-005**: Configs with `../` path traversal in data-source paths are rejected 100% of the time
- **SC-006**: Unknown fields in configuration files produce warnings that are visible to the user
- **SC-007**: Test completion always produces visible terminal output — zero silent exits
- **SC-008**: All 8 existing integration tests pass without external file dependencies
- **SC-009**: Configs using standard Taurus fields (`label`, `headers`, `timeout`, `body-file`) parse without errors and apply the configured behavior

## Assumptions

- Users run pummel in CI/CD environments where security boundaries around file access matter
- Users migrating from Taurus expect high compatibility with Taurus YAML schema
- The Goose engine is the only execution backend and will remain so for the foreseeable future
- Environment variables accessible to the process are intended to be accessible from config files, except for well-known sensitive patterns
- The mock server is a development/testing tool and does not need production-grade security hardening beyond localhost binding
- WebSocket and gRPC support is not yet a priority for users — the primary need is honest error messages until proper implementation
- CLI output is the primary feedback channel for users; HTML and JUnit reports are secondary
