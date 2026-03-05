# Headless Workflow API Migration Guide

## Audience
Consumers migrating from embedding-shaped request/response usage to generic workflow I/O bindings.

## New API Surface
- `workflow_run`
- `workflow_get_capabilities`
- `workflow_get_io`
- `workflow_preflight`
- `create_workflow_session`
- `run_workflow_session`
- `close_workflow_session`
- `workflow_get_session_status`
- `workflow_list_session_queue`
- `workflow_cancel_session_queue_item`
- `workflow_reprioritize_session_queue_item`
- `workflow_set_session_keep_alive`

## Legacy to New Mapping

### Legacy Pattern
- object-centric payloads (`objects[].text`, `results[].embedding`)
- embedding-specific response metadata at top level

### New Pattern
- call `workflow_get_io` to discover bindable input/output nodes and ports
- treat discovery as authoritative:
  - bind only `workflow_get_io.inputs[].ports[]` in `inputs[]`
  - target only `workflow_get_io.outputs[].ports[]` in `output_targets[]`
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
- create once with `create_workflow_session` for repeated runs (`keep_alive` optional)
- call `run_workflow_session` with `inputs[]`/`output_targets[]` (`priority` optional)
- inspect runtime queue/state via `workflow_get_session_status` and `workflow_list_session_queue`
- optionally cancel or reprioritize pending queue items
- close with `close_workflow_session` when finished
- handle scheduler errors (`scheduler_busy`, `queue_item_not_found`)

## Compatibility Notes
- This is a breaking contract change.
- No legacy alias exports are provided in UniFFI/Rustler.
- Tauri commands are not a headless API integration path.

## Binding Migration Guidance (UniFFI/Rustler)
- Recommended: consume `pantograph-workflow-service` directly in Rust hosts.
- For modular GUI HTTP hosting only:
  - enable `frontend-http` and call `frontend_http_workflow_*` exports.
