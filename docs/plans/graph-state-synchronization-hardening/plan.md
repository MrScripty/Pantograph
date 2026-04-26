# Plan: Graph State Synchronization Hardening

## Objective

Make graph editing, workflow loading, diagnostics-facing runtime data, and
backend graph session state correct under overlapping async operations while
reducing full-graph churn on common editor actions.

The resulting architecture must preserve backend-owned graph data as the source
of truth. The frontend may hold transient UI overlays, drag state, selection,
and in-flight request tokens, but persistent workflow graph structure must only
change from accepted backend mutation responses that still belong to the active
workflow edit session.

## Scope

### In Scope

- Prevent stale workflow load responses from replacing a newer selected
  workflow.
- Prevent stale graph mutation responses from applying after the active edit
  session changes.
- Make graph mutation actions awaitable where callers depend on ordering.
- Replace multi-edge and multi-node deletion races with ordered or bulk
  backend-owned mutations.
- Separate transient runtime/output overlays from persisted graph node data.
- Ensure workflow save uses structural graph data only.
- Add focused regression tests for workflow switching, stale responses,
  deletion ordering, runtime overlay persistence safety, and diagnostics history
  interactions.
- Reduce avoidable full-graph cloning, fingerprinting, and repeated frontend
  scans in touched paths when the change is local to a node/edge edit.
- Refactor touched surrounding code that violates the standards in
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.

### Out of Scope

- Redesigning scheduler-only workflow execution.
- Changing workflow run id or diagnostics run identity contracts.
- Rebuilding the diagnostics UI beyond changes required by graph/runtime data
  separation.
- Replacing the entire graph session store with event sourcing.
- Maintaining compatibility for transient runtime fields that were accidentally
  saved into workflow files.
- General visual redesign of the graph editor.

## Inputs

### Problem

Analysis of the graph code found correctness risks and performance costs:

- Async graph mutation responses are applied without checking that the response
  still belongs to the active session.
- Workflow loading has no request token or cancellation guard, so slower loads
  can overwrite newer selections.
- Edit sessions are created repeatedly and previous sessions are not closed
  when superseded.
- Runtime outputs are merged into the same node data used to save persistent
  workflow files.
- Multi-delete and reconnect flows perform multiple backend calls and apply
  full graph snapshots between related operations.
- Backend graph edit sessions clone full graphs for undo and response DTOs, and
  repeatedly canonicalize/fingerprint whole graphs.
- Similar graph editor behavior exists in both app-local and package-local
  component paths, increasing fix drift risk.

### Constraints

- Backend-owned persistent graph data remains authoritative.
- No optimistic updates for persistent graph structure.
- Frontend runtime output state is transient display data and must not be
  persisted.
- Source changes must conform to the coding, frontend, testing, tooling, and
  commit standards in the Coding-Standards repository.
- Existing unrelated dirty files must remain untouched unless explicitly
  assigned to this work.
- Implementation must proceed in logical slices, with verification and a
  detailed conventional commit after each completed slice.

### Assumptions

- Existing workflow files containing accidentally saved runtime fields may be
  cleaned opportunistically on load/save by structural projection. No durable
  migration is required unless implementation reveals checked-in fixtures with
  invalid shape.
- The backend can expose batch edge/node removal APIs without preserving old
  per-edge/per-node UI ordering behavior.
- The current edit session id remains the graph editing session identity; this
  plan does not change workflow execution run identity.
- Runtime overlay data can be keyed by node id and displayed by node components
  without changing the structural `WorkflowGraph` DTO.
- Both `src/components` and `packages/svelte-graph/src/components` paths are
  still relevant until the repo removes one of them; fixes must not land in
  only one active path.

### Dependencies

- `packages/svelte-graph/src/stores/createSessionStores.ts`
- `packages/svelte-graph/src/stores/createWorkflowStores.ts`
- `packages/svelte-graph/src/stores/workflowStoreMaterialization.ts`
- `packages/svelte-graph/src/stores/runtimeData.ts`
- `packages/svelte-graph/src/components/WorkflowGraph.svelte`
- `packages/svelte-graph/src/components/workflowGraphBackendActions.ts`
- `src/components/WorkflowGraph.svelte`
- `src/components/workflowGraphBackendActions.ts`
- `src/components/workflowToolbarEvents.ts`
- `src/components/WorkflowPersistenceControls.svelte`
- `src/backends/TauriWorkflowBackend.ts`
- `src/services/workflow/WorkflowService.ts`
- `crates/pantograph-workflow-service/src/graph/session.rs`
- `crates/pantograph-workflow-service/src/graph/session_state.rs`
- `crates/pantograph-workflow-service/src/graph/session_node_api.rs`
- `crates/pantograph-workflow-service/src/graph/session_connection_api.rs`
- `crates/pantograph-workflow-service/src/graph/types.rs`
- Tauri graph command wiring for any new batch mutation endpoints.

