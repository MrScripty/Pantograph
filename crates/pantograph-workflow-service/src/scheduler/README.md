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
| `lifecycle.rs` | Workflow-service scheduler lifecycle component registry that owns required component presence and explicit coarse state before public snapshots or ledger events are exposed. |
| `policy.rs` | Explicit scheduler ordering policy objects, internal admission-input/decision models, and stable decision vocabulary for queue placement and admission. |
| `policy_tests.rs` | Scheduler priority, FIFO, starvation-protection, warm-reuse bypass, runtime-capacity, and admission-wait tests extracted from the production policy module. |
| `readiness_lifecycle.rs` | Workflow-service lifecycle owner that builds typed dependency-readiness requests for admitted runtime tasks, calls a readiness provider, and applies scheduler readiness admission without owning dependency policy. |
| `task_lifecycle.rs` | Workflow-service task lifecycle owner skeleton for active task handles, shutdown state, and typed lifecycle diagnostics before durable lease, cancellation, retry, replay, and ledger slices are wired. |
| `store.rs` | In-memory scheduler session records, runtime-load state, runtime-unload candidate selection inputs, and stale-cleanup candidate logic. |
| `store_queue.rs` | Queue listing, enqueue/cancel/reprioritize/push-front, admission-input construction, queued-run admission, active-run scheduler task-state transition storage, and active-run finish transitions. |
| `store_task_results.rs` | Staged active-run scheduler task-result storage used by Milestone 5c before durable diagnostics-ledger replay replaces the storage backend. |
| `store_admission.rs` | Scheduler store admission ETA projection helper used by queue diagnostics. |
| `store_diagnostics.rs` | Scheduler snapshot diagnostics and runtime-diagnostics request projection helpers extracted from the store. |
| `task_orchestrator.rs` | Application-layer scheduler task orchestrator for source-input materialization, non-runtime node execution, scheduler dispatch-selection handoff, and runtime-host dispatch calls. |
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
GUI-admin queued-run cancellation, priority override, and push-front controls
so privileged transport callers do not scan or reinterpret scheduler
internals.
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
Active-run scheduler task records now use `pantograph-scheduler` queue
records and transition application for the Milestone 5c task-orchestration
bridge. The store only persists validated task state for the admitted run; it
does not choose task policy, execute nodes, or synthesize runtime handoff.
Active-run scheduler task results are staged in a separate focused store
module. They persist validated, path-free workflow task outputs for the
admitted run only until the durable diagnostics-ledger replay slice replaces
that storage; they are not a second scheduler policy model or runtime launch
path.
Request-provided source inputs complete through the same focused store module
with an atomic result-plus-task-state operation from `AwaitingInputs` to
`Completed`. That operation validates the immutable task graph class,
source-input template, source-input transition intent, and task-result
correlation; it does not fake a node-engine running state or execute source
inputs through runtime/non-runtime adapters.
The scheduler task orchestrator is the only caller that converts run-request
inputs into those source-input materialization transitions. Session execution
must consume that orchestrator method instead of duplicating source-input
transition construction or writing source values directly into task results.
Non-runtime-only session runs now advance through the scheduler task loop:
source inputs materialize into task results, dependent non-runtime tasks become
ready from those results, the node-engine single-task adapter executes only
typed non-runtime templates, and requested outputs are projected from
scheduler task results. This path bypasses runtime admission/load and the
legacy whole-workflow host run.
The scheduler task orchestrator can now consume a validated
`SchedulerDispatchSelectionRequest`, call the `pantograph-scheduler` selector,
build a dispatch-selected `SchedulerRuntimeHandoff`, and dispatch through the
shared runtime-host port. It does not create dispatch candidates, rank runtime
policy, inspect model paths, or call Pumas; candidate assembly remains a
separate provider/session concern.
It also owns the runtime task persistence helpers: ready runtime tasks can be
moved to `Running`, and terminal runtime-host results can be atomically stored
with the `Completed` transition through the same task-result store used by
non-runtime tasks. Session execution uses these helpers only after dependency
readiness admission and scheduler dispatch selection have produced a
dispatch-selected runtime-host handoff; default no-candidate production wiring
still fails closed before runtime-host dispatch.
Session execution now reaches that dispatch-selection boundary for admitted
runtime tasks. Workflow-service carries the readiness proof produced during
admission into dispatch-selection request assembly and asks the configured
runtime dispatch candidate provider for canonical candidates. The default
provider returns an empty candidate list, so the scheduler selector returns a
typed no-selection diagnostic before runtime-host dispatch instead of using
legacy graph paths, node-engine launch, or reduced execution-plan handoff.
The scheduler task orchestrator preserves those dispatch-selection diagnostics
on the runtime task's terminal failure state when no candidate is selected, so
run inspection can show scheduler-owned reasons such as missing candidates
without inventing fallback runtime choices.
When a canonical provider is configured, runtime tasks can complete through
the shared runtime-host execution port and persist typed scheduler task
results, but candidate assembly remains outside the scheduler task
orchestrator and must not be synthesized from graph/editor state or legacy
runtime contracts.
The scheduler task orchestrator now also emits reservation lifecycle events
around dispatch selection and runtime-host dispatch through the shared
runtime-host contracts port. Workflow-service owns event emission only; the
default lifecycle port fails closed before runtime-host dispatch, and
production reservation release/retention side effects remain embedded-runtime
or runtime-registry responsibilities.
The scheduler task orchestrator also owns the workflow-service bridge from
`WaitingDependencyReadiness` into scheduler readiness admission. It consumes
only a path-free `DependencyPreflightResult`, applies
`pantograph-scheduler` readiness policy, and persists the resulting validated
task-state transition; it must not resolve dependencies through node-engine,
legacy preflight, Tauri, or graph-authored paths.
The scheduler lifecycle registry owns the required component vocabulary for
future worker diagnostics: queue worker, dependency-readiness action, resource
observation loop, runtime-host dispatch, retry loop, and reservation cleanup.
Each component starts from an explicit `NotStarted` owner record. Public
diagnostics snapshots and ledger events must not invent missing component
facts; later slices must attach real owners to these registry records before
exposing them outside workflow-service.
The registry is available through a cloneable workflow-service handle so
component owners can share one lifecycle state core without depending on the
task lifecycle manager as an unrelated owner. Runtime-host dispatch currently
uses that handle first; future dependency-readiness, resource observation,
retry, queue, and reservation cleanup owners must use the same pattern.
The task lifecycle manager now owns the first concrete component attachment:
runtime-host dispatch state changes from explicit `NotStarted` to `Running`
only when a runtime task supervisor abort handle is tracked. Non-runtime task
handles do not mark runtime-host dispatch as running. Shutdown transitions move
that same component to `ShuttingDown` and `Shutdown` through the task lifecycle
owner, still without public snapshot projection or ledger writes.
The dependency-readiness lifecycle is the workflow-service owner for producing
the readiness request that precedes that bridge. It reads the admitted active
run scheduler task graph, constructs a validated
`DependencyReadinessRequestEnvelope` from scheduler intent, Pumas model
reference, and the task graph's saved dependency-readiness source, calls an
injected provider, validates provider preflight output against the envelope,
and then delegates task-state admission to the orchestrator. This lifecycle
currently owns no background tasks; if future provider I/O becomes
asynchronous, spawned work, cancellation, retries, shutdown, and tracing must
be owned by this module rather than by Tauri, frontend code, node-engine, or
runtime adapters.
The task lifecycle manager is the workflow-service owner for task handle and
shutdown state. It is synchronous and does not spawn work, retry, replay,
write diagnostics-ledger facts, dispatch runtime-host work, or change
scheduler task-state policy. The scheduler task orchestrator now claims a
lifecycle handle before applying a running attempt transition and releases
that handle only after the matching terminal store mutation succeeds, so
overlapping runner calls cannot bypass the shared lifecycle owner. Later
slices must compose cancellation tokens, retry/defer policy,
replay/bootstrap, and attempt/timing ledger facts through this owner instead
of distributing lifecycle behavior across the session runner, Tauri, frontend
code, runtime adapters, or diagnostics projections.
The first concrete lifecycle provider adapter is the canonical
`DependencyEnvironmentService` facade. It converts the readiness request into a
path-free dependency-environment resolve request, validates provider output,
projects it through `pantograph-dependency-planning` into
`DependencyReadinessProofEnvelope` before scheduler admission, and never
synthesizes proof from graph node data, technical-fit preview facts, reduced
execution-plan projections, or frontend/Tauri display state.
Runtime inference tasks that depend on upstream task outputs initialize as
`AwaitingInputs` even when their schedulable intent is otherwise complete.
They may become dispatch candidates only after scheduler-owned input-readiness
logic materializes the connected upstream results.
Runtime inference tasks with complete graph inputs and a schedulable intent now
initialize as `WaitingDependencyReadiness`, not `Ready`, until
scheduler-owned dependency readiness proof is admitted. Workflow-service must
not treat `SchedulableTaskIntent` alone as executable runtime authority or
bridge it back into dependency preflight/model-path contracts.
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
- Scheduler store active/queued workflow-run id accessors are local scheduler
  facts for the Network page. They expose placement only and must not be used
  to imply model, runtime, or cache residency.
