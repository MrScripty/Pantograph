# crates/pantograph-embedded-runtime/src

## Purpose
This directory contains the Pantograph-owned runtime composition layer that
binds workflow execution to host resources such as the inference gateway, Pumas
library extensions, Python sidecar execution, dependency preflight, and RAG
adapters. The directory boundary exists so host/runtime orchestration stays in
one crate instead of leaking infrastructure policy into generic workflow-node
packages.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `embedded_data_graph_execution.rs` | Owns embedded-runtime data-graph execution, terminal-node demand handling, runtime extension injection, and data-graph output collection. |
| `embedded_edit_session_execution.rs` | Owns embedded-runtime edit-session graph execution, embedding runtime preparation, workflow event emission, runtime trace projection, and inference-runtime restore coordination. |
| `embedding_workflow.rs` | Owns backend-side embedding workflow graph inspection, embedding model-path resolution, and workflow-specific runtime preparation rules. |
| `embedded_runtime_lifecycle.rs` | Owns embedded-runtime constructors, host wiring, registry injection, accessors, and shutdown coordination. |
| `embedded_workflow_graph_api.rs` | Owns embedded-runtime public graph persistence, edit-session, graph mutation, connection, and insert-preview facade methods that forward into the workflow service. |
| `embedded_workflow_host.rs` | Owns the embedded workflow host implementation that adapts host runtime, model metadata, runtime capabilities, session loading, inspection, technical-fit, and workflow runs into workflow-service contracts. |
| `embedded_workflow_host_helpers.rs` | Owns embedded workflow host helper logic for runtime reservations, retention hints, workflow I/O binding, and data-graph terminal output shaping. |
| `embedded_workflow_service_api.rs` | Owns embedded-runtime public workflow, session, queue, inspection, and keep-alive facade methods that forward into the workflow service. |
| `lib_tests.rs` | Legacy embedded-runtime facade, host, registry, and workflow-session tests extracted from the root facade file. |
| `lib_tests/data_graph_execution_tests.rs` | Embedded data-graph execution integration tests split out of the legacy root test module. |
| `lib_tests/edit_session_execution_tests.rs` | Embedded edit-session graph execution integration tests split out of the legacy root test module. |
| `lib_tests/graph_fixtures.rs` | Shared runtime data-graph builders and synthetic node-memory snapshot fixtures used by embedded-runtime behavior tests. |
| `lib_tests/host_helper_tests.rs` | Focused embedded workflow host helper and runtime-registry error-mapping unit tests split out of the legacy root test module. |
| `lib_tests/runtime_lifecycle_capability_tests.rs` | Embedded hosted-runtime lifecycle, shutdown, and injected-capability tests split out of the legacy root test module. |
| `lib_tests/runtime_preflight_tests.rs` | Embedded runtime preflight and unload-candidate selection tests split out of the legacy root test module. |
| `lib_tests/session_checkpoint_capacity_tests.rs` | Embedded keep-alive workflow-session capacity checkpoint tests split out of the legacy root test module. |
| `lib_tests/session_checkpoint_recovery_tests.rs` | Embedded keep-alive workflow-session checkpoint recovery tests split out of the legacy root test module. |
| `lib_tests/session_execution_state_tests.rs` | Embedded keep-alive workflow-session execution state tests split out of the legacy root test module. |
| `lib_tests/session_runtime_lifecycle_tests.rs` | Embedded workflow-session runtime lifecycle integration tests split out of the legacy root test module. |
| `lib_tests/workflow_run_execution_tests.rs` | Embedded workflow-run and session-run execution integration tests split out of the legacy root test module. |
| `lib.rs` | Composes the embedded runtime, workflow service, shared extensions, and public crate exports used by Tauri and standalone hosts. |
| `model_dependencies.rs` | Resolves Pantograph model dependency requirements and binds workflow requests to Pumas-backed execution facts. |
| `model_dependency_activity.rs` | Defines dependency activity event payloads, emitters, and request context projection shared by resolver phases and install streams. |
| `model_dependency_descriptors.rs` | Resolves stable model identity, cache keys, Pumas execution descriptors, backend aliases, task tags, and requirements ids for dependency preflight. |
| `model_dependency_python.rs` | Owns Python environment lookup, pip package checks, package install stream capture, binding checks, and per-environment install locks for the dependency resolver. |
| `model_dependency_requirements.rs` | Maps Pumas dependency requirement contracts into node-engine DTOs and applies validated user override patches. |
| `model_dependencies_tests.rs` | Pantograph model dependency resolver tests and Pumas descriptor fixture helpers extracted from the production resolver module. |
| `node_execution.rs` | Defines runtime-created node execution context, cancellation/progress handles, output summaries, lineage context, and guarantee classification. |
| `node_execution_capabilities.rs` | Defines managed capability route contracts and typed capability wrappers for model, resource, cache, progress, diagnostics, and external-tool access. |
| `node_execution_diagnostics.rs` | Adapts node-engine workflow events into enriched transient runtime-owned node diagnostics with attribution, contract, lineage, and guarantee context. |
| `node_execution_diagnostics_tests.rs` | Focused diagnostics adapter tests for lifecycle, output summary, progress, stream, failure, cancellation, and filtering behavior. |
| `node_execution_tests.rs` | Focused runtime-created node execution context, managed capability routing, cancellation, progress, output summary, and guarantee classification tests. |
| `python_runtime_execution.rs` | Owns captured execution metadata for Python-backed runtime runs so workflow diagnostics and registry projection can reuse one recorder contract outside the task-executor facade. |
| `task_executor.rs` | Hosts the Pantograph-specific task executor facade, construction, extension keys, and node-type dispatch while preserving core-node fallthrough. |
| `task_executor/` | Behavior modules for RAG search, Puma-Lib metadata projection, dependency environment/preflight, and Python runtime execution used by the host executor facade. |
| `task_executor_tests.rs` | Shared Pantograph host task-executor test fixtures and behavior-module index. |
| `task_executor_tests/` | Focused task-executor behavior tests for dependency preflight/fallback, input helpers, Puma-Lib metadata rebinding, and Python runtime recorder/stream behavior. |
| `technical_fit.rs` | Owns embedded-runtime technical-fit translation, including host-side runtime snapshot/candidate assembly, request projection into backend runtime-registry selector input, selector invocation, and decision projection back to workflow-service contracts without moving policy into adapters. |
| `python_runtime.rs` | Defines the out-of-process Python runtime adapter contract and the default process-backed implementation. |
| `python_runtime_bridge.py` | Bridge script executed by the Python adapter so Pantograph can invoke Python workers without linking Python in-process. |
| `rag.rs` | Defines the narrow RAG backend contract used by the host executor. |
| `runtime_capabilities.rs` | Owns backend-side mapping from producer-specific runtime facts into workflow runtime capabilities, including managed-runtime snapshot-to-capability projection, host-runtime, dedicated-embedding, and Python-sidecar capability builders plus capability-to-lifecycle projection. |
| `runtime_config.rs` | Owns embedded-runtime configuration and initialization error contracts re-exported by the crate facade. |
| `runtime_extensions.rs` | Owns shared runtime extension snapshots and executor extension injection for Pumas, KV cache, model dependencies, event sinks, execution ids, and Python runtime execution records. |
| `runtime_health.rs` | Owns backend-side health probe assessment, degraded/unhealthy threshold policy, and failure-count progression. |
| `runtime_recovery.rs` | Owns backend-side recovery restart planning, retry-strategy selection, retry-attempt sequencing, retry backoff, backend port overrides, clean-restart settle delays, and dedicated-embedding restart policy. |
| `runtime_registry.rs` | Owns backend-side translation from gateway and producer lifecycle facts into shared runtime-registry observations, active-runtime registration, active/embedding health-aware unhealthy reconciliation, sync, reclaim, stop-all, and restore coordination. |
| `runtime_registry_tests.rs` | Embedded runtime-registry translation, sync, reclaim, restore, and warmup coordination tests extracted from the production runtime-registry module. |
| `runtime_registry_tests/` | Behavior-focused runtime-registry test modules for observation, lifecycle, health, and warmup coordination coverage. |
| `runtime_registry_controller.rs` | Owns the inference-gateway implementations of embedded runtime-registry controller traits. |
| `runtime_registry_errors.rs` | Owns workflow-facing runtime-registry and warmup coordination error mapping so adapters keep stable workflow-service error codes. |
| `runtime_registry_lifecycle.rs` | Owns backend-side runtime-registry sync, snapshot, warmup coordination, reclaim, stop-all, and restore orchestration so lifecycle sequencing stays separate from observation mapping. |
| `runtime_registry_observations.rs` | Owns backend-side runtime-registry observation builders and health-overlay matching for active, embedding, and execution-observed producer facts. |
| `workflow_scheduler_diagnostics.rs` | Owns workflow scheduler diagnostics provider projection from host runtime mode and shared runtime-registry state. |
| `workflow_runtime.rs` | Owns backend-side workflow execution helpers for embedding metadata flag projection, runtime trace/model-target shaping, runtime diagnostics input grouping, and execution-path or stored-snapshot runtime-registry reconciliation used by workflow diagnostics transport. |
| `workflow_runtime_tests.rs` | Shared fixtures and module index for workflow runtime diagnostics, event projection, metric normalization, and registry reconciliation tests. |
| `workflow_runtime_tests/` | Focused workflow-runtime helper tests for diagnostics snapshot assembly, event projection, metrics/model-target helpers, and registry reconciliation behavior. |
| `workflow_session_execution.rs` | Owns backend-side keep-alive workflow-session executor storage, graph-change reuse/reconciliation, and unload-transition application so scheduler-driven reclaim and direct capacity rebalance share one logical-session path. |

