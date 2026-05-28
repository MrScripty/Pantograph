# crates/pantograph-workflow-service/src/graph

## Purpose
This directory contains the host-agnostic workflow graph-editing API for
Pantograph. It owns graph document contracts, edit-session lifecycle,
revision-aware mutation semantics, node-definition discovery, connection intent,
and persistence abstractions so adapters do not implement graph business logic.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | Public exports for graph-edit contracts and helper modules. |
| `types.rs` | Graph DTOs, edit-session request/response types, and persisted workflow file shapes. |
| `registry.rs` | Built-in node-definition discovery and canonical node-contract projection. |
| `canonicalization.rs` | Current saved graph canonicalization orchestration and current-schema response assembly. |
| `canonicalization_inference.rs` | Dynamic inference-setting schema expansion, per-node definition overlay rebuilds, and passthrough port helpers. |
| `canonicalization_tests.rs` | Current canonicalization, no-legacy, and inference-overlay regression tests. |
| `effective_definition.rs` | Resolves backend-owned effective node contracts and projects them into graph DTOs before validation or candidate lookup. |
| `effective_definition_tests.rs` | Effective-definition tests for dynamic overlays, inference authored snapshots, and no-fallback inference definition rejection. |
| `executable_topology.rs` | Canonical executable-topology projection and BLAKE3 workflow execution fingerprint calculation for workflow versioning. |
| `executable_validation_snapshot_source.rs` | Current graph-session executable snapshot source read model that joins current validation projections with current dependency proof freshness before durable workflow snapshot publication. |
| `presentation_revision.rs` | Canonical display-metadata projection and BLAKE3 presentation fingerprint calculation for historic graph presentation revisions. |
| `run_settings.rs` | Canonical node settings projection used by immutable workflow-run audit snapshots. |
| `contract_validation.rs` | Whole-graph contract validation and structured stale graph diagnostic classification. |
| `contract_validation_tests.rs` | Structured graph contract validation and stale diagnostic classification tests. |
| `validation.rs` | Shared connection compatibility helpers used by graph-edit flows. |
| `connection_intent.rs` | Canonical candidate-discovery and revision-aware connection/insert validation. |
| `connection_insert.rs` | Internal node-insert, edge-insert preview, and edge-bridge helpers used by `connection_intent.rs` while preserving the public graph-edit facade. |
| `diagnostics.rs` | Structured stale graph diagnostic DTOs and bounded diagnostic payload helpers. |
| `inspection.rs` | Shared graph inspection projection for saved graphs and future run graph wrappers. |
| `inference_interface_facts.rs` | Workflow-service fact-provider boundary that supplies path-free Pumas readiness, inference capability, and runtime availability facts to validation publishers without frontend/Tauri ownership. |
| `inference_interface_patch.rs` | Workflow-service-owned update proposal and typed graph patch-operation contracts for applying current inference descriptors to authored node snapshots. |
| `inference_interface_projection.rs` | Workflow-service-owned projection from resolved inference descriptors into minimal authored snapshots and backend validation summaries. |
| `inference_interface_publication.rs` | Workflow-service-owned synchronous validation publisher that extracts strict graph requests, resolves descriptor projections from supplied facts, emits scoped validation events, and returns bounded node projection records. |
| `inference_interface_request.rs` | Draft-graph extraction of path-free inference-interface resolver requests from connected `puma-lib` model references and explicit inference-node constraints. |
| `inference_interface_resolver.rs` | Synchronous facts-in descriptor resolver boundary that combines path-free Pumas model state, inference capability facts, runtime availability, and graph-authored constraints into typed inference descriptors. |
| `inference_interface_validation.rs` | Workflow-service live inference-validation session and scoped event envelope contracts, including descriptor, drift, diagnostic, update-proposal, and summary events. |
| `inference_validation_lifecycle.rs` | Workflow-service validation lifecycle owner for active validation-session identity, supersession, session-close rejection, bounded lifecycle event retention, and publication freshness checks. |
| `inference_validation_publisher.rs` | Workflow-service async validation publication attempt owner that coordinates graph snapshots, fact-provider calls, lifecycle freshness checks, descriptor publication, and current-state recording for refresh and explicit publication entrypoints. |
| `dependency_environment_subject.rs` | Workflow-service-owned dependency-environment action subject resolver for typed sidecar associations between dependency-environment control nodes and inference nodes. |
| `inference_validation_state.rs` | Workflow-service current inference-validation state owner for graph-revision freshness checks, dependency-environment action diagnostics, submit-gate summaries, and proof-aware scheduler/executable snapshot projections. |
| `group_mutation.rs` | Backend-owned create/ungroup/update-port graph mutations for collapsed node groups. |
| `session_contract.rs` | Additive graph snapshot contracts and response-assembly helpers, including the Phase 6 workflow-session state view and explicit backend-state projection seam surfaced to transport layers. |
| `session_graph.rs` | Graph utility helpers for embedding metadata sync, graph conversion into `node-engine`, and shared node-data merge behavior. |
| `session_runtime.rs` | Focused runtime/lifecycle state for one graph edit session, including active execution metadata, queue projection, and run counters. |
| `session_types.rs` | Edit-session request/response DTOs and local undo/redo/session-kind types that are shared by the graph session boundary. |
| `session.rs` | Edit-session store, undo/redo state, and graph mutation orchestration. |
| `session_connection_api.rs` | Edit-session connection candidate, direct connect, node insert-connect, and edge-insert API methods. |
| `session_inference_validation_api.rs` | Edit-session inference-validation publication API that records only current graph-revision validation summaries. |
| `session_tests.rs` | Graph edit-session mutation, undo/redo, insertion, connection, stale cleanup, event projection, and memory-impact tests extracted from the production session module. |
| `persistence.rs` | Graph-store trait plus the filesystem-backed `.pantograph/workflows` implementation. |

