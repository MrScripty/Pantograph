# Plan: Graph Loading Sync Guard

## Objective

Resolve the intermittent graph-switching failure where loading a second workflow
can leave the previous graph's nodes visible while edges disappear. The fix must
make store-to-SvelteFlow synchronization graph-identity-aware so a local
drag-position node-sync suppression cannot suppress a later full graph load.

## Scope

### In Scope

- `src/components/WorkflowGraph.svelte`: app graph synchronization wiring,
  including workflow-vs-architecture graph source transitions.
- `src/components/workflowGraphSync.ts` and
  `src/components/workflowGraphSync.test.ts`: app graph sync decision contract
  and regression tests.
- `src/components/workflowGraphSource.ts` and its tests only if graph-source
  identity needs to be projected there instead of inside the component.
- `packages/svelte-graph/src/components/WorkflowGraph.svelte`: reusable package
  graph synchronization wiring.
- `packages/svelte-graph/src/workflowGraphSync.ts` and
  `packages/svelte-graph/src/workflowGraphSync.test.ts`: package graph sync
  decision contract and regression tests.
- README updates for touched source directories when the synchronization
  ownership or invariants change.

### Out of Scope

- Backend workflow loading, scheduler execution, diagnostics history, and saved
  workflow persistence.
- General `WorkflowGraph.svelte` decomposition beyond small helper extraction
  required by this bug fix.
- Architecture graph redesign. The plan only preserves correct source
  transition behavior for the existing architecture graph mode.

## Inputs

### Problem

Switching workflows is intermittently stale. The first workflow renders, but a
later workflow sometimes does not replace the nodes; instead the old nodes stay
on screen and all edges disappear. This is consistent with SvelteFlow receiving
the new edge array without the matching new node array.

### Codebase Findings

- App graph sync state lives in `src/components/WorkflowGraph.svelte`.
  `_skipNextNodeSync` is set in `onNodeDragStop()` before
  `updateNodePosition()` so the next backend-confirmed node array does not
  overwrite SvelteFlow internals.
- App graph `$effect` calls `computeWorkflowGraphSyncDecision()` with the
  current store node and edge references. If `applyNodes` is false but
  `applyEdges` is true, the component can render mismatched node and edge
  snapshots.
- `src/components/workflowGraphSync.ts` currently treats
  `_skipNextNodeSync` as a graph-agnostic boolean. It also advances
  `nextPrevNodesRef` to the store node reference even when node assignment is
  intentionally skipped.
- The app graph has an additional source selector in
  `src/components/workflowGraphSource.ts`. Architecture mode bypasses the sync
  helper; architecture-pending intentionally returns without rendering workflow
  nodes.
- The same boolean suppression contract exists in the reusable package graph at
  `packages/svelte-graph/src/components/WorkflowGraph.svelte` and
  `packages/svelte-graph/src/workflowGraphSync.ts`.
- Existing tests in both graph surfaces explicitly preserve "skip nodes but
  allow edge updates" behavior without constraining it to the same graph
  identity. That test shape currently permits the reported failure.
- Workflow store graph snapshots carry a stable
  `derived_graph.graph_fingerprint`; `createWorkflowStoreGraphState()` builds
  one when a loaded graph omits it. This is the preferred graph revision source
  for workflow graph identity.

### Constraints

- Follow `PLAN-STANDARDS.md`, `CODING-STANDARDS.md`,
  `FRONTEND-STANDARDS.md`, `TESTING-STANDARDS.md`, and
  `DOCUMENTATION-STANDARDS.md`.
- The backend remains the source of truth for workflow data. The frontend may
  keep only transient UI state needed by SvelteFlow interaction lifecycle.
- No optimistic business-data updates. The node-sync suppression exists only to
  preserve SvelteFlow internal metadata after a backend-confirmed drag update.
- UI synchronization must remain event/reactivity-driven, not polling-based.
- Touched source directories already have README files; update them if the
  ownership contract changes.
- Worktree hygiene during implementation must account for currently unrelated
  dirty/deleted assets and `.pantograph` files. Do not revert or include those
  files unless the user explicitly assigns them to this work.

