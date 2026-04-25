# Plan: Scheduler-Only Workflow Execution

## Objective

Make the workflow scheduler the only public execution boundary for workflow
runs. Public Rust APIs, Tauri commands, frontend contracts, HTTP adapters, and
language bindings must be unable to execute a workflow without scheduler
admission, queue lifecycle tracking, and diagnostics attribution.

## Scope

### In Scope

- Remove direct workflow execution from public and binding-facing APIs.
- Route all user-facing and host-facing workflow runs through scheduler session
  creation and run requests.
- Remove frontend raw-graph execution fallbacks.
- Preserve current diagnostics visibility for GUI-triggered runs.
- Update README/API contract documentation for touched host-facing modules.
- Add guardrail tests that fail if a public direct-execution path reappears.

### Out of Scope

- Scheduler policy tuning unrelated to enforcing scheduler-only execution.
- UI redesign beyond backend contract and Run button behavior.
- Full decomposition of oversized modules except where required by the changed
  scheduler execution path.
- Backwards-compatible direct-run facades.

## Inputs

### Problem

Workflow execution is currently available through direct public paths such as
`workflow_run`, `execute_workflow_v2`, frontend `executeWorkflow(graph)`, and
binding/HTTP adapter exports. These paths can execute host/runtime work without
being admitted by the scheduler, which undermines platform stability,
capacity control, queue diagnostics, and trace attribution.

### Constraints

- No backwards compatibility is required for direct execution APIs.
- The scheduler is the backend-owned source of truth for run admission and run
  lifecycle state.
- Frontend state must remain display-only for backend-owned run state.
- Binding layers must stay thin and map to the same core scheduler contract.
- Dirty unrelated asset changes must not be modified or committed by this work.
- Implementation commits must be made after each verified logical slice.

### Assumptions

- Existing scheduler session APIs are the preferred durable execution contract:
  create session, run session, inspect queue/scheduler state, and close session.
- Internal host/runtime helpers may still execute actual runtime work, but only
  when called from scheduler-owned execution flow.
- Tests can use private helpers or fixtures where needed, as long as public API
  tests enforce the scheduler-only boundary.

### Dependencies

- `crates/pantograph-workflow-service`: canonical scheduler contracts and
  public service facade.
- `crates/pantograph-embedded-runtime`: host/runtime implementation behind the
  scheduler service facade.
- `src-tauri/src/workflow`: Tauri command registration and GUI transport.
- `packages/svelte-graph`: frontend backend interface consumed by the graph UI.
- `crates/pantograph-uniffi`, `crates/pantograph-rustler`, and
  `crates/pantograph-frontend-http-adapter`: host-facing bindings/adapters.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Unsaved editor graphs do not fit persisted workflow session APIs | High | Keep backend-owned edit sessions, but ensure their run path produces scheduler-visible lifecycle state and does not expose direct raw-graph execution as a public API. |
| Binding consumers lose `workflow_run` entrypoints | Medium | No compatibility facade by decision; update exported APIs, tests, and README contracts in the same slice. |
| Internal helper names remain confusing | Medium | Make direct runtime helpers private and add deferred rename cleanup after the main invariant is enforced. |
| Large touched files attract more responsibilities | Medium | Add only minimal glue in oversized files and extract focused modules when a touched area would grow materially. |
| Event and diagnostics attribution drift during migration | High | Add cross-layer tests that scheduler snapshots contain active run items for GUI/session runs. |

## Definition of Done

- No public Rust API, Tauri command, frontend backend contract, HTTP adapter, or
  language binding exposes direct workflow execution outside the scheduler.
- Public run entrypoints require a scheduler-owned session or create one
  internally before enqueueing through scheduler code.
- GUI Run uses scheduler-visible session execution only.
- Diagnostics scheduler snapshots show GUI-triggered running/queued state while
  execution is active.
- Touched README files document scheduler-only lifecycle, errors, and contract
  ownership.
- Guardrail tests or search checks catch reintroduction of direct public
  execution APIs.
- Each completed logical slice is committed with a detailed conventional
  commit message.

## Milestones

### Milestone 1: Freeze Scheduler Execution Contract

**Goal:** Define the only allowed public workflow run contract before removing
bypass APIs.

**Tasks:**
- [ ] Identify canonical scheduler DTOs and command names for public runs.
- [ ] Record no-compatibility decision for direct `workflow_run` and
  `executeWorkflow(graph)` facades.
- [ ] Decide allowed private/internal runtime helper names and visibility.
- [ ] Update plan execution notes with the final contract decision.

**Verification:**
- Review public exports in `crates/pantograph-workflow-service/src/lib.rs`.
- Review Tauri command registration in `src-tauri/src/app_setup.rs`.
- Review frontend `WorkflowBackend` type surface.

**Status:** Not started.

### Milestone 2: Remove Rust Core Direct-Run Public API

**Goal:** Make the workflow service public API scheduler-only.

**Tasks:**
- [ ] Remove or privatize `WorkflowService::workflow_run`.
- [ ] Keep `workflow_run_internal` private to scheduler-owned code.
- [ ] Update attribution and preflight paths so no public run path calls host
  execution without scheduler admission.
- [ ] Update workflow service tests from direct run APIs to scheduler session
  APIs.
- [ ] Update workflow service README API contract.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- Search check for forbidden public direct run surface in workflow-service.