## Problem
Pantograph needs a host-owned runtime layer that can execute workflow graphs,
resolve model/runtime dependencies, and bridge to external runtimes while still
using shared node and workflow contracts. Without this boundary, workflow-node
code would need to know too much about Tauri, Python process management, and
Pumas-specific dependency resolution.

## Constraints
- Preserve the workflow-service and node-engine facades consumed by higher
  layers.
- Keep Python execution out-of-process and consumer-managed.
- Treat Pumas executable model facts as an upstream contract, not something
  Pantograph re-derives from projected metadata.
- Keep dependency preflight deterministic because it can block workflow
  execution before node runtime starts.
- App-global runtime residency, admission, retention, and eviction policy must
  stay outside this crate even though it exposes Pantograph-specific runtime
  capabilities.
- If a runtime registry is injected, this crate may translate host-owned
  session load/unload lifecycle into registry reservation operations, but it
  still must not own the policy that decides when those reservations should
  exist.
- When workflow-service asks for an idle unload candidate under loaded-runtime
  pressure, this crate may project session ids into registry reservation-owner
  ids and consume the registry's ordered eviction candidates, but it still must
  not invent an alternate eviction policy locally.

## Decision
Keep this crate as the application/infrastructure integration layer for
Pantograph-owned runtime behavior. `model_dependencies.rs` is responsible for
mapping workflow dependency requests onto Pumas contracts, and it should prefer
`ModelExecutionDescriptor` when a request can resolve a model id. The crate
preserves the existing workflow-facing `model_path`, `model_type`, and
`task_type_primary` facades, but the values behind those fields may come from
the descriptor `entry_path` and descriptor task/type data rather than projected
record metadata. Python package checks, binding installation, install stream
capture, and per-environment install locks stay in `model_dependency_python.rs`
so the resolver facade remains focused on API orchestration, cache lookup, and
Pumas contract projection. The runtime registry may consume this crate's
capability and execution facts, and this crate may emit reservation lifecycle
signals into that registry when a host injects one. Registry ownership still
belongs to a higher Pantograph application-layer coordinator rather than to this
embedded-runtime crate.

