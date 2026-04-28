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
| `policy_tests.rs` | Scheduler priority, FIFO, starvation-protection, warm-reuse bypass, runtime-capacity, and admission-wait tests extracted from the production policy module. |
| `store.rs` | In-memory scheduler session records, runtime-load state, runtime-unload candidate selection inputs, and stale-cleanup candidate logic. |
| `store_queue.rs` | Queue listing, enqueue/cancel/reprioritize/push-front, admission-input construction, queued-run admission, and active-run finish transitions. |
| `store_admission.rs` | Scheduler store admission ETA projection helper used by queue diagnostics. |
| `store_diagnostics.rs` | Scheduler snapshot diagnostics and runtime-diagnostics request projection helpers extracted from the store. |
| `store_tests.rs` | Scheduler store admission-input and warm-session compatibility tests extracted from the production store module. |

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
  push-front, cleanup, and runtime-load transitions do not split across modules.
- Edit-session scheduler snapshots stay outside this directory; they may consume
  the same DTOs, but graph-edit lifecycle remains owned by `graph/`.

## Decision
Create a focused `scheduler/` boundary inside `pantograph-workflow-service`.
`contracts.rs` freezes the workflow-facing scheduler DTOs, while `store.rs`
owns in-memory session state that `WorkflowService` delegates to.
`store_queue.rs` owns queue/run mutation and canonical admission-input
construction so run-id ownership and queue policy do not keep growing the
general session store. It also owns the run-id-to-session lookup used by
GUI-admin queued-run cancellation and priority override so privileged
transport callers do not scan or reinterpret scheduler internals.
`policy.rs` makes the current priority/FIFO queue behavior explicit and now
also owns the first starvation-protection promotion rule plus the first
runtime-affinity unload-ranking rule instead of leaving that behavior as ad hoc
branching inside the store. That unload-ranking path now consumes backend-owned
workflow id, `required_backends`, `required_models`, and `usage_profile`
affinity facts refreshed by the service before runtime loading, and it now
folds those signals into an explicit backend-owned compatibility identity
instead of treating backend/model lists as the only reusable-runtime hint.
Queue items and trace-facing projections now also carry backend-owned
admission-outcome semantics instead of forcing adapters to reverse-engineer queued versus
admitted state. Store-owned queue transitions now also construct a canonical
internal admission-input model for policy evaluation from backend session
state, loaded-runtime posture, and warm-session compatibility facts instead of
keeping those inputs implicit inside one mutation path, and admitted runs now
surface backend-owned warm-reuse versus reload versus cold-start reasons
instead of a generic execution label. Admission selection now also has one
bounded fairness override for warm reuse: inside the highest-priority,
non-starved band, a compatible warm candidate may bypass at most the next cold
candidate, but it still cannot jump starved or higher-priority work.
Scheduler snapshots now also expose additive backend-owned diagnostics for
loaded-session pressure, reclaimable runtime counts, next-admission
prediction, skipped queue-head visibility for fairness-driven bypasses, and
earliest-known admission wait bounds so Tauri and other adapters can forward
canonical scheduler facts without reconstructing queue policy client-side.
When loaded-session capacity is saturated by active runs with no reclaimable
idle runtime, the selected candidate now stays queued with the explicit
`waiting_for_runtime_capacity` reason instead of being admitted and then
failing immediately with a capacity error.
When backend runtime-registry admission would currently reject a session load,
the candidate now also stays queued with `waiting_for_runtime_admission`
instead of dequeuing into an immediate runtime-load failure.
`workflow.rs` remains the public
application-service facade and orchestration entrypoint, but it no longer
needs to be the long-term home for scheduler contracts or queue mutation logic.

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
- When the scheduler selects a reclaimable keep-alive session, it must still
  forward `CapacityRebalance` through the backend host unload boundary rather
  than creating a second checkpoint or restore policy path in scheduler code.
- Scheduler DTOs are machine-consumable contracts that adapters forward
  without reconstructing local scheduler truth.
- Queue insertion should move the constructed queued-run record directly into
  the store so scheduler state transitions do not accumulate redundant
  rebinding or hidden policy steps.
- Scheduler priority, FIFO, starvation-protection, warm-reuse bypass,
  runtime-capacity, and admission-wait tests stay in `policy_tests.rs` so
  `policy.rs` remains focused on production queue and admission decisions.
- Scheduler store admission-input and warm-session compatibility tests stay in
  `store_tests.rs` so `store.rs` remains focused on production queue/session
  state mutation.
- Scheduler store admission ETA projection stays in `store_admission.rs` so
  queue diagnostics timing helpers do not keep `store.rs` above the
  decomposition threshold.
- Scheduler queue mutation, admission-input construction, and active-run finish
  transitions stay in `store_queue.rs` so canonical `workflow_run_id`
  lifecycle ownership is isolated from runtime-load and stale-cleanup state.
- Cross-session queue lookup for GUI-admin controls must stay inside the
  scheduler store boundary; adapters may request a privileged action but must
  not search or mutate session queues directly.
- Scheduler snapshot diagnostics and runtime-diagnostics request shaping stay
  in `store_diagnostics.rs` so read-side scheduler projection does not keep
  `store.rs` above the large-file threshold.

## Revisit Triggers
- Scheduler V2 needs policy modules that justify splitting `store.rs` further.
- Queue state becomes durable or distributed instead of process-local.
- Edit-session scheduler semantics grow enough shared behavior to warrant a
  narrower shared contract module.

## Dependencies
**Internal:** workflow service session contracts, runtime readiness facts,
technical-fit override DTOs, and trace-facing scheduler projections.

**External:** `serde`, `uuid`, and standard async/runtime primitives inherited
through the parent crate.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
Scheduler APIs are reached through the workflow service facade:

```rust
let snapshot = service.workflow_session_scheduler_snapshot(session_id).await?;
```
