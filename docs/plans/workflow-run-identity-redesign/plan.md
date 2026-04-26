# Plan: Workflow Run Identity Redesign

## Objective

Redesign workflow execution diagnostics around one canonical backend-owned
`workflow_run_id` so scheduler queue state, runtime execution events,
diagnostic traces, SQLite timing observations, and frontend active-run state all
refer to the same workflow run with the same id.

The resulting architecture must have a single source of truth for workflow run
identity and workflow identity:

```text
workflow_id
  Stable id of the saved workflow.

session_id
  Editor or loaded-session container id only.
  Never used as a workflow run id.

workflow_run_id
  Backend-generated id for one submitted workflow execution.
  Used as scheduler queue id, scheduler run id, runtime execution id,
  trace execution id, SQLite timing execution id, and frontend active run id.

runtime_instance_id
  Optional runtime/backend resource id used for diagnostics only.
  Not a workflow run id.
```

No backwards compatibility is required. Existing mixed identity contracts should
be replaced rather than adapted behind compatibility facades.

## Scope

### In Scope

- Replace GUI edit-session execution identity with a unique backend-generated
  `workflow_run_id` per submitted run.
- Use `workflow_id` as the only workflow label/identity in diagnostics.
- Remove diagnostics dependence on `workflow_name` and display-name side
  channels.
- Make scheduler queue items, runtime execution events, trace rows, timing
  observations, Tauri diagnostics snapshots, and frontend active run state use
  the same `workflow_run_id`.
- Persist enough run-summary data for diagnostics history to be available after
  GUI restart, if previous workflow diagnostics are expected in the diagnostics
  panel before a new run.
- Update Rust, Tauri, TypeScript, and frontend contracts to make invalid id
  mixing difficult or impossible.
- Add cross-layer tests proving a run id can be searched across scheduler,
  runtime, trace, timing history, and frontend projections.
- Refactor touched surrounding code that violates the standards in
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.

### Out of Scope

- Backwards-compatible aliases for old session-id-as-run-id behavior.
- Separate mutable workflow display names in diagnostics.
- Scheduler policy tuning unrelated to canonical run identity.
- Runtime resource identity redesign beyond preserving `runtime_instance_id` as
  a distinct diagnostic fact.
- Full diagnostics UI redesign beyond identity, history, and label correctness.
- Migration of old in-memory diagnostic traces. Durable SQLite timing data may
  be cleared or ignored if its identity contract is incompatible.

## Inputs

### Problem

The current diagnostics architecture reuses edit `session_id` as scheduler
queue id, scheduler run id, runtime execution id, and trace execution id for GUI
graph runs. Multiple runs from the same opened workflow therefore mutate the
same diagnostic trace. Workflow display names are also attached as side-channel
metadata keyed by execution id, so stale or missing names appear when workflows
are opened, switched, or run repeatedly.

The user expectation is simpler and correct for this platform: one id should
identify one workflow run everywhere. If a user has the queue/run id, they
should be able to find the scheduler item, runtime events, trace, timing
observations, and frontend active run with that same id.

### Constraints

- Backend-owned data is the source of truth. The frontend may display run state
  but must not repair backend identity or labels locally.
- No backwards compatibility is required.
- `workflow_id` is the only workflow identity needed by diagnostics.
- `workflow_run_id` is backend-generated once at run submission and never
  replaced by the editor `session_id`.
- Public and binding-facing execution must still go through the scheduler.
- Touched Rust APIs should follow correct-by-construction guidance with typed or
  clearly named domain ids instead of raw ambiguous strings.
- Existing unrelated dirty files must remain untouched unless explicitly
  assigned to this work.
- Implementation should commit each verified logical slice before moving to the
  next slice.

### Assumptions

- GUI graph runs continue to execute through backend-owned edit sessions, but a
  run submission creates a new `workflow_run_id` before scheduler admission.
- Scheduler queue id and scheduler run id can be the same canonical
  `workflow_run_id` for this platform because there is no product need to
  distinguish queue item identity from run identity.
- Runtime execution id should be set from `workflow_run_id` for all workflow
  execution events emitted by node-engine and embedded runtime paths.
- `runtime_instance_id` remains separate because it describes the runtime
  resource selected for work, not the submitted workflow run.
- Existing SQLite timing observations with old identity semantics may be
  pruned, ignored, or allowed to age out. No compatibility lookup is required.

### Dependencies

