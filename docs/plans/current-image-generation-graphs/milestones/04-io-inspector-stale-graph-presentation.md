# Milestone 4: IO Inspector Stale Graph Presentation

**Goal:** Let users inspect stale graphs in the IO inspector and see exactly
which nodes or edges are stale.

**Tasks:**

- [x] Extend the IO inspector graph projection consumer to read backend stale
  graph facts.
- [ ] Add saved-graph inspection mode so IO inspector can render a stale graph
  with diagnostics even when no workflow run exists.
- [ ] Rename or wrap run-specific frontend graph presenters/components where
  needed so saved-graph inspection does not depend on run-only concepts.
- [ ] Introduce or extract a shared graph inspection projection/presenter
  layer. Saved-graph inspection and run inspection may have different query
  sources, but they must not become separate graph display models.
- [x] Mark stale nodes and stale edge issues in the graph view.
- [ ] Keep selected-node state, panel sizing, layout, and visual grouping in
  frontend state only.
- [ ] Show stale node details in the lower artifact/details panel when a stale
  node is selected.
- [ ] Make stale-node selection, artifact read actions, graph navigation, and
  settings navigation reachable by keyboard and exposed with accessible names.
- [ ] Preserve focus after selecting nodes or opening stale details so keyboard
  users do not lose their place in the graph/detail split view.
- [ ] Keep graph facts, stale diagnostics, model facts, artifact records, and
  retention facts backend-owned. Frontend may own only transient selection,
  filtering, panel sizing, focus, and hover state.
- [ ] Use declarative Svelte rendering for stale markers and details; do not
  manually mutate DOM or graph state outside the presenter/store boundary.
- [ ] Do not add page-local polling. Use the existing query/event pattern or
  add a backend-owned push/subscription path with explicit lifecycle cleanup.
- [ ] If a push/subscription path is introduced, define one page-local
  subscription helper that owns subscribe/unsubscribe, duplicate-listener
  prevention, stale-event coalescing, and unmount cleanup.
- [ ] Preserve artifact reading behavior for valid executed runs.
- [ ] Use the same graph presenter helpers for run snapshots and saved-graph
  diagnostics so missing-edge and stale-port facts are not silently dropped.

**Verification:**

- Frontend test renders a graph with backend stale facts and shows stale
  markers without fabricating diagnostics.
- Frontend test verifies selecting a stale node shows backend-provided reasons.
- Frontend test verifies saved-graph inspection mode renders without a run id
  and without artifact controls.
- Frontend presenter tests verify run snapshots and saved-graph inspections use
  the same graph display model while keeping run-only metadata optional.
- Backend/query tests verify saved-graph inspection does not require or fake a
  `workflow_run_id`.
- Frontend tests use resilient accessible selectors and cover selected stale
  node details without relying on brittle DOM structure.
- Frontend accessibility tests cover keyboard selection, named controls,
  focus-visible behavior, and settings navigation from IO inspector.
- If a subscription or timer is added, cleanup tests prove it stops on unmount
  and does not create duplicate listeners.
- Existing IO artifact read tests continue passing.

**Verification Results:**

- `node --experimental-strip-types --test src/components/workbench/runGraphPresenters.test.ts`
  and `npm run typecheck` passed after extending the run graph presenter and
  `RunGraphSnapshot` to consume backend `graph_diagnostics`, mark stale nodes,
  and mark stale edge rows/canvas edges without fabricating diagnostics.
- `node --experimental-strip-types --test src/services/workflow/WorkflowService.graphInspection.test.ts`,
  `npm run typecheck`, `cargo fmt --check`,
  `cargo test -p pantograph-workflow-service workflow_graph_inspection`, and
  `cargo test -p pantograph workflow_graph` passed after exposing
  `workflow_graph_inspect` through the Tauri/frontend workflow service boundary
  as a direct `WorkflowGraphInspectionProjection` read with no fabricated run
  context.

**Remaining Follow-Up:**

- Add saved-graph inspection mode and a run/saved shared graph inspection
  presenter shape that consumes `WorkflowService.inspectWorkflowGraph`.
- Show selected stale-node details in the lower I/O details panel.
- Add focused component/accessibility tests for keyboard stale-node selection,
  focus preservation, and saved-graph mode without artifact controls.

**Status:** Partially completed on 2026-05-10
