# pantograph-scheduler

## Purpose
This crate owns Pantograph scheduler contracts for dynamic task dispatch:
queue state, scheduling policy boundaries, resource admission, dependency
readiness policy, batching, dispatch decisions, and scheduler lifecycle
ownership.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `src/lib.rs` | Public scheduler boundary contract version plus typed ownership enums for scheduler-owned capabilities and non-scheduler consumers. |

## Problem
Pantograph workflows can pause between ready DAG tasks while concurrent users,
batching, model residency, dependency readiness, and resource availability
change. Without a dedicated scheduler owner, policy can drift into graph
editing, node execution, frontend actions, Tauri commands, runtime adapters, or
diagnostics projections.

## Constraints
- The scheduler is the only owner of queue policy, runtime/device selection,
  resource admission, dependency-readiness action, batching, dispatch timing,
  retry/defer/fail decisions, and long-running scheduler lifecycle.
- Graph editor and node-engine remain path-free and may only submit typed
  intent or display backend-owned capability/task facts.
- Runtime hosts may consume short-lived dispatch decisions and resolve
  Pumas-approved load targets only at the runtime boundary.
- Frontend, Tauri, binding, and runtime-adapter crates may project or transport
  scheduler facts but must not own scheduler decisions.
- Core scheduler policy should remain synchronous unless a later slice records
  the I/O operation that requires an async shell.

## Decision
Create a focused scheduler crate before implementing dynamic task dispatch.
This makes scheduler ownership explicit in Cargo and gives future slices one
place to add task intent, queue state, dispatch decisions, resource observation,
batching, and lifecycle contracts without preserving legacy dependency
resolver behavior.

## Alternatives Rejected
- Put scheduler policy in `node-engine`: rejected because node-engine should
  validate graph semantics and submit task intent, not choose runtimes,
  resources, batching, or dependency actions.
- Put scheduler policy in `pantograph-embedded-runtime`: rejected because the
  embedded runtime is a composition/runtime host boundary, not the canonical
  policy owner.
- Put scheduler policy in Tauri/frontend adapters: rejected because adapters
  may transport or display scheduler state but must not become sources of
  backend execution semantics.

## Invariants
- `SchedulerOwnedCapability` entries are owned by this scheduler boundary.
- `SchedulerBoundaryConsumer` entries may consume scheduler facts but cannot
  own scheduler-owned capabilities.
- This crate must not inspect Pumas filesystem layout, join local model paths,
  execute workflow nodes, or launch runtimes.
- Legacy `ModelDependencyResolver`, `ModelDependencyRequest`, `ModelRefV2`,
  graph-visible `model_path`, and frontend `modelPath` paths are replacement
  targets, not compatibility contracts for this crate.

## Revisit Triggers
- A future implementation slice needs to put scheduler policy outside this
  crate.
- Runtime/resource observation cannot be represented behind a platform-neutral
  scheduler contract.
- Queue recovery or dispatch idempotency requires a persisted contract shape
  that does not fit this crate boundary.

## Dependencies
**Internal:** Future slices will consume contracts from dependency planning,
runtime identity/registry, diagnostics ledger, workflow service, and embedded
runtime host boundaries.

**External:** `serde` for scheduler DTO serialization.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```rust
use pantograph_scheduler::{
    owner_for_capability, SchedulerBoundaryOwner, SchedulerOwnedCapability,
};

assert_eq!(
    owner_for_capability(SchedulerOwnedCapability::DispatchDecision),
    SchedulerBoundaryOwner::Scheduler
);
```

## API Consumer Contract
- Inputs: scheduler-owned capability labels and non-scheduler consumer labels.
- Outputs: the canonical owner for scheduler capabilities and explicit
  deny-policy checks for non-scheduler consumers.
- Lifecycle: this crate currently starts no tasks and owns no runtime handles.
  Later scheduler services must add one lifecycle owner for queues, workers,
  cancellation, shutdown, and reservation cleanup.
- Errors: this first boundary slice has no fallible public API. Later boundary
  parsing and state transitions must return typed errors/diagnostics.
- Compatibility: `SCHEDULER_CONTRACT_VERSION` is the starting version for
  persisted or transported scheduler DTOs. Breaking persisted shape changes
  require migration planning.

## Structured Producer Contract
- Stable fields: `SCHEDULER_CONTRACT_VERSION`,
  `SchedulerOwnedCapability`, `SchedulerBoundaryConsumer`, and
  `SchedulerBoundaryOwner` enum semantics.
- Defaults: no default scheduler owner other than `Scheduler` exists.
- Enum semantics: owned capability variants identify decisions and state that
  belong to the scheduler boundary; consumer variants identify components that
  may consume scheduler facts but not own policy.
- Ordering: enum declaration order is not a runtime contract.
- Compatibility: new capability or consumer variants may be added as scheduler
  contracts expand; existing meanings must not be repurposed.
- Regeneration/migration: fixtures, bindings, adapters, and README content must
  update in the same slice when persisted scheduler DTOs are introduced or
  changed.