- `crates/pantograph-workflow-service/src/scheduler/`: scheduler contracts,
  queue ids, run ids, session snapshots, and admission state.
- `crates/pantograph-workflow-service/src/graph/`: edit-session lifecycle and
  current session-runtime queue projection.
- `crates/pantograph-workflow-service/src/trace/`: diagnostic trace state,
  event contracts, timing expectation lookup, and trace snapshots.
- `crates/pantograph-diagnostics-ledger/`: SQLite timing observation schema and
  durable lookup behavior.
- `crates/pantograph-embedded-runtime/`: embedded edit-session execution and
  node-engine event execution ids.
- `src-tauri/src/workflow/`: Tauri command inputs, run orchestration,
  diagnostics bridge, and snapshot projection.
- `src/services/diagnostics/types.ts`, `src/services/workflow/WorkflowService.ts`,
  `src/backends/TauriWorkflowBackend.ts`, and `packages/svelte-graph/`:
  frontend DTO mirrors and run submission/display paths.
- Existing verification commands: `cargo test -p pantograph-workflow-service`,
  `cargo test -p pantograph-diagnostics-ledger`, `cargo test -p
  pantograph-embedded-runtime`, `cargo check --manifest-path
  src-tauri/Cargo.toml`, `npm run typecheck`, `npm run test:frontend`, and
  `git diff --check`.

### Affected Structured Contracts

- Scheduler queue/run DTOs expose one `workflow_run_id` for a submitted run.
- Edit-session run APIs return the generated `workflow_run_id` when a run is
  submitted.
- Runtime execution events use `workflow_run_id` as `execution_id`.
- Trace events and summaries use `workflow_run_id` as the trace key.
- Diagnostics snapshots expose `workflow_id` and `workflow_run_id`, not
  `workflow_name`.
- SQLite timing observations use `workflow_run_id` as `execution_id` and
  `workflow_id` as workflow identity.
- Frontend diagnostics TypeScript types mirror the backend DTOs without
  display-name repair fields.

### Affected Persisted Artifacts

- `.pantograph/workflow-diagnostics.sqlite` timing observation rows.
- Any new durable diagnostics run-summary table required for restart-visible
  run history.
- Schema migration metadata in `pantograph-diagnostics-ledger`.
- Serialized diagnostics snapshot test fixtures, if present.

### Concurrency and Lifecycle Review

- `workflow_run_id` must be created exactly once per run submission before the
  scheduler item is visible.
- Repeated Run clicks for the same session must create distinct run ids or be
  rejected by scheduler admission before a second id is visible.
- Cancellation, completion, failure, waiting-for-input, and restart/recovery
  paths must preserve the original `workflow_run_id`.
- Scheduler state, runtime events, trace writes, and SQLite observation writes
  must be idempotent for duplicate terminal events.
- Frontend subscriptions must treat backend diagnostics snapshots as
  authoritative and ignore stale snapshots by backend-authored run/session
  context, not by local guesses.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Existing edit-session runtime is built around `session_id` as executor id | High | Introduce an explicit run context passed from scheduler submission into embedded runtime before changing event translation. |
| Queue id and run id have separate expectations in tests | Medium | Freeze the platform decision that they are the same canonical `workflow_run_id`, then update contracts and tests together. |
| Old SQLite timing observations no longer match new ids | Low | No backwards compatibility is required; document old data as incompatible and allow clearing or pruning. |
| Removing `workflow_name` exposes UI places that expected pretty labels | Medium | Diagnostics renders `workflow_id`. Non-diagnostics UI may use workflow catalog metadata separately, outside trace identity. |
| Frontend continues to pass workflow names into diagnostics requests | Medium | Remove the fields from DTOs and add type/tests so the compiler catches stale call sites. |
| Large files grow further during refactor | Medium | Split touched modules when edits add responsibility or exceed decomposition review thresholds. |
| Cross-layer identity breaks in language bindings | High | Include binding/API surface search and contract tests after Rust/Tauri changes. |

### Standards Compliance Audit

The planned architecture is standards-compliant if implemented as written
because it restores backend-owned data, single-owner run lifecycle state,
correct-by-construction id contracts, and frontend display-only diagnostics.

The immediate touched source areas have pre-existing compliance issues that
must be resolved during or immediately after the main implementation:

- `crates/pantograph-workflow-service/src/scheduler/store.rs` is 894 lines and
  owns queue storage, admission transitions, runtime lifecycle, and snapshot
  support. Milestone 2 must either split it or record a focused decomposition
  decision before adding run-id ownership.