### Assumptions

- The intermittent failure occurs when `_skipNextNodeSync` is still pending as
  another workflow load updates the store node and edge references.
- A workflow graph fingerprint change represents a graph replacement or
  structural revision where node and edge snapshots must be synchronized as one
  coherent snapshot.
- Preserving same-graph drag suppression is still required to avoid regressing
  SvelteFlow internals and measured dimensions.
- Package and app graph sync helpers should remain behaviorally aligned because
  both graph components use the same interaction model.

### Dependencies

- Svelte 5 reactive `$effect` ordering.
- `@xyflow/svelte` node and edge object enrichment.
- `packages/svelte-graph/src/graphRevision.ts` derived graph fingerprints.
- Workflow store materialization in
  `packages/svelte-graph/src/stores/workflowStoreGraphState.ts`.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Drag-position updates start reassigning nodes again and lose SvelteFlow internals. | Medium | Keep same-graph suppression behavior covered by unit tests. |
| Graph identity is derived from the wrong value and changes too often or not often enough. | High | Use workflow graph fingerprint for workflow graphs and an explicit source key for app architecture mode. Add tests for graph-key changes. |
| App and package graph sync behavior diverges. | Medium | Apply the same sync decision contract and equivalent regression tests to both surfaces. |
| Architecture-pending transitions flash workflow nodes or clear the canvas incorrectly. | Medium | Preserve the existing `architecture-pending` invariant and only clear/reset sync guards when the rendered graph source actually changes. |
| Existing large graph components exceed file-size/decomposition targets. | Low | Keep edits minimal, document that full component decomposition is a pre-existing follow-up from the graph editor decomposition plan. |

## Clarifying Questions

- None.
- Reason: The bug, likely cause, and acceptance behavior can be inferred from
  the code path and user report.
- Revisit trigger: Implementation shows the failure is caused by store load
  races outside the graph sync decision path.

## Definition of Done

- Loading a different workflow while a node-sync suppression is pending applies
  the new workflow's nodes and edges together.
- Same-graph node drag still suppresses the immediate store-to-local node
  reassignment needed to preserve SvelteFlow internals.
- App graph source transitions do not display stale workflow nodes while the
  architecture graph is pending.
- Package graph and app graph synchronization contracts are aligned and covered
  by tests.
- Touched README/invariant documentation reflects the graph-identity-aware sync
  contract.
- No unrelated dirty worktree files are included in implementation commits.

## Milestones

### Milestone 1: Freeze The Sync Contract

**Goal:** Define the graph identity and skip-token behavior before editing
component wiring.

**Tasks:**

- [ ] Replace the graph-agnostic skip boolean contract in both
  `workflowGraphSync.ts` helpers with a contract that accepts the current graph
  sync key and the graph key associated with the pending node-sync suppression.
- [ ] Make graph-key changes force coherent node and edge synchronization even
  when a node-sync suppression is pending.
- [ ] Preserve same-graph node reassignment suppression after local drag.
- [ ] Return updated previous refs and skip-token state explicitly so the
  component remains the single owner of mutable UI sync state.

**Verification:**

- Add or update app helper tests proving:
  - same-graph drag suppression skips node reassignment;
  - graph-key changes apply nodes and edges despite a pending skip;
  - edge updates are only allowed independently when they belong to the same
    graph key;
  - skipped same-graph node sync does not mark a different graph's node ref as
    already applied.
- Add equivalent package helper tests.
- Run the helper tests directly with Node's test runner.

**Status:** Complete.

### Milestone 2: Wire App Graph Identity

**Goal:** Ensure the app graph passes a stable graph sync key and clears stale
suppression across workflow and architecture source changes.

**Tasks:**

- [ ] Derive a workflow sync key from the active workflow graph fingerprint.
- [ ] Derive an explicit app architecture sync key when rendering the
  architecture graph.
- [ ] Keep `architecture-pending` from rendering stale workflow data while also
  preventing a pending workflow drag skip from leaking into the eventual
  architecture render.
