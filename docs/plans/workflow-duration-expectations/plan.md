# Plan: Workflow Duration Expectations

## Objective

Replace the universal diagnostics `Progress` presentation with backend-owned
duration intelligence that compares current node and run elapsed time against
durable timing history from previous matching workflow executions.

The resulting diagnostics UI should communicate elapsed time, typical duration
ranges, and faster/slower-than-usual indicators without implying that arbitrary
workflow nodes expose accurate percentage progress.

## Scope

### In Scope

- Add a backend-owned timing-history contract for workflow run and node
  durations.
- Persist completed timing observations in SQLite-backed diagnostics storage.
- Project timing expectations into Tauri diagnostics snapshots.
- Replace the main diagnostics progress display with duration comparison
  indicators in overview and timeline views.
- Preserve structured progress detail for nodes that genuinely report
  node-specific progress facts.
- Update README/API contract documentation for touched directories.
- Add tests and guardrails proving timing estimates are backend-owned,
  statistically bounded, and absent when history is insufficient.
- Refactor touched surrounding code that is non-compliant with the coding
  standards, even when the non-compliance was pre-existing.

### Out of Scope

- Manufacturing percentage progress for nodes that do not report bounded work.
- Predicting exact completion time for opaque or highly variable runtimes.
- Replacing scheduler queue timing, runtime warmup timing, or model/license
  usage ledger semantics.
- Changing workflow execution behavior.
- Full diagnostics UI redesign outside the duration/progress replacement.
- Full durable trace-store replacement beyond the timing-history data needed
  for this feature.

## Inputs

### Problem

The diagnostics UI currently shows `Progress` as `No progress` whenever a node
has not emitted scalar `NodeProgress` telemetry. This is technically accurate
for the existing field, but misleading for users because a node can run,
complete, and produce output without exposing meaningful percentage progress.

For most node types, accurate universal progress is not available. Historical
duration comparison is a more defensible diagnostics signal because the system
already records run and node timing, and repeated executions of the same
workflow can provide a useful expected range.

### Constraints

- Backend Rust remains the source of truth for diagnostics facts.
- Frontend diagnostics components must render snapshots declaratively and must
  not compute or persist backend-owned timing history.
- Timing history must be persisted in SQLite through a backend-owned storage
  boundary, with migrations and retention semantics owned by Rust.
- Estimates must be ranges or classifications, not fake precision.
- Timing expectations must be omitted or marked limited when sample size is too
  small.
- Existing structured progress detail must remain available for nodes with
  real bounded-progress domains such as downloads, fixed-step tasks, cache
  operations, or batch work.
- Touched modules must conform to the standards in
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.
- Existing unrelated dirty files must not be modified, staged, or committed by
  this work.

### Assumptions

- Matching history by `workflow_id`, `graph_fingerprint`, `node_id`,
  `node_type`, and runtime/backend identity is sufficient for the first
  implementation.
- Run-level history can use `workflow_id` plus `graph_fingerprint`, with
  workflow name used only as display metadata.
- Completed and cancelled/failed runs should be stored, but baseline estimates
  should initially use successful completed durations unless a later
  requirement asks for failure-specific baselines.
- Median plus percentile range is preferable to mean-only estimates.
- SQLite persistence can extend the existing diagnostics-ledger ownership or a
  sibling diagnostics-timing storage boundary, but the plan must choose one
  before implementation.

### Dependencies

- `crates/pantograph-workflow-service/src/trace/`: canonical run and node
  timing facts.
- `crates/pantograph-diagnostics-ledger/`: existing durable SQLite diagnostics
  storage and migration patterns.
- `src-tauri/src/workflow/diagnostics/`: Tauri diagnostics projection DTOs and
  overlay merge logic.
- `src/services/diagnostics/types.ts`: frontend diagnostics DTO mirror.
- `src/components/diagnostics/`: overview, timeline, and presentation helpers.
- Existing verification commands: `cargo test -p pantograph-workflow-service`,
  `cargo test -p pantograph-diagnostics-ledger`, `cargo check --manifest-path
  src-tauri/Cargo.toml`, `npm run typecheck`, `npm run test:frontend`, and
  `git diff --check`.

### Affected Structured Contracts

- `WorkflowTraceSummary` and `WorkflowTraceNodeRecord` remain canonical timing
  producers.
- Diagnostics snapshot DTOs gain optional timing expectation fields using
  stable enum labels and nullable/omitted semantics.
- Frontend diagnostics TypeScript types mirror those backend DTOs.
- SQLite schema gains versioned timing observation tables or a new
  diagnostics-timing schema, depending on the storage-boundary decision.
- `Progress` is removed from primary diagnostics presentation or renamed to
  node-specific reported progress detail where retained.

### Affected Persisted Artifacts

