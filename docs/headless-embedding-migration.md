# Headless Workflow API Migration Guide

## Audience
Consumers migrating from embedding-shaped request/response usage to generic workflow I/O bindings.

## New API Surface
- `workflow_run`
- `workflow_get_capabilities`
- `create_workflow_session`
- `run_workflow_session`
- `close_workflow_session`

## Legacy to New Mapping

### Legacy Pattern
- object-centric payloads (`objects[].text`, `results[].embedding`)
- embedding-specific response metadata at top level

### New Pattern
- call `workflow_run` with `inputs[]` node/port bindings
- optionally request explicit `output_targets[]`
- consume `outputs[]` bindings from output nodes

## Request Mapping
- workflow identifier:
  - legacy: workflow id + object list
  - new: `workflow_id` + optional `inputs[]`
- model override:
  - legacy: top-level model override fields
  - new: workflow graph + node data/input bindings decide model usage
- correlation:
  - legacy: `batch_id`
  - new: `run_id`

## Response Mapping
- execution id:
  - legacy: `run_id`
  - new: `run_id` (unchanged)
- embedding outputs:
  - legacy: `results[].embedding`
  - new: output-node bindings in `outputs[]`
- typed output families:
  - legacy: embedding-focused object payload
  - new: text/vector/audio/image/etc via generic `value`

## Session Migration Pattern
- create once with `create_workflow_session` for repeated runs
- call `run_workflow_session` with `inputs[]`/`output_targets[]`
- close with `close_workflow_session` when finished
- handle scheduler errors (`session_evicted`, `scheduler_busy`)

## Compatibility Notes
- This is a breaking contract change.
- No legacy alias exports are provided in UniFFI/Rustler.
- Tauri commands are not a headless API integration path.

## Binding Migration Guidance (UniFFI/Rustler)
- Recommended: consume `pantograph-workflow-service` directly in Rust hosts.
- For modular GUI HTTP hosting only:
  - enable `frontend-http` and call `frontend_http_workflow_*` exports.
