# crates/pantograph-workflow-service/src/scheduler

## Purpose
This directory contains the backend-owned workflow session scheduler boundary
for Pantograph. It owns scheduler-facing DTOs, queue/session state, admission
ordering primitives, keep-alive cleanup contracts, and the in-memory store used
by `WorkflowService` so adapters do not become queue-policy owners.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | Internal module entrypoint that re-exports scheduler contracts and store helpers to the workflow facade. |
| `contracts.rs` | Scheduler request/response DTOs, queue item contracts, keep-alive/unload semantics, and stale-cleanup worker types. |
| `policy.rs` | Explicit scheduler ordering policy objects, internal admission-input/decision models, and stable decision vocabulary for queue placement and admission. |
| `store.rs` | In-memory scheduler state, queue ordering, canonical admission-input construction, runtime-unload candidate selection inputs, and stale-cleanup candidate logic. |

## Problem
Pantograph previously kept workflow session scheduler contracts and queue/store
logic embedded directly in `workflow.rs`. That made the workflow facade too
large and left Scheduler V2 without a dedicated backend module boundary for
future fairness, affinity, and diagnostics policy.

## Constraints
- Scheduler state must remain backend-owned in Rust and free of Tauri or other
  transport-framework dependencies.
- Public workflow-service contracts remain facade-first and additive.
- Queue/session state needs one mutable owner so cancellation, reprioritizing,
  cleanup, and runtime-load transitions do not split across modules.
- Edit-session scheduler snapshots stay outside this directory; they may consume
  the same DTOs, but graph-edit lifecycle remains owned by `graph/`.

## Decision
Create a focused `scheduler/` boundary inside `pantograph-workflow-service`.
`contracts.rs` freezes the workflow-facing scheduler DTOs, while `store.rs`
owns the in-memory queue and session state that `WorkflowService` delegates to.
`policy.rs` makes the current priority/FIFO queue behavior explicit and now
also owns the first starvation-protection promotion rule plus the first
runtime-affinity unload-ranking rule instead of leaving that behavior as ad hoc
branching inside the store. That unload-ranking path now consumes backend-owned
workflow id, `required_backends`, `required_models`, and `usage_profile`
affinity facts refreshed by the service before runtime loading. Queue items and
trace-facing projections now also carry backend-owned admission-outcome
semantics instead of forcing adapters to reverse-engineer queued versus
admitted state. Store-owned queue transitions now also construct a canonical
internal admission-input model for policy evaluation from backend session
state, loaded-runtime posture, and warm-session compatibility facts instead of
keeping those inputs implicit inside one mutation path. `workflow.rs` remains the public
application-service facade and orchestration entrypoint, but it no longer
needs to be the long-term home for scheduler contracts or queue mutation
logic.

## Alternatives Rejected
- Leave scheduler logic in `workflow.rs`.
  Rejected because the file already exceeds decomposition thresholds and would
  keep growing as Scheduler V2 policy lands.
- Move scheduler ownership into Tauri or runtime adapters.
  Rejected because queue truth and scheduler policy belong in the backend
  workflow service, not transport layers.

## Invariants
- `WorkflowSessionStore` is the canonical owner of mutable workflow-session
  queue state.
- Runtime unload/reclaim decisions consume scheduler facts from this directory,
  but runtime-registry policy remains outside this boundary.
- Scheduler DTOs are machine-consumable contracts that adapters forward
  without reconstructing local scheduler truth.

## Revisit Triggers
- Scheduler V2 needs policy modules that justify splitting `store.rs` further.
- Queue state becomes durable or distributed instead of process-local.
- Edit-session scheduler semantics grow enough shared behavior to warrant a
  narrower shared contract module.
