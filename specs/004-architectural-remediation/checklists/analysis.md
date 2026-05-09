# Specification Analysis Report: Architectural Remediation

## Findings

| ID | Category | Severity | Location(s) | Summary | Recommendation |
|----|----------|----------|-------------|---------|----------------|
| A1 | Duplication | HIGH | spec.md:FR-005, FR-006 | FR-005 ("errors carry structured context") and FR-006 ("errors use structured type") are near-identical — both describe structured error handling with overlapping scope | Merge into a single requirement: "All errors MUST use a structured type carrying source context, error classification, and root cause chain" |
| B1 | Ambiguity | MEDIUM | spec.md:FR-015 | "restricted to an allowlist or exclude sensitive patterns" uses ambiguous OR — is it allowlist, blocklist, or both? research.md decides blocklist, spec leaves options open | Replace with: "restricted by a blocklist of sensitive variable name patterns" to match research.md decision |
| B2 | Ambiguity | LOW | spec.md:SC-003 | "steady-state conditions" not defined — how long must the system run before measurement is valid? | Add definition: "steady-state means after ramp-up completes and at least 30s of sustained traffic" |
| C1 | Underspecified | MEDIUM | spec.md:FR-002 | "consistent priority ordering" not defined in spec — research.md defines CLI > env > .env > defaults, but spec doesn't document this | Add priority order to FR-002 description or Assumptions section |
| C2 | Underspecified | MEDIUM | spec.md:FR-016 | "all standard HTTP methods" not enumerated in spec — only clarified in conversation history | Add explicit list: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS |
| E1 | Coverage Gap | MEDIUM | spec.md:SC-003 → tasks.md | SC-003 requires rate limits "within 5%" but no task verifies the 5% accuracy tolerance | Add a verification task in Phase 9: "Measure pacing accuracy against configured rate, verify within 5%" |
| E2 | Coverage Gap | LOW | tasks.md:T004 | Prometheus/opentelemetry removal has no corresponding FR — it's an implementation decision derived from research, not a spec requirement | Either remove from tasks (out of scope for this feature) or add an explicit requirement in spec |
| E3 | Coverage Gap | LOW | tasks.md:T080 | Interpolator unit tests have no corresponding FR — it's a quality gap from the architecture review, not a spec requirement | Add to spec as FR-017 or note in Assumptions as implicit quality requirement |
| F1 | Inconsistency | MEDIUM | spec.md:FR-001 ↔ tasks.md:T021-T023 | FR-001 allows flags to "work or produce error" but tasks REMOVE the flags entirely. Spec and plan disagree on approach | Align spec with plan: change FR-001 to "CLI flags for unimplemented features MUST NOT silently no-op — they MUST be either removed or rejected with a clear error" |
| F2 | Inconsistency | MEDIUM | spec.md → tasks.md, data-model.md | Entity models (PacingConfig, SlaCriterion, DataSourceDefinition) defined in data-model.md are only partially referenced in spec. Spec mentions them vaguely ("pacing", "structured data-sources") but doesn't detail structure | Add entity references or summaries in spec's Key Entities section for traceability |
| F3 | Ordering Issue | MEDIUM | spec.md:FR-014 ↔ tasks.md:T003, T077 | FR-014 requires "all existing integration tests must pass" but critical test fix (T077 data_integration.rs) is in Phase 9, not Phase 1 where T003 creates the fixture | Move T077 to Phase 3 (US1) or ensure FR-014 explicitly states "by the end of the feature, not from Phase 1" |
| G1 | Missing Artifact | LOW | spec.md, tasks.md, plan.md | No changelog/version entry documenting the breaking CLI flag removals | Add a task in Phase 9: "Document CLI flag removals in project changelog or version bump" |

## Coverage Summary Table