- `src-tauri/src/workflow/events.rs` is 642 lines and owns broad event DTOs.
  Milestone 5 must avoid adding more identity conversion logic there and should
  extract run-identity diagnostics DTOs if edits make the responsibility larger.
- `packages/svelte-graph/src/components/WorkflowToolbar.svelte` is 299 lines,
  above the UI component decomposition review threshold. Milestone 6 must split
  run submission/diagnostics controls or document why the touched scope remains
  safe.
- `src/services/workflow/WorkflowService.ts` is 484 lines and close to the
  generic file-size target while owning many service methods. Milestone 6 must
  avoid adding identity orchestration there without extracting focused workflow
  run or diagnostics transport helpers.
- `src/services/diagnostics/types.ts` and
  `src-tauri/src/workflow/diagnostics/types.rs` are DTO aggregation files near
  the file-size threshold. Contract changes should split run identity/history
  types if the files would grow materially.
- `crates/pantograph-diagnostics-ledger/src/` is under a source root and lacks
  `README.md`. Milestone 4 or the final compliance milestone must add it with
  the required API and structured producer contract sections.
- Existing raw `String` fields named `execution_id`, `trace_execution_id`,
  `queue_id`, `run_id`, and `session_id` are ambiguous across Rust, Tauri, and
  TypeScript contracts. Milestones 1 through 7 must replace cross-boundary
  ambiguity with typed Rust ids or explicit `workflow_run_id` wire fields.
- Tests and fixtures contain session-id-as-run-id examples. The refactor must
  update them rather than preserving old behavior.

### Collateral Damage Audit

Plan implementation will have cross-cutting impact beyond the initial
diagnostics files. These areas must be handled deliberately:

- Frontend graph mutation paths use `executionId` as an edit-session id in
  `src/backends/TauriWorkflowBackend.ts`,
  `src/services/workflow/WorkflowGraphMutationService.ts`, and
  `src/services/workflow/workflowConnectionActions.ts`. Those are not workflow
  run ids. Implementation must either leave them alone or rename them to
  `sessionId` in a separate graph-edit contract slice. Do not blindly replace
  every frontend `executionId` with `workflowRunId`.
- `packages/svelte-graph/src/workflowEventOwnership.ts`,
  `packages/svelte-graph/src/stores/workflowExecutionEvents.ts`, and
  `src/components/workflowToolbarEvents.ts` currently track
  `activeExecutionId`. These should become active workflow run ownership for
  runtime events only, while graph-edit session ownership remains separate.
- `src-tauri/src/llm/commands/registry*` uses workflow diagnostics filters
  named `execution_id` and `workflow_name`. Removing `workflow_name` from
  diagnostics will break LLM registry debug responses unless that command is
  updated in the same Tauri diagnostics contract slice.
- `src/components/diagnostics/DiagnosticsScheduler.svelte` displays both
  `queue_id` and `run_id`. If queue id and run id become the same
  `workflow_run_id`, the table should not keep two columns that imply separate
  identities.
- `src/components/diagnostics/DiagnosticsPanel.svelte`,
  `DiagnosticsGraph.svelte`, and `DiagnosticsWorkflowHistory.svelte` render
  `workflowName` fallbacks. These must move to `workflowId` for diagnostics,
  while non-diagnostics graph-selector UI can still use workflow catalog names.
- `crates/pantograph-frontend-http-adapter`, `crates/pantograph-rustler`,
  `crates/pantograph-uniffi`, and C# smoke fixtures still expose or assert
  `run_id`/`execution_id` contracts. Binding/API guardrails must include these
  surfaces, not only Tauri and Svelte.
- `crates/pantograph-runtime-attribution` and existing node execution
  attribution code already use `WorkflowRunId`. The redesign should reuse this
  domain type where possible instead of introducing a second workflow-run id
  type with the same meaning.
- `crates/node-engine` has generic `execution_id` APIs used by lower-level
  execution engines. Those can remain internal if the embedding layer passes
  the canonical `workflow_run_id` into them and the public Pantograph boundary
  exposes `workflow_run_id`.
- Documentation contains older plans and API files that explicitly bless
  caller-supplied `run_id`, `trace_execution_id`, and `workflow_name`.
  Active docs and examples must be updated; archived/completed plans can remain
  historical but should not be used as current contract references.
