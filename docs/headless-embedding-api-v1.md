# Headless Workflow API Contract

## Status
Implemented (breaking replacement for prior embedding-shaped API).

## Objective
Define a stable, Rust-first headless workflow API for external consumers embedding Pantograph as a framework.

## Integration Boundary (Required)
- Headless consumers integrate via `pantograph-workflow-service` (core API).
- Tauri commands are transport adapters for the desktop app runtime only.
- Frontend HTTP transport is optional and isolated in
  `pantograph-frontend-http-adapter` for modular standalone GUI hosting.
- UniFFI/Rustler HTTP workflow exports are feature-gated (`frontend-http`) and
  are not part of the default headless API surface.

## Design Principles
- Rust-first application API, transport-agnostic.
- Generic workflow I/O through node/port bindings.
- Workflow outputs are produced by output nodes or explicit `output_targets`.
- No embedding-specific top-level response fields.
- Capability computation is backend-owned in workflow service, with adapters as
  transport/wiring only.

## Operations

### 1) `workflow_run`
Primary workflow execution operation.

### 2) `workflow_get_capabilities`
Capability discovery for consumers before calling `workflow_run`.

### 3) `workflow_get_io`
Discover workflow input/output surfaces (node IDs, port IDs, optional labels and descriptions)
without parsing graph internals client-side.

### 4) `create_workflow_session`
Create scheduler-managed repeat-run session for a workflow.

### 5) `workflow_preflight`
Static, best-effort validation before execution.

### 6) `run_workflow_session`
Run one request through an existing scheduler-managed session.

### 7) `close_workflow_session`
Close a scheduler-managed session.

### 8) `workflow_get_session_status`
Get current session state summary.

### 9) `workflow_list_session_queue`
List pending/running queue items for a session.

### 10) `workflow_cancel_session_queue_item`
Cancel a pending queue item for a session.

### 11) `workflow_reprioritize_session_queue_item`
Change priority of a pending queue item for a session.

### 12) `workflow_set_session_keep_alive`
Update whether a session runtime should stay warm between runs.

### 13) `workflow_graph_save`
Persist a workflow graph document through the host-configured graph store.

### 14) `workflow_graph_load`
Load a persisted workflow graph document.

### 15) `workflow_graph_list`
List persisted workflow graph metadata.

### 16) `workflow_graph_create_edit_session`
Create an editable graph session from a supplied workflow graph snapshot.

### 17) `workflow_graph_close_edit_session`
Close an editable graph session and release its undo/redo state.

### 18) `workflow_graph_get_edit_session_graph`
Get the latest graph snapshot and revision for an editable graph session.

### 19) `workflow_graph_get_undo_redo_state`
Get undo/redo availability for an editable graph session.

### 20) `workflow_graph_update_node_data`
Submit a node-data mutation and receive the updated graph snapshot.

### 21) `workflow_graph_add_node`
Submit a node insertion and receive the updated graph snapshot.

### 22) `workflow_graph_add_edge`
Submit an edge insertion and receive the updated graph snapshot.

### 23) `workflow_graph_remove_edge`
Submit an edge removal and receive the updated graph snapshot.

### 24) `workflow_graph_undo`
Undo the last accepted graph mutation and receive the restored graph snapshot.

### 25) `workflow_graph_redo`
Redo the last undone graph mutation and receive the restored graph snapshot.

### 26) `workflow_graph_get_connection_candidates`
Discover backend-owned connection candidates for one source anchor.

### 27) `workflow_graph_connect`
Attempt a revision-aware connection commit and receive either the updated graph
or a structured rejection.

### 28) `workflow_graph_insert_node_and_connect`
Atomically insert a compatible node type and connect it, or return a structured
rejection.

## Request Contract: `WorkflowRunRequest`

### Required
- `workflow_id`: string

### Optional
- `inputs`: array of `WorkflowPortBinding` (default: empty)
- `output_targets`: array of `WorkflowOutputTarget`
- `timeout_ms`: integer > 0 (optional)
- `run_id`: string

### Value Schema: `WorkflowPortBinding`
- `node_id`: string (required)
- `port_id`: string (required)
- `value`: any JSON value (required)

### Output Target Schema: `WorkflowOutputTarget`
- `node_id`: string (required)
- `port_id`: string (required)

## Response Contract: `WorkflowRunResponse`

### Required
- `run_id`: string
- `outputs`: array of `WorkflowPortBinding`
- `timing_ms`: integer