### Affected Structured Contracts

- `WorkflowBackend` graph mutation methods should make mutation response
  ownership explicit enough that stale responses can be rejected before store
  application.
- Any new batch mutation request/response DTOs must be session-scoped and
  return one backend-authored graph snapshot for the completed logical action.
- `WorkflowGraph` must represent persisted graph structure only.
- Runtime overlay data must have a separate frontend store/type contract and
  must not be serialized through `save_workflow`.
- Tests should assert graph revision/session ownership rather than relying on
  local response ordering assumptions.

### Affected Persisted Artifacts

- Saved workflow JSON files may stop containing transient runtime output fields
  after save.
- Checked-in workflow fixtures or examples may need cleanup if they contain
  runtime-only fields.
- No SQLite diagnostics schema change is expected for this plan.

### Concurrency and Lifecycle Review

- `loadWorkflowByName` must create a monotonically increasing load token or
  equivalent cancellation guard. Only the latest active token may apply graph,
  metadata, session, or last-graph state.
- Superseded edit sessions must be closed through `removeSession` after a new
  session is safely active, with close failures logged but not allowed to
  corrupt the active session.
- Every async graph mutation must capture the session id used for the request
  and verify that it still matches the active session before applying any graph
  response or mutation-derived execution state.
- Ordered UI operations that depend on previous graph mutations must await
  mutation completion or use one backend batch endpoint.
- Runtime overlays must be cleared on workflow/session change and on explicit
  run reset, without modifying structural graph node data.
- Any timers or delayed UI state introduced by the fix must have explicit
  ownership and deterministic cleanup. This plan should not add polling.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Stale response guards hide backend errors that should be visible | Medium | Distinguish stale-success/stale-error handling and keep active-session errors surfaced. |
| Closing superseded sessions races with a newly active session | High | Close only captured previous session ids after the replacement session is applied and never close the current id by lookup. |
| Runtime overlay split requires node component plumbing | Medium | Introduce a narrow overlay selector API and migrate consumers incrementally in one logical slice. |
| Batch deletion changes undo semantics | Medium | Make one user delete action produce one backend undo snapshot and test undo after batch removal. |
| Duplicated graph editors diverge during fixes | High | Centralize shared action helpers first or apply paired edits with tests covering both active imports. |
| Backend performance work expands into broad refactor | Medium | Limit performance milestone to touched mutation/session paths and defer event-sourced session storage if needed. |
| Existing accidentally persisted runtime fields affect tests | Low | Strip runtime-only fields in structural projection and update fixtures explicitly. |

### Standards Compliance Review

The plan is standards-compliant only if the final implementation preserves the
following constraints:

- Persistent graph structure remains backend-owned. Frontend code may hold
  transient overlays and request tokens, but it must not optimistically mutate
  saved graph structure or repair backend graph state locally.
- Async lifecycle ownership must be explicit. Each workflow load, graph
  mutation, session close, and delayed UI timer must have one owner, stale
  response protection, and deterministic cleanup.
- New structured contracts must be session-scoped and graph-specific. Avoid raw
  ambiguous id fields in new DTOs; use explicit names such as `session_id`,
  `graph_revision`, `node_ids`, and `edge_ids`.
- Runtime/output data must be modeled as transient UI state, not business data
  inside `WorkflowGraph`.
- Tests must cover stale responses and overlapping operations, not just
  happy-path type compatibility.

Immediate touched code review found these compliance issues or review triggers:

- `packages/svelte-graph/src/components/WorkflowGraph.svelte` is 898 lines and
  `src/components/WorkflowGraph.svelte` is 890 lines. Both exceed the 250-line
  UI component decomposition review trigger. Any implementation that touches
  graph interaction logic must extract focused controller/helper components or
  record which duplicate path is inactive and remove/defer it with a traceable
  decision.
- `packages/svelte-graph/src/stores/createWorkflowStores.ts` is 519 lines and
  owns structural graph state, runtime output mutation, session mutation
  dispatch, grouping, streaming, selection helpers, and graph materialization.
  The runtime overlay and stale mutation work must split this into focused
  modules rather than growing it further.
- `crates/pantograph-workflow-service/src/graph/types.rs` is 558 lines. Batch
  mutation DTOs and related edit-session request/response types should live in
  `session_types.rs` or another focused graph contract module, and any touched
  type groups should be moved out of `types.rs` when practical.