## Alternatives Rejected
- Resolve executable paths directly from `ModelRecord.metadata`.
  Rejected because metadata is a projection and can drift from the authoritative
  runtime contract exposed by Pumas.
- Move Pantograph-specific runtime binding into `workflow-nodes`.
  Rejected because `workflow-nodes` should remain host-agnostic and not own
  host/runtime infrastructure policy.

## Invariants
- Pantograph-specific runtime orchestration stays in this crate, not in generic
  node packages.
- Embedded-runtime construction, host projection, registry injection, and
  shutdown sequencing stay in `embedded_runtime_lifecycle.rs` so the root
  facade keeps only type definitions, exports, and remaining workflow API
  forwarding until those surfaces are split.
- The root `lib.rs` facade should import only items used by facade code; tests
  and feature-specific constructors should import executor-extension locks in
  their owning modules instead of keeping stale facade imports.
- Extracted root-facade tests should import shared executor-extension and async
  lock helpers from `lib_tests.rs` so production facade imports remain warning
  clean while all-target checks still compile test modules.
- Embedded-runtime data-graph execution stays in
  `embedded_data_graph_execution.rs` so terminal-node demand handling and
  output shaping remain separate from graph persistence and edit-session API
  forwarding.
- Embedded-runtime edit-session graph execution stays in
  `embedded_edit_session_execution.rs` so embedding runtime preparation,
  workflow event emission, runtime trace projection, and inference-runtime
  restore coordination are isolated from the root facade.
