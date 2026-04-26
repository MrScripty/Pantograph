# Plan: Diagnostics Run History Projection

## Objective

Eliminate duplicate and stale workflow diagnostics run rows by enforcing that
only real backend-generated `workflow_run_id` values can create canonical
diagnostic run traces. Opened-workflow timing history must load from the
SQLite diagnostics ledger before any new run starts and must remain visible
independently from retained live/recent run rows.

## Scope

### In Scope

- Remove any session-id fallback that treats `session_id` as a
  `workflow_run_id`.
- Make diagnostics snapshot reads side-effect safe for canonical run history.
- Ensure scheduler/runtime diagnostics for idle sessions can be displayed
  without creating a run trace.
- Ensure final run diagnostics use the generated `workflow_run_id` and do not
  create a second idle-session row after completion.
- Separate frontend display policy for opened-workflow timing history from
  retained run-trace selection.
- Add regression tests for duplicate row prevention, stale running row
  prevention, workflow switching, and restart-visible timing history.
- Update touched README/API contracts so the invariants match the corrected
  architecture.
- Refactor touched surrounding code that is non-compliant with the standards.

### Out of Scope

- Changing scheduler admission policy.
- Changing runtime selection policy.
- Adding backwards compatibility for old session-id-as-run-id rows.
- Rebuilding the diagnostics UI beyond the selection/history behavior needed
  for this bug.
- Full durable trace-store replacement. This plan uses existing retained
  traces plus existing SQLite timing history.

## Inputs

### Problem

Running one workflow currently creates two diagnostics entries in the left
column: the real run eventually completes, while a second row can remain in
`running` state. The extra row changes identity when the opened workflow
changes, which can create unnecessary or misleading diagnostics history. After
restart, the opened workflow can fail to show duration history until a run is
started even though prior diagnostics exist in SQLite.

Investigation found three coupled causes:

- Embedded runtime diagnostics projection falls back from missing
  `scheduler_snapshot.workflow_run_id` to `scheduler_snapshot.session_id`.
- Tauri diagnostics snapshot reads can record scheduler/runtime snapshots into
  canonical trace state, so opening or refreshing diagnostics can mutate run
  history.
- The frontend auto-selects retained run rows before showing opened-workflow
  timing history, so a phantom or stale run row can hide SQLite-backed history.

### Constraints

- Backend Rust remains the source of truth for run identity, trace lifecycle,
  scheduler state, runtime state, and timing history.
- `session_id` identifies an edit/workflow session only. It must never be used
  as a canonical run id.
- `workflow_run_id` is backend-generated once per submitted run and is the only
  key for scheduler queue/run, runtime execution events, trace rows, and timing
  observations.
- Frontend components render diagnostics projections declaratively and must not
  repair backend identity locally.
- Snapshot/read paths must not create canonical run records.
- Touched code must conform to
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.
- Existing unrelated dirty files must remain untouched unless explicitly
  assigned to this work.

### Assumptions

- No backwards compatibility is required for old fake rows whose run id equals
  a session id.
- It is acceptable for idle scheduler/runtime diagnostics to have no
  `workflow_run_id`.
- The existing SQLite timing ledger is the durable source for opened-workflow
  timing history.
- The left diagnostics column is for retained canonical run traces only; a
  separate opened-workflow timing history panel may be visible even when no
  retained trace exists.

### Dependencies

- `crates/pantograph-embedded-runtime/src/workflow_runtime.rs`: scheduler and
  runtime diagnostics snapshot projection.
- `crates/pantograph-embedded-runtime/src/embedded_edit_session_execution.rs`:
  edit-session execution lifecycle and terminal event emission.
- `src-tauri/src/workflow/workflow_execution_runtime.rs`: run orchestration,
  final diagnostics emission, and channel snapshots.
- `src-tauri/src/workflow/headless_diagnostics.rs` and
  `src-tauri/src/workflow/headless_diagnostics_transport.rs`: diagnostics
  snapshot request projection.
- `src-tauri/src/workflow/diagnostics/`: projection store, overlay merge,
  trace adaptation, DTOs, and tests.