- Existing tests intentionally cover separate `queue_id` and `run_id` values,
  workflow-name filters, and edit-session ids as execution ids. These tests
  will fail by design and must be rewritten to assert the new invariants rather
  than patched piecemeal.

## Clarifying Questions

None blocking.

Assumption: scheduler queue id and run id should be the same canonical
`workflow_run_id`, matching the user's searchability expectation.

## Definition of Done

- One backend-generated `workflow_run_id` is created for every workflow run.
- Scheduler queue item id, scheduler run id, runtime execution id, trace
  execution id, SQLite timing execution id, and frontend active run id all equal
  that `workflow_run_id`.
- `session_id` is never used as a workflow run id.
- Diagnostics use `workflow_id` only for workflow identity and labels.
- `workflow_name` is removed from diagnostics contracts, trace metadata,
  timing history lookup, and frontend diagnostics rendering.
- Opening a workflow after GUI restart shows backend-owned prior diagnostics
  history when durable run summaries exist for that `workflow_id`.
- Running workflow A, then workflow B, then workflow A again shows distinct run
  rows with correct `workflow_id` values and no stale labels.
- Tests cover scheduler, runtime, trace, SQLite timing, Tauri projection,
  frontend diagnostics rendering, and at least one cross-layer GUI run path.
- Touched READMEs or ADRs document the identity model and source-of-truth
  rules.
- Each completed implementation slice is committed with a detailed conventional
  commit message.

## Milestones

### Milestone 1: Freeze Canonical Identity Contract

**Goal:** Define the run identity model before changing producers or consumers.

**Tasks:**
- [x] Add or update an ADR documenting `workflow_id`, `session_id`,
  `workflow_run_id`, and `runtime_instance_id`.
- [x] Define Rust domain types or unambiguous contract fields for the four ids.
- [x] Decide exact wire names for Tauri, frontend, bindings, and SQLite.
- [x] Remove `workflow_name` from planned diagnostics contracts.
- [x] Document that old mixed-identity SQLite rows are unsupported.

**Verification:**
- Review against backend-owned data and single-owner state-flow rules in
  `CODING-STANDARDS.md`.
- Review against correct-by-construction and parse-at-boundary guidance in
  `RUST-API-STANDARDS.md`.
- Search planned public contracts for ambiguous `execution_id` use and record
  whether it will be renamed or explicitly documented as `workflow_run_id`.

**Status:** Completed.

**Notes:**
- Added `docs/adr/ADR-012-canonical-workflow-run-identity.md` as the
  source-of-truth identity decision for `workflow_id`, `session_id`,
  `workflow_run_id`, and `runtime_instance_id`.
- Confirmed existing validated Rust id types are owned by
  `pantograph-runtime-attribution` and re-exported
  `WorkflowId`, `ClientSessionId`, and `WorkflowRunId` from
  `pantograph-workflow-service`.
- Froze wire names as `workflow_id`, `session_id`, `workflow_run_id`, and
  `runtime_instance_id`.
- Documented that diagnostics contracts remove `workflow_name` and that old
  mixed-identity SQLite timing rows are unsupported.
- Source search confirmed ambiguous `execution_id`, `trace_execution_id`,
  `queue_id`, `run_id`, and `workflow_name` fields still exist in the planned
  touched surfaces; they remain assigned to Milestones 2 through 7.

**Verification Results:**
- Reviewed against backend-owned data and single-owner state-flow guidance in
  `CODING-STANDARDS.md`.
- Reviewed against correct-by-construction and parse-at-boundary guidance in
  `languages/rust/RUST-API-STANDARDS.md`.
- `rg` search recorded stale public/cross-layer identity names for later
  milestone replacement.
- `git diff --check -- docs/adr/ADR-012-canonical-workflow-run-identity.md docs/adr/README.md crates/pantograph-workflow-service/src/lib.rs`
- `cargo check -p pantograph-workflow-service`

### Milestone 2: Make Scheduler Own Run Id Creation

**Goal:** Ensure every run submission creates exactly one canonical
`workflow_run_id` before queue visibility.

**Tasks:**
- [x] Update scheduler run submission contracts so queue id and run id are the
  same generated `workflow_run_id`.
- [x] Update edit-session runtime state so `mark_running` receives a
  `workflow_run_id` rather than deriving execution identity from `session_id`.
- [x] Ensure scheduler snapshots expose `workflow_run_id` and never infer trace
  identity from `session_id`.