- Public embedded-runtime graph persistence, edit-session, mutation,
  connection, and insert-preview facade methods stay in
  `embedded_workflow_graph_api.rs` so graph API forwarding remains separate
  from graph execution.
- The `WorkflowHost` implementation for embedded runtime stays in
  `embedded_workflow_host.rs` so host adaptation, runtime capability exposure,
  session loading, inspection, technical-fit, and workflow-run execution are
  separated from root crate composition.
- Embedded workflow host helper methods stay in
  `embedded_workflow_host_helpers.rs` so reservation shaping, runtime retention
  sync, workflow I/O binding, and data-graph output shaping do not accumulate
  inside the trait implementation facade.
- Public embedded-runtime workflow, session, queue, inspection, and keep-alive
  facade methods stay in `embedded_workflow_service_api.rs` so root composition
  remains separate from workflow-service API forwarding.
- Root embedded-runtime facade tests stay outside `lib.rs` so production
  runtime composition remains reviewable; split `lib_tests.rs` further when a
  behavior-focused test module boundary is introduced.
- Workflow diagnostics snapshot builders group scheduler/runtime inputs into
  explicit input structs so registry synchronization and projection helpers do
  not grow long positional argument lists.
- Host helper and runtime-registry error-mapping unit tests stay in
  `lib_tests/host_helper_tests.rs`; continue splitting the remaining legacy
  integration tests by behavior area rather than growing `lib_tests.rs`.
- Data-graph execution integration tests stay in
  `lib_tests/data_graph_execution_tests.rs` so Python sidecar runtime
  observation and waiting-for-input propagation tests follow the production
  data-graph execution boundary.
- Edit-session graph execution integration tests stay in
  `lib_tests/edit_session_execution_tests.rs` so embedding runtime
  prepare/restore reconciliation, runtime trace metrics, and waiting-for-input
  event behavior follow the production edit-session execution boundary.
- Workflow-session runtime lifecycle integration tests stay in
  `lib_tests/session_runtime_lifecycle_tests.rs` so reservation lifecycle,
  warmup/preflight, unload, and non-keep-alive release behavior follow the
  production workflow-session runtime boundary.
- Workflow-run and session-run execution integration tests stay in
  `lib_tests/workflow_run_execution_tests.rs` so public run facades,
  cancellation, human-input validation, and Python sidecar runtime observation
  follow the production workflow execution boundary.
- Interactive-input invalid-request errors from keep-alive session execution
  must preserve the workflow id in the message, matching direct workflow-run
  behavior so bindings do not drift on caller-visible error envelopes.
- Keep-alive workflow-session execution state tests stay in
  `lib_tests/session_execution_state_tests.rs` so backend executor reuse,
  carried inputs, graph-change reconciliation, and inspection state follow the
  production workflow-session execution boundary.
- Keep-alive workflow-session capacity checkpoint tests stay in
  `lib_tests/session_checkpoint_capacity_tests.rs` so checkpoint preservation,
  scheduler-driven rebalance, repeated unload idempotence, and keep-alive
  disable cleanup follow the production session checkpoint boundary.
- Keep-alive workflow-session checkpoint recovery tests stay in
  `lib_tests/session_checkpoint_recovery_tests.rs` so failed restore,
  runtime-not-ready resume, and scheduler reclaim isolation keep checkpoint
  cleanup separate from capacity-only checkpoint tests.
- Runtime preflight and unload-candidate selection tests stay in
  `lib_tests/runtime_preflight_tests.rs` so install-state availability checks
  and registry eviction-order selection do not accumulate in the root test
  harness.
- Runtime graph builders and synthetic node-memory snapshot fixtures stay in
  `lib_tests/graph_fixtures.rs` so shared data-graph/checkpoint fixtures do not
  keep the root test harness over the large-file threshold.
- Pantograph host task-executor tests and Python runtime fixture helpers stay
  in `task_executor_tests.rs` so `task_executor.rs` remains focused on
  production host execution for Python-backed nodes, RAG-backed nodes, and core
  executor fallthrough.
- Task-executor test coverage stays split under `task_executor_tests/` by
  dependency preflight/fallback, input helper, Puma-Lib, and Python
  recorder/stream behavior so runtime execution changes remain reviewable by
  behavior family.