## Capabilities Contract: `WorkflowCapabilitiesResponse`
- `max_input_bindings`: integer
- `max_output_targets`: integer
- `max_value_bytes`: integer
- `runtime_requirements`: object
  - `estimated_peak_vram_mb`: integer or null
  - `estimated_peak_ram_mb`: integer or null
  - `estimated_min_vram_mb`: integer or null
  - `estimated_min_ram_mb`: integer or null
  - `estimation_confidence`: string
  - `required_models`: array<string>
  - `required_backends`: array<string>
  - `required_extensions`: array<string>
- `models`: array<object>
  - `model_id`: string
  - `model_revision_or_hash`: string or null
  - `model_type`: string or null
  - `node_ids`: array<string>
  - `roles`: array<string>

## Workflow I/O Contract

### `WorkflowIoRequest`
- `workflow_id`: string

### `WorkflowIoResponse`
- `inputs`: array<WorkflowIoNode>
- `outputs`: array<WorkflowIoNode>
- `inputs[]` includes only nodes where `definition.category == "input"` and
  `definition.io_binding_origin == "client_session"`.
- `outputs[]` includes only nodes where `definition.category == "output"` and
  `definition.io_binding_origin == "client_session"`.
- Nodes marked `definition.io_binding_origin == "integrated"` are never exposed
  by `workflow_get_io`.
- Missing/invalid `definition.io_binding_origin` on input/output nodes is a
  schema error.

### `WorkflowIoNode`
- `node_id`: string (required)
- `node_type`: string (required)
- `name`: string (optional)
- `description`: string (optional)
- `ports`: array<WorkflowIoPort>
- For input nodes, `ports[]` are bindable input surfaces only (`definition.inputs`).
- For output nodes, `ports[]` are readable output surfaces only (`definition.outputs`).
- No cross-direction fallback or node-type suffix inference is applied.

### `WorkflowIoPort`
- `port_id`: string (required)
- `name`: string (optional)
- `description`: string (optional)
- `data_type`: string (optional)
- `required`: boolean (optional)
- `multiple`: boolean (optional)

## Session Contracts

### `WorkflowSessionCreateRequest`
- `workflow_id`: string
- `usage_profile`: string (optional)
- `keep_alive`: boolean (optional, default `false`)

### `WorkflowSessionCreateResponse`
- `session_id`: string

### `WorkflowSessionRunRequest`
- `session_id`: string
- `inputs`: array of `WorkflowPortBinding` (optional)
- `output_targets`: array of `WorkflowOutputTarget` (optional)
- `timeout_ms`: integer > 0 (optional)
- `run_id`: string (optional)
- `priority`: integer (optional, default `0`)

### `WorkflowSessionCloseRequest`
- `session_id`: string

### `WorkflowSessionCloseResponse`
- `ok`: boolean

### `WorkflowSessionStatusRequest`
- `session_id`: string

### `WorkflowSessionStatusResponse`
- `session`: `WorkflowSessionSummary`

### `WorkflowSessionSummary`
- `session_id`: string
- `workflow_id`: string
- `usage_profile`: string (optional)
- `keep_alive`: boolean
- `state`: `idle` | `running` | `queued`
- `queued_runs`: integer
- `active_run_id`: string (optional)

### `WorkflowSessionQueueListRequest`
- `session_id`: string

### `WorkflowSessionQueueListResponse`
- `items`: array of `WorkflowSessionQueueItem`

### `WorkflowSessionQueueItem`
- `queue_id`: string
- `run_id`: string
- `priority`: integer
- `status`: `pending` | `running`

### `WorkflowSessionQueueCancelRequest`
- `session_id`: string
- `queue_id`: string

### `WorkflowSessionQueueCancelResponse`
- `ok`: boolean

### `WorkflowSessionQueueReprioritizeRequest`
- `session_id`: string
- `queue_id`: string
- `priority`: integer

### `WorkflowSessionQueueReprioritizeResponse`
- `ok`: boolean

### `WorkflowSessionKeepAliveRequest`
- `session_id`: string
- `keep_alive`: boolean

### `WorkflowSessionKeepAliveResponse`
- `session`: `WorkflowSessionSummary`

## Graph Document Contracts

### `WorkflowGraphSaveRequest`
- `name`: string
- `graph`: `WorkflowGraph`

### `WorkflowGraphSaveResponse`
- `path`: string

### `WorkflowGraphLoadRequest`
- `path`: string

### `WorkflowGraphListResponse`
- `workflows`: array of `WorkflowMetadata`

## Graph Edit Session Contracts

Graph edit sessions are distinct from scheduler-managed workflow run sessions.
They own editable graph state, revision tracking, and undo/redo history only.

### `WorkflowGraphEditSessionCreateRequest`
- `graph`: `WorkflowGraph`

### `WorkflowGraphEditSessionCreateResponse`
- `session_id`: string
- `graph_revision`: string