- [x] Update scheduler and graph edit-session tests for repeated runs from the
  same session.
- [x] Refactor touched scheduler/session modules for standards compliance.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- Focused tests proving two runs from one edit session produce two distinct
  `workflow_run_id` values.
- Search check proving `session_id` is not assigned into queue/run identity.

**Status:** Completed.

**Notes:**
- Removed caller-authored `run_id` from
  `WorkflowExecutionSessionRunRequest`.
- Changed scheduler queue items and scheduler snapshots to expose
  `workflow_run_id` instead of separate `queue_id`, `run_id`, and
  `trace_execution_id` fields.
- Scheduler enqueue now generates the backend-owned run id with
  `WorkflowRunId::generate()` before the item is visible in queue state.
- Edit-session runtime `mark_running` now receives the workflow run id
  explicitly and graph scheduler snapshots use that id instead of the
  `session_id`.
- Added a repeated-run session test proving two runs from the same workflow
  execution session produce distinct backend-owned run ids, and updated queue,
  scheduler, trace, contract, and example fixtures to the new scheduler
  contract.
- Downstream Tauri, frontend, and binding surfaces are expected to need
  matching DTO updates in Milestones 5 through 7.

**Verification Results:**
- `cargo test -p pantograph-workflow-service`
- `rg` search checked for `session_id` assigned into scheduler queue,
  runtime execution, trace, or workflow-run identity in the workflow-service
  slice; remaining matches pass `session_id` and `workflow_run_id` as separate
  arguments.
- `git diff --check -- crates/pantograph-workflow-service docs/plans/workflow-run-identity-redesign/plan.md`

### Milestone 3: Propagate Workflow Run Id Through Runtime Events

**Goal:** Make embedded runtime and node-engine event emission use the scheduler
owned `workflow_run_id`.

**Tasks:**
- [ ] Introduce a run context passed from Tauri/session execution into embedded
  runtime.
- [ ] Replace edit-session `execution_id: session_id` event emission with
  `execution_id: workflow_run_id`.
- [ ] Preserve `session_id` only as session attribution where needed.
- [ ] Keep `runtime_instance_id` in runtime diagnostics metrics, separate from
  run identity.
- [ ] Update embedded-runtime tests for completion, failure, waiting-for-input,
  cancellation, and runtime metrics.

**Verification:**
- `cargo test -p pantograph-embedded-runtime`
- Cross-crate workflow-service plus embedded-runtime test proving scheduler
  active item and node-engine terminal event share the same `workflow_run_id`.
- Search check for edit-session `execution_id: session_id` event construction.

**Status:** Not started.

### Milestone 4: Rebuild Trace And Timing Around Workflow Run Id

**Goal:** Make diagnostic traces and timing observations keyed by the canonical
run id and workflow id only.

**Tasks:**
- [ ] Replace trace side-channel name metadata with explicit run descriptors.
- [ ] Remove `workflow_name` from trace summaries, trace snapshot requests, and
  timing expectation fallback logic.
- [ ] Ensure trace creation requires or receives `workflow_run_id`,
  `session_id`, `workflow_id`, and graph context.
- [ ] Update SQLite timing observation schema/use to treat `execution_id` as
  `workflow_run_id`.
- [ ] Add or update durable run-summary storage if restart-visible diagnostic
  run history is required beyond timing expectations.
- [ ] Make terminal timing writes idempotent by `workflow_run_id` and node id.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- `cargo test -p pantograph-diagnostics-ledger`
- Replay/idempotency tests for duplicate terminal events.
- Fresh database and existing incompatible database behavior tests.

**Status:** Not started.

### Milestone 5: Update Tauri Diagnostics Projection

**Goal:** Make Tauri transport and diagnostics snapshots expose the canonical
identity model without frontend repair.

**Tasks:**
- [ ] Update Tauri run command responses/events to include
  `workflow_run_id` where a run is submitted.
- [ ] Update diagnostics snapshot request/projection DTOs to use `workflow_id`,
  `session_id`, and `workflow_run_id`.
- [ ] Remove `workflow_name` from diagnostics snapshot requests and projections.
- [ ] Ensure pre-run and post-run diagnostics snapshots join on
  `workflow_run_id`.
- [ ] Update Tauri tests for workflow switching, restart-opened workflow
  history, and scheduler/runtime/trace joins.