- Pantograph host task-executor behavior stays grouped by execution family:
  dependency environment and preflight helpers in
  `task_executor/dependency_environment.rs`, Puma-Lib metadata projection in
  `task_executor/puma_lib.rs`, Python sidecar execution and stream replay in
  `task_executor/python_execution.rs`, and RAG search in
  `task_executor/rag_search.rs`.
- Embedded-runtime source formatting and import grouping should stay
  rustfmt-compatible across runtime capability, registry, workflow runtime, and
  workflow-session modules so later behavior refactors do not mix semantic
  changes with mechanical formatting churn.
- Hosted runtime lifecycle and injected capability tests stay in
  `lib_tests/runtime_lifecycle_capability_tests.rs` so construction,
  shutdown-state reconciliation, and capability injection checks do not
  accumulate in the shared root test harness.
- Python-backed nodes execute through the runtime adapter boundary.
- Shared runtime extension snapshots and executor injection must stay in a
  backend-owned helper so workflow execution paths do not drift on extension
  keys or recorder wiring.
- Python runtime execution metadata and recorder state stay in backend Rust so
  workflow diagnostics and registry projection do not depend on Tauri-local or
  executor-local ad hoc payloads.
- Failed Python runtime executions may also carry a backend-owned health
  assessment so execution-observed producer failures reuse the shared degraded
  versus unhealthy threshold contract, only escalating matching registry
  observations to `unhealthy` when the backend threshold is crossed instead of
  remaining snapshot-only errors.
- Dependency preflight and runtime execution must agree on executable model
  paths for the same resolved model.
- Pantograph must preserve workflow-facing field names even when the underlying
  values come from Pumas execution descriptors.
- This crate may expose runtime capabilities and execute Pantograph-owned
  runtime paths, but it must not become the owner of app-global runtime
  residency or admission policy.
- Any registry interaction from this crate must remain a narrow translation of
  session lifecycle into explicit registry operations, not an alternate policy
  engine.
- Inference-gateway runtime-registry controller trait implementations stay in
  `runtime_registry_controller.rs` so lifecycle adapter code does not accrete
  in the root runtime facade.
- Technical-fit bridging in this crate must stay a translation layer from
  workflow-service request context into runtime-registry selector input; factor
  scoring and final policy ownership remain outside this crate.
- Producer-specific runtime capability mapping must stay in backend Rust so
  adapters do not drift on runtime ids, install state, or backend-key aliases.
- Managed-runtime, host-runtime, and Python-sidecar capability shaping must
  stay in the shared backend capability helper module rather than being
  rebuilt inside `EmbeddedWorkflowHost` or Tauri adapters.
- Runtime configuration and initialization error contracts stay in
  `runtime_config.rs` and are re-exported by the crate facade so embedding
  hosts do not couple to the root composition file.
- Managed-runtime capability shaping must consume backend-owned managed-runtime
  snapshots rather than flatter install-only capability records, so workflow
  preflight can see backend-owned readiness and selected-version context
  without Tauri-local reconstruction.
- Capability parity across managed, host, dedicated-embedding, and Python-
  sidecar producers must be pinned by backend tests so later producer-specific
  additions do not silently drift on runtime ids, install-state semantics, or
  required capability fields.
- Capability-driven lifecycle projection for diagnostics and workflow fallback
  paths must stay in the shared backend capability helper module rather than
  being rebuilt inside workflow adapters.
- Gateway and producer observation mapping for the shared runtime registry must
  stay in backend Rust so adapters do not drift on runtime-id, backend-key, or
  lifecycle-status translation.
- Runtime-registry observation builders and health-overlay matching may be
  decomposed into helper modules, but those helpers must remain backend-owned
  and must not be reintroduced as Tauri-local mapping code.
- Runtime-registry sync-before-snapshot and sync-before-reclaim semantics must
  stay in backend Rust so host adapters do not drift on when authoritative
  mode-info reconciliation happens.
- Runtime-registry stop-all and restore reconciliation semantics must stay in
  backend Rust so shutdown, restart, and restore wrappers do not drift on
  post-transition registry convergence.
- Runtime-registry sync, snapshot, reclaim, stop-all, and restore orchestration
  may be decomposed into helper modules, but those helpers must remain
  backend-owned and must not be reintroduced as Tauri-local sequencing code.
- Runtime warmup polling, mode-info reconciliation, and `WarmupStarted`
  transition sequencing for active runtimes must stay in shared backend
  registry helpers so workflow execution does not own a separate warmup policy
  loop.