- [ ] Store the node-drag suppression token with the current graph sync key,
  not as an unqualified boolean.
- [ ] Update `src/components/README.md` if the sync/source ownership invariant
  changes.

**Verification:**

- Extend `workflowGraphSource` or component-adjacent tests as needed to cover
  workflow, architecture, and architecture-pending source key behavior.
- Run app graph sync/source unit tests.
- Run frontend typecheck for the changed TypeScript/Svelte surface.

**Status:** Complete.

### Milestone 3: Wire Package Graph Identity

**Goal:** Apply the same graph-identity-aware sync behavior to the reusable
package graph.

**Tasks:**

- [ ] Derive the package graph sync key from
  `workflowGraphStore.derived_graph.graph_fingerprint`.
- [ ] Use a deterministic fallback key if a graph snapshot lacks a fingerprint
  before store materialization completes.
- [ ] Store the node-drag suppression token with the current package graph key.
- [ ] Update package README/component README invariants if the sync ownership
  contract changes.

**Verification:**

- Run package graph sync unit tests.
- Run package/frontend typecheck covering `packages/svelte-graph`.

**Status:** Complete.

### Milestone 4: Cross-Surface Regression Verification

**Goal:** Prove the fix covers the reported user flow and does not regress graph
interaction behavior.

**Tasks:**

- [ ] Add an integration-style unit test, if feasible without a browser, that
  simulates: graph A applied, drag suppression set for graph A, graph B store
  refs and graph key arrive, graph B nodes and edges both apply.
- [ ] Confirm no call site still uses an unqualified boolean skip state for
  store-to-SvelteFlow node sync.
- [ ] Check touched areas against standards for unrelated non-compliance that
  implementation must address in a final cleanup step.

**Verification:**

- `node --experimental-strip-types --test src/components/workflowGraphSync.test.ts packages/svelte-graph/src/workflowGraphSync.test.ts`
- `npm run -w frontend test:run`
- `npm run -w frontend check:types`

**Status:** Complete.

### Milestone 5: Standards Cleanup

**Goal:** Resolve standards issues in touched code that remain after the bug
fix.

**Tasks:**

- [ ] Review touched files for file-size, ownership, magic-string, README, and
  frontend synchronization compliance.
- [ ] Extract tiny pure helpers only if needed to avoid increasing the existing
  component ownership burden.
- [ ] Update the plan execution notes with verification results and any
  deviations.

**Verification:**

- Re-run the affected tests after cleanup.
- Confirm `git status --short` only shows intended implementation/docs changes
  plus pre-existing unrelated dirty files.

**Status:** Complete.

## Standards Compliance Review

- `PLAN-STANDARDS.md`: This plan includes objective, scope, inputs,
  assumptions, risks, ordered milestones, verification, re-plan triggers, and
  completion criteria.
- `CODING-STANDARDS.md`: The intended implementation keeps backend workflow
  data source-of-truth ownership intact and limits frontend state to transient
  SvelteFlow sync lifecycle state. The touched `WorkflowGraph.svelte` files are
  already above the UI component review trigger; this plan restricts edits to
  helper wiring and records broader decomposition as a separate follow-up.
- `FRONTEND-STANDARDS.md`: The fix remains reactive/event-driven and does not
  introduce polling. The graph component remains the single owner of local
  SvelteFlow synchronization lifecycle state.
- `TESTING-STANDARDS.md`: Tests stay colocated with the pure sync helpers and
  cover the race-prone state transition directly.
- `DOCUMENTATION-STANDARDS.md`: Touched source directories already contain
  README files. Implementation must update the relevant invariants if the sync
  contract changes.

## Execution Notes

- 2026-04-26: Plan created after reading the standards and tracing the app and
  package graph synchronization paths. Current worktree contains unrelated
  deleted `.pantograph`/asset files and untracked diagnostics/assets; leave them
  untouched during implementation unless the user explicitly changes scope.
