# crates/node-engine/src/engine

## Purpose
This directory contains focused helper modules behind the public
`node_engine::engine` facade. It exists to keep graph-mutation event helpers
and future multi-demand coordination logic out of the monolithic `engine.rs`
entrypoint while preserving the current public API.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `graph_events.rs` | Dirty-subgraph collection and incremental graph-event helpers. |
| `multi_demand.rs` | Current multi-demand execution helpers, including the executor-facing facade path and the future insertion point for bounded parallel coordination. |

## Problem
`engine.rs` owns both workflow execution and graph-mutation orchestration, and
it is already large enough that adding event-contract completion or bounded
parallel planning directly into the file would make later review and testing
harder. The helpers in this directory create narrower insertion points without
changing the public executor surface.

## Constraints
- `node_engine::engine` remains the stable public facade for callers.
- Graph-modification and incremental-run semantics stay backend-owned in Rust.
- Multi-demand helpers must not change behavior until the dedicated parallel
  execution phase intentionally does so.
- `WorkflowExecutor::demand_multiple` should delegate into `multi_demand.rs`
  so later bounded-parallel coordination does not have to be inserted back
  into `engine.rs`.

## Decision
Extract graph-event and multi-demand helper logic into focused modules under
`engine/` while keeping `engine.rs` as the public facade and workflow-executor
entrypoint. This preserves compatibility while creating standards-compliant
boundaries for later event and concurrency work.

## Alternatives Rejected
- Continuing to grow `engine.rs` directly.
  Rejected because the file already exceeds decomposition thresholds.
- Introducing a new public engine API immediately.
  Rejected because the current phase is a facade-first refactor, not an API
  redesign.

## Invariants
- Public callers continue to use `node_engine::engine` and `WorkflowExecutor`.
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
- Dirty-task lists remain sorted and stable for consumer comparison and tests.
- Multi-demand helper behavior remains sequential until the Phase 2 bounded
  parallel coordinator intentionally changes that contract.
