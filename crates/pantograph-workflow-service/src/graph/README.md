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
| `validation.rs` | Shared connection compatibility helpers used by graph-edit flows. |
| `connection_intent.rs` | Canonical candidate-discovery and revision-aware connection/insert validation. |
| `session.rs` | Edit-session store, undo/redo state, graph mutation orchestration, and graph-to-engine conversion helpers. |
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

## Decision
Define a dedicated graph-editing module inside `pantograph-workflow-service`
that owns graph contracts, edit-session orchestration, and persistence
abstractions. Host adapters may expose those operations over IPC/FFI/HTTP, but
the logic and contracts live here.

## Alternatives Rejected
- Keep graph editing in Tauri and expose only execution in core.
  Rejected because it keeps headless clients incomplete and breaks backend-owned state rules.
- Put graph-edit types directly into `workflow.rs`.
  Rejected because graph editing is a distinct contract surface with its own lifecycle and persistence concerns.

## Invariants
- Edit sessions are distinct from scheduler-managed workflow run sessions.
- Graph mutations return backend-owned graph snapshots or structured rejections.
- Connection candidate lookup never mutates session state.
- Persisted derived graph metadata is advisory and must be recomputed when stale.

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
- Treat `graph_revision` as an opaque concurrency token.
- Expect structured rejection for stale revisions or incompatible connections.
- Persist graphs explicitly through a `WorkflowGraphStore`; mutations do not autosave.

## Structured Producer Contract
- Request/response DTO field names are stable unless an explicit breaking change is documented.
- `WorkflowFile.version` is the persisted file-format version.
- `WorkflowGraph.derived_graph` is volatile advisory metadata and may be regenerated.
- `WorkflowGraphMetadata.id` is derived from the persisted filename stem when listed from a store.
