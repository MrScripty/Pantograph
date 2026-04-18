# crates/node-engine/src/engine

## Purpose
This directory contains focused helper modules behind the public
`node_engine::engine` facade. It exists to keep graph-mutation event helpers
and future multi-demand coordination logic out of the monolithic `engine.rs`
entrypoint while preserving the current public API.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `dependency_inputs.rs` | Dependency-output to node-input mapping helpers, including Puma-Lib model-path context propagation. |
| `execution_core.rs` | Private recursive demand-orchestration owner that coordinates dependency recursion, cache reuse, node preparation, event emission, and completed-output finalization. |
| `execution_events.rs` | Backend-owned task event emission helpers for started, waiting, and completed demand states. |
| `graph_events.rs` | Dirty-subgraph collection and incremental graph-event helpers. |
| `inflight_tracking.rs` | In-flight node bookkeeping helpers for cycle detection and cleanup around recursive demand. |
| `multi_demand.rs` | Current multi-demand execution helpers, including the executor-facing facade path, request-plan contract, result-merge contract, execution-budget contract, coordinator owner, and the future insertion point for bounded parallel coordination. |
| `node_preparation.rs` | Static node-data injection and human-input pause preparation for demand execution. |
| `output_cache.rs` | Fresh-cache resolution and completed-output cache/version finalization helpers. |
| `single_demand.rs` | Executor-facing single-target demand helper that keeps facade lock choreography out of `engine.rs`. |

## Problem
`engine.rs` owns both workflow execution and graph-mutation orchestration, and
it is already large enough that adding event-contract completion or bounded
parallel planning directly into the file would make later review and testing
harder. The helpers in this directory create narrower insertion points without
changing the public executor surface.

## Constraints
- `node_engine::engine` remains the stable public facade for callers.
- Graph-modification and incremental-run semantics stay backend-owned in Rust.
- Dependency-input assembly and model-context propagation stay backend-owned in
  Rust.
- Static node-data injection and human-input pause detection stay backend-owned
  in Rust.
- Cache hit resolution and completed-output version finalization stay
  backend-owned in Rust.
- Task-start, waiting-for-input, and task-completed demand events stay
  backend-owned in Rust.
- In-flight node bookkeeping for cycle detection and cleanup stays
  backend-owned in Rust.
- The remaining recursive node-demand orchestration should live under
  `engine/execution_core.rs` rather than growing back into `engine.rs`.
- Executor-facing single-demand and multi-demand lock choreography should live
  under `engine/` helpers rather than expanding `engine.rs`.
- Multi-demand request normalization should stay under `multi_demand.rs` so
  caller-visible target order and future execution scheduling remain separated.
- Multi-demand result aggregation should stay under `multi_demand.rs` so
  deterministic merge semantics remain explicit before concurrent completion
  paths are introduced.
- Multi-demand execution traversal should stay behind a private coordinator
  owner so future bounded scheduling does not have to reopen facade
  orchestration or graph-lock choreography.
- Multi-demand execution-budget semantics should stay behind a private contract
  so default behavior and future additive controls remain explicit.
- Multi-demand helpers must not change behavior until the dedicated parallel
  execution phase intentionally does so.
- `WorkflowExecutor::demand_multiple` should delegate into `multi_demand.rs`
  so later bounded-parallel coordination does not have to be inserted back
  into `engine.rs`.

## Decision
Extract graph-event and multi-demand helper logic into focused modules under
`engine/` while keeping `engine.rs` as the public facade and workflow-executor
entrypoint. This preserves compatibility while creating standards-compliant
boundaries for later event and concurrency work. Dependency-input mapping is
also extracted so future planners and coordinators do not need to own port
assembly details directly. The same applies to executor-facing single-demand
facade choreography, node preparation, output-cache lifecycle handling, task
event emission, and in-flight bookkeeping. The remaining recursive demand
orchestration now lives behind `execution_core.rs` so `engine.rs` can stay a
thin facade while Phase 2 introduces bounded parallel coordination later. The
same directory now also owns the private multi-demand request-plan contract so
future scheduling changes do not have to redefine the facade event payload.
The same applies to deterministic multi-demand result aggregation semantics.
The current sequential traversal now also lives behind a coordinator owner that
the future bounded scheduler can extend. The same directory now also owns the
current execution-budget contract.

## Alternatives Rejected
- Continuing to grow `engine.rs` directly.
  Rejected because the file already exceeds decomposition thresholds.
- Introducing a new public engine API immediately.
  Rejected because the current phase is a facade-first refactor, not an API
  redesign.

