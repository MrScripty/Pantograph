# 07: Verification, Rollout, And Refactor Gates

## Status

Draft plan. Not implemented.

## Objective

Define the verification, documentation, worktree hygiene, rollout, and
refactor-gate rules for executing the run-centric GUI workbench stages safely.

## Scope

### In Scope

- Stage-start worktree hygiene.
- Standards checklist.
- Per-stage verification expectations.
- Documentation and ADR update expectations.
- Commit cadence expectations.
- Cutover strategy for replacing the current identity/version contracts and
  app shell.
- Criteria for expanding a stage into parallel implementation waves.

### Out of Scope

- Actual implementation of previous stages.
- CI infrastructure changes unrelated to this work.
- PR publication.

## Inputs

### Problem

The requirements span backend persistence, scheduler policy, APIs, and a major
frontend shell change. Without explicit gates, the implementation can easily
mix concerns, leave dirty files between slices, or ship frontend pages before
backend truth exists.

### Constraints

- Follow the coding standards under
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.
- Commit logical slices after verification.
- Do not modify or revert unrelated dirty files.
- Source directory README files must stay current.
- Cross-layer features need cross-layer acceptance checks.
- Frontend polling/timers need lifecycle ownership and cleanup tests.
- New diagnostic facts must use typed event contracts, validated payloads, and
  backend-owned event builders.

### Assumptions

- Implementation will happen over multiple turns/commits.
- Some stages may need follow-up ADRs.
- Parallel workers are optional and should only be used after contracts are
  frozen and write sets can be separated.

### Dependencies

- All previous stage plans.
- Repo scripts from `package.json`.
- Cargo workspace tests for touched crates.
- Existing traceability and lint scripts.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Implementation starts with overlapping dirty files. | High | Apply start gate and pause when write set is dirty. |
| Cross-layer behavior is only typechecked, not exercised. | High | Require acceptance checks for backend-to-frontend projections. |
| Old and new identity/version contracts coexist in active paths. | High | Treat Stage `01` as a breaking cutover and reject, delete, or regenerate old records that cannot satisfy new invariants. |
| New diagnostics bypass typed event validation. | High | Require event-builder tests and source audits for any new diagnostic/scheduler/audit fact. |
| Projection rebuild cost grows with total event history. | High | Require materialized projection cursors, hot/warm/cold update classes, startup/page-load no-full-replay checks, and non-trivial event-count projection tests. |
| READMEs/ADRs drift from new ownership. | Medium | Update docs in the same logical slice as ownership changes. |
| A stage becomes too broad for one implementer. | Medium | Split into implementation waves with non-overlapping write sets. |
| Shell rollout leaves two competing navigation models. | Medium | Relocate or retire old surfaces under one workbench shell. |

## Definition of Done

- Each implementation stage has a recorded verification summary.
- Required READMEs and ADRs are updated with accepted ownership boundaries.
- New diagnostic, scheduler, I/O, retention, and Library audit facts are
  emitted through typed event builders or explicitly documented as
  non-diagnostic authoritative state.
- Ledger-derived page projections are materialized read models with
  projection-version and event-sequence cursors. Full rebuild is an explicit
  maintenance/migration/test path, not normal page-load behavior.
- Scheduler-first shell is rolled out with explicit relocation or retirement of
  old surfaces.
- Old persisted workflow/run data that cannot satisfy the new identity/version
  contracts is explicitly deleted, ignored, or regenerated.
- No implementation stage begins with dirty overlapping source/test/config files
  unless explicitly allowed.
- Each logical implementation slice is committed after verification.
- Any skipped verification is recorded with reason and residual risk.

## Stage-Start Gate

Before editing source code for any numbered stage:

- [ ] Read this file, the selected stage file, and relevant standards.
- [ ] Run `git status --short`.
- [ ] Identify primary write set and adjacent write set.
- [ ] Confirm dirty implementation files do not overlap the write set.
- [ ] Review affected module READMEs.
- [ ] Decide whether an ADR is required before implementation.
- [ ] Decide whether the stage is serial or needs implementation waves.
- [ ] Define the smallest logical slice to implement first.

If dirty implementation files overlap the selected stage and are not yours,
stop and ask for direction. Do not revert them.

## Per-Stage Verification Guidance

