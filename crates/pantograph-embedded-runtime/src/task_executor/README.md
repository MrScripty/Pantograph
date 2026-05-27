# task_executor

## Purpose

This directory contains behavior modules for the Pantograph host task executor
facade in `../task_executor.rs`. The parent module owns the public executor
type, extension keys, construction, and node-type dispatch, while these modules
hold execution families that need host resources.

## Contents

| File | Description |
| ---- | ----------- |
| `dependency_environment.rs` | Dependency preflight gates for Python-backed runtime nodes and model-ref resolution. |
| `dependency_environment/` | Helper modules for dependency preflight input projection and stable runtime environment keys. |
| `puma_lib.rs` | Puma-Lib selected-model lookup through explicit selector-access roles, selected-detail inference-settings refresh, optional owner full-package-facts enrichment, execution descriptor projection, metadata normalization, and model-path output preparation. |
| `python_execution.rs` | Python runtime input normalization, runtime instance metadata, adapter invocation, failure health recording, and stream replay. |
| `rag_search.rs` | RAG search execution against the host-provided RAG backend. |

## Problem

The host task executor coordinates several unrelated host behaviors: RAG search,
Puma-Lib metadata projection, dependency preflight, and Python sidecar runtime
execution. Keeping those responsibilities in one impl made
`task_executor.rs` exceed the large-file threshold and made it harder to review
changes to one execution family without touching the rest.

## Constraints

- `TauriTaskExecutor` remains the exported host executor facade.
- Core node fallthrough behavior stays in the parent `TaskExecutor`
  implementation.
- Dependency preflight remains backend-owned and must not move into Tauri or
  frontend code.
- Python runtime execution continues through the adapter boundary and remains
  out-of-process.

## Decision

Keep dispatch and construction in `../task_executor.rs`, then split helper
methods by node family. Each module adds an impl block for `TauriTaskExecutor`
with restricted visibility so tests and the parent dispatcher can exercise the
same behavior without exposing helper paths outside this module boundary.

## Alternatives Rejected

- Leaving all helper methods in `task_executor.rs`: rejected because the file
  exceeded the large-file threshold and mixed unrelated execution families.
- Making each execution family a public executor type: rejected because host
  callers need one composite fallthrough-aware executor, not several public
  task-executor entrypoints.

## Invariants

- Parent dispatch remains the only place that maps node types onto host
  execution families.
- Parent dispatch must not route retired direct inference shapes such as
  `diffusion-inference` into Python sidecar execution. Canonical image
  generation enters through inference task metadata, not this host Python
  bridge.
- Dependency environment actions are not executable tasks in embedded-runtime.
  Workflow-service owns action intent validation and calls the dependency
  environment service with canonical dependency-planning DTOs.
- Dependency preflight request construction may carry Pumas model/task
  evidence, but canonical Python-backed execution must not derive executable
  backend selection from explicit `backend_key` fields, package hints,
  dependency requirements, or node-type defaults. Runtime selection for
  canonical inference belongs to the scheduler/admission path.
- Python runtime execution strips legacy backend-key inputs before adapter
  dispatch and derives lifecycle runtime identity from the resolved model ref
  or node family, not graph-authored backend hints.
- Python execution helpers may normalize runtime inputs and record health facts,
  but dependency gating must remain in dependency preflight helpers.
- Puma-Lib helpers prepare model metadata outputs and must not own dependency
  installation decisions.
- Puma-Lib selected `model_id` refresh must use `PUMAS_SELECTOR_ACCESS`.
  Read-only selector rows may rebind executable path/backend/task metadata, but
  only owner `PumasApi` access may enrich outputs with full package facts.
- Legacy Puma-Lib nodes that persisted only a library-owned
  `shared-resources/models/...` path may recover the relative Pumas `model_id`
  from that path before selected-detail hydration. This is a compatibility
  bridge to current Pumas facts, not a second model index.
- Owner and local-client selected-detail hydration may replace saved
  `inference_settings` with Pumas batch settings when they are non-empty.
  Read-only selector access must leave saved settings intact because selector
  rows do not own rich inference-setting detail.
- Stream artifact helpers may emit pass-through ArtifactStore metadata, but
  they must leave managed conversion status and dependency lease attribution
  empty unless a host-owned conversion executor supplies those typed facts.

## Usage Examples

Callers should continue constructing the parent executor:

```rust
use pantograph_embedded_runtime::task_executor::TauriTaskExecutor;

let executor = TauriTaskExecutor::new(Default::default());
```

Tests under `task_executor_tests.rs` may continue to cover helper behavior via
`TauriTaskExecutor` associated methods re-exported through the parent module.

## Revisit Triggers

- A new host node family adds a distinct execution lifecycle.
- Dependency preflight needs a public backend API outside task execution.
- Python runtime execution moves to a different adapter or recorder contract.

## Dependencies

**Internal:** `python_runtime`, `python_runtime_execution`, `rag`,
`runtime_health`, node-engine dependency contracts.

**External:** `pumas_library`, `serde_json`, `chrono`, `dirs`.

## Related ADRs

- [../../../../../docs/standards-compliance-analysis/refactor-plan.md](../../../../../docs/standards-compliance-analysis/refactor-plan.md)