- SQLite diagnostics timing history.
- Diagnostics ledger schema migration metadata if the existing ledger owns the
  new timing tables.
- Tests and fixtures that serialize diagnostics snapshots.

### Concurrency and Lifecycle Review

- Timing observation persistence happens on terminal trace events or snapshot
  finalization, not from frontend polling.
- SQLite writes must be transactional and must not hold unrelated workflow
  session locks across blocking database work.
- Duplicate terminal events and replay/restart paths must not double-count the
  same execution/node observation.
- Clearing diagnostics history must define whether it clears timing history,
  trace history, or only retained overlays.
- Retention/pruning must be explicit so historical timing does not grow without
  bounds.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Timing history is matched too broadly and gives misleading expectations | High | Use graph fingerprint, node identity, node type, and runtime/backend facts in match keys. Show no estimate when keys are incomplete or sample size is too low. |
| Mean duration is distorted by cold starts or outliers | Medium | Use median and percentile ranges; include sample count and outlier-aware classifications. |
| SQLite diagnostics ledger becomes a mixed-responsibility crate | Medium | Make a storage-boundary decision up front. If extending `pantograph-diagnostics-ledger` would violate its model/license README contract, create or split a focused timing-history module/crate. |
| Tauri overlay continues to own scalar progress | Medium | Move universal diagnostics presentation to backend-projected timing expectation fields. Keep overlay-only progress only for optional node-specific detail. |
| Frontend starts computing estimates locally | High | Add tests and README contract text requiring timing expectations to come from backend DTOs. |
| Duplicate replay or retry events inflate sample counts | High | Add idempotency keys based on execution id and node id; verify replay recovery. |
| Touched Svelte component exceeds standards threshold | Medium | Decompose overview/timeline presentation helpers or subcomponents during implementation if edited files grow materially. |

## Clarifying Questions

- None blocking.
- Assumption: clearing diagnostics history should clear retained traces and
  overlays, but durable timing history should use its own retention/pruning
  command unless product requirements later define a destructive clear-all
  diagnostics action.

## Definition of Done

- Diagnostics no longer presents `No progress` as a universal node status in
  the overview or timeline.
- Running and completed nodes show duration plus backend-projected timing
  expectation where enough comparable history exists.
- Timing expectation DTOs include sample count, median, typical range, current
  comparison classification, and a clear absence/limited-history state.
- SQLite persists timing observations with schema migration and retention
  tests.
- Replayed or duplicate terminal events do not double-count observations.
- Frontend rendering is declarative and does not compute historical estimates.
- Structured progress detail remains available for nodes that genuinely report
  it, but it is not treated as universal progress.
- Touched READMEs document the new contract and ownership.
- Touched surrounding code has been checked and refactored for standards
  compliance where required.
- Required Rust, TypeScript, frontend, and cross-layer tests pass.

## Milestones

### Milestone 1: Freeze Timing Expectation Contract

**Goal:** Define the backend-owned DTO and matching semantics before changing
storage or UI.

**Tasks:**
- [x] Define timing expectation DTOs with enum classifications such as
  `insufficient_history`, `within_expected_range`, `faster_than_expected`, and
  `slower_than_expected`.
- [x] Define observation match keys for run-level and node-level history.
- [x] Define minimum sample count and percentile range policy.
- [x] Decide whether timing history extends `pantograph-diagnostics-ledger` or
  gets a focused sibling persistence boundary.
- [x] Record the clear-history and retention policy for timing observations.
- [ ] Update this plan if the storage-boundary decision changes affected files.

**Verification:**
- Review against `CODING-STANDARDS.md` backend-owned data and layered
  architecture rules.
- Review against `RUST-API-STANDARDS.md` for typed enums, validated boundary
  inputs, and `Result`-based public APIs.
- Review against `DOCUMENTATION-STANDARDS.md` structured producer contract
  requirements.

**Status:** Complete.

### Milestone 2: Persist Timing Observations

**Goal:** Store completed run/node timing observations durably and
idempotently.

**Tasks:**
- [x] Add SQLite schema/migration for timing observations or a new focused
  timing-history store.
- [ ] Persist run and node observations from backend-owned terminal trace
  state.
- [x] Store execution id and node id idempotency keys to prevent duplicate
  observation counts.
- [x] Store match-key fields needed for future lookup, including graph
  fingerprint and runtime/backend facts when available.
- [x] Add retention/pruning behavior for timing observations.
- [x] Update persistence README/API contract documentation.

**Verification:**
- `cargo test -p pantograph-diagnostics-ledger` or the selected timing storage
  crate.
- New SQLite migration tests for fresh database, existing schema, unsupported
  schema version, idempotent writes, and retention pruning.
