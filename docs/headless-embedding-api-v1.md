# Headless Embedding API v1 Contract

## Status
Frozen for implementation

## Objective
Define a stable, Rust-first headless embedding API for external consumers embedding Pantograph as a framework.

## Design Principles
- Rust-first application API, transport-agnostic.
- Explicit request/response metadata; no event-stream parsing required.
- Deterministic correlation and ordering.
- Versioned contracts with additive evolution.

## Operations

### 1) `embed_objects_v1`
Primary embedding operation for object-in/object-out workflow execution.

### 2) `get_embedding_workflow_capabilities_v1`
Capability discovery for consumers before calling `embed_objects_v1`.

## Request Contract: `EmbedObjectsV1Request`

### Required
- `api_version`: string literal `"v1"`
- `workflow_id`: string
  - identifies embedding workflow or named embedding preset
- `objects`: array of 1..N embedding objects

### Optional
- `model_id`: string
  - explicit override if supported by workflow
- `batch_id`: string
  - caller correlation/idempotency key

### Object Schema: `EmbedInputObject`
- `object_id`: string (required)
- `text`: string (required, non-empty)
- `metadata`: object (optional, passthrough caller metadata)

## Response Contract: `EmbedObjectsV1Response`

### Required
- `api_version`: string literal `"v1"`
- `run_id`: string
- `model_signature`: object
- `results`: array of per-object results
- `timing_ms`: integer

### `model_signature`
Always present on successful responses.

Required fields:
- `model_id`: string
- `backend`: string
- `vector_dimensions`: integer

Optional fields:
- `model_revision_or_hash`: string

### `results[]`: `EmbedObjectResult`
- `object_id`: string
- `embedding`: array<number> or null
- `token_count`: integer (optional)
- `status`: enum
  - `"success"`
  - `"failed"`
- `error`: object (optional when `status=failed`)
  - `code`: string
  - `message`: string

## Behavior Requirements

### Deterministic Correlation
- Response `results` preserves input object order.
- Every input object yields exactly one result with same `object_id`.

### Partial Failures
- Individual object failures must not fail whole batch by default.
- Batch-level transport/service failure is reserved for fatal preconditions:
  - invalid request schema
  - unknown workflow
  - missing runtime dependencies that prevent all execution

### Model Signature Guarantee
- Successful batch response requires non-empty `model_signature`.
- If signature cannot be resolved deterministically, request fails with explicit error.

### `batch_id` Semantics
- Treated as correlation key by default.
- Idempotency behavior is best-effort unless host config enables strict idempotency storage.

## Capabilities Contract: `GetEmbeddingWorkflowCapabilitiesV1Response`

Required fields:
- `api_version`: string literal `"v1"`
- `supported_models`: array<string>
- `max_batch_size`: integer
- `max_text_length`: integer

Optional fields:
- `notes`: array<string>

## Error Model

### API/Request Errors
- `invalid_request`
- `unsupported_api_version`
- `workflow_not_found`
- `capability_violation`

### Runtime Errors
- `runtime_not_ready`
- `model_signature_unavailable`
- `internal_error`

### Object-Level Errors
- `object_validation_failed`
- `embedding_failed`

## Versioning Rules
- v1 is append-only for non-breaking evolution.
- Additive fields must be optional for consumers.
- Removing/renaming required fields requires new version.
- Semantic behavior changes require new version or explicit feature flag.

## Compatibility Guarantees
- v1 request/response shapes are stable once released.
- Adapters (Tauri/UniFFI/Rustler) must delegate to the same service contract.
- Transport-specific wrappers must not alter business semantics.