- Local run-placement records may include session runtime posture and required
  backend/model facts from scheduler state. They also include typed scheduler
  model-cache posture for known not-required or unknown states. They remain
  scheduler observations, not proof that a model is loaded in a runtime cache.
- Queue insertion should move the constructed queued-run record directly into
  the store so scheduler state transitions do not accumulate redundant
  rebinding or hidden policy steps.
- Scheduler store methods must not write durable diagnostics or call runtime
  hosts while holding mutable store state. Store mutations return or expose
  immutable decision facts; workflow-service emitters append typed diagnostics
  events after scheduler/session guards have been released.
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
- Active-run scheduler task records must validate through
  `pantograph-scheduler` queue contracts before storage. Workflow-service
  must not define a second task-state transition table or bypass scheduler
  transition validation.
- Runtime scheduler tasks whose dispatch-selected runtime-host handoff is not
  wired must fail closed by applying a scheduler-validated terminal task-state
  transition with typed diagnostics. They must not enter runtime admission,
  runtime preflight/load, node-engine output demand, or the legacy whole-run
  host execution path as a compatibility route.
- Runtime scheduler tasks may reach runtime-host execution only after
  `pantograph-scheduler` returns a selected dispatch decision. A no-selection
  dispatch-selection result stops before runtime-host dispatch and preserves
  scheduler diagnostics as the reason the task did not run.