| Requirement Key | Has Task? | Task IDs | Notes |
|-----------------|-----------|----------|-------|
| FR-001 (CLI flags no no-op) | ✅ | T019-T027 | 9 tasks across tests + implementation |
| FR-002 (env var resolution) | ✅ | T028, T033, T040 | env.rs module |
| FR-003 (SLA criteria) | ✅ | T030, T035, T037, T038, T079 | sla.rs + inline refactor |
| FR-004 (pacing/rps limits) | ✅ | T029, T034, T036, T039 | pacing.rs |
| FR-005 (contextual errors) | ✅ | T042-T047 | US3 error messages |
| FR-006 (structured error type) | ✅ | T005-T018 | BztError migration (foundational) |
| FR-007 (path traversal) | ✅ | T048, T051 | data_sources.rs |
| FR-008 (file size limit) | ✅ | T049, T052 | data_sources.rs |
| FR-009 (deny unknown fields) | ✅ | T068, T074 | config model + parser |
| FR-010 (localhost bind) | ✅ | T054 | mock.rs |
| FR-011 (CLI summary table) | ✅ | T056-T060 | reporting.rs |
| FR-012 (protocol stub errors) | ✅ | T025 | translator/mod.rs |
| FR-013 (Taurus fields) | ✅ | T061-T075 | config model + wiring |
| FR-014 (tests pass) | ✅ | T003, T077 | fixture + fix |
| FR-015 (env var blocklist) | ✅ | T050, T053, T055 | macros.rs |
| FR-016 (all HTTP methods) | ✅ | T076 | translator/mod.rs |
| SC-001 (no silent no-op) | ✅ | T019-T027 | Covered by US1 |
| SC-002 (env var resolve) | ✅ | T028, T033, T040 | Covered by US2 |
| SC-003 (5% rate accuracy) | ⚠️ Partial | T029, T034 | No verification task for the 5% tolerance |
| SC-004 (3+ context pieces) | ✅ | T042-T047 | Covered by US3 |
| SC-005 (traversal rejected) | ✅ | T048, T051 | Covered by US4 |
| SC-006 (unknown field warnings) | ✅ | T068, T074 | Covered by US6 |
| SC-007 (terminal output) | ✅ | T056-T060 | Covered by US5 |
| SC-008 (8 tests pass) | ✅ | T003, T077, T078, T079 | Multiple tasks |
| SC-009 (Taurus fields apply) | ✅ | T061-T075 | Covered by US6 |

## Constitution Alignment Issues

**None.** All constitution principles are respected:
- **III (Test-First)**: TDD sections in every US phase ✅
- **V (Coverage)**: T083 mandates >=80% coverage ✅
- **IX (Pedantic Observability)**: T027, T041, T047, T055, T060, T075 all add DEBUG logging ✅
- **X (Security-First)**: T001-T002 for config/env files; T048-T055 for security hardening ✅
- **VI (Atomic Commits)**: Header states atomic commit requirement ✅

## Unmapped Tasks

| Task | Description | Notes |
|------|-------------|-------|
| T004 | Remove prometheus/opentelemetry deps | Consequence of research decision, not explicitly required by spec |
| T080 | Interpolator unit tests | Quality gap fill — no explicit FR, but consistent with testing principles |
| T018 | Verify #[allow] annotations | Polish clean-up, no explicit FR |

## Metrics

| Metric | Value |
|--------|-------|
| Total Requirements (FR) | 16 |
| Total Success Criteria (SC) | 9 |
| Total Tasks | 85 |
| Coverage % (FR with >=1 task) | 100% (16/16) |
| Ambiguity Count | 2 (B1-B2) |
| Duplication Count | 1 (A1) |
| Critical Issues | 0 |
| High Severity | 1 (A1) |
| Medium Severity | 6 (B1, C1, C2, E1, F1, F2, F3) |
| Low Severity | 3 (B2, E2, E3, G1) |

## Next Actions

**0 CRITICAL issues.** The feature is well-specified and fully covered. The 1 HIGH issue (A1) and 6 MEDIUM issues are refinement-level concerns.

**Recommended before `/speckit.implement`:**
1. Fix **A1** (HIGH): Merge FR-005 and FR-006 in spec.md to eliminate duplication
2. Fix **F1** (MEDIUM): Align spec.md FR-001 with plan's decision to strip flags
3. Fix **C1** (MEDIUM): Add env var priority order to spec.md FR-002
4. Fix **E1** (MEDIUM): Add 5% accuracy verification task for pacing

These 4 fixes can be done via a quick edit to `spec.md` and `tasks.md` before implementation begins. The remaining findings (C2, F2, F3, B2, E2, E3, G1) are LOW severity and can be addressed during implementation.

Would you like me to suggest concrete remediation edits for the top issues?