**Verification:**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Focused Tauri diagnostics tests for workflow A then B then A.
- Cross-layer projection test proving a scheduler queue item and trace row
  share the same `workflow_run_id`.

**Status:** Not started.

### Milestone 6: Update Frontend Contracts And Rendering

**Goal:** Make the frontend display backend-authored diagnostics identity
without owning or correcting it.

**Tasks:**
- [ ] Update TypeScript diagnostics and workflow service types to mirror the
  new backend DTOs.
- [ ] Remove `workflowName` from diagnostics calls and run diagnostics
  rendering.
- [ ] Make Run button capture and track returned `workflow_run_id`.
- [ ] Render diagnostics run labels from `workflow_id`.
- [ ] Ensure opening a workflow triggers a diagnostics snapshot/history request
  by `workflow_id` and graph context before first run.
- [ ] Update frontend tests for startup history display and workflow switching.

**Verification:**
- `npm run typecheck`
- `npm run test:frontend`
- Frontend acceptance test or component test proving diagnostics rows do not
  reuse stale workflow labels after switching workflows.

**Status:** Not started.

### Milestone 7: Binding And Public API Guardrails

**Goal:** Prevent public API drift back into ambiguous ids or scheduler bypass.

**Tasks:**
- [ ] Update language bindings and HTTP adapter DTOs to expose
  `workflow_run_id` consistently.
- [ ] Remove `workflow_name` from binding diagnostics contracts.
- [ ] Add source-surface guardrails for forbidden `session_id` as run id usage
  in public/binding-facing code.
- [ ] Update README/API docs for workflow-service, embedded-runtime, Tauri
  workflow, frontend diagnostics, and bindings touched by the refactor.

**Verification:**
- Binding native tests for changed crates.
- Host-language smoke tests where existing harnesses support them.
- Search guardrail for stale `workflow_name` diagnostics fields and
  session-id-as-run-id patterns.
- Documentation traceability review.

**Status:** Not started.

### Milestone 8: Standards Compliance Refactor

**Goal:** Ensure touched code and immediate surroundings conform to the coding,
documentation, frontend, Rust API, and testing standards after the identity
model is implemented.

**Tasks:**
- [ ] Complete a decomposition review for each touched file over the standards
  thresholds and split modules/components where the refactor added
  responsibility.
- [ ] Split or narrow `scheduler/store.rs` so run-id ownership, admission
  decisions, queue storage, and diagnostics projection do not remain one broad
  responsibility.
- [ ] Split or narrow `src-tauri/src/workflow/events.rs` if run identity DTOs
  or conversions grow the file further.
- [ ] Decompose `WorkflowToolbar.svelte` or extract run/diagnostics controls
  when Milestone 6 edits touch that component.
- [ ] Extract focused helpers from `WorkflowService.ts` if new run identity
  transport logic would push it beyond the file-size target or keep it as a
  broad mixed service.
- [ ] Add `crates/pantograph-diagnostics-ledger/src/README.md` with required
  purpose, contract, persistence, migration, and structured producer sections.
- [ ] Re-run source searches for stale `workflow_name`, ambiguous public
  `execution_id`, and `session_id` assigned as run identity.
- [ ] Audit collateral-damage findings above and verify each affected surface
  is either updated for `workflow_run_id`, explicitly left as edit-session
  identity, or documented as historical-only.
- [ ] Update module READMEs and ADR links for every touched source directory.

**Verification:**
- `git diff --check`
- Source README coverage check for touched `src/` directories.
- File-size/decomposition review recorded in this plan or linked ADR/README.
- Search guardrails for stale identity fields pass or have explicit allowlists
  for private runtime-resource identity only.
- Relevant Rust and frontend tests from prior milestones still pass after
  refactors.

**Status:** Not started.

### Milestone 9: Release Verification

**Goal:** Verify the redesigned identity model works in the built application.

**Tasks:**
- [ ] Run the complete affected Rust and frontend test set.
- [ ] Run `git diff --check`.
- [ ] Compile the app.
- [ ] Make the release build.
- [ ] Manually smoke the GUI flow: open workflow A, inspect prior history, run
  A, run B, run A again, restart GUI, reopen A, verify history and ids.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- `cargo test -p pantograph-diagnostics-ledger`
