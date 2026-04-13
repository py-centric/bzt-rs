# Tasks: Dry-Run and Mock Server

**Input**: Design documents from `/specs/002-dry-run-mock-server/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: As per Constitution Principle III, TDD is MANDATORY. Tests MUST be written and fail before implementation.

**Commits**: As per Constitution Principle VI, an ATOMIC COMMIT must be created immediately after the completion of each discrete task. Use conventional commit messages.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Rust project**: `src/`, `tests/` at repository root.
- Use `src/lib.rs` or `src/main.rs` as entry points.
- Submodules in `src/[module]/mod.rs` or `src/[module].rs`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [x] T001 Verify `axum` and `tokio` dependencies in `Cargo.toml` are ready for mock server implementation
- [x] T002 [P] Create placeholder modules for validation and mock in `src/engine/validation.rs` and `src/engine/mock.rs`
- [x] T003 [P] Register new submodules in `src/engine/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T004 Define `MockResponse` and `ValidationSummary` internal models in `src/engine/mock.rs` and `src/engine/validation.rs`
- [x] T005 [P] Setup shared error types for validation and mock server in `src/lib.rs` or `src/engine/mod.rs`
- [x] T006 [P] Add `--dry-run`, `--mock`, and `--mock-run` flags to `Args` struct in `src/main.rs` using `clap`

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Configuration Validation (Dry-Run) (Priority: P1) 🎯 MVP

**Goal**: Validate configuration syntax and dependencies without executing the test.

**Independent Test**: Run `bzt-rs --dry-run config.yaml` and verify exit codes (0 for valid, 1 for invalid/missing files).

### Tests for User Story 1 (MANDATORY - TDD) ⚠️

- [x] T007 [P] [US1] Unit tests for file existence validation of `data-sources` in `src/engine/validation.rs`
- [x] T008 [P] [US1] Integration tests for `--dry-run` success and failure scenarios in `tests/dry_run_integration.rs`

### Implementation for User Story 1

- [x] T009 [US1] Implement `ValidationSummary` and CLI reporting logic in `src/engine/validation.rs`
- [x] T010 [US1] Implement configuration normalization and translation check in the validation pipeline in `src/engine/validation.rs`
- [x] T011 [US1] Integrate `validate_config` call in `src/main.rs` when `--dry-run` flag is present
- [x] T012 [US1] Add pedantic DEBUG logging for each validation step in `src/engine/validation.rs`

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently.

---

## Phase 4: User Story 2 - Local Functional Testing (Mock Server) (Priority: P2)

**Goal**: Start a local mock server based on scenario definitions.

**Independent Test**: Start mock server and verify it returns assertion-based bodies for defined URLs.

### Tests for User Story 2 (MANDATORY - TDD) ⚠️

- [x] T013 [P] [US2] Unit tests for assertion-to-body mapping logic in `src/engine/mock.rs`
- [x] T014 [P] [US2] Integration tests for mock server startup and basic request handling in `tests/mock_integration.rs`

### Implementation for User Story 2

- [x] T015 [US2] Implement dynamic `axum` route handler that matches `Configuration` requests in `src/engine/mock.rs`
- [x] T016 [US2] Implement `MockResponse` generator that concatenates "contains" assertions in `src/engine/mock.rs`
- [x] T017 [US2] Implement random port assignment (0.0.0.0:0) and address reporting in `src/engine/mock.rs`
- [x] T018 [US2] Integrate mock server startup in `src/main.rs` when `--mock` flag is present
- [x] T019 [US2] Add pedantic logging for incoming mock requests and matched responses in `src/engine/mock.rs`

**Checkpoint**: At this point, User Story 2 should work independently.

---

## Phase 5: User Story 3 - Integrated Mock Execution (Priority: P3)

**Goal**: Run a complete load test against an automatically managed mock server.

**Independent Test**: Run `bzt-rs --mock-run config.yaml` and verify test passes against the internal mock.

### Tests for User Story 3 (MANDATORY - TDD) ⚠️

- [x] T020 [P] [US3] Integration test for the full `--mock-run` lifecycle (start -> run -> stop) in `tests/mock_integration.rs`

### Implementation for User Story 3

- [x] T021 [US3] Update `StateTranslator::translate` or add a helper to override the `host` setting in `src/translator/mod.rs`
- [x] T022 [US3] Implement the orchestration logic in `src/main.rs` to start the mock server task, update host, and run the attack
- [x] T023 [US3] Ensure graceful shutdown of the mock server task after `run_attack` completes in `src/main.rs`
- [x] T024 [US3] Add meaningful INFO logging for the integrated mock-run lifecycle in `src/main.rs`

**Checkpoint**: User Stories 1, 2, and 3 are functional and integrated.

---

## Phase 6: Polish, Quality Gates & Documentation

**Purpose**: Final verification and exhaustive documentation

- [x] T025 [P] Run `cargo fmt`, `clippy`, and `audit` for quality pass
- [x] T026 [P] Verify code coverage >= 80% using `cargo-llvm-cov`
- [x] T027 [P] Update Mermaid diagrams in `docs/architecture.md` to include validation and mock server components
- [x] T028 Run `quickstart.md` validation on the final implementation
- [x] T029 [US1, US2] Verify performance targets (SC-001: Dry-run < 500ms, SC-002: Mock startup < 1s) using execution timing logs

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)** & **Phase 2 (Foundational)**: MUST be completed first.
- **Phase 3 (US1)**: Can start after Phase 2. MVP milestone.
- **Phase 4 (US2)**: Can start after Phase 2.
- **Phase 5 (US3)**: Depends on US2 implementation (mock server logic).
- **Phase 6 (Polish)**: Final verification.

### Parallel Opportunities

- T002 and T003 can run in parallel.
- All Foundational tasks (T004-T006) can run in parallel.
- US1 (T007-T012) and US2 (T013-T019) can be developed in parallel after the Foundation is ready.
- All tasks marked [P] are candidates for parallel execution.

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Setup + Foundational.
2. Complete US1 (Dry-Run).
3. **STOP and VALIDATE**: Verify users can now validate their configs without external tools.

### Incremental Delivery

1. Add US2 (Mock Server) to enable local functional debugging.
2. Add US3 (Mock-Run) to streamline CI/CD integration.
3. Polish and Document.
