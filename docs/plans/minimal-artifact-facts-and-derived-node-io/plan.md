# Plan: Minimal Artifact Facts And Derived Node IO

## Objective

Remove redundant retained artifact bodies from workflow run inspection while
preserving clear node input/output provenance. A normal connected node input
should be derived from the immutable run graph plus the upstream node output
artifact, not recorded as a duplicate durable artifact. Persist only canonical
produced values, workflow boundary values, and explicit resolved-input
exceptions that cannot be derived from the graph.

## Scope

### In Scope

- Make the run graph snapshot the structural source of truth for ordinary
  connected node inputs.
- Split retained payload identity from durable artifact fact identity.
- Keep diagnostics ledger and ArtifactStore as the backend source of truth for
  persisted run inspection data.
- Store each retained body once per workflow run and logical payload lineage.
- Record canonical node output artifacts and workflow input/output boundary
  artifacts.
- Record explicit resolved-input facts only when the runtime input cannot be
  derived from a run graph edge and an upstream output or boundary value.
- Treat `_data` and similar full node-state snapshots as metadata-only
  diagnostics by default, not user-facing retained payload bodies.
- Update run-inspection and I/O Inspector contracts so node inputs are assembled
  by backend read models from graph edges, output artifacts, boundary artifacts,
  and resolved-input exceptions.
- Add tests that reproduce the current duplicate text-output case and cover
  large descriptor-backed media payloads.
- Update affected module documentation when ArtifactStore, diagnostics ledger,
  workflow-service, embedded-runtime, or frontend contracts change.

### Out of Scope

- Cross-run global content-addressed storage.
- Backward compatibility with earlier Pantograph run-inspection artifact
  shapes. Local development data may be rebuilt or discarded when the persisted
  contract changes.
- A second node-I/O persistence system outside diagnostics ledger projections
  and ArtifactStore.
- Durable duplicate input artifact rows for ordinary graph-connected inputs.
- Scheduler policy, inference backend routing, model-library behavior, or
  Pumas Library changes.
- Changing frontend layout beyond what is required to display node I/O from the
  new backend resolved-node-I/O contract.

## Inputs

### Problem

Current retained I/O writes are role-centric. The same text can be materialized
as separate ArtifactStore bodies for `node_input`, `node_output`, and
`workflow_output`, and a `_data` JSON node-state snapshot can embed the same text
again. In run `run_d5ae8c00-0a8f-43c7-ae37-b853f4e137dd`, the text output node
showed three identical retained text artifacts plus a JSON artifact containing
the same text. This is confusing for inspection and becomes unacceptable for
large images, audio, video, or other binary payloads.

The system still needs reliable provenance, but normal graph edges already
explain how an output reaches a downstream input. It is valid to record that a
payload was produced by an inference node and surfaced as a terminal workflow
output. It is unnecessary to also persist a duplicate input artifact for a
text-output node when the run graph can derive that input from the upstream
output edge.

### Constraints

- Backend service/domain crates remain the source of truth for retained
  artifacts, resolved I/O read models, and artifact read handles.
- Tauri and frontend code may transport and display data, but must not invent
  durable payload identity or repair duplicate backend records locally.
- Run graph snapshots are immutable for completed runs and must be available to
  derive ordinary connected inputs.
- A workflow run executes once. Artifact body reuse is run-scoped and lineage
  scoped unless a future plan intentionally introduces cross-run storage.
- Identical bytes from unrelated nodes are distinct produced-output facts. Do
  not merge them by content hash alone.
- The first implementation milestone must be a validated vertical slice that
  proves the intended contract with real producer-to-consumer data flow.
- Artifact read APIs must remain descriptor-first and lazy-body by default so
  graph and I/O pages do not eagerly load large payloads.
- File-size limits are review triggers, not mechanical cutoffs. Split touched
  files where a stable responsibility boundary is created by this work.

### Assumptions

- The existing diagnostics ledger can carry canonical output, workflow boundary,
  and resolved-input exception facts without replacing the ledger.
- The existing ArtifactStore can either reuse an existing descriptor for the
  same logical payload lineage or add a small alias/body-reference layer without
  a broad storage rewrite.
- `_data` snapshots are diagnostic state and should not be shown as normal user
  input/output artifacts unless a future raw-debug mode explicitly asks for
  them.