## Problem
Pantograph previously kept graph-editing logic inside Tauri modules, which made
headless clients second-class consumers and allowed transport layers to become
business-logic owners.

## Constraints
- Graph-edit contracts must remain transport-agnostic.
- Persisted workflow files use the validated `WorkflowIdentity` grammar for
  file stems. Existing workflow files with incompatible names are rejected or
  skipped during the no-legacy Stage 01 cutover.
- Mutation rejection must be structured for expected incompatibility cases.
- Edit-session state must serialize mutations per session without global blocking.
- Active execution metadata, queue projection, and run counters for graph edit
  sessions must stay backend-owned and must not be recomputed in adapters.
- Saved graphs may persist additive `node.data.definition` port overlays for
  model-derived settings, but those overlays must never replace registry-owned
  static contracts wholesale.

## Decision
Define a dedicated graph-editing module inside `pantograph-workflow-service`
that owns graph contracts, edit-session orchestration, and persistence
abstractions. Host adapters may expose those operations over IPC/FFI/HTTP, but
the logic and contracts live here. Node definitions are projected from
backend-owned `pantograph-node-contracts` records. Dynamic per-node port
overlays are resolved through `effective_definition.rs` as
`EffectiveNodeContract` values, then projected back to workflow-service DTOs
for existing graph-edit callers.
Dependency-environment sidecar edges use the exact-only
`dependency_environment_sidecar` port data type. Graph editing can display and
persist that typed association, but workflow-service subject resolution is the
only owner that may interpret it for dependency actions.
Dependency-environment action handling derives and validates the canonical
`DependencyEnvironmentRequest` from current backend validation state before it
reports `RequestReady`. The derived request is not supplied by frontend or
Tauri callers; `Check` and `Install` also require the current sidecar choices
to match the stored dependency requirements proof. Workflow-service then passes
the validated request to the canonical dependency-environment service outside
the graph/session lock; the graph action response remains an intent response
while full dependency-environment results stay backend-owned.
Graph-session executable validation snapshot publication consumes the current
validation-state read model rather than caller-supplied publications. That
read model must include current path-free dependency requirements proof
freshness for each executable inference node before workflow-service may
persist a runtime-admissible executable snapshot.
Graph-session current validation summary reads are the backend-owned submit
gate for the editor and queue-admission boundary. They report whether the
latest graph revision has a current executable validation session, preserve
typed diagnostics for unavailable/stale/invalid states, and never ask Tauri or
frontend code to resolve descriptors, Pumas state, dependency proofs, or
scheduler runtime policy.
Graph-session current validation refreshes are also backend-owned. A refresh
request carries graph-session identity and the graph revision observed by the
caller; workflow-service snapshots the current graph, rejects stale requested
revisions as typed summary responses, generates the validation-session id,
runs the existing descriptor publication core outside the graph lock, records
current validation state only after re-checking that the graph revision is
still current, and returns the same summary/gate DTO used by submit plus
bounded per-node projection records from that validation pass. The
projection records are transport data for editor port rendering and review;
the summary/gate remains the only submit authority.