## Invariants
- Public callers continue to use `node_engine::engine` and `WorkflowExecutor`.
- Dependency-input mapping stays derived from backend graph state and upstream
  outputs rather than adapter-local preprocessing.
- Human-input pause detection remains driven by backend node type and input
  state rather than by adapter-local interpretation.
- Cache freshness and version bump semantics remain derived from backend-owned
  version tracking rather than adapter-local memoization.
- Demand event emission remains derived from backend execution state rather
  than adapter-local reconstruction.
- In-flight cycle detection remains derived from backend recursive execution
  state rather than adapter-local guards.
- Recursive node-demand orchestration remains backend-owned and private to
  `node-engine` rather than becoming a new public or binding-facing surface.
- Single-demand and multi-demand facade helpers remain behaviorally equivalent
  to the prior inline executor methods until the bounded parallel coordinator
  intentionally changes the multi-demand path.
- Multi-demand request planning currently preserves caller-visible requested
  target order while keeping execution-target planning as a separate backend
  concern.
- Multi-demand result aggregation currently preserves deterministic
  last-write-wins map semantics as a separate backend concern from execution
  traversal.
- Multi-demand traversal currently remains sequential even though it now runs
  through a dedicated coordinator owner.
- Multi-demand execution currently uses an explicit one-in-flight budget even
  though additive runtime controls have not landed yet.
- Graph-modification events remain derived from backend graph state, not from
  adapter-local inference.
- The current executor-facing and engine-facing multi-demand helpers remain
  behaviorally sequential until the bounded parallel coordinator lands
  intentionally.

## Revisit Triggers
- Bounded parallel demand execution requires additional planner or coordinator
- modules.
- Graph mutation semantics expand enough to warrant a dedicated event-owner
  type instead of helper functions.
- Dependency planning grows to include explicit plan objects or reusable
  topological layers.
- Node preparation grows enough to warrant a shared execution-preparation
  object across single and multi-demand coordination paths.
- Cache lifecycle handling grows enough to warrant a shared execution-state
  helper across sequential and future bounded-parallel demand paths.
- Demand event emission grows enough to warrant a shared execution-notification
  helper across sequential and future bounded-parallel demand paths.
- In-flight bookkeeping grows enough to warrant a shared coordination-state
  helper across sequential and future bounded-parallel demand paths.
- Recursive demand orchestration grows enough to warrant a more explicit
  planner/coordinator split than a single execution-core owner.
- Single-demand execution preparation grows enough to warrant a shared
  execution-preparation helper across single and multi-demand paths.
- Execution-state recovery or persistence introduces a different owner for
  multi-demand coordination.

## Dependencies
**Internal:** `crate::events`, `crate::types`, `crate::extensions`, and the
public `engine.rs` facade.

**External:** None beyond the crate’s existing async and serialization
dependencies.

## Related ADRs
- None identified as of 2026-04-17.
- Reason: This extraction preserves the current architecture boundary instead
  of introducing a new subsystem.
- Revisit trigger: A future engine planner/coordinator split changes package
  ownership or public API shape.

## Usage Examples
```rust
use node_engine::{TaskExecutor, WorkflowExecutor};
```

## API Consumer Contract
- External callers continue to interact through `WorkflowExecutor` and
  `DemandEngine`.
- The helper modules in this directory are internal implementation details and
  should not be imported directly by downstream crates.

## Structured Producer Contract
- Graph-modification and incremental-execution helpers emit the canonical
  backend-owned `WorkflowEvent` variants.
- Dependency-input assembly preserves existing model-path context propagation
  semantics for Puma-Lib-backed runtime selection.
- Node preparation preserves existing `_data` injection and human-input pause
  semantics for demand execution.
- Output-cache helpers preserve existing cache-hit reuse and completed-output
  version bump semantics for demand execution.
- Execution-event helpers preserve existing task-started, waiting, and
  task-completed semantics for demand execution.
- In-flight helpers preserve existing cycle-detection and cleanup semantics for
  recursive demand execution.
- Execution-core behavior preserves existing recursive dependency-demand,
  cache-check, node-preparation, and completion-finalization semantics for the
  current sequential demand path.
- Single-demand helper behavior remains semantically identical to the prior
  inline `WorkflowExecutor::demand` path.
- Dirty-task lists remain sorted and stable for consumer comparison and tests.
- Multi-demand helper behavior remains sequential until the Phase 2 bounded
  parallel coordinator intentionally changes that contract.