- `src/services/workflow/WorkflowService.ts` is 486 lines and exposes many
  public service methods across sessions, execution, diagnostics, undo/redo,
  and persistence. This plan must avoid adding graph orchestration there; if it
  must be touched beyond narrow forwarding, extract focused transport helpers.
- `src/components/workflowGraphBackendActions.ts` and
  `packages/svelte-graph/src/components/workflowGraphBackendActions.ts` each
  expose multiple graph action functions and duplicate responsibility. The
  implementation must centralize shared graph action behavior or document a
  removal path for the inactive copy before final verification.
- Source-root README files already exist for the immediate touched directories:
  `packages/svelte-graph/src`, `src/components`, `src/backends`,
  `src/services/workflow`, and
  `crates/pantograph-workflow-service/src/graph`. They must be updated if the
  implementation changes ownership or invariants.

Because these findings are in touched areas, the final refactor phase is
mandatory rather than optional. If implementation reveals a broader rewrite is
required to make the touched surroundings compliant, the re-plan trigger must
pause the work before release verification.

## Definition of Done

- Rapid workflow switching cannot apply stale workflow, metadata, session, or
  last-graph state.
- Graph mutation responses from an old session are ignored after a workflow
  switch.
- Deleting multiple selected nodes/edges creates one consistent backend graph
  transition and does not resurrect stale snapshots.
- Runtime node outputs display in the UI without changing `WorkflowGraph` or
  being saved into workflow JSON.
- Saving a workflow serializes structural node data and backend-derived graph
  metadata only.
- Previous edit sessions are closed when superseded, or an explicit lifecycle
  reason is recorded for any session that remains open.
- Touched oversized or multi-responsibility files are split, centralized, or
  covered by a traceable decomposition decision that keeps new code
  standards-compliant.
- Tests cover stale load, stale mutation, ordered deletion, runtime overlay
  persistence safety, and backend batch deletion behavior.
- Touched files satisfy standards or are covered by the final compliance
  refactor milestone.
- Final verification passes, or any blocked verification is recorded with a
  concrete blocker and follow-up.

## Milestones

### Milestone 1: Lifecycle Guard Foundation

**Goal:** Establish explicit active-session/load ownership before changing
mutation behavior.

**Tasks:**
- [x] Add a workflow-load request token to `createSessionStores`.
- [x] Capture and close superseded edit sessions without closing the current
  session.
- [x] Add stale-load regression tests for out-of-order workflow loads.
- [x] Add session lifecycle tests proving `removeSession` is called for
  superseded sessions.

**Verification:**
- `node --experimental-strip-types --test packages/svelte-graph/src/stores/createSessionStores.test.ts`
- `npm run typecheck`

**Status:** Complete.

### Milestone 2: Stale Mutation Rejection

**Goal:** Ensure backend mutation responses can only update the graph store
when they belong to the current edit session.

**Tasks:**
- [x] Make `syncGraphMutationFromBackend` return a promise with accepted,
  stale, and failed outcomes.
- [x] Capture request session ids and check active session before applying
  graph responses.
- [x] Apply the same guard to group, ungroup, update-group-ports, reconnect,
  edge-removal, and compatible-node insert paths.
- [x] Add regression tests for stale add-node, stale position update, stale
  group mutation, and edge-removal responses. Reconnect stale handling is
  implemented in both helper paths and covered by typecheck; direct helper
  runtime tests are deferred to Milestone 6 helper centralization because the
  current package helper uses publish-style `.js` source imports that Node's
  strip-types test runner cannot resolve without a build.

**Verification:**
- `node --experimental-strip-types --test packages/svelte-graph/src/stores/createWorkflowStores.test.ts`
- `npm run typecheck`

**Status:** Complete.

### Milestone 3: Ordered And Batch Graph Mutations

**Goal:** Make user-level delete/reconnect actions one ordered backend-owned
transition instead of a chain of full-graph races.

**Tasks:**
- [x] Add backend graph session requests for batch edge removal and, if needed,
  batch node removal.
- [x] Make one selected-delete action produce one backend response and one undo
  snapshot.
- [x] Update Tauri and TypeScript backend contracts for batch mutation calls.
- [x] Update both graph editor component paths to await ordered mutation
  outcomes.
- [x] Add Rust tests for batch delete graph shape and undo behavior.
- [x] Add frontend tests for selected multi-delete ordering.