- `crates/pantograph-workflow-service/src/trace/`: canonical trace store and
  timing history lookup.
- `crates/pantograph-diagnostics-ledger/`: persisted timing observations and
  run summaries.
- `src/stores/diagnosticsStore.ts`,
  `src/stores/diagnosticsProjection.ts`,
  `src/components/diagnostics/DiagnosticsPanel.svelte`, and
  `src/components/diagnostics/DiagnosticsWorkflowHistory.svelte`: frontend
  diagnostics selection and rendering.

### Affected Structured Contracts

- `WorkflowExecutionDiagnosticsSnapshot` must allow idle scheduler/runtime
  diagnostics without a run id.
- `WorkflowExecutionSchedulerSnapshot.workflow_run_id` and
  `WorkflowExecutionRuntimeSnapshot.workflow_run_id` must not be populated from
  `session_id`.
- `WorkflowDiagnosticsProjection.runsById` and `runOrder` must contain only
  canonical run traces keyed by real `workflow_run_id`.
- `WorkflowDiagnosticsProjection.workflowTimingHistory` remains keyed by
  `workflow_id` plus graph fingerprint and is independent from selected run
  traces.
- Frontend TypeScript mirrors must preserve nullable run id semantics for
  scheduler/runtime panels where no run is active.

### Affected Persisted Artifacts

- Existing `.pantograph/workflow-diagnostics.sqlite` timing observations remain
  the history source.
- No compatibility migration is required for fake session-id rows. If the
  implementation reveals persisted fake rows are being written, add a cleanup
  step that prunes or ignores them explicitly.

### Concurrency and Lifecycle Review

- Run submission creates exactly one `workflow_run_id` before execution begins.
- Scheduler/running state must be finished on completed, failed, and cancelled
  terminal paths, while waiting-for-input may remain active.
- Final runtime/scheduler diagnostics must be attributed to the known
  `workflow_run_id` for the run that just emitted terminal events.
- Diagnostics snapshot refreshes may overlap workflow switching, graph loading,
  and event-driven snapshots. Refresh results must remain relevant by backend
  context and must not create new trace entries.
- Frontend refresh tokens already drop stale async responses; this plan must
  preserve or strengthen that behavior when selection/history policy changes.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Runtime diagnostics DTOs currently require a run id | High | Make idle/no-run state explicit in contracts before changing projection behavior. |
| Read paths currently mutate diagnostics state | High | Split snapshot projection from canonical trace recording and test that reads do not alter `runOrder`. |
| Final diagnostics may lose runtime details if scheduler is already idle | Medium | Carry the known generated `workflow_run_id` into final diagnostics emission instead of deriving it from scheduler snapshot fallback. |
| Frontend hides history whenever a run row exists | Medium | Separate opened-workflow timing history display from retained run selection. |
| Existing tests encode session-id fallback behavior | Medium | Update tests to assert no trace is created without a real run id. |
| Large touched modules could grow further | Medium | Add a standards refactor milestone for touched modules and split responsibilities where needed. |

## Definition of Done

- One GUI workflow run creates exactly one diagnostics run row.
- Repeated runs create one row per generated `workflow_run_id` and no permanent
  stale `running` row.
- Switching workflows does not rewrite any retained run row's workflow id.
- Opening the app and loading a workflow with prior timing observations shows
  duration history before running the workflow.
- Diagnostics snapshot reads do not create canonical traces when no run is
  active.
- Tests cover the backend projection, Tauri diagnostics read path, execution
  terminal path, and frontend selection/history behavior.
- Touched README/API contracts document the corrected source-of-truth and
  no-session-id-fallback invariants.
- Final verification passes, or any blocked verification is recorded with the
  blocker and follow-up.

## Milestones

### Milestone 1: Contract Hardening

**Goal:** Make no-run diagnostics states explicit and remove session-id fallback
from the planned contract.

**Tasks:**
- [x] Change embedded runtime diagnostics DTO planning targets so scheduler and
  runtime snapshots can represent `workflow_run_id: None` for idle/no-run
  state.