- 2026-04-26: Milestones 1-4 implemented as one buildable behavior slice
  because the helper contract and Svelte component call sites must change
  together. App and package sync helpers now use graph-keyed node-sync
  suppression. App and package graph components pass graph identity keys and
  clear stale suppression during architecture-pending transitions. Frontend unit
  tests and repo typecheck passed; the plan's obsolete workspace-form commands
  were replaced with the repository's actual `npm run test:frontend` and
  `npm run typecheck` scripts.
- 2026-04-26: Final `npm run lint:full` found an unrelated unused
  `currentGraphName` import in `src/components/WorkflowToolbar.svelte`. This is
  outside the graph-sync write set but blocks the repository lint gate, so it
  will be fixed as a separate compile-unblocking standards cleanup commit.

## Commit Cadence Notes

- Commit after each milestone is implemented and verified.
- Follow `COMMIT-STANDARDS.md` for conventional commit style, detailed commit
  body, and agent metadata.
- Keep app/package sync contract changes and their tests in the same logical
  commit when they form one behavior slice.
- Keep README-only standards updates separate only if they are not part of the
  behavior slice.

## Optional Subagent Assignment

- None planned.
- Reason: The fix is small and touches shared sync semantics where serial
  implementation reduces drift risk.
- Revisit trigger: Implementation expands into broader component decomposition
  or independent browser-level reproduction work.

## Re-Plan Triggers

- Workflow graph fingerprints are unavailable, unstable, or not updated when
  switching loaded workflows.
- The package graph and app graph require incompatible graph identity sources.
- Preserving same-graph drag suppression conflicts with coherent graph-load
  synchronization.
- The failure reproduces without a pending node-drag suppression token.
- Fixing this correctly requires changing workflow store load order,
  backend session transition semantics, or SvelteFlow component lifecycle.
- Browser verification reveals an additional stale-state owner outside
  `workflowGraphSync.ts` and `WorkflowGraph.svelte`.

## Recommendations

- Prefer a graph-keyed suppression token over an additional debounce or timer.
  The bug is an ownership/identity issue, and timing-based mitigation would hide
  the race without proving node and edge snapshots belong together.
- Keep the app and package helper contracts structurally identical. Divergence
  would make future graph editor fixes harder to reason about.

## Completion Summary

### Completed

- Milestone 1: Graph sync helpers now require graph identity and graph-scoped
  skip tokens.
- Milestone 2: App graph sync now keys workflow and architecture renders and
  clears stale suppression during architecture-pending transitions.
- Milestone 3: Package graph sync now derives the same graph identity from the
  workflow graph fingerprint.
- Milestone 4: Cross-surface regression tests and call-site search completed.
- Milestone 5: README invariants updated and an unrelated lint blocker removed
  in a separate cleanup commit.

### Deviations

- The plan's initial workspace-form verification commands were not usable
  because this repository has no `frontend` workspace. The implementation used
  the repository's actual root scripts instead.
- Milestones 1-4 were committed as one integrated behavior slice because the
  sync helper contract and component call sites had to change together to keep
  the tree buildable.
- `npm run lint:full` exposed an unrelated unused import in
  `src/components/WorkflowToolbar.svelte`; it was fixed separately because it
  blocked final verification.

### Follow-Ups

- Continue the pre-existing graph editor decomposition follow-up for
  `WorkflowGraph.svelte` file size and ownership reduction after this targeted
  bug fix is complete.

### Verification Summary

- `node --experimental-strip-types --test src/components/workflowGraphSync.test.ts packages/svelte-graph/src/workflowGraphSync.test.ts`: passed.
- `npm run test:frontend`: passed.
- `npm run typecheck`: passed.
- `npm run lint:full`: passed after the unrelated toolbar import cleanup.

### Traceability Links

- Module README updated: `src/components/README.md`,
  `packages/svelte-graph/src/README.md`, and
  `packages/svelte-graph/src/components/README.md`.
- ADR added/updated: N/A.
  Reason: This plan changes a local frontend synchronization invariant, not a
  durable architecture boundary.
  Revisit trigger: The fix expands into workflow store or backend session
  identity redesign.