- Global-state/durable-resource isolation review from `TESTING-STANDARDS.md`.

**Status:** In progress.

### Milestone 3: Project Timing Expectations Into Workflow Diagnostics

**Goal:** Attach historical timing expectations to backend trace diagnostics
without moving ownership to Tauri or the frontend.

**Tasks:**
- [ ] Add workflow-service query/use-case APIs for run and node timing
  expectations.
- [ ] Merge timing expectations into Tauri-facing `DiagnosticsRunTrace` and
  `DiagnosticsNodeTrace` DTOs.
- [ ] Keep scalar reported progress separate from duration expectations and
  mark it optional/node-specific.
- [ ] Ensure running nodes compare elapsed time against historical ranges.
- [ ] Ensure completed nodes compare final duration against historical ranges.
- [ ] Add replay and duplicate-event tests so expectations remain stable after
  recovery.
- [ ] Update `src-tauri/src/workflow/diagnostics/README.md` and
  `crates/pantograph-workflow-service/src/trace/README.md`.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Focused Tauri diagnostics tests covering insufficient history, within range,
  faster, slower, running-over-expected, and duplicate replay behavior.
- Cross-layer acceptance check from persisted timing observation to diagnostics
  snapshot DTO.

**Status:** Not started.

### Milestone 4: Replace Frontend Progress Presentation

**Goal:** Render backend timing expectations in the diagnostics UI and remove
misleading universal progress copy.

**Tasks:**
- [ ] Update TypeScript diagnostics DTOs to mirror backend timing expectation
  fields.
- [ ] Replace `formatDiagnosticsPercent()` usage in overview and timeline with
  duration/timing expectation presenters.
- [ ] Remove the primary `Progress` row/column or rename any retained scalar
  field to `Reported Progress` only where node-specific progress detail exists.
- [ ] Render limited/no-history states explicitly without implying an error.
- [ ] Add visual indicators for faster/slower/within-range states using the
  existing restrained diagnostics style.
- [ ] Keep Svelte files declarative and extract presenter helpers if component
  size grows past standards thresholds.
- [ ] Update `src/components/diagnostics/README.md`.

**Verification:**
- `npm run typecheck`
- `npm run test:frontend`
- Presenter tests for duration expectation formatting and classification labels.
- Component tests or snapshot-level checks for no history, running over
  expected, completed faster, completed slower, and structured progress detail
  fallback.
- Accessibility review for indicator labels and non-color-only status
  communication.

**Status:** Not started.

### Milestone 5: Standards Compliance Refactor Pass

**Goal:** Ensure every touched area and its immediate surroundings comply with
the applicable standards after the feature works.

**Tasks:**
- [ ] Re-check file sizes and responsibility boundaries for touched Svelte,
  TypeScript, Rust diagnostics, trace, and persistence files.
- [ ] Refactor oversized or mixed-responsibility touched files where the
  implementation made the violation worse or the surrounding code now blocks a
  compliant change.
- [ ] Verify every touched `src/` directory has a standards-compliant
  `README.md` with updated API/structured producer contracts where applicable.
- [ ] Ensure no frontend component owns backend timing state or performs local
  historical estimation.
- [ ] Ensure Rust APIs use typed enums/newtypes where the contract crosses crate
  or host boundaries.
- [ ] Ensure SQLite tests isolate durable state and do not depend on shared
  process-global state.
- [ ] Record any unrelated issues that are found but intentionally deferred.

**Verification:**
- `git diff --check`
- `npm run typecheck`
- `npm run test:frontend`
- `cargo test -p pantograph-workflow-service`
- `cargo test -p pantograph-diagnostics-ledger` or the selected timing storage
  crate.
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Documentation traceability review against `DOCUMENTATION-STANDARDS.md`.

**Status:** Not started.

## Compliance Review For Touched Areas

### Current Findings To Resolve During Implementation

- `src/components/diagnostics/DiagnosticsOverview.svelte` is currently 263
  lines, which exceeds the 250-line UI component review trigger. If this file
  is edited, implementation must either extract focused subcomponents/helpers or
  document why the resulting shape remains safe. The preferred path is to
  extract node timing-detail rendering or table column presentation.
- `src-tauri/src/workflow/diagnostics/types.rs` is currently 446 lines. Adding
  timing DTOs may keep it under 500 lines but increases DTO responsibility.
  If the file grows materially, extract timing expectation DTOs into a focused
  `timing.rs` module and re-export through `mod.rs`.
- `src-tauri/src/workflow/diagnostics/overlay.rs` currently owns progress
  overlay state and retained events in one file. If scalar progress is changed,
  review whether progress-detail overlay handling should move into a smaller
  overlay helper instead of adding timing concerns here.
