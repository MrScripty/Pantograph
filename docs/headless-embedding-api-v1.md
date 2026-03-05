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

Recommended client flow:
- `workflow_get_io` -> `workflow_preflight` -> `workflow_run`

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
