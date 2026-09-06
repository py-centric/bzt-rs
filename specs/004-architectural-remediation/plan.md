# Implementation Plan: Architectural Remediation

**Branch**: `004-architectural-remediation` | **Date**: 2026-05-09 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/004-architectural-remediation/spec.md`

## Summary

Remediate gaps between spec claims and actual implementation in pummel: implement no-op engine modules (env.rs, pacing.rs, sla.rs), fix misleading CLI flags (--manager, --worker, --metrics, --otel), replace stringly-typed errors with structured PummelError, add security hardening (path traversal, file size limits, env var protection, unknown field rejection, localhost binding), implement CLI reporter for post-test output, complete Taurus config model (label, headers, timeout, body-file, structured data-sources, pacing config), add missing unit tests, and support all standard HTTP methods.

## Technical Context

**Language/Version**: Rust 2024 edition  
**Primary Dependencies**: goose 0.18, tokio 1.x, serde/serde_yaml/serde_json, toml 0.8, regex, axum 0.8, clap 4.6, thiserror 2.0, csv 1.3, fake 2.9, uuid 1.6, tracing/tracing-subscriber, quick-xml 0.31, prometheus 0.13 (candidate for removal), opentelemetry 0.21 (candidate for removal)  
**Storage**: N/A — CLI load testing tool, file-based config only  
**Testing**: cargo test (cargo-llvm-cov for coverage, 80% minimum)  
**Target Platform**: Linux (cross-platform Rust)  
**Project Type**: CLI tool  
**Performance Goals**: No measurable overhead added to hot path (request execution); validation must complete in <500ms; mock server startup in <1s  
**Constraints**: Must maintain backward compatibility with existing Taurus YAML configs; all existing integration tests must continue to pass; no regressions in existing functionality  
**Scale/Scope**: 13 engine modules, 3 parser modules, 2 model files, 10 integration tests, 6 CLI flags, ~4000 lines of Rust

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Library-First | ✅ PASS | Modifies existing engine modules within current library structure |
| II. CLI Interface | ✅ PASS | CLI flags produce correct stdout/stderr; --help documents available features honestly |
| III. Test-First | ⚠️ GATE | Tests must be written BEFORE implementation for each changed module. Zero-test modules (data_sources, reporting, goose, interpolation) require new unit tests first |
| IV. Integration Testing | ✅ PASS | Integration tests exist for affected areas; some need fixes (data_integration, distributed_integration, mock_integration) |
| V. Automation & Verification | ⚠️ GATE | 80% coverage must be maintained/improved. Modules with 0% coverage (goose.rs, reporting.rs, data_sources.rs, interpolation.rs) must be brought to >=80% |
| VI. Conventional & Atomic Commits | ✅ PASS | Standard practice |
| VII. History Management | ✅ PASS | Rebase to main |
| VIII. Licensing & Compliance | ✅ PASS | prometheus/opentelemetry are Apache 2.0 — safe to remove if stripping flags |
| IX. Pedantic Observability | ✅ PASS | New env/pacing/sla implementations must include DEBUG-level logging |
| X. Security-First Architecture | ✅ PASS | Security hardening is core to this feature. config/settings.env format must be respected |
| XI. Simplicity & YAGNI | ✅ PASS | Scope bounded to fixing existing gaps; no new feature creep |

### Quality Gates

| Gate | Status | Notes |
|------|--------|-------|
| Linting/Formatting | ✅ PASS | cargo clippy, cargo fmt, cargo check must pass |
| Security Audit | ⚠️ GATE | cargo audit must report no high-severity issues. May fail due to dependency vulnerabilities |
| Test Coverage | ⚠️ GATE | >=80% coverage required. Current coverage unknown — must measure baseline first |
| Code Review | ✅ PASS | All changes reviewed for constitution compliance |

## Project Structure

### Documentation (this feature)

```text
specs/004-architectural-remediation/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 API contracts
└── tasks.md             # Phase 2 (created by /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── main.rs              # CLI entrypoint — fix flag behavior
├── lib.rs               # Module declarations
├── models/
│   └── config.rs        # Extend Configuration model (label, headers, timeout, etc.)
├── parser/
│   ├── mod.rs
│   ├── yaml.rs, json.rs, toml.rs   # Parser implementations
├── normalizer/
│   └── mod.rs
├── translator/
│   └── mod.rs            # Extend HTTP method support (PUT/DELETE/PATCH/HEAD/OPTIONS)
└── engine/
    ├── mod.rs            # Populate PummelError enum variants
    ├── env.rs            # Implement EnvironmentLoader
    ├── pacing.rs         # Implement PacingEngine
    ├── sla.rs            # Implement SlaEngine
    ├── goose.rs          # Add CLI reporter call after execution
    ├── control_flow.rs   # No changes expected
    ├── data_sources.rs   # Add path validation, file size limits
    ├── extraction.rs     # No changes expected
    ├── interpolation.rs  # No changes expected
    ├── macros.rs         # Add env var allowlist/blocklist
    ├── mock.rs           # Fix bind address to 127.0.0.1
    ├── reporting.rs      # Add CliReporter
    └── validation.rs     # No changes expected

tests/
├── skeleton_integration.rs
├── state_integration.rs
├── data_integration.rs   # Fix: create mock users.csv
├── distributed_integration.rs  # Fix: remove or implement properly
├── dry_run_integration.rs
├── mock_integration.rs   # Fix: replace sleep-based race condition
├── sla_integration.rs    # Strengthen: add threshold, verify breach
├── shorthand_integration.rs
└── branching_integration.rs
```

**Structure Decision**: Single Rust project (existing structure unchanged). All remediation work happens in existing files under `src/engine/`, `src/models/config.rs`, `src/translator/mod.rs`, and `src/main.rs`. No new source files needed — the no-op modules (env.rs, pacing.rs, sla.rs) already exist as stubs.

## Complexity Tracking

No violations — feature is well-scoped, uses existing project structure, and adds no new architectural layers.