## Alternatives Rejected
- Keep graph editing in Tauri and expose only execution in core.
  Rejected because it keeps headless clients incomplete and breaks backend-owned state rules.
- Put graph-edit types directly into `workflow.rs`.
  Rejected because graph editing is a distinct contract surface with its own lifecycle and persistence concerns.

## Invariants
- Edit sessions are distinct from scheduler-managed workflow run sessions.
- Graph mutations return backend-owned graph snapshots or structured rejections.
- Graph edit-session mutation responses may also carry an additive canonical
  backend-owned `workflow_event` so bindings and adapters can forward
  `GraphModified` semantics without synthesizing them locally.
- When backend graph-diff compatibility analysis is available, that additive
  `GraphModified` event should also carry `memory_impact` so transports can
  forward preserved vs invalidated node-memory facts without reconstructing
  backend policy.
- Graph edit-session snapshot responses may also carry an additive backend-
  owned `workflow_session_state` view so transports can forward Phase 6 node-
  memory, checkpoint, and mutation-impact contracts without owning them.
- Edit-session graph mutation responses currently use the backend-owned session
  id for both `workflow_id` and `execution_id` inside that additive event
  contract because the session-scoped graph DTO does not yet carry a separate
  persisted workflow identity.
- Graph edit-session mutation responses should project Phase 6 memory impact
  from backend-owned graph-diff compatibility analysis when that richer
  context is available; generic event-only fallbacks remain a compatibility
  backstop rather than the primary source of truth.
- Node group create, ungroup, and port-mapping edits are session mutations that
  return whole-graph mutation responses; UI stores must not reconstruct group
  boundary edges locally.
- KV-capable inference nodes should emit explicit backend-owned memory-impact
  reasons for model changes, runtime/backend changes, tokenizer-or-config
  changes, upstream prefix changes, and prefix-breaking topology edits so
  transports and later rerun policy do not infer invalidation heuristics
  locally.
- Graph edit-session snapshot reads should retain the most recent backend-owned
  memory-impact decision for inspection until a later non-invalidating edit
  explicitly clears that persisted compatibility state.
- Successful direct connection and insertion mutation responses should forward
  the same additive backend-owned `workflow_event` and
  `workflow_session_state` projection as graph snapshot mutations so transport
  clients do not need a second read to observe mutation impact facts.
- Direct incompatible connection rejections should include a backend-owned
  `contract_diagnostic` projection when canonical type compatibility produced a
  typed rejection.
- Stale graph diagnostics are backend-owned `WorkflowGraphDiagnostic` records.
  The graph contract validator is the classification source for unknown node
  types, retired node types, unresolved effective definitions, missing edge
  endpoints, missing handles, incompatible ports, capacity errors, and cycles.
  String validation output is a compatibility projection of those structured
  diagnostics, not a separate validator.
- Shared graph inspection uses `WorkflowGraphInspectionProjection` so saved
  graph inspection and future run inspection wrappers can carry the same graph
  snapshot, selected-node facts, and stale diagnostics without frontend
  inference.
- Edit-session graph snapshot responses carry `graph_diagnostics` produced by
  the same backend contract classifier; transports and frontend callers should
  render these facts instead of inferring stale state locally.
- Historic run graph projections carry `graph_diagnostics` for the
  reconstructed run snapshot so run inspection can expose stale facts without
  reading current graph files or rewriting retired nodes.