### Stage 01

Expected checks:

- `cargo test -p pantograph-workflow-service`
- `cargo test -p pantograph-diagnostics-ledger`
- `cargo test -p pantograph-node-contracts` if node contract metadata changes
- targeted schema/cutover tests when persistence changes
- tests proving invalid old identity/fingerprint records are rejected, deleted,
  or regenerated according to the chosen cutover rule
- run-submission transaction/idempotency tests proving workflow version
  resolution, run snapshot creation, and queue insertion cannot partially
  succeed in active paths

### Stage 02

Expected checks:

- scheduler policy/store tests in `pantograph-workflow-service`
- runtime load/unload integration tests if callbacks change
- diagnostics/event repository tests when scheduler events persist through the
  shared typed event ledger
- typed event validation tests for scheduler event kinds, schema versions,
  required correlation ids, and disallowed producers

### Stage 03

Expected checks:

- `cargo test -p pantograph-diagnostics-ledger`
- persistence migration tests
- typed event envelope and payload validation tests
- projection rebuild tests from ledger events
- projection cursor/checkpoint tests proving only events after
  `last_applied_event_seq` are applied during normal recovery
- idempotency tests for duplicate projection-application attempts
- hot/warm/cold projection class tests or contract tests proving page-critical
  projections are current and warm projections expose freshness state
- non-trivial event-count projection tests to make full-replay regressions
  visible
- tests proving normal startup/page/API read paths do not full-replay the
  ledger
- retention cleanup/replay tests
- Pumas/Library audit tests or documented local limitations
- durable-resource isolation for persistence, migration, replay, cleanup, and
  projection tests: each test must own its database/temp root/payload store and
  cache path
- artifact reference, Library/Pumas resource id, and approved-root path
  validation tests for download/delete/access and retention operations

### Stage 04

Expected checks:

- backend projection tests for touched adapters
- frontend service/presenter tests
- `npm run typecheck`
- at least one cross-layer acceptance check for run list/detail projection
- at least one acceptance check proving typed event input appears through a
  projection without exposing raw ledger rows to the page service
- native and host-language binding checks when Rustler, UniFFI, Tauri command,
  HTTP adapter, or generated binding contracts are touched
- platform-abstraction and degraded-state tests for local Network/system
  metrics providers

### Stage 05

Expected checks:

- `npm run typecheck`
- `npm run test:frontend`
- `npm run lint:full`
- accessibility checks for top-level navigation
- focused tests for active-run and page state

### Stage 06

Expected checks:

- `npm run typecheck`
- `npm run test:frontend`
- `npm run lint:full`
- focused page/presenter tests
- targeted backend tests for any projection gaps found during page work
- rejected Pumas/Library action tests proving the page does not optimistically
  mutate backend-owned state
- local Network degraded-metrics state tests for platforms or permissions where
  CPU, memory, GPU, disk, or cache facts are unavailable

### Cross-Cutting

When a stage changes traceability-sensitive decisions:

- run `npm run traceability` if docs/ADR references are affected
- update affected `README.md` files
- update or add ADRs for accepted architecture ownership changes

When a stage introduces or changes dependencies:

- record the dependency owner, reason, alternatives considered, feature scope,
  lockfile impact, and removal path if the dependency is temporary
- keep dependency ownership in the narrowest crate/package that needs it
- run the repo's package-manager validation/audit command when the changed
  dependency type has one

When a stage touches formatting, lint, accessibility, or security-sensitive
surfaces:

- confirm the current repo commands before implementation and record skipped
  checks with reason and residual risk
- include formatting checks for touched Rust/TypeScript/Svelte files where the
  repo exposes them
- include accessibility lint or focused accessibility checks for changed
  interactive frontend surfaces
- include security/path/resource validation checks for any boundary that
  accepts external identifiers, paths, payload references, or resource actions

### Source-Audit Gates

After each broad contract cutover, run source searches that prove old active
semantics are gone or explicitly quarantined:

- Stage `01`: audit `graph_fingerprint`, `derived_graph`,
  `currentGraphFingerprint`, `computeGraphFingerprint`, `workflow_id`,
  workflow version ids, run snapshot ids, and semantic version conflict errors.
- Stage `02`: audit scheduler queue, event, estimate, delay, model load, and
  model unload terms against the accepted typed scheduler event owner.
