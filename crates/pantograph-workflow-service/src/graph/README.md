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
| `canonicalization.rs` | Saved graph canonicalization orchestration and migration-record response assembly. |
| `canonicalization_inference.rs` | Dynamic inference-setting schema expansion, per-node definition overlay rebuilds, and passthrough port helpers. |
| `canonicalization_legacy_migration.rs` | Legacy saved-node rewrites and typed contract-upgrade record production. |
| `canonicalization_tests.rs` | Canonicalization migration and inference-overlay regression tests. |
| `effective_definition.rs` | Resolves backend-owned effective node contracts and projects them into graph DTOs before validation or candidate lookup. |
| `executable_topology.rs` | Canonical executable-topology projection and BLAKE3 workflow execution fingerprint calculation for workflow versioning. |
| `validation.rs` | Shared connection compatibility helpers used by graph-edit flows. |
| `connection_intent.rs` | Canonical candidate-discovery and revision-aware connection/insert validation. |
| `connection_insert.rs` | Internal node-insert, edge-insert preview, and edge-bridge helpers used by `connection_intent.rs` while preserving the public graph-edit facade. |
| `group_mutation.rs` | Backend-owned create/ungroup/update-port graph mutations for collapsed node groups. |
| `session_contract.rs` | Additive graph snapshot contracts and response-assembly helpers, including the Phase 6 workflow-session state view and explicit backend-state projection seam surfaced to transport layers. |
| `session_graph.rs` | Graph utility helpers for embedding metadata sync, graph conversion into `node-engine`, and shared node-data merge behavior. |
| `session_runtime.rs` | Focused runtime/lifecycle state for one graph edit session, including active execution metadata, queue projection, and run counters. |
| `session_types.rs` | Edit-session request/response DTOs and local undo/redo/session-kind types that are shared by the graph session boundary. |
| `session.rs` | Edit-session store, undo/redo state, and graph mutation orchestration. |
| `session_connection_api.rs` | Edit-session connection candidate, direct connect, node insert-connect, and edge-insert API methods. |
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
- Workflow save/delete file stems are not sanitized from arbitrary names; they
  must already be valid workflow identities so diagnostics and future workflow
  versions can use the same stable id.
- Filesystem workflow load path validation is tested at `FileSystemWorkflowGraphStore`;
  transport adapters must not keep parallel path-boundary implementations.
- Dynamic `node.data.definition` overlays may add or override ports for a
  specific node instance through backend-owned effective contracts, but they
  must not invalidate the registry node type or silently remove unrelated
  static ports.
- Graph DTO defaults should derive from the declared enum default when the
  public default remains the first-class reactive mode.
- Revision comparison and canonical definition fallbacks should use eager,
  explicit option/result helpers so graph-contract projection stays
  warning-clean without changing fallback semantics.

## Revisit Triggers
- Graph edit payloads need streaming patches instead of whole-graph snapshots.
- Persisted workflow files require schema migration beyond additive metadata.
- Node-definition discovery needs pluggable registries instead of built-in inventory.

## Dependencies
**Internal:** `node-engine`, `workflow-nodes`, workflow service error types.

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
- `WorkflowGraphMetadata.id` is derived from the persisted filename stem when listed from a store.
- `node.data.definition.inputs` and `node.data.definition.outputs` are additive
  per-node overlays resolved into `EffectiveNodeContract` during connection
  intent and validation; consumers must preserve stable port IDs when
  persisting them.
