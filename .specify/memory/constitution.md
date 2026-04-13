<!--
Version change: none → 1.0.0
List of modified principles: none → initial
Added sections: Core Principles, Quality Gates & Documentation, Development Workflow, Governance
Removed sections: none
Templates requiring updates: 
- ✅ .specify/templates/plan-template.md
- ✅ .specify/templates/spec-template.md
- ✅ .specify/templates/tasks-template.md
-->

# bzt-rs Constitution

## Core Principles

### I. Library-First
Every feature starts as a standalone library. Libraries must be self-contained, independently testable, and documented. Clear purpose required - no organizational-only libraries.

### II. CLI Interface
Every library exposes functionality via CLI. Text in/out protocol: stdin/args → stdout, errors → stderr. Support JSON + human-readable formats. Text I/O ensures debuggability and composability.

### III. Test-First (NON-NEGOTIABLE)
TDD mandatory: Tests written → User approved → Tests fail → Then implement. Red-Green-Refactor cycle strictly enforced. Tests must be implemented for each feature, including unit and integration tests.

### IV. Integration Testing
Focus areas requiring integration tests: New library contract tests, Contract changes, Inter-service communication, and Shared schemas.

### V. Automation & Verification
Pre-commit and pre-push hooks MUST be implemented to enforce standards locally. Aim for a minimum of 80% code coverage. Tests MUST be run and pass in the CI environment (e.g., GitHub Actions) and in pre-commit hooks.

### VI. Conventional & Atomic Commits
Maintain a clean, linear history. Use conventional commit messages and ensure commits are atomic (one logical change per commit). An atomic commit MUST be created immediately after the completion of each discrete task in the task list.

### VII. History Management
Always rebase to `main`; merge commits are PROHIBITED.

### VIII. Licensing & Compliance
All dependencies (libraries, modules, assets) MUST use OSI-compliant licenses. Use of proprietary or non-OSI compliant software must be explicitly justified and approved. All third-party licenses MUST be documented in the project's documentation.

### IX. Pedantic Observability
- **Pedantic Debugging**: `DEBUG` level logs MUST be as pedantic as possible, capturing detailed state and flow information.
- **Meaningful Insights**: `INFO` level logs MUST provide meaningful production insight (e.g., user actions, system state changes) without exposing sensitive data.

### X. Security-First Architecture
Security is integrated into the core architecture.
- **Data Protection**: ALL data MUST be encrypted at rest and in transit, with the sole exception of UUIDs.
- **Encryption Standards**: Use minimum SHA256 for hashing. 
- **Privacy**: End-to-end encryption (E2EE) MUST be implemented for all private messages.
- **Secret Management**: Configuration files MUST be stored in `config/settings.env` (public) and `config/secrets.env` (sensitive). `config/secrets.env` MUST be in `.gitignore` and NEVER committed.

### XI. Simplicity & YAGNI
Start simple, YAGNI (You Aren't Gonna Need It) principles. Avoid over-engineering; keep the implementation focused on current requirements.

## Quality Gates & Documentation

### Quality Gates
1. **Linting/Formatting**: Must pass `cargo clippy`, `cargo fmt`, and `cargo check`.
2. **Security**: `cargo audit` must report no high-severity issues.
3. **Testing**: `cargo test` (with `cargo-tarpaulin` or `cargo-llvm-cov`) MUST pass with >= 80% coverage.
4. **Review**: All changes must be reviewed for compliance with architecture and licensing principles.

### Exhaustive Documentation
Detailed documentation is essential for maintainability and commercial readiness.
- **Tools**: Use `rustdoc` for code documentation and high-level docs. Use diagrams (e.g., Mermaid) to visualize architecture, low-level implementation, and deployment strategies.
- **Content**: Document architecture, design choices, technical debt, and exhaustive library/module versions. Provide detailed deployment guides.
- **Readme**: Use `README.md` for high-level, TLDR type documentation.
- **Comments**: Avoid inline comments unless absolutely necessary; use descriptive variable/function names.

## Development Workflow

### Hooks
Pre-commit and pre-push hooks are mandatory. They must run quality gates and tests before any code leaves the local environment.

### Clarification
Always ask clarifying questions to ensure completeness of requirements for each feature/spec request.

## Governance
This Constitution supersedes all other practices. Amendments require documentation, approval, and a migration plan. All PRs and reviews must verify compliance. Complexity must be justified.

The versioning follows semantic rules:
- **MAJOR**: Backward incompatible governance/principle removals or redefinitions.
- **MINOR**: New principle/section added or materially expanded guidance.
- **PATCH**: Clarifications, wording, typo fixes, non-semantic refinements.

**Version**: 1.0.0 | **Ratified**: 2026-04-12 | **Last Amended**: 2026-04-12