- Stage `03`: audit retention, artifact, payload, and Pumas/Library audit
  usage against the accepted typed event builders, ledger owners,
  `event_seq`, `projection_state`, and materialized projection readers. Audit
  that normal page/API paths do not call explicit full-rebuild commands.
- Stage `04`: audit Rust and TypeScript projection DTOs for field parity,
  default behavior, optional degraded states, and error taxonomy.
- Stage `05`: audit `viewMode`, canvas/workflow mode toggles, shortcuts, and
  old shell lifecycle ownership.

A remaining old field name is acceptable only when its new meaning is
documented in the owning module README and covered by tests.

## Cutover Strategy

Execute the architecture change in controlled cutovers:

1. Stage `01` replaces old workflow identity, fingerprint, version, and run
   snapshot contracts. Existing data that cannot satisfy the new invariants is
   deleted, ignored, or regenerated.
2. Stage `03` ledger bootstrap introduces the shared typed diagnostic event
   envelope, append boundary, validation, monotonic event sequences,
   projection state/cursors, and incremental materialized projection pattern
   before any durable scheduler event persistence is completed.
3. Stage `02` durable scheduler estimates/events/control persist through the
   shared event ledger and keep `scheduler.*` ownership separate from `run.*`
   lifecycle ownership.
4. Stage `03` completes diagnostics, retention, I/O artifact, runtime,
   Library/Pumas, and projection rebuild behavior before pages depend on new
   diagnostics.
5. Backend projections exist and are tested against the new contracts.
6. App shell opens Scheduler by default with backend or fixture data.
7. Existing graph/drawing surfaces are relocated into the workbench or retired.
8. Active-run context drives page shells.
9. Pages become feature-complete enough to replace old panels.
10. Old canvas/workflow toggle and ambiguous old projection APIs are removed.

## Optional Concurrent Worker Plan

Use parallel workers only after a stage contract is frozen.

Potential wave split if needed:

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| backend-versioning | Stage 01 storage/contracts | committed backend tests and README/ADR notes | workflow version API frozen |
| diagnostics-ledger-bootstrap | Stage 03 shared event envelope, append boundary, validation, event cursors, and incremental projection pattern | validation, migration, source ownership, and projection contracts | shared event ledger core frozen |
| scheduler-events | Stage 02 scheduler estimates/events | scheduler tests and typed event projections using the shared ledger | scheduler event family frozen |
| diagnostics-retention | Stage 03 retention/I/O/Library audit | retention, artifact, Library audit, incremental projection, explicit rebuild, and query contracts | artifact/retention DTO frozen |
| frontend-shell | Stage 05 shell/active-run | app shell tests and route scaffolding | shell can render page placeholders |
| frontend-pages | Stage 06 page bodies | page tests and presenter updates | backend DTOs stable |

Before launching workers, create a dedicated implementation-wave plan with:

- non-overlapping primary write sets
- allowed adjacent write sets
- forbidden shared files
- worker report paths
- integration order
- required verification after integration

## Commit Cadence Notes

- Commit after each logical slice is complete and verified.
- Keep code, tests, and docs together when they belong to the same slice.
- Keep schema/contract changes separate from broad frontend page work.
- Do not carry dirty implementation files into the next logical slice.

## Re-Plan Triggers

- A stage cannot meet its verification target with the current architecture.
- A cross-layer acceptance check reveals DTO drift.
- A page requires backend facts not planned in Stages `01` through `04`.
- Breaking contract cutover affects more source areas than the stage write set
  can safely cover.
- Typed diagnostic event volume or projection rebuild cost requires a storage
  redesign before page implementation.
- Any implementation path makes startup, Scheduler page load, run detail, or
  I/O Inspector reads depend on replaying all diagnostic events.
- Parallel workers are needed.

## Completion Summary

### Completed

- None. Draft plan only.

### Deviations

- None.

### Follow-Ups

- Convert any broad stage into implementation-wave specs before using parallel
  workers.
- Add concrete command outputs during implementation.

### Verification Summary

- Not run. Draft plan only.

### Traceability Links

- Standards: Plan Standards, Documentation Standards, Architecture Patterns,
  Frontend Standards, Testing Standards, Commit Standards.
