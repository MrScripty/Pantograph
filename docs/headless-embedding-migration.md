# Headless Embedding API Migration Guide

## Audience
Consumers currently relying on workflow execution commands/events who need deterministic object-in/object-out embedding integration.

## New API Surface
- `embed_objects_v1`
- `get_embedding_workflow_capabilities_v1`

## Legacy to New Mapping

### Legacy Pattern
- `execute_workflow_v2` / `run_workflow_session`
- parse `NodeCompleted` / `Completed` events for embedding outputs
- reconstruct object correlation in client code

### New Pattern
- call `embed_objects_v1` with explicit object list
- receive ordered `results[]` with per-object status and errors
- consume `model_signature` directly from response

## Request Mapping
- workflow identifier:
  - legacy: graph/session setup + execution call
  - new: `workflow_id` in request payload
- model override:
  - legacy: encoded in node graph wiring and runtime model state
  - new: optional `model_id` field
- correlation:
  - legacy: caller-managed around event streams
  - new: optional `batch_id` (mirrored as `run_id`)

## Response Mapping
- execution id:
  - legacy: command return value + event correlation
  - new: `run_id`
- embedding outputs:
  - legacy: node-level outputs/events
  - new: `results[].embedding`
- failures:
  - legacy: run-level failure or event parsing
  - new: per-object `status` + `error`
- model metadata:
  - legacy: inferred from node metadata and runtime state
  - new: explicit `model_signature`

## Compatibility Notes
- New API is versioned (`api_version = "v1"`).
- v1 changes are additive-only; breaking changes require a new version.
- Existing workflow commands remain available during migration.
