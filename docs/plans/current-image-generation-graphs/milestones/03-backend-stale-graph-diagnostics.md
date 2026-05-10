# Milestone 3: Backend Stale Graph Diagnostics

**Goal:** Add backend-owned stale graph diagnostics that make invalid graphs
inspectable and explain why they cannot be submitted.

**Tasks:**

- [ ] Define a stale graph diagnostic DTO in the backend service/domain layer.
- [ ] Classify existing node-engine validation, workflow-service contract
  validation, and effective-definition errors into that DTO instead of adding
  a parallel validator.
- [ ] Replace or wrap string-returning contract-validation paths with
  structured diagnostics before exposing diagnostics across backend/frontend
  boundaries.
- [ ] Use the retired-node classification path created by the canonicalization
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
- [ ] Keep stale graph validation as a replayable projection over saved graph
  data. Loading the same saved graph should produce the same stale facts without
  requiring workflow-run side effects.
- [ ] Bound diagnostic reason length and field counts before values cross IPC,
  submit/admission, or run-inspection boundaries.
- [ ] Update diagnostics or workflow-service README ownership notes.

**Verification:**

- Unit tests cover each stale graph diagnostic kind.
- Cross-layer test loads a stale graph and verifies backend returns stale node
  and edge facts without frontend inference.
- Test verifies a retired node is not silently rewritten by current load/save
  paths.
- Test verifies stale diagnostic records are structured backend DTOs, not
  frontend-parsed strings.
- Serde round-trip tests cover stale graph diagnostic DTOs and bounded reason
  payloads.
- Replay test verifies the same stale saved graph produces stable diagnostics
  across repeated backend inspection calls.
- Boundary test verifies oversized stale reason payloads are rejected or
  summarized before IPC/admission exposure.
- Submit/admission test verifies stale executable graphs are blocked with
  bounded visible reasons.

**Status:** Not started