- Workflow submit/admission rejects graphs with blocking stale diagnostics
  before queue insertion and includes the same typed diagnostic records in the
  workflow error envelope's graph details.
- Edit-session connection and insertion API methods stay in
  `session_connection_api.rs` so revision-aware connection orchestration and
  insertion response projection remain separate from lifecycle and basic graph
  mutation methods.
- Graph session response helpers that exist only to support contract tests stay
  test-scoped; production response assembly should use the state-aware
  projection path.
- Graph edit-session mutation, undo/redo, insertion, connection, stale cleanup,
  event projection, and memory-impact tests stay in `session_tests.rs` so
  `session.rs` remains focused on production session orchestration.
- Connection candidate lookup never mutates session state.
- Persisted derived graph metadata is advisory and must be recomputed when stale.
- Workflow execution fingerprints are computed from executable topology only:
  sorted node ids, node types, node behavior versions, and sorted port
  connections. Node positions, node data, edge ids, derived graph caches, and
  other display metadata are excluded.
- Workflow presentation fingerprints are computed from display metadata only:
  sorted node positions and edge display ids/endpoints. They are persisted as a
  separate attribution record so historic graph viewers can restore layout
  without changing execution-version diagnostics grouping.
- Workflow graph run settings snapshots are computed from sorted node ids,
  node types, and node data. They exclude positions, edges, and derived graph
  caches so editable execution parameters can be audited as run context without
  changing workflow-version identity.
- Workflow save/delete file stems are not sanitized from arbitrary names; they
  must already be valid workflow identities so diagnostics and future workflow
  versions can use the same stable id.
- Filesystem workflow load path validation is tested at `FileSystemWorkflowGraphStore`;
  transport adapters must not keep parallel path-boundary implementations.
- Persisted workflow files may carry historical append-only `contract_upgrades`
  records from earlier releases. Current save/load paths do not add
  compatibility migration records for retired graph shapes; stale graph
  diagnostics classify retired nodes without rewriting them into executable
  current graphs.
- Current `puma-lib` nodes persist canonical graph-visible model identity and
  strip derived Pumas facts, including legacy `modelPath`/`model_path` values.
  Path-only `puma-lib` state is not a successful model-reference branch;
  schedulable inference must resolve through the canonical Pumas model-ref
  and scheduler-owned runtime handoff boundaries.
- Inference-interface request extraction accepts only one canonical
  `pumas_model_ref` source per inference node. Duplicate incoming bindings,
  wrong source handles, wrong source node types, connected-versus-inline
  disagreement, and wrong-type optional constraints are typed diagnostics, not
  silent fallback or "scheduler decides" paths.
- Dynamic `node.data.definition` overlays may add or override ports for a
  specific non-inference node instance through backend-owned effective
  contracts, but they must not invalidate the registry node type or silently
  remove unrelated static ports. Generic inference nodes must not use
  `node.data.definition` as an executable interface source; their model-specific
  ports must come from authored inference interface snapshots and backend
  descriptor validation.
- `llm-inference` effective port projection consumes the typed
  `node.data.inference_interface_snapshot` shape from
  `pantograph-inference-interface-contracts`. Invalid snapshots produce
  blocking effective-definition diagnostics; unknown future snapshot value
  categories must be rejected until the graph contract mapping is extended
  deliberately.
- Inference-interface descriptor resolution is a workflow-service boundary:
  lookup adapters provide path-free Pumas model state, selected-artifact state,
  inference capability facts, and runtime availability facts. The resolver
  assembles descriptors and typed unavailable diagnostics without guessing from
  names, paths, package facts, or runtime-host payloads.
- Scheduler inference projections and executable snapshot source projections
  require current dependency requirements proof for executable inference
  nodes. Missing, stale, unavailable, or invalid proof is a typed validation
  state failure, not a best-effort scheduler projection.
- Draft graph inference-interface request extraction accepts only canonical
  `pumas_model_ref` values from a connected source node or the inference node's
  own typed input value. It must not inspect `model_ref`, `model_path`, package
  summaries, or executable Pumas load targets as alternate model identity
  sources.
