# Headless Workflow API Migration Guide

## Audience
Consumers currently relying on workflow execution commands/events who need deterministic object-in/object-out embedding integration.

## New API Surface
- `workflow_run`
- `workflow_get_capabilities`
- `create_workflow_session`
- `run_workflow_session`
- `close_workflow_session`

## Legacy to New Mapping

### Legacy Pattern
- `execute_workflow_v2` / `run_workflow_session`
- parse `NodeCompleted` / `Completed` events for embedding outputs
- reconstruct object correlation in client code

### New Pattern
- call `workflow_run` with explicit object list
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
- workflow model inventory:
  - legacy: ad-hoc model traversal
  - new: `workflow_get_capabilities.models[]`

## Session Migration Pattern
- create once with `create_workflow_session` for repeated runs
- call `run_workflow_session` for each batch
- close with `close_workflow_session` when finished
- handle explicit scheduler errors (`session_evicted`, `scheduler_busy`)

## Compatibility Notes
- Core headless API remains a breaking replacement for older embed/event-driven
  patterns.
- Existing consumers should migrate from embed-specific calls to
  workflow-level service calls.
- Tauri commands should not be used as a headless API integration path.
- UniFFI/Rustler provide temporary legacy alias exports only behind
  `frontend-http-legacy` for frontend HTTP migration.

## Binding Migration Guidance (UniFFI/Rustler)
- Recommended: consume `pantograph-workflow-service` directly in Rust hosts.
- For modular GUI HTTP hosting only:
  - enable `frontend-http` and call `frontend_http_workflow_*` exports.
  - optionally enable `frontend-http-legacy` for temporary
    `workflow_*` alias compatibility.
