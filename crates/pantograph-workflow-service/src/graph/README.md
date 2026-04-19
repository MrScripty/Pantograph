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
| `registry.rs` | Built-in node-definition discovery and node-engine metadata conversion. |
| `effective_definition.rs` | Merges registry metadata with additive per-node definition overlays before validation or candidate lookup. |
| `validation.rs` | Shared connection compatibility helpers used by graph-edit flows. |
| `connection_intent.rs` | Canonical candidate-discovery and revision-aware connection/insert validation. |
| `session_contract.rs` | Additive graph snapshot contracts and response-assembly helpers, including the Phase 6 workflow-session state view and explicit backend-state projection seam surfaced to transport layers. |
| `session_graph.rs` | Graph utility helpers for embedding metadata sync, graph conversion into `node-engine`, and shared node-data merge behavior. |
| `session_runtime.rs` | Focused runtime/lifecycle state for one graph edit session, including active execution metadata, queue projection, and run counters. |
| `session_types.rs` | Edit-session request/response DTOs and local undo/redo/session-kind types that are shared by the graph session boundary. |
| `session.rs` | Edit-session store, undo/redo state, and graph mutation orchestration. |
| `persistence.rs` | Graph-store trait plus the filesystem-backed `.pantograph/workflows` implementation. |

## Problem
Pantograph previously kept graph-editing logic inside Tauri modules, which made
headless clients second-class consumers and allowed transport layers to become
business-logic owners.

## Constraints
- Graph-edit contracts must remain transport-agnostic.
- Persisted workflow files must stay compatible with existing `.pantograph/workflows` JSON.
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
the logic and contracts live here. Dynamic per-node port overlays are resolved
through `effective_definition.rs`, which starts from the registry definition and
applies additive `inputs`/`outputs` overrides from persisted node data only
when the node type matches.

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
- Connection candidate lookup never mutates session state.
- Persisted derived graph metadata is advisory and must be recomputed when stale.
- Dynamic `node.data.definition` overlays may add ports for a specific node
  instance, but they must not invalidate the registry node type or silently
  remove unrelated static ports.

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
- Expect structured rejection for stale revisions or incompatible connections.
- Persist graphs explicitly through a `WorkflowGraphStore`; mutations do not autosave.

## Structured Producer Contract
- Request/response DTO field names are stable unless an explicit breaking change is documented.
- `WorkflowFile.version` is the persisted file-format version.
- `WorkflowGraph.derived_graph` is volatile advisory metadata and may be regenerated.
- `WorkflowGraphMetadata.id` is derived from the persisted filename stem when listed from a store.
- `node.data.definition.inputs` and `node.data.definition.outputs` are additive
  per-node overlays consumed during connection intent and validation; consumers
  must preserve stable port IDs when persisting them.
