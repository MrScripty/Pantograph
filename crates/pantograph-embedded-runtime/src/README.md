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
| `lib_tests/host_helper_tests.rs` | Focused embedded workflow host helper and runtime-registry error-mapping unit tests split out of the legacy root test module. |
| `lib.rs` | Composes the embedded runtime, workflow service, shared extensions, and public crate exports used by Tauri and standalone hosts. |
| `model_dependencies.rs` | Resolves Pantograph model dependency requirements and binds workflow requests to Pumas-backed execution facts. |
| `python_runtime_execution.rs` | Owns captured execution metadata for Python-backed runtime runs so workflow diagnostics and registry projection can reuse one recorder contract outside the task-executor facade. |
| `task_executor.rs` | Hosts Pantograph-specific task execution for Python-backed nodes and RAG-backed nodes while preserving core-node fallthrough. |
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
| `runtime_registry_controller.rs` | Owns the inference-gateway implementations of embedded runtime-registry controller traits. |
| `runtime_registry_errors.rs` | Owns workflow-facing runtime-registry and warmup coordination error mapping so adapters keep stable workflow-service error codes. |
| `runtime_registry_lifecycle.rs` | Owns backend-side runtime-registry sync, snapshot, warmup coordination, reclaim, stop-all, and restore orchestration so lifecycle sequencing stays separate from observation mapping. |
| `runtime_registry_observations.rs` | Owns backend-side runtime-registry observation builders and health-overlay matching for active, embedding, and execution-observed producer facts. |
| `workflow_scheduler_diagnostics.rs` | Owns workflow scheduler diagnostics provider projection from host runtime mode and shared runtime-registry state. |
| `workflow_runtime.rs` | Owns backend-side workflow execution helpers for embedding metadata flag projection, runtime trace/model-target shaping, runtime diagnostics projection, and execution-path or stored-snapshot runtime-registry reconciliation used by workflow diagnostics transport. |
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
record metadata. The runtime registry may consume this crate's capability and
execution facts, and this crate may emit reservation lifecycle signals into
that registry when a host injects one. Registry ownership still belongs to a
higher Pantograph application-layer coordinator rather than to this
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
- Host helper and runtime-registry error-mapping unit tests stay in
  `lib_tests/host_helper_tests.rs`; continue splitting the remaining legacy
  integration tests by behavior area rather than growing `lib_tests.rs`.
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
- Workflow technical-fit calls may also reuse this crate's request-projection
  helpers, but transport adapters must not build registry selector input or
  project selector reasons on their own.
- `model_dependencies.rs` accepts workflow dependency requests and returns
  machine-consumable dependency status or validation errors suitable for
  preflight blocking.
- Python-backed execution always crosses the adapter/process boundary; callers
  must expect process-launch and external-runtime failures to surface as
  execution errors.
- Compatibility policy: preserve existing workflow-facing field names and
  response envelopes unless an explicit Pantograph API break is approved.

## Structured Producer Contract
- `model_dependencies.rs` produces resolved dependency requirements and
  normalized model descriptors for Pantograph workflow execution.
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