- `crates/pantograph-workflow-service/src/trace/types.rs` is currently 374
  lines. Timing expectation query/result contracts may belong in a new
  `trace/timing.rs` or workflow diagnostics API module instead of expanding
  trace type definitions indefinitely.
- `crates/pantograph-diagnostics-ledger/README.md` currently describes the
  crate as model/license usage specific. If timing history is added there, the
  README and crate-level docs must be updated to avoid a contract mismatch.
  If that broadening is undesirable, create a focused timing-history boundary.

### Standards Applied

- `PLAN-STANDARDS.md`: this plan includes objective, scope, inputs, affected
  contracts/artifacts, concurrency review, milestones, verification,
  re-plan triggers, and completion criteria.
- `CODING-STANDARDS.md`: backend-owned data, layered architecture, named
  constants, file-size review, and single-owner state-flow rules.
- `FRONTEND-STANDARDS.md`: declarative rendering, no frontend polling or local
  backend-state ownership, and accessible status indicators.
- `TESTING-STANDARDS.md`: durable SQLite isolation, replay/recovery checks,
  cross-layer acceptance coverage, and descriptive test names.
- `DOCUMENTATION-STANDARDS.md`: update READMEs for changed directory
  responsibilities, API consumer contracts, and structured producer contracts.
- `RUST-API-STANDARDS.md`: typed enums/newtypes for boundary contracts,
  `Result` errors, public API documentation, and no `unwrap`/`expect` in
  production paths.
- `CONCURRENCY-STANDARDS.md`: avoid holding unrelated locks across SQLite
  writes and keep state transitions single-owner.

## Execution Notes

- 2026-04-25: Plan created. Existing unrelated dirty files were present before
  this plan and must remain untouched unless the user explicitly assigns them
  to this work.
- 2026-04-25: Milestone 1 completed. Timing history will extend
  `pantograph-diagnostics-ledger` because it already owns durable diagnostics
  SQLite storage. Added typed timing observation/query/expectation contracts,
  minimum sample-count policy, percentile range classification, and pure ledger
  tests for insufficient, faster, within-range, and slower states. Verification:
  `cargo test -p pantograph-diagnostics-ledger`.
- 2026-04-25: Milestone 2 storage slice completed. Added diagnostics ledger
  schema version 2 for `workflow_timing_observations`, v1-to-v2 migration,
  idempotent observation recording, expectation lookup over completed
  observations, and timing retention pruning. Trace terminal-event submission
  remains for the next slice. Verification:
  `cargo test -p pantograph-diagnostics-ledger`.

## Commit Cadence Notes

- Commit when each logical slice is implemented and verified.
- Keep storage/schema changes, API projection changes, frontend presentation
  changes, and compliance refactors in separate atomic commits unless a test
  fixture must move with its contract.
- Follow `COMMIT-STANDARDS.md` for detailed conventional commit messages.

## Optional Subagent Assignment

No subagents are required for the initial implementation. If the work is split,
use one worker wave only after Milestone 1 freezes contracts.

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| Backend worker | SQLite timing persistence and workflow-service expectation API | Committed Rust changes, tests, and README updates within storage/workflow-service write set | After Milestone 2 verification passes |
| Frontend worker | Diagnostics DTO mirror and UI presentation | Committed TypeScript/Svelte changes, presenter tests, and README updates within frontend diagnostics write set | After backend DTO shape is frozen |

Workers must use separate worktrees or temporary clones if they are allowed to
commit. Shared DTO/schema files must be owned by exactly one worker or handled
serially by the integrator.

## Re-Plan Triggers

- The storage-boundary decision changes from extending
  `pantograph-diagnostics-ledger` to creating a new crate, or the reverse.
- Historical timing must include failed/cancelled runs in baseline estimates.
- Runtime/backend identity is unavailable often enough that match keys are too
  weak to produce useful estimates.
- Diagnostics clear-history semantics are changed to delete durable timing
  history.
- Touched files require a broader refactor than the current milestone can
  safely contain.
- Verification shows duplicate replay events can still inflate timing samples.

## Recommendations

- Prefer naming the UI concept `Expected Duration` or `Typical Duration`, not
  `Progress`.
- Keep scalar progress events internally as optional node-specific telemetry,
  but remove them from universal overview/timeline columns.
- Prefer p25-p75 or p10-p90 ranges plus sample count over averages alone.
- Use `No history` and `Limited history` states instead of silently hiding
  timing expectation fields.

## Completion Summary

### Completed

- Not started.

### Deviations

- None.

### Follow-Ups

- None yet.

### Verification Summary

- Not run. This is a planning artifact only.

### Traceability Links

- Plan: `docs/plans/workflow-duration-expectations/plan.md`
- Module README updates: pending implementation.
- ADR added/updated: pending implementation decision on durable timing
  ownership.
