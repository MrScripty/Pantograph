# Milestone 3: Backend Stale Graph Diagnostics

**Goal:** Add backend-owned stale graph diagnostics that make invalid graphs
inspectable and explain why they cannot be submitted.

**Tasks:**

- [x] Define a stale graph diagnostic DTO in the backend service/domain layer.
- [x] Classify existing node-engine validation, workflow-service contract
  validation, and effective-definition errors into that DTO instead of adding
  a parallel validator.
- [x] Replace or wrap string-returning contract-validation paths with
  structured diagnostics before exposing diagnostics across backend/frontend
  boundaries.
- [x] Use the retired-node classification path created by the canonicalization
  split to emit stale diagnostics for retired node types.
- [ ] Cover unknown node types, retired node types, missing definitions,
  missing nodes, missing edge endpoints, missing edge handles, unresolved Puma
  model references, and incompatible or stale port contracts.
- [ ] Return stale graph diagnostics from graph load/read-model paths used by
  graph editor, the shared graph inspection projection, IO inspector
  saved-graph mode, and run inspection.
- [ ] Include bounded submit/admission reasons when stale executable graphs are
  blocked.
- [ ] Ensure diagnostics are factual and presentation-neutral.
- [ ] Ensure stale graph diagnostics can represent node-level, edge-level, and
  graph-level facts without requiring frontend inference.
- [x] Keep stale graph validation as a replayable projection over saved graph
  data. Loading the same saved graph should produce the same stale facts without
  requiring workflow-run side effects.
- [x] Bound diagnostic reason length and field counts before values cross IPC,
  submit/admission, or run-inspection boundaries.
- [x] Update diagnostics or workflow-service README ownership notes.

**Verification:**

- Unit tests cover the foundational stale graph diagnostic kinds.
- Cross-layer test loads a stale graph and verifies backend returns stale node
  and edge facts without frontend inference.
- Test verifies a retired node is not silently rewritten by current load/save
  paths.
- Test verifies foundational stale diagnostic records are structured backend
  DTOs, not frontend-parsed strings.
- Serde round-trip tests cover stale graph diagnostic DTOs and bounded reason
  payloads.
- Replay test verifies the same stale saved graph produces stable diagnostics
  across repeated backend inspection calls.
- Boundary test verifies oversized stale reason payloads are rejected or
  summarized before IPC/admission exposure.
- Submit/admission test verifies stale executable graphs are blocked with
  bounded visible reasons.

**Verification Results:**

- `cargo test -p pantograph-workflow-service graph::` passed after adding
  `WorkflowGraphDiagnostic`, bounded diagnostic payload helpers, the structured
  graph contract-validation classifier, and tests for retired node types,
  unknown node types, missing edge endpoint nodes, missing source outputs,
  missing target inputs, serde round-trip, and payload bounds.
- `cargo test -p pantograph-workflow-service graph::inspection` passed after
  adding `WorkflowGraphInspectionProjection`, selected-node diagnostic
  projection, optional run context, and replay-stability tests.
- `cargo test -p pantograph-workflow-service workflow_graph_inspection` passed
  after adding a backend service facade test that saves a stale retired
  diffusion graph, loads it through graph inspection, returns backend-owned
  node and edge diagnostics, and repeats the read to prove stable facts.
- `cargo test -p pantograph-workflow-service graph::session_contract` passed
  after adding `graph_diagnostics` to edit-session graph snapshot responses and
  proving stale session snapshots carry backend-owned retired-node and missing
  edge endpoint diagnostics.
- `cargo test -p pantograph-workflow-service run_graph`,
  `cargo test -p pantograph-workflow-service workflow_run_inspection_query_returns_factual_run_snapshot_parts`,
  and `npm run typecheck` passed after adding `graph_diagnostics` to historic
  `WorkflowRunGraphProjection`, proving run inspection can carry backend-owned
  stale graph facts from the reconstructed run snapshot.

**Remaining Follow-Up:**

- Extend coverage to unresolved Puma model references and stale dynamic port
  contracts once the graph inspection projection exists.
- Add cross-layer saved-graph inspection and submit/admission blocking tests in
  separate slices to avoid mixing editor stale facts with execution admission
  behavior.

**Status:** Partially completed on 2026-05-10