- Reservation release plus retention-driven reclaim sequencing must stay in
  shared backend registry helpers so workflow execution does not own a second
  release-and-evict policy path outside the registry lifecycle boundary.
- Reservation retention-hint mutation must stay in shared backend registry
  helpers so workflow execution does not depend directly on the lower-level
  registry update call when session keep-alive policy changes.
- Keep-alive session executor unload behavior must stay backend-owned in Rust:
  capacity rebalance may checkpoint and retain logical workflow-session state,
  repeated capacity-rebalance unload must preserve the original checkpoint
  identity for that retained session state, and explicit keep-alive disable and
  session close may tear that state down.
- Checkpoint-backed session restore semantics must also stay backend-owned in
  Rust: failed resumed execution must preserve the existing checkpoint marker
  and timestamp by returning the session to checkpoint-backed residency, while
  only a successful resumed run may clear the checkpoint and advance the
  session to warm residency.
- Multi-session keep-alive checkpoint isolation must stay keyed by workflow
  session id in backend Rust so reclaim pressure cannot cross-wire one
  session's carried inputs, checkpoint summary, or executor ownership into
  another session's resumed run.
- Scheduler-driven reclaim for keep-alive workflow sessions must route through
  the same backend session-execution unload transition as direct
  `CapacityRebalance` unload so the runtime-registry boundary does not become a
  second owner of checkpoint semantics.
- Live workflow-session inspection for diagnostics must read node memory,
  checkpoint, and residency state from the retained backend session executor
  instead of reconstructing that state inside Tauri transports.
- Scheduler runtime-registry diagnostics shaping, including reclaim-candidate
  lookup and warmup-decision translation, must stay in shared backend registry
  helpers so workflow providers do not drift on registry-to-workflow mapping.
- Active-runtime registration used by scheduler diagnostics and reservation
  paths must stay in shared backend registry helpers so workflow execution does
  not re-derive registration shape in multiple call sites.
- Active-runtime reservation request shaping must stay in shared backend
  registry helpers so admission checks and reservation acquisition do not drift
  on runtime id, owner id, model target, or retention payload shape.
- Execution-path runtime snapshot override reconciliation must stay in backend
  Rust so workflow adapters do not drift on when Python-sidecar or
  embedding-path execution facts become shared registry observations, and so
  execution-local snapshots do not erase a matching producer's existing
  backend-owned `unhealthy` registry state.
- Stored-runtime replay and live-host-runtime skip rules for diagnostics and
  restore projections must stay in shared backend registry helpers so workflow
  diagnostics code does not own a second post-restore registry reconciliation
  path.
- Restore-time registry reconciliation must preserve a matching unhealthy
  assessment for the restored runtime instance while replacing stale unhealthy
  records from older instances, so recovery and restore paths do not drift on
  runtime-instance identity semantics.
- Restore-time replay of stored non-live runtime snapshots into the shared
  registry must stay in backend Rust so diagnostics paths can rehydrate
  execution-observed producer state without overriding live gateway or
  embedding producer observations.
- Diagnostics-path sequencing that combines stored-snapshot replay with runtime
  event projection must stay in backend Rust so Tauri transport does not own
  the order in which registry reconciliation and runtime projection occur.
- Diagnostics-path sync-before-projection sequencing must stay in backend Rust
  so headless workflow adapters do not own the order in which gateway-backed
  registry sync and runtime-event projection occur.
- Workflow-execution diagnostics sync-before-snapshot sequencing must stay in
  backend Rust so interactive execution adapters do not own the order in which
  gateway-backed registry sync and execution snapshot projection occur.
- Post-transition runtime-registry reconciliation must stay in backend
  lifecycle helpers so recovery and other host transitions do not drift on
  whether failed or successful transitions still publish the latest registry
  state.
- Workflow-facing runtime-registry coordination failures must be mapped in
  backend Rust onto stable workflow-service error codes so adapters do not
  collapse admission or runtime-unavailable decisions into generic internal
  failures.
- Scheduler diagnostics provider projection and runtime-registry error mapping
  must stay in focused backend helpers instead of being embedded in the public
  runtime facade.
- The concrete embedded workflow host must preserve backend-owned
  non-streaming cancellation semantics at the runtime boundary itself, so a
  pre-cancelled `WorkflowRunHandle` returns `WorkflowServiceError::Cancelled`
  before execution begins instead of being flattened by adapter-local error
  handling.