**Verification:**
- `cargo test -p pantograph-workflow-service graph`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run -w frontend test:run -- WorkflowGraph workflowGraphBackendActions`
- `npm run -w frontend check:types`

**Status:** Complete.

### Milestone 4: Runtime Overlay Separation

**Goal:** Remove runtime outputs from persisted graph node data.

**Tasks:**
- [ ] Introduce a transient runtime overlay store keyed by node id.
- [ ] Move `updateNodeRuntimeData`, stream content, and clear-runtime behavior
  into the overlay path.
- [ ] Update node rendering and toolbar event handling to read overlay data
  without mutating structural nodes.
- [ ] Ensure workflow load/session changes clear overlays.
- [ ] Ensure save uses structural graph projection only.
- [ ] Add tests proving runtime outputs do not appear in `$workflowGraph` or
  saved workflow payloads.

**Verification:**
- `npm run -w frontend test:run -- runtimeData workflowStoreMaterialization workflowToolbarEvents`
- `npm run -w frontend check:types`

**Status:** Not started.

### Milestone 5: Derived Graph And Snapshot Efficiency

**Goal:** Reduce avoidable full-graph recomputation in paths touched by the
correctness work.

**Tasks:**
- [ ] Prefer backend-provided `derived_graph` when applying backend-owned graph
  snapshots.
- [ ] Restrict frontend derived graph rebuilds to mock/local-only graph
  construction paths.
- [ ] Avoid rebuilding the materialized `WorkflowGraph` in response to runtime
  overlay-only updates.
- [ ] Review backend mutation responses for duplicate fingerprint or dirty-task
  calculations in touched paths and remove repeated local recomputation where
  behavior is unchanged.
- [ ] Add tests that protect backend-derived graph identity from frontend
  recalculation drift.

**Verification:**
- `npm run -w frontend test:run -- graphRevision workflowStoreMaterialization createWorkflowStores`
- `cargo test -p pantograph-workflow-service graph`
- `npm run -w frontend check:types`

**Status:** Not started.

### Milestone 6: Final Standards Compliance Refactor

**Goal:** Bring touched surroundings into standards compliance after the main
behavior is corrected and before release verification.

**Tasks:**
- [ ] Split `createWorkflowStores.ts` so structural graph state, runtime
  overlays, mutation dispatch, streaming, grouping, and graph query helpers have
  focused owners.
- [ ] Extract graph editor controller/helper modules from both active
  `WorkflowGraph.svelte` paths so touched interaction logic is no longer owned
  by oversized components.
- [ ] Centralize duplicated graph backend action logic shared by
  `src/components` and `packages/svelte-graph/src/components`, or record which
  path is active and remove/defer the inactive copy with a traceable decision.
- [ ] Keep new Rust graph edit-session DTOs out of `types.rs`; move touched
  session/batch DTO groups into focused contract modules when needed to avoid
  growing the oversized type file.
- [ ] Keep `WorkflowService.ts` as a transport facade. If implementation
  requires new orchestration there, extract focused graph-session or diagnostics
  transport helpers before final verification.
- [ ] Split any other touched component, store, service, or Rust module whose
  responsibilities grow during implementation.
- [ ] Add or update README documentation for any source directory whose touched
  responsibilities are non-obvious and not already documented.
- [ ] Run a final source search for stale direct mutation patterns,
  session-id-unchecked response application, duplicate graph action paths, and
  runtime fields in structural graph serialization.
- [ ] Record any intentionally deferred decomposition with the reason, risk,
  and follow-up owner in this plan before release verification.

**Verification:**
- `git diff --check`
- `npm run -w frontend check:types`
- `npm run -w frontend test:run`
- `cargo test -p pantograph-workflow-service`
- `cargo check --manifest-path src-tauri/Cargo.toml`

**Status:** Not started.

### Milestone 7: Release Verification

**Goal:** Prove the graph editor and backend graph paths are shippable after
the refactor.

**Tasks:**
- [ ] Run the plan's full verification set.
- [ ] Run the release build command expected for Pantograph.
- [ ] Update this plan with completion notes, deviations, and verification
  results.
- [ ] Confirm only intended files are dirty before final commit/report.

**Verification:**
- `npm run -w frontend test:run`
- `npm run -w frontend check:types`
- `cargo test -p pantograph-workflow-service`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `bash launcher.sh --build-release`

**Status:** Not started.

## Execution Notes

Update during implementation:

- 2026-04-26: Plan created from graph code analysis. No implementation has
  started.
- 2026-04-26: Standards compliance pass added mandatory final refactor scope
  for oversized graph components, `createWorkflowStores.ts`, `types.rs`,
  duplicated backend action helpers, and `WorkflowService.ts` facade ownership.
- 2026-04-26: Milestone 1 started. Preflight confirmed the plan has required
  objective, ordered milestones, verification criteria, risks, re-plan
  triggers, and completion criteria. Existing dirty workflow/assets/SQLite
  artifacts are unrelated and will remain untouched.
- 2026-04-26: Milestone 1 completed. The planned `npm run -w frontend ...`
  verification commands are invalid because this repo has no `frontend`
  workspace, so verification used the equivalent root scripts and direct
  colocated Node test command. Added load transition guards, stale-created
  session cleanup, previous-session cleanup after replacement, and regression
  tests for stale loads and session lifecycle cleanup.
- 2026-04-26: Milestone 2 started. The implementation will keep stale-response
  ownership in the workflow store first, then thread session checks through
  package and app graph backend action helpers.
- 2026-04-26: Milestone 2 completed. Added awaitable mutation outcomes,
  active-session response guards, stale-safe backend graph sync, guarded group
  mutations, and stale-safe package/app graph action helper application.
  Regression coverage now includes stale update-node-data, add-node,
  node-position, group, and edge-removal responses. Direct package helper tests
  for reconnect were attempted, but the current Node strip-types harness cannot
  import package component helpers that use publish-style `.js` relative
  imports before build; Milestone 6 must make the centralized helper boundary
  directly testable.
- 2026-04-26: Milestone 3 started. Implementation will add a combined
  session-scoped delete-selection backend mutation so mixed node/edge
  selections produce one backend-authored graph snapshot and one undo entry.
- 2026-04-26: Milestone 3 completed. Added backend `remove_edges` and
  `delete_selection` session mutations, Tauri command wiring, TypeScript
  backend/service/store contracts, graph editor delete-selection usage, and
  regression coverage for batch edge removal, mixed selected deletion, undo
  behavior, and one frontend store-level selected-delete backend call. The
  planned frontend workspace commands remain invalid for this repo layout, so
  verification used the direct store test and root typecheck commands.

## Commit Cadence Notes

- Inspect `git status --short` before implementation.
- Do not begin implementation while unrelated dirty implementation files remain
  unresolved unless the user explicitly allows them.
- Commit each milestone or smaller verified logical slice before beginning the
  next slice.
- Use conventional commits from `COMMIT-STANDARDS.md`, for example:
  `fix(graph): ignore stale mutation responses`,
  `fix(graph): separate runtime overlays from persisted nodes`,
  `perf(graph): batch selected edge deletion`.
- Do not include verification logs or command output in commit messages.
- Review unpushed history for obvious regression/fix pairs before each commit.

## Optional Subagent Assignment

No concurrent worker execution is planned initially. The work crosses shared
frontend/backend contracts, so serial implementation is safer until the stale
response and runtime overlay contracts are established.

If later split, use one worker wave only after Milestone 2:

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| frontend-worker | Runtime overlay migration in frontend stores/components | Patch plus tests limited to frontend overlay write set | After Milestone 2 is committed |
| backend-worker | Batch graph mutation endpoint and Rust tests | Patch plus Rust tests limited to workflow-service/Tauri command write set | After frontend/backend DTO contract is frozen |

Workers must use isolated worktrees, non-overlapping write sets, and Markdown
reports per `PLAN-STANDARDS.md`.

## Re-Plan Triggers

- A required fix changes scheduler/workflow run identity contracts.
- Runtime overlay separation requires backend schema changes.
- Batch delete cannot preserve expected undo semantics without a larger
  command model.
- Either duplicated graph editor path is proven inactive and can be removed
  instead of maintained.
- Existing saved workflow files require a durable migration rather than
  projection-time cleanup.
- Final verification reveals broad failures outside graph/session/runtime
  overlay scope.
- Standards compliance review finds a touched module needs a larger refactor
  than the current milestone can safely contain.

## Recommendations

- Prefer a single shared graph action/controller module used by both graph
  editor component paths. This reduces drift and gives stale-response guards one
  owner.
- Prefer backend batch mutation endpoints over frontend loops. That gives one
  undo snapshot, one graph revision, and one response application per user
  action.
- Defer replacing full-graph undo snapshots with structural sharing or
  operation logs unless profiling after batch/stale fixes still shows backend
  graph sessions as a bottleneck.

## Completion Summary

### Completed

- Not started.

### Deviations

- None.

### Follow-Ups

- None recorded.

### Verification Summary

- Not run.

### Traceability Links

- Module README updated: N/A until implementation touches documentation
  boundaries.
- ADR added/updated: N/A unless implementation changes graph/session identity
  architecture.
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A.
