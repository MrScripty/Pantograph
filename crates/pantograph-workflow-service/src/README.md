# crates/pantograph-workflow-service/src

## Purpose
Host-agnostic application service contracts and orchestration entrypoints for Pantograph workflow APIs.

## Boundaries
- No transport framework dependencies (Tauri/UniFFI/Rustler).
- No UI concerns.
- Host/runtime dependencies exposed via traits.

## Contents
- `workflow.rs`: headless workflow contracts, host traits, and orchestration logic.
- `scheduler/`: backend-owned workflow-session scheduler contracts and queue
  store boundary used by `workflow.rs` so queue policy does not stay embedded
  in the service facade.
- `technical_fit.rs`: host-agnostic technical-fit request and decision DTOs plus
  normalization helpers, session queue-pressure/context assembly, and
  workflow-service request/session entrypoints plus runtime-preflight
  assessment glue that freeze how workflow and session context is projected
  into backend runtime selection without owning the selector policy.
- `capabilities.rs`: shared workflow capability/validation utilities used by all adapters.
- `trace/`: host-agnostic workflow trace modules that separate backend-owned
  contracts and request validation, in-memory trace state/store ownership, and
  runtime/scheduler snapshot merge helpers so diagnostics transport stays
  additive at adapter boundaries.

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
  by graph fingerprint, runtime capability fingerprint, and normalized
  technical-fit override selection
- execution never triggers runtime installation implicitly
- when the service asks a host to load session runtime resources, it now passes
  a backend-owned retention hint derived from session `keep_alive` state so
  adapters can forward intent without becoming retention-policy owners
- hosts may also tune loaded-runtime residency separately from session count
  through `WorkflowService::set_loaded_runtime_capacity_limit`, keeping the
  capacity boundary backend-owned even when the value comes from app config

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
- `WorkflowTechnicalFitRequest`
- `WorkflowTechnicalFitDecision`
- `WorkflowTraceStore`

## Capability Ownership

- Runtime requirement extraction/estimation is backend-owned in this crate.
- Adapters should provide host dependencies (workflow roots, backend identity,
  optional model metadata), not duplicate capability business logic.
- Session `keep_alive` interpretation starts here; adapters may forward the
  resulting retention hint to lower-level runtime infrastructure, but they must
  not invent separate retention policy.
- The workflow-session scheduler queue/store now lives under `scheduler/`,
  while `workflow.rs` remains the facade and orchestration entrypoint that
  delegates into that backend-owned scheduler boundary.
- This crate owns session-idle/runtime-loaded facts, but hosts may consume
  those facts through an explicit unload-candidate contract so backend runtime
  registries remain the owner of reservation eviction ordering.
- Session capacity and loaded-runtime capacity may now diverge here, which
  makes runtime rebalance reachable without conflating "how many sessions may
  exist" with "how many runtimes may stay loaded".
- Technical-fit request normalization also belongs here: this crate may shape
  workflow and session context into a backend-owned selector request contract,
  but it must not become the owner of runtime policy or candidate scoring.
- Additive `override_selection` fields on workflow preflight, direct run, and
  session-run requests are also owned here so adapters can forward explicit
  backend/model intent without reconstructing selector policy locally.
- Workflow preflight and runtime-not-ready reporting may also consume the
  backend-owned technical-fit decision here so hosts surface one selector
  result instead of drifting between preflight and execution-time runtime
  readiness semantics.
- Graph edit sessions, graph persistence contracts, revision-aware connection
  intent, and undo/redo semantics are backend-owned in this crate.
- Workflow trace and metrics contract ownership is backend-owned in this crate;
  adapters may project or transport traces but must not invent timing or
  lifecycle state locally.
- The in-memory trace store owns retention plus canonical run/node event
  timestamp capture for ordinary workflow events, so adapters forward events
  into backend-owned timing rather than stamping trace chronology locally.
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
- Duplicate terminal run/node events are normalized here as idempotent replay,
  preserving the first terminal timestamps and durations instead of asking
  adapters to de-duplicate delivery.
- When the same execution id receives a new terminal-to-running `RunStarted`
  transition, this crate resets attempt-scoped trace state before recording the
  new attempt so retry or replay flows do not leak stale node, queue, or
  runtime facts into the restarted run.
- When backend scheduler or runtime snapshots replay for the same execution id
  during recovery or restore, this crate updates the canonical trace in place
  instead of materializing duplicate runs, leaving adapters free to reread one
  backend-owned execution record after reconciliation.
- Canonical trace summaries can now represent explicit cancelled run and node
  states when upstream adapters emit a cancellation outcome instead of a
  generic failure.
- Canonical trace summaries also retain the originating `session_id` when a
  queued/run execution id diverges from the session identity, so trace reads
  can filter by session without adapter-local reconstruction.
- `workflow_get_capabilities` includes `models[]` inventory with `model_id`,
  optional `model_revision_or_hash`, optional `model_type`, `node_ids`, and
  `roles`.
- Runtime install/remove/status actions remain outside this crate. Clients use
  the host's inference/runtime facade to change runtime availability, then read
  updated capability state from workflow contracts.

## Verification

- Contract tests: `crates/pantograph-workflow-service/tests/contract.rs`
- CI gate: `.github/workflows/headless-embedding-contract.yml`
