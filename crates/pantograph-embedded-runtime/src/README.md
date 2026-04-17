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
| `embedding_workflow.rs` | Owns backend-side embedding workflow graph inspection, embedding model-path resolution, and workflow-specific runtime preparation rules. |
| `lib.rs` | Composes the embedded runtime, workflow service, shared extensions, and public crate exports used by Tauri and standalone hosts. |
| `model_dependencies.rs` | Resolves Pantograph model dependency requirements and binds workflow requests to Pumas-backed execution facts. |
| `python_runtime_execution.rs` | Owns captured execution metadata for Python-backed runtime runs so workflow diagnostics and registry projection can reuse one recorder contract outside the task-executor facade. |
| `task_executor.rs` | Hosts Pantograph-specific task execution for Python-backed nodes and RAG-backed nodes while preserving core-node fallthrough. |
| `technical_fit.rs` | Owns embedded-runtime technical-fit translation, including host-side runtime snapshot/candidate assembly, request projection into backend runtime-registry selector input, selector invocation, and decision projection back to workflow-service contracts without moving policy into adapters. |
| `python_runtime.rs` | Defines the out-of-process Python runtime adapter contract and the default process-backed implementation. |
| `python_runtime_bridge.py` | Bridge script executed by the Python adapter so Pantograph can invoke Python workers without linking Python in-process. |
| `rag.rs` | Defines the narrow RAG backend contract used by the host executor. |
| `runtime_capabilities.rs` | Owns backend-side mapping from producer-specific runtime facts into workflow runtime capabilities, including dedicated embedding and Python-sidecar capability builders plus capability-to-lifecycle projection. |
| `runtime_health.rs` | Owns backend-side health probe assessment, degraded/unhealthy threshold policy, and failure-count progression. |
| `runtime_recovery.rs` | Owns backend-side recovery restart planning, retry-strategy selection, retry backoff, backend port overrides, and dedicated-embedding restart policy. |
| `runtime_registry.rs` | Owns backend-side translation from gateway and producer lifecycle facts into shared runtime-registry observations, active/embedding health-aware unhealthy reconciliation, sync, reclaim, stop-all, and restore coordination. |
| `runtime_registry_observations.rs` | Owns backend-side runtime-registry observation builders and health-overlay matching for active, embedding, and execution-observed producer facts. |
| `workflow_runtime.rs` | Owns backend-side workflow execution helpers for embedding metadata flag projection, runtime trace/model-target shaping, runtime diagnostics projection, and execution-path runtime-registry override reconciliation. |

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
- Python-backed nodes execute through the runtime adapter boundary.
- Python runtime execution metadata and recorder state stay in backend Rust so
  workflow diagnostics and registry projection do not depend on Tauri-local or
  executor-local ad hoc payloads.
- Failed Python runtime executions may also carry a backend-owned unhealthy
  assessment so execution-observed producer failures converge on the shared
  runtime-registry `unhealthy` contract instead of remaining snapshot-only
  errors.
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
- Technical-fit bridging in this crate must stay a translation layer from
  workflow-service request context into runtime-registry selector input; factor
  scoring and final policy ownership remain outside this crate.
- Producer-specific runtime capability mapping must stay in backend Rust so
  adapters do not drift on runtime ids, install state, or backend-key aliases.
- Python-sidecar capability shaping must stay in the shared backend capability
  helper module rather than being rebuilt inside `EmbeddedWorkflowHost` or
  Tauri adapters.
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
- Execution-path runtime snapshot override reconciliation must stay in backend
  Rust so workflow adapters do not drift on when Python-sidecar or
  embedding-path execution facts become shared registry observations, and so
  execution-local snapshots do not erase a matching producer's existing
  backend-owned `unhealthy` registry state.
- Restore-time replay of stored non-live runtime snapshots into the shared
  registry must stay in backend Rust so diagnostics paths can rehydrate
  execution-observed producer state without overriding live gateway or
  embedding producer observations.
- Workflow-facing runtime-registry coordination failures must be mapped in
  backend Rust onto stable workflow-service error codes so adapters do not
  collapse admission or runtime-unavailable decisions into generic internal
  failures.
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
    None,
);
```

## API Consumer Contract
- Hosts create the runtime by supplying gateway, workflow-service, and shared
  extension dependencies; this directory does not own those outer application
  boot decisions.
- `EmbeddedRuntimeConfig` and `StandaloneRuntimeConfig` may carry an optional
  `max_loaded_sessions` limit so hosts can tune loaded-runtime residency
  without moving unload policy ownership out of backend Rust services.
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
