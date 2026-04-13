# crates/pantograph-workflow-service/src

## Purpose
Host-agnostic application service contracts and orchestration entrypoints for Pantograph workflow APIs.

## Boundaries
- No transport framework dependencies (Tauri/UniFFI/Rustler).
- No UI concerns.
- Host/runtime dependencies exposed via traits.

## Contents
- `workflow.rs`: headless workflow contracts, host traits, and orchestration logic.
- `capabilities.rs`: shared workflow capability/validation utilities used by all adapters.
- `trace.rs`: host-agnostic workflow trace and metrics DTOs used to freeze
  backend-owned diagnostics contracts before adapter-specific projections and
  to keep trace request validation and in-memory trace ownership in the Rust
  service boundary.

## Headless Workflow API

Primary operations:

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
- `workflow_graph_save`
- `workflow_graph_load`
- `workflow_graph_list`
- `workflow_graph_create_edit_session`
- `workflow_graph_close_edit_session`
- `workflow_graph_get_edit_session_graph`
- `workflow_graph_get_undo_redo_state`
- `workflow_graph_update_node_data`
- `workflow_graph_add_node`
- `workflow_graph_add_edge`
- `workflow_graph_remove_edge`
- `workflow_graph_undo`
- `workflow_graph_redo`
- `workflow_graph_get_connection_candidates`
- `workflow_graph_connect`
- `workflow_graph_insert_node_and_connect`

`workflow_get_io` strict discovery rule:
- only nodes marked `definition.category` in `{input, output}` with
  `definition.io_binding_origin == "client_session"` are externally bindable.

Runtime capability rule:
- hosts expose runtime availability through `runtime_capabilities()`
- `workflow_get_capabilities` and `create_workflow_session` return those
  capabilities to clients
- `workflow_preflight` reports runtime warnings and blocking runtime issues
- `run_workflow_session` reuses a session-scoped runtime preflight cache keyed
  by graph fingerprint and runtime capability fingerprint
- execution never triggers runtime installation implicitly

Primary contract types:

- `WorkflowRunRequest`
- `WorkflowRunResponse`
- `WorkflowPortBinding`
- `WorkflowOutputTarget`
- `WorkflowCapabilitiesRequest`
- `WorkflowCapabilitiesResponse`
- `WorkflowIoRequest`
- `WorkflowIoResponse`
- `WorkflowPreflightRequest`
- `WorkflowPreflightResponse`
- `WorkflowRuntimeCapability`
- `WorkflowRuntimeIssue`
- `WorkflowSessionCreateRequest`
- `WorkflowSessionCreateResponse`
- `WorkflowSessionRunRequest`
- `WorkflowSessionCloseRequest`
- `WorkflowSessionStatusRequest`
- `WorkflowSessionQueueListRequest`
- `WorkflowSessionQueueCancelRequest`
- `WorkflowSessionQueueReprioritizeRequest`
- `WorkflowSessionKeepAliveRequest`
- `WorkflowTraceSummary`
- `WorkflowTraceEvent`
- `WorkflowTraceGraphContext`
- `WorkflowTraceNodeRecord`
- `WorkflowTraceQueueMetrics`
- `WorkflowTraceRuntimeMetrics`
- `WorkflowTraceSnapshotRequest`
- `WorkflowTraceSnapshotResponse`
- `WorkflowTraceStore`

## Capability Ownership

- Runtime requirement extraction/estimation is backend-owned in this crate.
- Adapters should provide host dependencies (workflow roots, backend identity,
  optional model metadata), not duplicate capability business logic.
- Graph edit sessions, graph persistence contracts, revision-aware connection
  intent, and undo/redo semantics are backend-owned in this crate.
- Workflow trace and metrics contract ownership is backend-owned in this crate;
  adapters may project or transport traces but must not invent timing or
  lifecycle state locally.
- Scheduler snapshots now expose additive `trace_execution_id` attribution when
  the backend can unambiguously identify the active or uniquely-visible queued
  run. Adapters must treat an omitted value as "identity ambiguous" rather than
  guessing from session-local state.
- Trace snapshot filter validation belongs here with the request DTOs so Tauri
  command handlers can reject malformed interop payloads without duplicating
  request policy in adapter code.
- The in-memory recent-trace store also belongs here so adapters can forward
  canonical trace events into one backend-owned owner instead of accumulating
  lifecycle state locally.
- `workflow_get_capabilities` includes `models[]` inventory with `model_id`,
  optional `model_revision_or_hash`, optional `model_type`, `node_ids`, and
  `roles`.
- Runtime install/remove/status actions remain outside this crate. Clients use
  the host's inference/runtime facade to change runtime availability, then read
  updated capability state from workflow contracts.

## Verification

- Contract tests: `crates/pantograph-workflow-service/tests/contract.rs`
- CI gate: `.github/workflows/headless-embedding-contract.yml`
