# Implementation Plan: Dry-Run and Mock Server

**Branch**: `002-dry-run-mock-server` | **Date**: 2026-04-13 | **Spec**: [specs/002-dry-run-mock-server/spec.md](spec.md)
**Input**: Feature specification from `/specs/002-dry-run-mock-server/spec.md`

## Summary

Implement configuration validation (`--dry-run`) and a dynamic HTTP mock server (`--mock`, `--mock-run`) for `pummel`. The technical approach involves stopping the execution pipeline after the translation phase for dry-runs, and using `axum` to host a dynamic, assertion-based mock server that simulates backend responses for functional testing of load test scenarios.

## Technical Context

**Language/Version**: Rust 2024+
**Primary Dependencies**: `goose`, `tokio`, `serde`, `axum`
**Storage**: N/A
**Testing**: `cargo test`, `cargo clippy`, `cargo audit`, `cargo-llvm-cov`
**Target Platform**: Linux (single binary)
**Project Type**: CLI tool
**Performance Goals**: Dry-run < 500ms, Mock startup < 1s
**Constraints**: Runtime overhead < 5%
**Scale/Scope**: Support standard Taurus YAML + Shorthand.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **Principle I (Library-First)**: Mock server will be a standalone module in `src/engine/mock.rs`.
- [x] **Principle II (CLI Interface)**: New flags `--dry-run`, `--mock`, `--mock-run` added to `src/main.rs`.
- [x] **Principle III (Test-First)**: TDD mandatory. Tests for mock responses and validation logic will be written first.
- [x] **Principle V (Automation)**: Pre-commit and CI will enforce coverage >= 80%.
- [x] **Principle X (Security-First)**: Secrets ignored, encryption defaults applied.
- [x] **Principle XI (Simplicity)**: Focused on assertion-based mocking (YAGNI for complex stateful mocks).

## Project Structure

### Documentation (this feature)

```text
specs/002-dry-run-mock-server/
├── plan.md              # This file
├── research.md          # Decision log
├── data-model.md        # AST and internal state representation
├── quickstart.md        # User guide for core features
├── contracts/           # CLI argument and config schema definitions
└── tasks.md             # Implementation tasks (Phase 2 output)
```

### Source Code (repository root)

```text
src/
├── main.rs              # CLI entry point update
├── engine/
│   ├── mod.rs           # Engine module exports
│   ├── validation.rs    # Dry-run validation logic
│   └── mock.rs          # Dynamic mock server implementation
└── translator/
    └── mod.rs           # StateTranslator updates for host override

tests/
├── dry_run_integration.rs # Dry-run tests
└── mock_integration.rs    # Mock server and mock-run tests
```

**Structure Decision**: Integrated into existing `src/engine/` and `src/translator/` modules. New unit and integration tests added.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| None      |            |                                     |