- Inference-interface projection is also workflow-service owned. Minimal
  authored snapshots are projected from validated descriptors, and draft
  validation summaries are derived from descriptor availability and diagnostics.
- Dependency-environment service calls happen after graph/session state has
  been snapshotted and released. Provider output is accepted only after the
  canonical dependency-environment service validates the result contract.
  Frontend submit state and backend admission must consume the typed summary
  instead of inferring enqueue permission from raw diagnostics.
- Synchronous inference-validation publication is workflow-service owned. It
  snapshots graph state under the session lock, runs descriptor projection after
  the graph lock is released, emits node-scoped descriptor events plus a
  graph-scoped summary event, and records current graph/node validation state
  for later dependency actions and scheduler admission. Publication rejects
  graphs whose inference-node projection count exceeds the explicit bounded
  projection policy instead of serializing or emitting unbounded projection
  records.
- Validation publication attempts use the dedicated workflow-service publisher
  boundary after graph state has been snapshotted. Refresh and explicit
  publication entrypoints share that publisher so fact lookup, lifecycle
  freshness checks, descriptor publication, and current-state recording cannot
  drift into parallel implementations.
- Validation-session lifecycle identity is workflow-service owned. Starting a
  validation session supersedes any active validation session for the graph edit
  session, publication is accepted only for the active graph revision and
  validation-session id, and session close rejects later validation publication.
  Each active validation session also has an owner-issued cancellation receiver;
  starting a replacement validation session or closing the graph edit session
  signals cancellation before stale provider results can publish.
- Validation-session lifecycle events are retained in a bounded workflow-service
  buffer keyed by graph edit-session identity. Events use backend-owned graph
  session id, graph revision, validation-session id, and monotonic per-session
  sequence numbers. Publication acceptance and rejection are recorded after
  active-session locks are released so event retention cannot extend validation
  identity lock lifetimes across async boundaries. When the buffer reaches
  capacity the oldest events are dropped and the owner records the dropped-event
  count; transport delivery will project that bounded state as typed graph-
  validation lifecycle diagnostics rather than replaying unbounded history.
  Closing a graph edit session removes the session's buffered lifecycle events
  so stale validation events cannot be delivered after close.
- Current inference-validation state may store a bounded, path-free dependency
  requirements proof keyed to the associated inference node, graph revision,
  validation session, descriptor fingerprint, model ref, task kind, and
  validated constraints. The proof must not store executable paths, Pumas
  package facts, runtime load targets, scheduler dispatch decisions, frontend
  display state, media payloads, metadata bags, or arbitrary JSON.
- Dependency-environment `Resolve` actions snapshot sidecar-authored selected
  bindings and manual override patches from graph node data, parse them once
  into `pantograph-dependency-planning` typed values, and ask the
  dependency-planning producer to create the current proof. `Check` and
  `Install` actions remain fail-closed until that proof exists. Malformed
  sidecar choices become typed graph-action diagnostics instead of transport
  errors or best-effort defaults.
- Inference-interface facts are supplied through a workflow-service provider
  boundary. Frontend and transport adapters may request validation by identity
  and graph revision, but must not provide raw Pumas facts, runtime facts,
  package summaries, executable load targets, local paths, or capability blobs
  as validation authority.
- Live inference-validation events have typed graph/node scope. Descriptor,
  drift, diagnostic, and update-proposal payloads must be node-scoped so
  multi-inference graphs can route updates unambiguously; summary payloads are
  graph-scoped and must not use sentinel node ids.
- Inference-interface update proposals are graph-service contracts, not shared
  descriptor contracts. They may reference shared drift reports and authored
  snapshots, but typed operations that replace snapshots, remove invalid
  edges, or clear invalid literals live in this module because graph mutation
  ownership belongs to workflow-service.
- Destructive inference-interface patch operations must mark the proposal
  destructive and require explicit confirmation before a later apply endpoint
  mutates graph state.
- Live inference validation events use backend-issued validation session ids,
  the same opaque graph fingerprint revision used by graph edit sessions and
  dependency-environment action intents, and strictly increasing event sequence
  numbers. Update-proposal events are workflow-service-owned because they carry
  graph patch operations.
