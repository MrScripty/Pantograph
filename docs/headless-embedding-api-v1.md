# Headless Workflow API Contract

## Status
Implemented (breaking replacement for prior embed-specific API).

## Objective
Define a stable, Rust-first headless workflow API for external consumers embedding Pantograph as a framework.

## Design Principles
- Rust-first application API, transport-agnostic.
- Explicit request/response metadata; no event-stream parsing required.
- Deterministic correlation and ordering.
- No embed-specific top-level method names.
- Capability computation is backend-owned in workflow service, with adapters as
  transport/wiring only.

## Operations

### 1) `workflow_run`
Primary workflow operation for object-in/object-out execution.

### 2) `workflow_get_capabilities`
Capability discovery for consumers before calling `workflow_run`.

### 3) `create_workflow_session`
Create scheduler-managed repeat-run session for a workflow.

### 4) `run_workflow_session`
Run one batch through an existing scheduler-managed session.

### 5) `close_workflow_session`
Close a scheduler-managed session.

## Request Contract: `WorkflowRunRequest`

### Required
- `workflow_id`: string
- `objects`: array of 1..N workflow input objects

### Optional
- `model_id`: string
- `batch_id`: string

### Object Schema: `WorkflowInputObject`
- `object_id`: string (required)
- `text`: string (required, non-empty)
- `metadata`: object (optional passthrough metadata)

## Response Contract: `WorkflowRunResponse`

### Required
- `run_id`: string
- `model_signature`: object
- `results`: array of per-object results
- `timing_ms`: integer

### `model_signature`: `RuntimeSignature`
- `model_id`: string
- `backend`: string
- `vector_dimensions`: integer
- `model_revision_or_hash`: string (optional)

### `results[]`: `WorkflowObjectResult`
- `object_id`: string
- `embedding`: array<number> or null
- `token_count`: integer (optional)
- `status`: `"success"` or `"failed"`
- `error`: object (optional when failed)
  - `code`: string
  - `message`: string

## Capabilities Contract: `WorkflowCapabilitiesResponse`
- `max_batch_size`: integer
- `max_text_length`: integer
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

## Session Contracts

### `WorkflowSessionCreateRequest`
- `workflow_id`: string
- `usage_profile`: string (optional)

### `WorkflowSessionCreateResponse`
- `session_id`: string

### `WorkflowSessionRunRequest`
- `session_id`: string
- `objects`: array of `WorkflowInputObject`
- `model_id`: string (optional)
- `batch_id`: string (optional)

### `WorkflowSessionCloseRequest`
- `session_id`: string

### `WorkflowSessionCloseResponse`
- `ok`: boolean

## Behavior Requirements
- Preserve input ordering and `object_id` correlation.
- Support per-object partial failure.
- Return non-empty `model_signature` on success responses.
- Use `batch_id` as correlation/run identifier when provided.

## Error Model
- `invalid_request`
- `workflow_not_found`
- `capability_violation`
- `runtime_not_ready`
- `model_signature_unavailable`
- `session_not_found`
- `session_evicted`
- `scheduler_busy`
- `internal_error`