- Large media values should normally already be ArtifactStore descriptors; the
  fix should preserve those descriptors instead of copying bodies.
- Most node inputs in saved workflows are edge-derived. Explicit resolved-input
  facts are exceptional and should carry a reason such as `literal_default`,
  `workflow_input`, `coerced`, `cached_replay`, `dynamic_route`,
  `redacted_secret`, or `runtime_injected`.

### Dependencies

- `crates/pantograph-workflow-service/src/workflow/session_io_artifacts.rs`
- `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`
- `crates/pantograph-workflow-service/src/workflow/artifact_store.rs` and
  `crates/pantograph-workflow-service/src/workflow/artifact_store/`
- `crates/pantograph-embedded-runtime/src/node_io_artifacts.rs`
- `crates/pantograph-diagnostics-ledger/src/event.rs`,
  `schema.rs`, and `sqlite/event_sqlite.rs`
- `src/components/workbench/IoInspectorPage.svelte` and related presenters
- Existing workflow run inspection, artifact read, and retention tests
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`

### Affected Structured Contracts

- `IoArtifactObservedPayload` or its replacement canonical artifact fact DTO
- `IoArtifactProjectionRecord`
- A resolved-node-I/O read DTO that can distinguish `derived_from_edge` from
  explicit resolved-input facts.
- Canonical identifier semantics:
  - `artifact_fact_id`: durable diagnostics/projection fact identity.
  - `payload_artifact_id`: ArtifactStore body/read identity.
  - `logical_payload_lineage_id`: optional backend-only or DTO-visible lineage
    key used to prove aliasing between produced outputs, derived inputs, and
    workflow boundary outputs.
- Workflow-service diagnostics and run-inspection DTOs
- ArtifactStore descriptor/read-handle semantics if an alias/body-reference
  field is added
- I/O Inspector service types and presenters

### Affected Persisted Artifacts

- `io_artifact_projection` schema and indexes
- Diagnostics ledger event payload JSON for future runs, with fewer durable
  input rows for ordinary connected inputs
- ArtifactStore manifest/body records for retained workflow run artifacts
- Local development run-inspection data from earlier implementations, with no
  legacy compatibility requirement

### Concurrency And Race-Risk Review

- Concurrent node completions may attempt to retain the same lineage-scoped
  body. The ArtifactStore path must write through one backend-owned lock or an
  atomic temp-file/rename path and must verify lineage plus content equality
  before reusing a body.
- Projection draining may observe events incrementally. Each canonical artifact
  or resolved-input fact must be independently valid even when body retention
  happened earlier in the same run.
- Retention cleanup must not delete a body while retained canonical output,
  boundary, or resolved-input facts still reference it. Cleanup should operate
  from backend-owned reference facts or delete all facts and bodies for an
  expired run together.
- Stream finalization must map the final retained body to the same payload
  identity used by completed node output inspection.

### Ownership And Lifecycle

- Workflow-service owns artifact materialization, retained body identity, and
  run-inspection query assembly.
- Diagnostics-ledger owns durable artifact fact events, projection schema,
  projection draining, and projection queries.
- ArtifactStore owns physical body files, manifests, read handles, stream
  finalization, memory cache entries, and retention cleanup.
- Embedded-runtime may emit node output and explicit resolved-input exception
  facts, but it must use workflow-service/ArtifactStore contracts for retained
  payload identity.
- Tauri owns IPC transport only. It must not define persistent artifact
  identity, retention semantics, or durable grouping behavior.
- Frontend owns selected-node state, grouping presentation, labels, and layout.
  It must consume backend resolved-node-I/O records instead of deriving durable
  identity from hashes, role labels, or graph edges locally.
- Cleanup is backend-owned. Retention cleanup must be cancellable/retry-safe and
  must not overlap with active body writes outside the existing ArtifactStore
  locking and manifest update boundaries.

### Public Facade And Compatibility

- This is an API-breaking persisted-contract change for local run-inspection
  artifacts. Pantograph does not need backward compatibility with earlier local
  artifact projections for this work.
- Preserve public service facades where doing so keeps call sites simple, but do
  not preserve `artifact_id` semantics if clearer canonical artifact and payload
  id fields reduce ambiguity.
- The implementation must not keep `artifact_id` as both fact identity and body
  identity. Pick an explicit meaning for legacy field names within the same
  breaking contract slice and route all reads through `payload_artifact_id`.
- Any breaking DTO changes must be made in one verified slice across Rust
  contracts, Tauri bindings/service types, and frontend consumers.
- Old local development databases or artifact stores may require reset/rebuild.
  Do not add compatibility shims unless a re-plan trigger changes that
  requirement.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Provenance is lost while removing duplicates | High | Derive ordinary input provenance from immutable run graph edges and record explicit resolved-input facts only for non-derivable cases. |
| ArtifactStore changes become a broad rewrite | High | Start with a run-scoped vertical slice and choose the smallest body-reference model that satisfies tests. |
| Large media bodies are copied during aliasing | High | Preserve existing `ArtifactDescriptor` values and add tests that assert descriptor reuse without body rewrite. |
| `_data` hides useful debug information | Medium | Keep metadata, hash, size, and lifecycle state; expose raw state only through a future explicit debug mode. |
| Existing local databases contain stale schema/data | Medium | No legacy compatibility; document reset/rebuild expectation for development data. |
| Body hashes collide or mismatched aliases are reused | Low | Reuse bodies only when lineage matches; hash with existing `blake3` and verify byte length/hash before reuse; on mismatch write a distinct body. |
| Frontend cannot explain a node input after duplicate input rows are removed | High | Workflow-service returns a resolved-node-I/O read model that includes derivation source, upstream node/port, boundary source, or explicit exception reason. |
| Dynamic or transformed inputs are incorrectly derived from static edges | High | Require resolved-input exception facts for defaults, workflow inputs, coercions, cache replay, dynamic routing, redaction, fan-in aggregation, and runtime-injected values. |
| Unrelated nodes produce identical bytes and are incorrectly merged | Medium | Use logical payload lineage, not content hash alone, to decide whether a fact aliases an existing body. |

## Definition of Done

- A workflow that sends one generated text value through an output node records
  one retained text body and one canonical produced-output fact for that run.
- Two unrelated nodes that independently produce identical bytes are still
  represented as separate produced-output facts and are not treated as the same
  logical payload lineage.
- The downstream text-output node input is visible in I/O Inspector as
  `derived_from_edge`, not as a duplicate retained input artifact.
- Workflow input/output boundary values are retained as boundary facts when
  values enter or leave the graph.
- Explicit resolved-input facts exist only for non-derivable runtime inputs and
  include a machine-readable reason.
- `_data`/full node-state snapshots are not retained or displayed as normal
  user artifacts by default.
- Descriptor-backed media values are referenced without duplicate body writes.
- I/O Inspector shows selected node inputs and outputs from a backend
  resolved-node-I/O read model, including whether each input is derived or
  explicitly recorded.
- Artifact reads use the canonical retained body id/read handle and return the
  expected body from canonical output, boundary, or resolved-input facts that
  reference it.
- `artifact_fact_id` and `payload_artifact_id` have distinct meanings in Rust
  DTOs, TypeScript types, projection rows, and read requests.
- Retention cleanup leaves no orphan retained bodies and does not break
  retained facts that still reference retained bodies.
- Relevant Rust, TypeScript, and documentation verification passes.

## Milestones

### Milestone 1: Contract Slice And Failing Tests

**Goal:** Freeze the smallest backend contract that proves ordinary connected
inputs are derived from graph edges while non-derivable inputs are explicit
facts.

**Tasks:**
- [ ] Add a vertical-slice regression test that reproduces the text-output
  duplicate case from a workflow execution path, not only a unit helper.
- [ ] Assert that the inference node `response` output is a canonical produced
  output fact with a retained payload body.
- [ ] Assert that the text-output node input is reconstructed as
  `derived_from_edge` from the immutable run graph and upstream output fact,
  with no duplicate retained input artifact row.
- [ ] Assert that workflow terminal output is represented as a boundary fact or
  alias to the produced output body without a duplicate body.
- [ ] Assert that two unrelated nodes producing identical text bytes remain
  separate produced-output facts and do not collapse into one logical payload
  lineage.
- [ ] Assert that workflow input bindings and unconnected/default inputs remain
  inspectable as explicit resolved-input or boundary facts.
- [ ] Assert that multi-input/fan-in cases either produce an explicit
  resolved-input fact with a reason or return a clear unsupported derivation
  signal in the read model.
- [ ] Assert that `_data` snapshots are metadata-only and excluded from normal
  user-facing artifact groups.
- [ ] Name and document the new contract fields before implementation. Preferred
  terminology is `artifact_fact_id` for canonical produced/boundary facts,
  `payload_artifact_id` for retained body/read targets, and
  `logical_payload_lineage_id` for backend-proven aliases. Use
  `input_resolution` for derived or explicit node input records, unless code
  review finds a clearer existing convention.

**Verification:**
- `cargo test -p pantograph-workflow-service workflow_execution_session_records_retained_node_io_artifact_bodies`
- `cargo test -p pantograph-diagnostics-ledger io_artifact`
- The new tests should fail for the expected duplicate-body and duplicate-input
  row reasons before the implementation changes land.

**Status:** In progress. Workflow-service and embedded-runtime now create
family-scoped payload ids (`input` or `output`) separately from exact artifact
fact ids, so paired workflow/node boundary facts for the same node port can
share one retained body. Broader ArtifactStore alias/reference support and
large descriptor-backed media checks still remain.

### Milestone 2: Payload Identity And Read Target Contract

**Goal:** Split durable artifact fact identity from retained body/read identity
before changing storage behavior.

**Tasks:**
- [ ] Add or rename DTO fields so canonical artifact facts carry
  `artifact_fact_id`, `payload_artifact_id`, and source/lineage metadata.
- [ ] Route `artifact_descriptor`, `read_artifact_body`, stream reads, download,
  and consume acknowledgement through `payload_artifact_id`.
- [ ] Keep `artifact_id` only as a deliberate compatibility alias or remove it
  from new DTOs in the same breaking contract slice.
- [ ] Preserve existing descriptor-backed media values without copying their
  bodies.
- [ ] Keep body storage backend-owned; do not expose body-path decisions through
  Tauri or frontend types.
- [ ] Update contract snapshot tests for Rust, Tauri command payloads, UniFFI if
  affected, and TypeScript service types.

**Verification:**
- `cargo test -p pantograph-workflow-service --test contract workflow_io_artifact_query_contract_snapshot`
- `npm run typecheck`
- `cargo test -p pantograph-uniffi workflow_artifact`

**Status:** In progress. Diagnostics-ledger projection now carries separate
`artifact_fact_id`, `payload_artifact_id`, and `logical_payload_lineage_id`
fields and can retain multiple projected facts for one payload id. Workflow
service and embedded-runtime artifact emissions now populate those fields.
I/O Inspector read, preview, stream, download, and consume actions now prefer
`payload_artifact_id` when present. ArtifactStore read APIs and Tauri command
payload field names still need the full payload-id cutover.

### Milestone 3: Lineage-Scoped ArtifactStore Reuse

**Goal:** Let backend artifact materialization reuse one retained body for the
same logical payload lineage while preserving descriptor metadata.

**Tasks:**
- [ ] Add the minimal ArtifactStore or workflow-service API needed to retain or
  resolve a body by lineage-scoped payload identity.
- [ ] Do not dedupe unrelated artifacts by content hash alone.
- [ ] Preserve existing descriptor-backed media values without copying their
  bodies.
- [ ] Ensure same-body writes are idempotent and safe under concurrent execution
  attempts.
- [ ] Update ArtifactStore README or an ADR if the descriptor/body relationship
  changes.

**Verification:**
- `cargo test -p pantograph-workflow-service artifact_store`
- Add focused tests proving same lineage reuses a body, unrelated identical
  content does not alias, and different content does not alias.
- Add a descriptor-backed media test proving no duplicate body is written.

**Status:** Not started

### Milestone 4: Artifact Fact Materialization Cutover

**Goal:** Change workflow-service and embedded-runtime artifact emission so the
ledger records canonical produced/boundary facts plus explicit resolved-input
exceptions, not duplicate ordinary input rows.

**Tasks:**
- [ ] Refactor `session_io_artifacts.rs` and `node_io_artifacts.rs` to share the
  same retained-body materialization rule instead of duplicating role-specific
  body writes.
- [ ] Stop using role labels as part of retained body identity. Keep producer,
  boundary, and explicit input-resolution metadata in artifact facts.
- [ ] Remove workflow-service's current paired emission of workflow/node roles
  for each binding and replace it with canonical boundary facts plus aliases to
  existing produced-output payloads when lineage proves they match.
- [ ] Stop emitting durable `node_input` artifact facts for ordinary connected
  inputs that can be derived from run graph edges and upstream output facts.
- [ ] Emit explicit resolved-input facts only for unconnected literals/defaults,
  workflow/external inputs, adapter coercion/normalization, cache replay,
  dynamic route selection, fan-in aggregation, redacted secrets, or
  runtime-injected values.
- [ ] Make terminal workflow outputs reference the completed node output body
  when they represent the same logical payload.
- [ ] Treat `_data` and full node-state snapshots as metadata-only diagnostics
  unless they are explicit user output ports.
- [ ] Keep stream finalization aligned with the final response body identity.

**Verification:**
- `cargo test -p pantograph-workflow-service workflow_execution_session_records_retained_node_io_artifact_bodies`
- `cargo test -p pantograph-embedded-runtime node_io_artifacts`
- `cargo test -p pantograph-workflow-service workflow_io_artifact_query`

**Status:** Not started

### Milestone 5: Diagnostics Projection And Resolved IO Query Shape

**Goal:** Project canonical artifact facts and expose a backend read model that
resolves node inputs from graph edges or explicit input facts.

**Tasks:**
- [ ] Replace or narrow `IoArtifactObservedPayload` semantics so the ledger
  stores canonical produced-output, workflow-boundary, and explicit
  resolved-input facts.
- [ ] Extend `IoArtifactProjectionRecord` or add a companion read DTO with
  explicit `artifact_fact_id`, `payload_artifact_id`, source kind, node, port,
  producer, boundary, and input-resolution fields.
- [ ] Update the projection primary key/upsert behavior so multiple facts can
  reference one `payload_artifact_id` without overwriting each other.
- [ ] Update SQLite schema, projection draining, query mapping, and retention
  summary behavior for the new identity model.
- [ ] Decide whether `artifact_id` remains the canonical artifact fact id or
  becomes a compatibility alias for `payload_artifact_id`; because legacy
  compatibility is not required, use the clearer model and update all consumers
  in the same slice.
- [ ] Add a workflow-service resolved-node-I/O query that joins run graph edges,
  canonical artifact facts, workflow boundary facts, and explicit
  resolved-input exceptions.
- [ ] Make `workflow_run_inspection_query` return resolved node I/O as the
  primary I/O Inspector contract. Keep raw artifact projection queries only for
  diagnostics/browsing use cases where raw facts are needed.
- [ ] Ensure artifact read requests use the retained body identity/read handle,
  not a graph-derived or fact-only id.
- [ ] Update diagnostics-ledger and workflow-service READMEs for the new
  projection/read-model invariant.

**Verification:**
- `cargo test -p pantograph-diagnostics-ledger io_artifact`
- `cargo test -p pantograph-workflow-service diagnostics`
- Query a completed run and confirm an ordinary connected input appears as a
  derived read-model row with upstream node/port and one readable payload body.
- Query a completed run with two unrelated identical outputs and confirm they
  remain separate produced-output facts.

**Status:** Not started

### Milestone 6: I/O Inspector Resolved IO View

**Goal:** Make the UI display logical node I/O from the backend resolved-node-I/O
read model instead of from redundant input artifact rows.

**Tasks:**
- [ ] Update frontend service types to include artifact fact id, payload id,
  source kind, and input-resolution metadata.
- [ ] Stop double-fetching raw artifact projection data for normal I/O
  inspection; use `workflow_run_inspection_query` as the primary source for
  graph, node status, and resolved node I/O.
- [ ] Render selected-node outputs from canonical produced-output facts.
- [ ] Render selected-node inputs from backend resolved-input rows, showing
  `derived_from_edge` or the explicit exception reason.
- [ ] Show upstream node/port, workflow boundary, cache, coercion, redaction, or
  dynamic-route provenance as metadata from the backend read model.
- [ ] Hide metadata-only `_data` diagnostics from normal artifact lists by
  default.
- [ ] Route preview/download/read actions to the canonical payload body id.
- [ ] Keep layout and selection state frontend-owned; do not move presentation
  decisions into backend DTOs.

**Verification:**
- `npm run typecheck`
- `npm run test:frontend`
- Manual smoke: run a text-output workflow, open I/O Inspector, select the text
  output node, and verify one readable input derived from the inference node
  output plus one canonical output/boundary view without duplicate artifacts.

**Status:** Not started

### Milestone 7: Retention Cleanup, Storage Audit, And Performance

**Goal:** Prove lineage-scoped body reuse is durable, cleanup-safe, and suitable
for large payloads.

**Tasks:**
- [ ] Update retention cleanup so retained bodies are deleted only when no live
  retained canonical artifact facts or explicit resolved-input facts require
  them, or when an expired run is deleted as a unit.
- [ ] Add storage/stat assertions that derived inputs and workflow boundary
  aliases do not multiply retained body bytes.
- [ ] Verify binary/large media flows remain descriptor-first and lazy-read.
- [ ] Record any remaining file-size or responsibility-boundary cleanup items
  in this plan before implementation continues.

**Verification:**
- `cargo test -p pantograph-workflow-service artifact_retention`
- `cargo test -p pantograph-workflow-service artifact_store`
- Manual storage audit on a generated text workflow and one descriptor-backed
  media workflow.

**Status:** Not started

### Milestone 8: Documentation And Release Validation

**Goal:** Close the contract change with documentation and release-grade checks.

**Tasks:**
- [ ] Update source module READMEs or add an ADR for the new canonical artifact
  fact, derived input, and explicit resolved-input exception model.
- [ ] Update this plan with implementation notes, deviations, and any unresolved
  follow-ups discovered during execution.
- [ ] Build release binaries and frontend after all slices pass.

**Verification:**
- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `npm run build`
- `bash launcher.sh --build-release`

**Status:** Not started

## Execution Notes

- 2026-05-09: Plan created after confirming duplicate retained text artifacts
  come from role-specific materialization and `_data` state snapshot retention.
- 2026-05-09: Plan updated so ordinary connected node inputs are derived from
  the immutable run graph plus upstream output facts; durable input facts are
  limited to non-derivable runtime input exceptions.
- 2026-05-09: Plan updated after codebase impact review to require separate
  `artifact_fact_id` and `payload_artifact_id` semantics, lineage-scoped body
  reuse instead of content-hash-only dedupe, and workflow-service-owned
  resolved-node-I/O as the I/O Inspector contract.
- 2026-05-09: First implementation slice added diagnostics-ledger support for
  separate artifact fact, payload artifact, and logical lineage identifiers.
  Projection upsert identity now uses `artifact_fact_id`, allowing multiple
  durable facts to reference one retained `payload_artifact_id` without row
  overwrite.
- 2026-05-09: Producer identifier slice updated workflow-service and
  embedded-runtime I/O artifact emission so new artifact events populate
  `artifact_fact_id`, `payload_artifact_id`, and
  `logical_payload_lineage_id`. Existing `artifact_id` remains the read-target
  compatibility field until the ArtifactStore/API cutover lands.
- 2026-05-09: Frontend read-target slice updated I/O Inspector actions to route
  descriptor, preview, stream, download, and consume requests through
  `payload_artifact_id` when present, falling back to `artifact_id` for older
  records.
- 2026-05-09: Lineage-scoped payload id slice changed workflow-service and
  embedded-runtime small-value materialization to use family-scoped retained
  payload ids (`input` vs `output`) while keeping fact ids role-specific. This
  lets `node_output` and `workflow_output` facts for the same node/port share
  a body without risking input/output port name collisions.

## Commit Cadence Notes

- Commit the plan as its own documentation slice if requested.
- During implementation, commit after each milestone or smaller verified
  vertical slice when tests pass.
- Keep schema/contract changes, backend implementation, frontend consumption,
  and documentation together only when they are part of one verified slice.
- Follow `COMMIT-STANDARDS.md` for detailed commit messages and atomic history.

## Optional Subagent Assignment

Use subagents only if implementation is explicitly parallelized from a clean
integration commit.

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| Backend contract owner | Diagnostics artifact fact/projection fields, payload read-target fields, and workflow-service resolved-node-I/O DTOs | Contract patch, tests, and README/ADR notes | Contract tests pass before other workers consume fields |
| ArtifactStore owner | Lineage-scoped retained body reuse and retention cleanup | Backend storage patch and focused storage tests | Store tests pass and public facade is documented |
| Runtime emission owner | Embedded-runtime and workflow-service canonical artifact fact emission | Node/workflow artifact emission patch and vertical-slice tests | Duplicate text-output workflow test passes |
| Frontend owner | I/O Inspector resolved-node-I/O rendering and read-target updates | Type/presenter/component patch and frontend tests | Frontend consumes frozen backend DTOs without local durable inference |

Shared contracts, schema files, generated bindings, lockfiles, and public DTO
types must be edited serially or by one explicit owner.

## Re-Plan Triggers

- The ArtifactStore cannot support lineage-scoped body reuse without a larger
  manifest redesign.
- Workflow-service cannot derive or assign reliable logical payload lineage
  from current executed graph, boundary, and output facts.
- Diagnostics projection cannot represent canonical artifact facts plus
  resolved-node-I/O derivation without breaking necessary run-inspection
  queries.
- `artifact_fact_id` and `payload_artifact_id` cannot be separated without
  breaking artifact read APIs in a larger way than expected.
- Streamed outputs use a separate finalization path that cannot share completed
  node output payload identity.
- `_data` is found to be the only source for a real user-visible output value.
- Run graph snapshots do not preserve enough executed edge information to
  derive ordinary connected inputs reliably.
- Retention cleanup requires cross-run reference counting.
- Manual testing shows the UI still presents duplicate payloads or cannot show
  provenance clearly from the backend resolved-node-I/O read model.

## Recommendations

- Prefer run-scoped, lineage-scoped body reuse first. Cross-run deduplication
  adds privacy, retention, and cleanup complexity without being needed to fix
  workflow run inspection.
- Avoid content-addressed global dedupe for this work. Content hash equality
  alone is not enough to prove two graph values are the same logical payload.
- Prefer a backend `resolved_node_io` read model over frontend graph joins.
  Frontend graph joining would duplicate backend semantics and miss execution
  edge cases.
- Prefer explicit `payload_artifact_id` and `artifact_fact_id` over frontend
  hash grouping. Hash-only grouping would hide the body ownership problem and
  make read behavior depend on UI inference.
- Keep `_data` metadata-only. If raw execution state inspection becomes useful,
  add an explicit debug artifact role later with separate retention controls.

## Completion Summary

### Completed

- Diagnostics-ledger identifier split slice:
  `IoArtifactObservedPayload` and `IoArtifactProjectionRecord` now expose
  `artifact_fact_id`, `payload_artifact_id`, and
  `logical_payload_lineage_id`; the SQLite projection stores those fields and
  upserts by artifact fact identity.
- Producer identifier slice: workflow-service session I/O and embedded-runtime
  node I/O artifact events populate artifact fact, payload artifact, and
  logical lineage identifiers while preserving the existing `artifact_id` read
  target for current callers.
- Frontend read-target slice: I/O Inspector now uses `payload_artifact_id` as
  the retained body target when artifact records expose it.
- Lineage-scoped payload id slice: paired workflow/node boundary artifact facts
  now share retained payload ids by input/output family while preserving
  distinct fact ids.

### Deviations

- None.

### Follow-Ups

- None recorded.

### Verification Summary

- 2026-05-09:
  - `cargo test -p pantograph-diagnostics-ledger io_artifact`
  - `cargo test -p pantograph-workflow-service diagnostics`
  - `cargo test -p pantograph-workflow-service --test contract workflow_io_artifact_query_contract_snapshot`
  - `cargo check -p pantograph-workflow-service -p pantograph-embedded-runtime`
  - `npm run typecheck`
  - `git diff --check`
  - `npm run -w frontend check:types` was attempted from the original plan
    text and failed because this repository has no `frontend` workspace; the
    plan now uses the root `npm run typecheck` and `npm run test:frontend`
    scripts.
- 2026-05-09:
  - `cargo test -p pantograph-workflow-service workflow_execution_session_records_retained_node_io_artifact_bodies`
  - `cargo test -p pantograph-embedded-runtime node_execution_workflow_sink_records_task_completed_outputs_as_retained_node_artifacts`
  - `cargo check -p pantograph-workflow-service -p pantograph-embedded-runtime`
- 2026-05-09:
  - `npm run typecheck`
  - `npm run test:frontend`
- 2026-05-09:
  - `cargo test -p pantograph-workflow-service workflow_execution_session_records_retained_node_io_artifact_bodies`
  - `cargo test -p pantograph-embedded-runtime node_execution_workflow_sink_records_task_completed_outputs_as_retained_node_artifacts`
  - `cargo check -p pantograph-workflow-service -p pantograph-embedded-runtime`

### Traceability Links

- Module README updated: N/A for planning slice.
- ADR added/updated: N/A for planning slice.
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A.