- Current inference-validation state is workflow-service-owned and keyed by
  graph session plus graph revision. Dependency-environment actions, submit
  gating, and scheduler admission must consume that owner instead of carrying
  separate validation-summary caches or reconstructing freshness in Tauri or
  frontend code.
- Closing a graph edit session clears that session's current inference-
  validation state before reporting success. Later validation lifecycle owners
  must use the same workflow-service cleanup boundary for cancellation,
  buffered event cleanup, and stale provider-result rejection.
- Graph-session locks may be held only long enough to canonicalize/snapshot
  graph state, compute the current graph revision, and check target-node
  existence. Pumas lookup, inference capability resolution, runtime availability
  checks, dependency requirement resolution, and dependency-environment request
  derivation must run after releasing the graph lock.
- Dynamic `node.data.definition` overlays that carry `inference_payloads` must
  preserve them as structured task/input/result/role metadata through effective
  definition and contract projection. Those payloads remain backend-neutral
  graph facts and must not encode backend choice, runtime residency, scheduler
  admission, reservation, eviction, or priority.
- Graph DTO defaults should derive from the declared enum default when the
  public default remains the first-class reactive mode.
- Revision comparison and canonical definition fallbacks should use eager,
  explicit option/result helpers so graph-contract projection stays
  warning-clean without changing fallback semantics.
- `ollama-inference` is not a supported saved-graph compatibility target.
  Workflows must use canonical `llm-inference` with a Pumas model reference and
  a supported runtime hint; the graph service does not synthesize
  `retired_ollama` placeholders.

## Revisit Triggers
- Graph edit payloads need streaming patches instead of whole-graph snapshots.
- Persisted workflow files require schema migration beyond additive metadata.
- Node-definition discovery needs pluggable registries instead of built-in inventory.
- Canonical inference-node work introduces a richer model-reference port that
  can preserve old backend-specific model edges without diagnostic removal.

## Dependencies
**Internal:** `node-engine`, `workflow-nodes`,
`pantograph-inference-interface-contracts`, workflow service error types.

**External:** `serde`, `tokio`, `uuid`, `chrono`.

## Related ADRs
- `ADR-001` headless workflow service boundary.

## Usage Examples
```rust
use pantograph_workflow_service::{
    WorkflowGraph, WorkflowGraphEditSessionCreateRequest, WorkflowService,
};

let service = WorkflowService::new();
let response = service
    .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
        graph: WorkflowGraph::new(),
    })
    .await?;
```

## API Consumer Contract
- Create an edit session before calling mutation commands.
- Treat the returned edit-session response as the canonical source for session
  identity and session kind; transport adapters must not hardcode that
  classification locally.
- Treat `graph_revision` as an opaque concurrency token.
- Expect structured rejection for stale revisions or incompatible connections;
  incompatible type rejections may include a canonical `contract_diagnostic`
  with source/target node ids, port ids, value types, and rejection reason.
- Persist graphs explicitly through a `WorkflowGraphStore`; mutations do not autosave.

## Structured Producer Contract
- Request/response DTO field names are stable unless an explicit breaking change is documented.
- `WorkflowFile.version` is the persisted file-format version.
- `WorkflowGraph.derived_graph` is volatile advisory metadata and may be regenerated.
- `WorkflowExecutableTopology` is the contract used for execution
  fingerprinting; callers must not use `WorkflowGraph.compute_fingerprint()` as
  workflow-version identity.
- `WorkflowPresentationMetadata` is the contract used for presentation
  fingerprinting; consumers must not use it as execution identity or
  diagnostics grouping input.
- `WorkflowGraphRunSettings` is the contract used when queue submission stores
  editable node settings in immutable run snapshots.
- `WorkflowGraphMetadata.id` is derived from the persisted filename stem when listed from a store.
- `node.data.definition.inputs` and `node.data.definition.outputs` are additive
  per-node overlays resolved into `EffectiveNodeContract` during connection
  intent and validation; consumers must preserve stable port IDs when
  persisting them.