- Runtime-registry unhealthy projection from host health assessment must stay
  in backend Rust so adapters do not drift on when a failed runtime transitions
  from observed-ready lifecycle into registry `unhealthy` state.
- Runtime-registry unhealthy projection for the dedicated embedding producer
  must stay in backend Rust so host polling loops do not invent a separate
  embedding-runtime failure policy.
- Ordinary runtime-registry synchronization must consume the latest matching
  host-provided health snapshot in backend Rust so later mode-info refreshes do
  not erase a previously assessed `unhealthy` runtime back to lifecycle-ready.
- Recovery restart-plan derivation must stay in backend Rust so wrappers do not
  drift on backend port-override or dedicated-embedding restart policy.
- Health probe assessment and degraded/unhealthy threshold interpretation must
  stay in backend Rust so host polling loops do not drift on failure-count
  progression or status transitions.
- Recovery retry-strategy and backoff derivation must stay in backend Rust so
  wrappers do not drift on attempt sequencing or retry-delay policy.
- Recovery alternate-port fallback and clean-restart settle-delay policy must
  stay in backend Rust so wrappers only provide host port-availability facts
  instead of branching on reconnect behavior locally.
- Embedding workflow graph inspection and Puma-Lib model-id resolution for
  runtime mode preparation must stay in backend Rust so adapters do not drift
  on workflow validation rules or required wiring.
- Embedding model-path resolution and workflow-specific embedding runtime
  preparation must stay in backend Rust so RAG, workflow execution, and server
  startup consume one runtime-preparation rule set.
- Workflow execution extension wiring plus runtime trace/model-target shaping
  must stay in backend Rust so adapters do not drift on execution metadata or
  diagnostics semantics.

## Revisit Triggers
- A second runtime integration path needs the same dependency-resolution policy
  and this crate no longer provides a clear ownership boundary.
- The Python bridge evolves into a long-lived managed service with its own
  lifecycle policy.
- Pumas changes the execution descriptor contract in a way that requires a
  Pantograph facade change.

## Dependencies
**Internal:** `node_engine`, `pantograph_workflow_service`, `inference`, and
Pantograph host wiring that injects shared extensions and adapters.

**External:** `pumas_library` for model records, execution descriptors, and
dependency contracts; Python worker scripts executed through the runtime
adapter.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- Reason: it freezes this crate as a runtime producer/executor rather than the
  owner of the planned `RuntimeRegistry` policy layer.
- Revisit trigger: runtime-registry implementation requires this crate to
  expose a new host-facing facade or changes its ownership boundary.

## Usage Examples
```rust
use std::sync::Arc;

use node_engine::ExecutorExtensions;
use pantograph_embedded_runtime::{
    EmbeddedRuntime, EmbeddedRuntimeConfig, RuntimeExtensionsSnapshot,
};
use tokio::sync::RwLock;

let config = EmbeddedRuntimeConfig::new(app_data_dir, project_root);
let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
let snapshot = RuntimeExtensionsSnapshot::from_shared(&extensions).await;

let runtime = EmbeddedRuntime::with_default_python_runtime(
    config,
    gateway,
    extensions,
    workflow_service,
    rag_backend,
);
```

## API Consumer Contract
- Hosts create the runtime by supplying gateway, workflow-service, and shared
  extension dependencies; this directory does not own those outer application
  boot decisions.
- `EmbeddedRuntimeConfig` and `StandaloneRuntimeConfig` may carry an optional
  `max_loaded_sessions` limit so hosts can tune loaded-runtime residency
  without moving unload policy ownership out of backend Rust services, and the
  embedded-runtime constructors apply that limit to the injected
  `WorkflowService` so scheduler-driven reclaim pressure is enforced on the
  same backend path outside the standalone bootstrap.
- Hosts may optionally inject a shared runtime registry; when present, session
  runtime load/unload lifecycle is translated into registry reservation
  acquire/release operations.
- Hosts that own additional producer snapshots beyond the core
  `inference::InferenceGateway` may pass a richer `HostRuntimeModeSnapshot`
  into the hosted runtime constructor so backend Rust can derive registry
  observations and additive runtime capabilities from one contract.
- Direct embedded workflow runs may also reconcile Python-sidecar execution
  snapshots into that shared registry so producer-specific runtime facts do not
  depend on Tauri-only diagnostics paths.
- Tauri and other adapters may reuse this crate's runtime-registry translation
  helpers, but they must not own separate gateway-to-registry observation
  mapping logic.