**Status:** Not started.

### Milestone 3: Remove Tauri And Frontend Raw-Graph Execution

**Goal:** Ensure the graph editor and Tauri invoke surface cannot run a raw
workflow graph outside scheduler-visible session lifecycle.

**Tasks:**
- [ ] Remove `execute_workflow_v2` from Tauri command registration.
- [ ] Remove `executeWorkflow(graph)` from `WorkflowBackend`, mocks, and app
  services.
- [ ] Ensure the graph editor creates or reuses a backend-owned session before
  enabling Run.
- [ ] Keep diagnostics events sourced from scheduler-visible session execution.
- [ ] Update Tauri/frontend READMEs for the scheduler-only contract.

**Verification:**
- `npm run typecheck`
- `npm run test:frontend`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Tauri search check for forbidden `execute_workflow_v2` registration.

**Status:** Complete.

### Milestone 4: Migrate Bindings And HTTP Adapter

**Goal:** Make all host-facing bindings expose scheduler-backed execution only.

**Tasks:**
- [ ] Remove UniFFI direct `workflow_run` export and tests.
- [ ] Remove Rustler/frontend HTTP direct `workflow_run` export and tests.
- [ ] Expose scheduler session create/run/close paths as the supported binding
  execution surface.
- [ ] Update binding and adapter README API consumer contracts.
- [ ] Update generated or host-language smoke tests as required by binding
  standards.

**Verification:**
- Binding native tests for UniFFI and Rustler.
- Host-language smoke tests where existing harnesses support them.
- Search check for forbidden exported direct run names.

**Status:** Not started.

### Milestone 5: Guardrails And Documentation

**Goal:** Prevent reintroduction of direct public execution and make the
scheduler invariant visible to maintainers.

**Tasks:**
- [ ] Add or update an ADR for scheduler-only workflow execution.
- [ ] Add a repo check or focused test that scans public/frontend/binding
  surfaces for forbidden direct execution APIs.
- [ ] Update examples to use scheduler session APIs.
- [ ] Record deferred follow-up refactors below.

**Verification:**
- Documentation traceability review.
- Guardrail test/check fails before allowlist updates when direct public run
  APIs are present.
- `git diff --check`

**Status:** Not started.

## Execution Notes

- 2026-04-25: Existing dirty source files were committed before this plan. The
  remaining dirty asset changes are unrelated and must stay untouched.
- 2026-04-25: Removed frontend and Tauri raw-graph execution surfaces. GUI and
  package toolbar run actions now require an active backend-owned session and
  use `runSession(sessionId)` only. Tauri `execute_workflow_v2` is no longer a
  registered invoke command.

## Commit Cadence Notes

- Commit when each logical slice is complete and verified.
- Keep code, tests, and README updates for the same contract slice together.
- Follow `COMMIT-STANDARDS.md`; do not include test output in commit messages.
- Do not stage unrelated asset changes.

## Optional Subagent Assignment

None. This plan will be implemented serially unless the work is explicitly
delegated later.

## Re-Plan Triggers

- Scheduler session APIs cannot represent unsaved edit-session graph runs
  without new contract work.
- Binding generation requires an incompatible artifact workflow not covered by
  current scripts.
- A direct execution helper must remain public for a documented product use
  case.
- Guardrail checks produce false positives that cannot be resolved with a small
  allowlist.
- Dirty unrelated implementation files appear during a milestone.

## Recommendations

- Prefer API removal over compatibility wrappers because direct workflow
  execution is a platform stability hazard and backwards compatibility is not
  required.
- Keep the actual runtime execution helper private and scheduler-invoked rather
  than deleting low-level host execution mechanics; the scheduler still needs a
  runtime implementation behind its admission boundary.

## Deferred Follow-Up Refactors

These are intentionally deferred until the scheduler-only execution invariant is
implemented and verified.

- Break up oversized binding/runtime files:
  - `crates/pantograph-uniffi/src/runtime.rs`
  - `crates/pantograph-frontend-http-adapter/src/lib.rs`
  - `src/services/workflow/WorkflowService.ts`
  - `packages/svelte-graph/src/backends/MockWorkflowBackend.ts`
- Split large contract modules where scheduler-only work leaves natural
  boundaries:
  - `crates/pantograph-workflow-service/src/workflow/contracts.rs`
- Refactor toolbar structure after API migration:
  - `packages/svelte-graph/src/components/WorkflowToolbar.svelte`
  - Extract run-state/event handling if the component remains over the review
    threshold.
- Rename private/internal low-level runtime helpers so the scheduler boundary is
  obvious.
- Strengthen architecture docs beyond touched README updates:
  - Add or update an ADR stating that scheduler admission is the exclusive
    workflow execution boundary.
  - Document intentionally private runtime execution helpers and allowed
    callers.
- Add longer-term guardrail tooling:
  - A repo check that rejects new public, binding, or frontend APIs shaped like
    direct workflow execution.
  - A search-based allowlist for private scheduler-invoked runtime helpers.

## Completion Summary

### Completed

- Not started.

### Deviations

- None.

### Follow-Ups

- Deferred follow-up refactors listed above.

### Verification Summary

- Not started.

### Traceability Links

- Module README updates: planned for touched modules.
- ADR added/updated: planned.
- PR notes: N/A until PR preparation.