### `WorkflowGraphEditSessionCloseRequest`
- `session_id`: string

### `WorkflowGraphEditSessionCloseResponse`
- `ok`: boolean

### `WorkflowGraphEditSessionGraphRequest`
- `session_id`: string

### `WorkflowGraphEditSessionGraphResponse`
- `session_id`: string
- `graph_revision`: string
- `graph`: `WorkflowGraph`

### `WorkflowGraphUndoRedoStateRequest`
- `session_id`: string

### `WorkflowGraphUndoRedoStateResponse`
- `can_undo`: boolean
- `can_redo`: boolean
- `undo_count`: integer

### `WorkflowGraphUpdateNodeDataRequest`
- `session_id`: string
- `node_id`: string
- `data`: JSON value

### `WorkflowGraphAddNodeRequest`
- `session_id`: string
- `node`: `GraphNode`

### `WorkflowGraphAddEdgeRequest`
- `session_id`: string
- `edge`: `GraphEdge`

### `WorkflowGraphRemoveEdgeRequest`
- `session_id`: string
- `edge_id`: string

### `WorkflowGraphConnectionCandidatesRequest`
- `session_id`: string
- `source_anchor`: `ConnectionAnchor`
- `graph_revision`: string (optional)

### `WorkflowGraphConnectRequest`
- `session_id`: string
- `source_anchor`: `ConnectionAnchor`
- `target_anchor`: `ConnectionAnchor`
- `graph_revision`: string

### `WorkflowGraphInsertNodeAndConnectRequest`
- `session_id`: string
- `source_anchor`: `ConnectionAnchor`
- `node_type`: string
- `graph_revision`: string
- `position_hint`: `InsertNodePositionHint`
- `preferred_input_port_id`: string (optional)

## Preflight Contract

### `WorkflowPreflightRequest`
- `workflow_id`: string
- `inputs`: array of `WorkflowPortBinding` (optional)
- `output_targets`: array of `WorkflowOutputTarget` (optional)

### `WorkflowPreflightResponse`
- `missing_required_inputs`: array of `{ node_id, port_id }`
- `invalid_targets`: array of `WorkflowOutputTarget`
- `warnings`: array<string>
- `can_run`: boolean

### Preflight Scope
- Static validation only; not a runtime guarantee.
- Validates request shape, discovered output targets, and required external
  input surfaces.
- Required external inputs are `workflow_get_io.inputs[].ports[]` with
  `required == true`.
- Missing `required` metadata is treated as optional and reported as a warning.

## Behavior Requirements
- Workflow inputs are supplied via node/port bindings.
- Workflow outputs are returned from output nodes or explicit targets.
- `workflow_get_io` exposes the workflow input/output node surfaces for client binding.
- `output_targets[]` must reference entries advertised in `workflow_get_io.outputs[].ports[]`.
- Duplicate input bindings (`node_id + port_id`) are rejected.
- Duplicate output targets (`node_id + port_id`) are rejected.
- Node-level `name` and `description` are optional and can be provided in node data.
- `run_id` is caller-provided when supplied; otherwise generated by service.
- `timeout_ms` is enforced by the service and propagates cancellation intent to
  the host runtime.
- Session runs are scheduler-managed and queued by `priority` then FIFO.
- Runtime warm/unload is scheduler-owned; `keep_alive` is intent, not an
  override of scheduler safety decisions.
- Generic workflows are not constrained to embedding-specific output types.
- Backend-owned graph state must not be mutated optimistically in the GUI.
  Clients submit graph actions and render the returned graph snapshot.
- Graph edit sessions serialize mutations per session; one session's edits must
  not block unrelated sessions.
- Graph revisions are volatile concurrency tokens, not persisted identifiers.
- Connection candidate lookup must not mutate edit-session state.
- Insert-and-connect must be atomic from the client's perspective.

Recommended client flow:
- `workflow_get_io` -> `workflow_preflight` -> `workflow_run`

Recommended graph-edit client flow:
- `workflow_graph_create_edit_session` -> graph mutation commands ->
  `workflow_graph_get_edit_session_graph` as needed -> `workflow_graph_save`

## Error Model
- Canonical transport payload is `WorkflowErrorEnvelope` JSON:
  - `code`: string
  - `message`: string
- Error codes:
  - `invalid_request`
  - `workflow_not_found`
  - `capability_violation`
  - `runtime_not_ready`
  - `session_not_found`
  - `session_evicted`
  - `queue_item_not_found`
  - `scheduler_busy`
  - `output_not_produced`
  - `runtime_timeout`
  - `internal_error`
- Decision rules:
  - non-discovered target -> `invalid_request` (pre-run)
  - discovered target not emitted -> `output_not_produced` (post-run)