- Runtime inference tasks with upstream dependencies must not initialize as
  `Ready`; they remain `AwaitingInputs` until scheduler task-result materialized
  inputs can prove dispatch inputs are available.
- Runtime inference tasks without pending graph inputs must not initialize as
  `Ready` solely because a `SchedulableTaskIntent` exists; they remain
  `WaitingDependencyReadiness` until scheduler admission consumes the canonical
  dependency readiness proof and can produce a dispatch-selected runtime
  handoff.
- Dependency readiness admission for runtime tasks must consume
  `DependencyReadinessProofEnvelope` through `pantograph-scheduler` and persist
  only scheduler task-state transitions. It must not call
  `ModelDependencyRequest`, build `ModelRefV2`, read graph `modelPath`, or
  synthesize executable load targets.
- Dependency readiness request construction for admitted runtime tasks must
  stay in `readiness_lifecycle.rs`. It may project scheduler intent into the
  shared `DependencyReadinessRequestEnvelope` using the saved
  `dependency_readiness_source`, but it must not become a Pumas load-target
  resolver, dependency installer, runtime-host dispatcher, or graph-validation
  preview producer.
- Dependency readiness provider resolution must run outside session-store
  locks. The lifecycle builds a validated request from active scheduler state,
  releases store ownership while the configured provider resolves path-free
  readiness evidence, then reacquires the store only to apply
  scheduler-validated admission. The default provider is the no-I/O
  not-implemented dependency-environment service, which keeps session runner
  runtime tasks fail-closed before dispatch until production readiness evidence
  is wired.
- Active-run scheduler task results must validate through
  `WorkflowSchedulerTaskResult` before storage. The store may index staged
  results by task id for the active run, but it must not store executable
  paths, runtime handoff, Pumas load targets, or raw node-engine internals.
- Dequeued-run records carry the scheduler session's required backend/model
  facts at admission time so workflow-service diagnostics can emit selected
  runtime and reserved model audit fields without rereading mutable graph
  files.
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