- [x] Identify tests that currently expect fallback to `session_id` and mark
  them for replacement with no-run assertions.
- [x] Update affected README/API contract notes in the implementation slice.

**Verification:**
- Rust unit tests for embedded diagnostics snapshot projection prove missing
  scheduler run id remains missing.
- `cargo test -p pantograph-embedded-runtime diagnostics_snapshot`.

**Status:** Complete.

### Milestone 2: Side-Effect-Safe Diagnostics Reads

**Goal:** Ensure explicit diagnostics refresh/open-workflow reads cannot create
or mutate canonical run traces.

**Tasks:**
- [ ] Split Tauri diagnostics projection into read-only snapshot assembly and
  event/run recording paths.
- [ ] Preserve runtime and scheduler panel data for idle sessions without
  writing to `WorkflowTraceStore`.
- [ ] Add tests showing repeated `workflow_get_diagnostics_snapshot` calls with
  no active run leave `runOrder` empty.
- [ ] Add tests showing workflow switching does not rewrite existing run
  `workflowId` values.

**Verification:**
- `cargo test --manifest-path src-tauri/Cargo.toml workflow::headless_diagnostics`
- `cargo test --manifest-path src-tauri/Cargo.toml workflow::diagnostics`

**Status:** Not started.

### Milestone 3: Terminal Run Attribution

**Goal:** Ensure final diagnostics for a run are attributed to that run and do
not create an idle-session trace after completion.

**Tasks:**
- [ ] Carry the generated `workflow_run_id` through final diagnostics emission
  instead of deriving it from scheduler snapshot fallback.
- [ ] Finish edit-session scheduler state on all non-waiting terminal paths
  while preserving waiting-for-input behavior.
- [ ] Ensure runtime metrics emitted after completion update the existing run
  trace or remain overlay-only when no real run id exists.
- [ ] Add regression tests for success, failure, cancellation, and waiting
  lifecycles.

**Verification:**
- `cargo test -p pantograph-embedded-runtime`
- `cargo check --manifest-path src-tauri/Cargo.toml`

**Status:** Not started.

### Milestone 4: Frontend History And Selection Policy

**Goal:** Show opened-workflow timing history independently from retained run
trace selection.

**Tasks:**
- [ ] Adjust diagnostics store selection so phantom/no-run rows cannot become
  selected.
- [ ] Ensure `workflowTimingHistory` can render before first run after app
  restart.
- [ ] Decide whether the history view is always visible in the overview when no
  run is selected, or whether it gets a dedicated history section that is not
  hidden by retained runs.
- [ ] Add frontend tests for restart/open workflow history, no auto-selection
  of invalid rows, and run selection reset on workflow switch.

**Verification:**
- `npm run -w frontend check:types`
- `npm run -w frontend test:run`

**Status:** Not started.

### Milestone 5: Cross-Layer Regression Coverage

**Goal:** Prove the corrected architecture holds from GUI run submission through
diagnostics display.

**Tasks:**
- [ ] Add an integration-style test or focused multi-layer regression proving a
  run produces one canonical row.
- [ ] Verify retained trace rows and SQLite timing history are independent but
  keyed consistently by `workflow_run_id`, `workflow_id`, and graph
  fingerprint.
