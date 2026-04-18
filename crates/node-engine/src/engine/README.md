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
| `multi_demand.rs` | Current multi-demand execution helpers, including the executor-facing facade path, request-plan contract, root-target planning, execution-batch schedule, result-merge contract, execution-budget contract, coordinator owner, and the future insertion point for bounded parallel coordination. |
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
- Multi-demand planning should keep caller-visible requested targets separate
  from minimal root execution targets so later bounded scheduling can prune
  redundant top-level drives without changing the facade contract.
- Multi-demand schedule construction should stay private to `multi_demand.rs`
  so the coordinator consumes an explicit batch shape before real parallelism
  is introduced.
- Multi-demand batch planning should separate roots whose transitive
  dependency closures overlap while allowing independent roots to share a
  batch, so the current parallel-eligibility rule remains backend-owned.
- Multi-demand batch execution should keep failure cleanup semantics explicit so
  later bounded execution cannot accidentally continue into later batches after
  an earlier batch fails.
- Multi-demand batch execution should treat waiting-for-input as the same kind
  of stop boundary as failure for later scheduled batches until a future phase
  intentionally changes that contract.
- Multi-demand batch execution should apply the explicit execution budget
  through a private dispatch plan so later bounded concurrency widens an
  existing backend-owned dispatch contract instead of inventing one.
- Multi-demand dispatch windows should own their own completion and
  interruption outcomes so later concurrent execution preserves the same stop
  semantics as the current sequential runner.
- Multi-demand dispatch windows should reach `DemandEngine` through a private
  runner owner so later concurrent execution can reshape engine access without
  reworking planning or outcome contracts.
- Multi-demand dispatch windows should carry an explicit parallel-eligibility
  decision so future concurrent execution branches from a backend-owned
  contract instead of recomputing eligibility ad hoc.
- Multi-demand dispatch windows should expose that decision through an explicit
  execution-mode contract so later coordinator work branches from a typed
  backend-owned value rather than a boolean flag.
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
current execution-budget contract and the requested-target versus root-target
planning split, plus the private execution-batch schedule derived from it.

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
- Multi-demand planning currently preserves requested-target order for facade
  events while pruning redundant top-level execution targets that are already
  covered by other requested dependents.
- Multi-demand scheduling currently runs one sequential batch even though the
  coordinator now consumes an explicit schedule structure.
- Multi-demand batch planning currently groups independent roots together and
  separates roots with shared transitive dependencies into later sequential
  batches.
- Multi-demand batch execution currently stops after the first failed batch and
  does not continue into later scheduled batches.
- Multi-demand batch execution currently also stops later batches when an
  earlier batch pauses for interactive input.
- Multi-demand batch execution currently dispatches budget-sized windows
  sequentially within each overlap-safe batch.
- Multi-demand dispatch windows currently execute sequentially, but they now
  report completion and interruption through explicit backend-owned outcome
  types.
- Multi-demand dispatch windows currently also route demand/cache access
  through a private backend-owned window runner.
- Multi-demand dispatch windows now also record whether they are eligible for
  bounded concurrent execution under the current overlap and budget rules.
- Multi-demand dispatch windows now also expose that decision as an explicit
  execution mode rather than a bare boolean.
- Private multi-demand entry points now also thread an explicit execution
  budget through coordinator ownership rather than hard-coding a sequential
  path.
- Parallel-eligible dispatch windows now also run through isolated engine
  clones that reconcile changed state back into the main engine rather than
  sharing mutable engine state directly.
- Parallel-eligible dispatch windows now also execute their isolated target
  runs concurrently in place and reconcile the finished runs back in
  deterministic target order.
- Public multi-demand entry points now also default to a conservative bounded
  budget of two in-flight targets instead of hard-coding sequential execution.
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
