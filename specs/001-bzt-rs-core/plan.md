# Implementation Plan: pummel Core Architecture & Roadmap

**Branch**: `001-pummel-core` | **Date**: 2026-04-12 | **Spec**: [specs/001-pummel-core/spec.md](spec.md)

## Summary

Implement the core architecture for `pummel`, a Taurus configuration interpreter and runner using the Goose load engine. The technical approach involves building a multi-format parser, a schema normalizer to bridge Taurus and "Shorthand" schemas, and a state translator to map these configurations into Goose's native execution tasks.

## Technical Context

**Language/Version**: Rust 2024+
**Primary Dependencies**: goose, tokio, serde, serde_yaml, serde_json, toml, regex, fake, uuid
**Storage**: N/A (initial phases focus on stateless request execution)
**Testing**: cargo test, cargo clippy, cargo audit, cargo-llvm-cov
**Target Platform**: Linux (single binary, no external dependencies)
**Project Type**: CLI tool / Load generator
**Performance Goals**: Runtime overhead < 5% of native Goose capability
**Constraints**: Zero-dependency executable; lock-free or highly sharded concurrency for shared state.
**Scale/Scope**: Support standard Taurus YAML plus JSON/TOML shorthand.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **Principle I (Library-First)**: Core engine and parsers will be implemented as self-contained modules.
- [x] **Principle II (CLI Interface)**: Tool follows text in/out protocol; supports JSON/YAML/TOML.
- [x] **Principle III (Test-First)**: TDD mandatory. Tests for parsers and translators will be written before implementation.
- [x] **Principle V (Automation & Verification)**: pre-commit hooks and CI will enforce 80% coverage.
- [x] **Principle VI (Conventional & Atomic Commits)**: Commit after each task.
- [x] **Principle X (Security-First)**: Config files in `config/` with secrets ignored.
- [x] **Principle XI (Simplicity & YAGNI)**: Phase 1 focused on "Walking Skeleton."

## Project Structure

### Documentation (this feature)

```text
specs/001-pummel-core/
├── plan.md              # This file
├── research.md          # Decision log
├── data-model.md        # AST and internal state representation
├── quickstart.md        # User guide for core features
├── contracts/           # CLI argument and config schema definitions
└── tasks.md             # Implementation tasks
```

### Source Code (repository root)

```text
src/
├── main.rs              # CLI entry point
├── parser/              # YAML/JSON/TOML deserializers
├── normalizer/          # Schema unification (Shorthand -> Taurus AST)
├── translator/          # Taurus AST -> Goose Task mapping
└── engine/              # Goose integration and execution wrapper

tests/
├── integration/         # Taurus YAML end-to-end tests
└── unit/                # Parser and translator logic tests
```

**Structure Decision**: Single project layout with modular sub-crates/modules for parsing, normalization, and translation.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| None      |            |                                     |