- `cargo test -p pantograph-embedded-runtime`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run typecheck`
- `npm run test:frontend`
- `bash launcher.sh --build-release`

**Status:** Not started.

## Execution Notes

- 2026-04-26: Plan created from architecture analysis of diagnostic history and
  workflow identity bugs. No implementation changes made in this planning
  step.
- 2026-04-26: Milestone 1 completed. Added ADR-012, updated the ADR index,
  re-exported canonical attribution id types from workflow-service, and
  verified the contract slice with diff, source-search, and workflow-service
  cargo check.
- 2026-04-26: Milestone 2 completed. Scheduler/session contracts now generate
  and expose `workflow_run_id`; edit-session running state no longer derives
  run identity from `session_id`; workflow-service tests pass.
- Current known unrelated dirty files must remain untouched unless the user
  explicitly assigns them to this work:
  - `.pantograph/workflows/tiny-sd-turbo-diffusion.json`
  - `assets/3c842e69-080c-43ad-a9f0-14136e18761f.jpg`
  - `assets/grok-image-6c435c73-11b8-4dcf-a8b2-f2735cc0c5d3.png`
  - `assets/grok-image-e5979483-32c2-4cf5-b32f-53be66170132.png`
  - `.pantograph/workflow-diagnostics.sqlite`
  - `assets/banner_3.jpg`
  - `assets/banner_3.png`
  - `assets/github_social.jpg`
  - `assets/reject/`

## Commit Cadence Notes

- Commit after each milestone or smaller logical slice is complete and
  verified.
- Keep contract, implementation, tests, and documentation for the same slice
  together.
- Keep unrelated compliance refactors in separate commits when they are not
  required to make the current slice compliant.
- Do not stage unrelated asset or workflow-file changes.
- Follow `COMMIT-STANDARDS.md` for detailed conventional commit messages.

## Optional Subagent Assignment

None planned for initial implementation. This refactor crosses shared
contracts, so serial implementation is preferred unless later split into
isolated worker branches with non-overlapping write sets.

If parallel work is approved later, use one worker wave at a time:

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| Scheduler worker | `crates/pantograph-workflow-service/src/scheduler` and graph session queue projection | Committed scheduler-owned `workflow_run_id` contract and tests | After Milestone 2 tests pass |
| Runtime worker | `crates/pantograph-embedded-runtime` event identity propagation | Committed runtime event propagation and tests | After scheduler contract is frozen |
| Frontend worker | TypeScript DTOs and diagnostics rendering | Committed frontend contract/rendering changes and tests | After Tauri DTOs are frozen |

Shared contracts, SQLite schema, Tauri command DTOs, and documentation must be
owned serially by the integration implementer.

## Re-Plan Triggers

- Scheduler cannot make queue id and run id the same without violating an
  existing durable queue invariant.
- Embedded runtime requires a separate node-engine execution id for correctness.
- Durable restart-visible run history requires more than a focused run-summary
  table.
- Binding generation introduces additional public contract changes.
- Standards review finds touched modules need larger decomposition before the
  identity change can be compliant.
- New dirty implementation files appear that are unrelated to the active slice.

## Recommendations

- Prefer renaming public and cross-layer fields to `workflow_run_id` rather
  than keeping generic `execution_id` where possible. This is a breaking change,
  but it makes the contract self-documenting.
- Keep `runtime_instance_id` explicitly named in runtime diagnostics so nobody
  confuses runtime resource identity with workflow run identity.
- Add an ADR before code changes. This identity model is foundational enough
  that future scheduler, runtime, trace, binding, and frontend work should have
  one stable reference.

## Completion Summary

### Completed

- Milestone 1: Freeze Canonical Identity Contract.
- Milestone 2: Make Scheduler Own Run Id Creation.

### Deviations

- None.

### Follow-Ups

- None yet.

### Verification Summary

- Milestone 1:
  - `git diff --check -- docs/adr/ADR-012-canonical-workflow-run-identity.md docs/adr/README.md crates/pantograph-workflow-service/src/lib.rs`
  - `cargo check -p pantograph-workflow-service`
- Milestone 2:
  - `cargo test -p pantograph-workflow-service`
  - `git diff --check -- crates/pantograph-workflow-service docs/plans/workflow-run-identity-redesign/plan.md`
  - Source search for `session_id` assigned into workflow-run identity in the
    workflow-service scheduler/session slice.

### Traceability Links

- Plan artifact: `docs/plans/workflow-run-identity-redesign/plan.md`
- ADR added/updated:
  `docs/adr/ADR-012-canonical-workflow-run-identity.md`
- Module README updates: pending implementation milestones.