- [ ] Add a regression fixture for run A then run B, confirming each row keeps
  its original workflow id.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- `cargo test -p pantograph-diagnostics-ledger`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run -w frontend test:run`

**Status:** Not started.

### Milestone 6: Standards Compliance Refactor

**Goal:** Keep touched code and immediate surrounding areas compliant with the
coding and documentation standards.

**Tasks:**
- [ ] Review touched Rust modules for ambiguous raw id handling, oversized
  responsibilities, and read/write lifecycle mixing.
- [ ] Review touched frontend store/components for presentation ownership,
  stale async handling, and component responsibility boundaries.
- [ ] Update `src-tauri/src/workflow/diagnostics/README.md`,
  `crates/pantograph-workflow-service/src/trace/README.md`,
  `crates/pantograph-embedded-runtime/src/README.md`, and
  `src/components/diagnostics/README.md` as needed.
- [ ] Record any unrelated standards issues discovered but not required for
  this plan as follow-ups.

**Verification:**
- `git diff --check`
- Relevant targeted tests from prior milestones.
- Final manual standards review against `CODING-STANDARDS.md`,
  `ARCHITECTURE-PATTERNS.md`, `FRONTEND-STANDARDS.md`,
  `TESTING-STANDARDS.md`, and `DOCUMENTATION-STANDARDS.md`.

**Status:** Not started.

### Milestone 7: Release Verification

**Goal:** Confirm the app builds and the user-facing workflow is corrected.

**Tasks:**
- [ ] Run final backend, frontend, and Tauri verification.
- [ ] Build the release app.
- [ ] Manually verify the GUI behavior:
  - one row per run,
  - no stale running row after completion,
  - workflow switch does not rewrite old row identities,
  - restart/open workflow shows timing history before run.
- [ ] Update this plan with completion summary, deviations, and remaining
  follow-ups.

**Verification:**
- `cargo test -p pantograph-embedded-runtime`
- `cargo test -p pantograph-workflow-service`
- `cargo test -p pantograph-diagnostics-ledger`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run -w frontend check:types`
- `npm run -w frontend test:run`
- `bash launcher.sh --build-release`

**Status:** Not started.

## Execution Notes

- 2026-04-26: Plan created after read-only investigation of diagnostics
  projection, workflow execution runtime, trace store, timing ledger, and
  frontend diagnostics selection. Existing unrelated dirty files are present in
  the worktree and must remain untouched unless the user assigns them to this
  plan.
- 2026-04-26: Milestone 1 completed. Embedded-runtime diagnostics snapshots now
  preserve idle/no-run scheduler state as `workflow_run_id: None` instead of
  projecting the edit `session_id` as a run id. Added a focused regression test
  and documented the invariant in the embedded-runtime README. Verification:
  `cargo test -p pantograph-embedded-runtime diagnostics_snapshot` passed.

## Commit Cadence Notes

- Commit after each milestone or independently verified logical slice.
- Follow `COMMIT-STANDARDS.md`; do not include tool logs or raw verification
  output in commit messages.
- Keep compile-unblocking fixes separate when they are not part of the active
  milestone.
- Do not begin the next implementation slice with dirty implementation files
  from the previous slice.

## Optional Subagent Assignment

No subagents are required for the initial implementation. If parallel work is
later requested, split by non-overlapping write sets:

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| Backend | Embedded/runtime/Tauri diagnostics projection | Tests and patch for no-run snapshot semantics | Before frontend selection changes |
| Frontend | Diagnostics store and component rendering policy | Tests and patch for restart-visible history | After backend DTO shape stabilizes |

## Re-Plan Triggers

- Scheduler/runtime diagnostics cannot represent idle no-run state without a
  wider DTO redesign.
- Snapshot reads still need to mutate trace state for a currently undocumented
  requirement.
- Waiting-for-input lifecycle requires a different active-run retention model.
- SQLite timing history cannot be keyed reliably by opened workflow id and
  graph fingerprint.
- Verification reveals existing persisted fake rows are still being read as
  canonical rows.
- Any touched module requires a larger compliance refactor than Milestone 6 can
  safely contain.

## Recommendations

- Prefer removing the session-id fallback at the embedded runtime contract
  boundary rather than filtering fake rows in the frontend. This fixes the
  source of the bad identity and keeps the GUI display-only.
- Prefer keeping durable opened-workflow timing history separate from retained
  run traces. They answer different questions: "what has happened recently in
  this session" versus "what does prior SQLite history say about this
  workflow graph."

## Completion Summary

### Completed

- Not started.

### Deviations

- None yet.

### Follow-Ups

- None yet.

### Verification Summary

- Not run. This plan is planning-only.

### Traceability Links

- Module README updates planned in Milestone 6.
- ADR added/updated: N/A unless implementation reveals a broader diagnostics
  architecture decision.
- PR notes: N/A until implementation begins.