- Embedded hosted-runtime shutdown, live gateway sync, and restore paths should
  also reuse this crate's shared runtime-registry lifecycle helpers rather than
  recomposing raw gateway calls with local reconcile steps.
- Embedded runtime-registry translation, sync, reclaim, restore, and warmup
  coordination tests stay in `runtime_registry_tests.rs` so production
  observation mapping and registry orchestration stay separate from mocked host
  controller coverage. Larger observation, lifecycle, health, and warmup
  behavior families live under `runtime_registry_tests/` so the parent test
  module remains a fixture/index boundary.
- Workflow runtime diagnostics, runtime event projection, metric normalization,
  model-target selection, and registry reconciliation tests stay under
  `workflow_runtime_tests/`, while `workflow_runtime_tests.rs` retains shared
  fixtures and module registration so production workflow-runtime projection
  helpers stay separate from diagnostics fixtures.
- Workflow technical-fit calls may also reuse this crate's request-projection
  helpers, but transport adapters must not build registry selector input or
  project selector reasons on their own.
- `model_dependencies.rs` accepts workflow dependency requests and returns
  machine-consumable dependency status or validation errors suitable for
  preflight blocking.
- `model_dependency_activity.rs` owns dependency activity event payloads and
  request context projection while `model_dependencies.rs` remains the public
  re-export surface used by Tauri event emitters.
- `model_dependency_descriptors.rs` owns descriptor/cache/model identity
  resolution so model ids, executable paths, backend aliases, selected binding
  ids, and task tags are normalized before dependency requirement and model-ref
  projection.
- `model_dependency_requirements.rs` owns Pumas dependency-contract mapping,
  binding selection, runtime-state aggregation, install-target normalization,
  and override patch validation so the resolver facade can focus on API,
  cache, Python process, and Pumas lookup orchestration.
- `model_dependency_python.rs` owns Python environment lookup, package version
  checks, pip install invocation, output stream capture, binding install
  checks, and per-environment install locks so dependency-resolution API flow
  stays separate from process orchestration.
- Model dependency resolver tests and Pumas descriptor fixture helpers stay in
  `model_dependencies_tests.rs` so production resolver changes are not coupled
  to integration-fixture churn.
- Python-backed execution always crosses the adapter/process boundary; callers
  must expect process-launch and external-runtime failures to surface as
  execution errors.
- Compatibility policy: preserve existing workflow-facing field names and
  response envelopes unless an explicit Pantograph API break is approved.

## Structured Producer Contract
- `model_dependencies.rs` produces resolved dependency requirements and
  normalized model descriptors for Pantograph workflow execution.
- `model_dependency_activity.rs` preserves the serialized dependency activity
  event shape emitted to Tauri so frontend listeners keep receiving stable
  phase, binding, requirement, stream, and model-path fields.
- `model_dependency_descriptors.rs` preserves stable cache keys,
  requirements-id shape, Pumas descriptor fallback semantics, backend-key
  canonicalization, and workflow-facing `model_path`/`task_type_primary`
  compatibility before the resolver runs dependency checks or installation.
- `model_dependency_requirements.rs` preserves stable dependency error codes,
  binding ids, validation scopes, selected binding order, install targets, and
  user override validation before those facts are cached or returned by the
  resolver facade.
- `model_dependency_python.rs` preserves dependency package check/install
  semantics, pip output projection, binding installation status, and
  per-environment install serialization before those results are cached or
  returned by the resolver facade.
- When Pumas descriptor resolution succeeds, the executable path contract is the
  descriptor `entry_path`; projected metadata fields such as `entry_path`,
  `storage_kind`, and `bundle_format` are compatibility fallbacks only.
- Dependency-preflight errors must remain machine-consumable with stable codes,
  scopes, and binding association so the host can block execution
  deterministically.
- Runtime capability payloads emitted here are producer facts for host/runtime
  consumers; a future `RuntimeRegistry` may compose them with admission or
  residency state, but this crate must not silently fold policy-level decisions
  into those producer contracts.
- Registry reconciliation should consume the richest available producer
  snapshot contract, typically a host-owned `HostRuntimeModeSnapshot`; callers
  should not bypass that contract with a narrower core-gateway-only helper when
  additional producer facts such as the dedicated embedding sidecar are
  available.
- If the descriptor contract changes, this directory must regenerate its README
  contract text and add ADR coverage if the compatibility boundary expands.
