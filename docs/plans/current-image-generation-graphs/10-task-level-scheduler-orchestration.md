# Task-Level Scheduler Orchestration

## Objective

Plan option 4 as the target execution architecture. Pantograph workflow runs
are durable DAG runs whose ready nodes become scheduler-owned task units. The
scheduler owns when each task runs, pauses, batches, retries, defers, or fails,
and runtime-host execution is one task transition selected by scheduler policy.

This replaces the remaining whole-workflow execution assumption. A workflow run
must not be treated as one uninterrupted object once multiple users, batching,
multi-model workflows, CPU plus multi-GPU/NPU execution, and runtime residency
are in scope.

## Current Gap

The current session execution path still admits a whole workflow run, builds a
reduced `WorkflowExecutionPlan`, stores that projection on the active run, then
asks node-engine to demand output nodes. Runtime inference can still be reached
through node-engine planned-inference launch ownership.

That shape cannot safely wire `RuntimeHostExecutionRequest` because the actual
dispatch-selected `SchedulerRuntimeHandoff` is not produced or stored at the
task boundary. Building a handoff from `WorkflowExecutionPlanNodeDecision`
would make the reduced diagnostics projection executable again, which violates
the no-fallback/no-legacy rule.

## Target Architecture

Canonical option 4 flow:

```text
workflow run submitted
  -> graph/topology validation
  -> path-free scheduler task graph
  -> durable scheduler task state
  -> ready task admission
  -> dependency/resource/batch/runtime policy
  -> dispatch-selected SchedulerRuntimeHandoff
  -> runtime-host or non-runtime task execution
  -> persisted task result and diagnostics
  -> dependent task unblocking
```

The scheduler sees tasks, not only whole workflow runs. It may admit one task
from a workflow, pause that workflow while resources are used elsewhere, batch
compatible tasks across runs, and resume dependent tasks later when their
inputs are ready.

2026-05-29 re-plan refinement: task-level orchestration must carry the
canonical `DependencyReadinessProofEnvelope` and runtime-host handoff through
the remaining Milestone 5b legacy-retirement path. The scheduler task state
record is the only place that can carry readiness and dispatch evidence
forward to runtime-host execution. Workflow-service may materialize node inputs
from completed upstream task results, but it must not derive runtime authority
from reduced execution plans, graph-authored paths, Tauri payloads, or old
dependency preflight outputs.

## Ownership Boundaries

- **Workflow-service application layer:** owns the use case orchestration that
  submits workflow runs, derives scheduler task graphs, persists task state,
  invokes scheduler policy, records diagnostics, and exposes user-visible run
  status.
- **Pantograph scheduler crate:** owns validated scheduler contracts and pure
  queue, readiness, resource, batching, dispatch, and lifecycle policy. Core
  policy stays synchronous unless a specific I/O operation is planned.
- **Scheduler task orchestrator:** owns the async shell around scheduler
  policy: dependency readiness calls, runtime-host dispatch calls, ledger
  writes, cancellation, retry timing, and bounded worker lifecycle.
- **Node-engine:** validates graph semantics and can execute non-runtime node
  tasks from materialized scheduler-owned inputs. It does not launch runtime
  inference, resolve dependencies, choose runtimes/devices, or inspect Pumas
  load targets.
- **Runtime host:** consumes only dispatch-selected runtime-host execution
  requests, resolves Pumas-approved load targets at the host boundary, and
  executes runtime-specific work.
- **Runtime-host contract crate:** owns the shared runtime-host execution DTOs,
  validated request/response wrappers, execution port trait, dispatcher, and
  typed port/dispatch errors. This crate must sit below both
  `pantograph-workflow-service` and `pantograph-embedded-runtime` so
  workflow-service can orchestrate runtime tasks while embedded-runtime
  implements the port without a dependency cycle.
- **Graph editor/frontend:** displays task/capability/diagnostic state and
  submits optional typed user constraints. It does not rank candidates, resolve
  paths, join Pumas storage, or optimistically mark backend-owned task state.
- **Diagnostics/history ledger:** records typed task state, dispatch, timing,
  resource, batching, dependency, and runtime facts for users and future
  scheduler policy.

## Contracts And State To Add

- `WorkflowSchedulerTaskGraph`: path-free run-scoped DAG task projection with
  workflow/run/node/task correlation, dependency edges, task kind, model ref,
  typed trait settings, optional hard runtime/device constraints, and bounded
  estimate hints. It remains the immutable workflow-service task definition
  owner for dependency edges, input bindings, and intent templates; scheduler
  lifecycle state must not duplicate graph bindings or become a second graph
  definition source.
- `SchedulerTaskStateRecord` and `SchedulerTaskStateTransition`: scheduler-owned
  durable lifecycle contracts that replace the current intent-required
  `SchedulerQueueTaskRecord` and `SchedulerQueueTransition` shapes. The record
  carries workflow/run/node/task correlation, state version, transition
  correlation, bounded diagnostics, and a state-specific payload. Pre-intent
  states such as awaiting materialized inputs, invalid graph projection, or
  input-unavailable must not carry executable intent. Executable states carry a
  typed execution intent payload; runtime execution states carry a validated
  `SchedulableTaskIntent`, while non-runtime execution states carry a narrower
  non-runtime task intent that cannot be used for runtime handoff, resource
  placement, Pumas load-target resolution, or model execution.
- `SchedulerTaskExecutionIntent`: typed execution payload for states such as
  ready, running, paused/deferred, retryable failed, and completed. The runtime
  variant wraps `SchedulableTaskIntent` and is the only variant accepted by
  readiness, resource, batching, runtime selection, and handoff policy. The
  non-runtime variant carries workflow/run/node/task correlation plus a
  validated non-runtime task kind for the workflow-service node-engine adapter;
  it must not carry graph bindings, local paths, Pumas load targets, runtime
  ids, device placement, backend decisions, or reduced execution-plan facts.
- `WorkflowSchedulerTaskResult`: typed task completion value that stores
  output references, media/artifact refs, scalar values, and diagnostics
  without executable paths. The first implementation stage uses active-run
  storage, but the DTO must be durable-ledger-ready: stable schema version,
  workflow/run/node/task correlation, typed value variants, invalid and
  unavailable states, bounded diagnostics, no raw path metadata, and no
  runtime-host launch details.
- `SchedulerTaskExecutionPort`: application-owned async port for executing one
  admitted task. Runtime inference variants call the runtime-host dispatch
  port; non-runtime variants call a narrow node-engine task execution adapter.
- `DependencyReadinessProofEnvelope`: scheduler/workflow-service-owned readiness
  evidence for runtime tasks. It is produced after backend validation and
  dependency-planning facts are available, consumed before queue admission and
  again before dispatch, and carried into runtime-host requests only as typed
  readiness evidence. It includes workflow/run/node/task correlation,
  descriptor fingerprint and freshness, selected runtime/device constraints,
  Pumas model/artifact identity, selected dependency environment identity when
  applicable, dependency availability status, proof status, and bounded typed
  diagnostics. It does not contain executable paths, Pumas storage paths,
  graph-visible `model_path`, worker launch metadata, or frontend/Tauri
  inferred state.
- `RuntimeHostExecutionRequest` / `RuntimeHostExecutionResponse` /
  `RuntimeHostExecutionPort`: shared runtime-host execution contracts moved
  out of `pantograph-embedded-runtime` into a lower-level contract crate. The
  request must carry a dispatch-selected `SchedulerRuntimeHandoff` plus typed
  materialized runtime inputs assembled from the canonical inference interface
  descriptor, completed upstream task results, and the current
  `DependencyReadinessProofEnvelope`. It must reject readiness-only handoffs,
  missing or stale readiness proof, reduced execution-plan projections, graph
  paths, local Pumas load targets, scheduler-owned execution payloads, and
  worker launch metadata.
- `SchedulerTaskStateReadModel`: backend-owned status for graph editor,
  run-inspection, and diagnostics views. It exposes waiting/running/failed
  facts, not scheduler internals or executable load targets.

The names are planning names. Implementation may choose shorter local names,
but the ownership and data boundaries must remain explicit.

2026-05-31 production-composed image session boundary: resource-backed runtime
dispatch now depends on explicit scheduler resource claims derived from
`SchedulerEstimateHint` values. Current executable validation snapshot
projection stores empty estimate hints for inference nodes, so a
production-composed image session cannot dispatch without either explicit
estimate facts or a typed fail-closed diagnostic. Do not treat missing
estimates as zero-resource reservations. Before the full production-composed
image session path is implemented, re-plan whether estimate hints are produced
from descriptor/Pumas/runtime facts during validation, whether dispatch blocks
with a resource-estimates-missing diagnostic until Milestone 6 facts exist, or
whether an explicit authored resource constraint contract is added.

2026-05-31 decision update: use conservative resource estimate production as
the target path, with an explicit typed missing-estimates diagnostic as the
next thin fail-closed checkpoint. Required claims must be produced before
scheduler dispatch from backend-owned facts: Pumas/model load memory,
descriptor/task execution shape, runtime/device capability facts, and selected
constraints. The estimates must distinguish model/load memory from
execution/context memory so an already-resident model can avoid a full reload
claim while still reserving incremental prompt/context, batch, dimension, KV,
or temporary-buffer resources. Runtime registry remains the owner of available
resource state, active reservations, residency, and reservation release. The
scheduler may reconsider waiting/deferred tasks after reservations are released
when running work completes, fails, or is cancelled, but it must not assume
future release as current capacity. Same-model reuse is allowed only when
registry/candidate facts explicitly prove compatible residency; missing
residency or missing estimate facts must block with typed diagnostics, not
fall back to zero-resource execution.

## Task Definition And Task State Replan

The current `pantograph-scheduler` queue contracts require a complete
`SchedulableTaskIntent` on every durable task record and transition. That no
longer matches the task-level scheduler architecture: a workflow task can
exist, be visible to users, and be blocked on upstream materialized outputs
before it can become a valid scheduler intent.

The chosen replacement keeps two separate truths:

- `WorkflowSchedulerTaskGraph` is immutable run-scoped task definition owned
  by workflow-service. It carries topology, dependency edges, input bindings,
  and `WorkflowSchedulerTaskIntentTemplate` values.
- `SchedulerTaskStateRecord` is mutable lifecycle state owned by the scheduler
  crate. It carries only lifecycle phase, transition/version metadata, bounded
  diagnostics, and phase-specific payload.

`SchedulableTaskIntent` remains strict. It is introduced only when
workflow-service binding resolution has validated materialized inputs for a
runtime task. Scheduler readiness, resource admission, batching, dispatch, and
runtime handoff policy must operate only on execution-intent states carrying
the runtime variant plus current readiness proof. Non-runtime tasks may become
ready, run, and complete through their own non-runtime execution-intent
variant, but they must not fabricate a runtime intent or readiness proof to
satisfy state storage.

2026-05-23 investigation update: the current phase-aware scheduler state
contract still requires `SchedulableTaskIntent` on `Ready`, `Running`, and
`Completed`. That is sufficient for runtime tasks, but it cannot represent a
completed pure non-runtime task without creating a fake runtime intent. Before
the non-runtime adapter slice executes real tasks, the state contract must be
updated so executable states carry `SchedulerTaskExecutionIntent` or an
equivalent typed state-specific payload.

Rejected approaches:

- Lazy-create scheduler records only after intent materialization. This hides
  blocked tasks from scheduler state and user-visible run status.
- Add `Option<SchedulableTaskIntent>` to the current record shape. This makes
  invalid state combinations representable and forces every policy consumer to
  rediscover whether a task is actually schedulable.
- Reuse `SchedulableTaskIntent` for non-runtime node-engine tasks with dummy
  model refs or synthetic task types. This hides a contract violation and
  would let non-runtime tasks leak into runtime readiness, resource, batching,
  and handoff policy.
- Move workflow graph input bindings/templates into the scheduler crate. This
  couples scheduler policy to graph composition and makes future runtime or
  graph changes harder to reason about.

### Task-State Replacement Standards Gates

The task-state replacement slice must satisfy these gates before source
implementation can be considered complete:

- Implement the scheduler lifecycle contract in focused scheduler modules and
  tests. Do not grow unrelated dispatch, readiness, batching, or handoff files
  except for narrow imports or deletion of old queue exports. If a touched file
  crosses the repository decomposition threshold, record a split or an explicit
  short-term exception in this plan before continuing.
- Use correct-by-construction Rust APIs. Represent lifecycle as an enum or
  equivalent typed payload structure where pre-intent states cannot contain an
  executable intent, runtime execution states contain only validated
  `SchedulableTaskIntent`, and non-runtime execution states contain only a
  validated non-runtime execution intent. Use stable contract/schema versions,
  typed ids, bounded diagnostics, `serde(deny_unknown_fields)`, `TryFrom`
  validated wrappers for raw persisted/IPC values, typed error enums,
  `#[must_use]` on validation/transition results, and `#[non_exhaustive]`
  where public states or diagnostics are expected to evolve.
- Keep scheduler policy synchronous. State validation, transition application,
  readiness eligibility, resource eligibility, batching eligibility, and
  dispatch eligibility must be synchronous pure policy. Async remains in the
  workflow-service orchestrator shell for store access, runtime-host calls,
  ledger writes, dependency readiness I/O, and task lifecycle management.
- Preserve dependency ownership. The replacement should not require new
  third-party crates. If implementation discovers a genuine dependency need,
  stop and record the dependency owner, transitive cost, feature impact, and
  verification plan before editing manifests or lockfiles.
- Replace old queue contracts directly. Remove or rewrite public exports,
  workflow-service active-run storage, read models, tests, and documentation
  that depend on `SchedulerQueueTaskRecord` or `SchedulerQueueTransition`.
  Do not keep aliases, adapters, compatibility modules, or dual successful
  queue/state paths.
- Treat old dependency/preflight contracts as retired execution authority.
  `ModelDependencyRequest`, `ModelRefV2`, node-engine dependency preflight,
  and path-shaped runtime task payloads may exist only as temporary
  diagnostic-only rejection paths until their callers are migrated. They must
  not be adapted from canonical task state, readiness proof, or runtime-host
  handoff data.
- Treat persisted artifacts and fixtures as contract artifacts. Because this
  plan intentionally does not preserve legacy compatibility, old queue-shaped
  fixtures or saved workflow artifacts must be regenerated to the new
  phase-aware contract or rejected with typed diagnostics. Do not add silent
  migration, best-effort parsing, or fallback hydration from old shapes.
- Update source-directory READMEs and crate docs for any directory whose
  ownership changes. The docs must explain scheduler state ownership,
  workflow-service task-definition ownership, transition invariants, rejected
  alternatives, lifecycle/cancellation expectations, and consumer contract
  behavior for read models.
- Add vertical acceptance coverage for pre-intent lifecycle states. Tests must
  prove a task can be created as awaiting inputs, shown in read models,
  resolved into a schedulable intent after materialization, admitted through
  scheduler policy, and dispatched through runtime host without node-engine
  output demand or reduced-plan handoff synthesis.
- Add replay/idempotency coverage for every state transition, including
  duplicate transition ids, stale expected state, terminal-state closure,
  cancellation, retry/defer, and recovery after partial progress. Run affected
  suites with normal parallelism and isolate durable test state paths.
- Add targeted deletion checks to verification, including searches for old
  successful uses of `SchedulerQueueTaskRecord`, `SchedulerQueueTransition`,
  planned-inference runtime launch ownership, reduced-plan handoff synthesis,
  `ModelRefV2`, `model_path`, frontend `modelPath`, and executable Pumas load
  target exposure outside runtime host.

## Staged Implementation

1. Add task graph extraction from validated workflow topology and graph inputs.
   This is inspection/persistence only and must not change execution behavior.
   Completed 2026-05-23 as a path-free workflow-service projection that emits
   typed diagnostics instead of accepting legacy `model_ref`/`model_path`
   identity.
2. Replace the current intent-required queue records with phase-aware durable
   scheduler task-state records and transition tests for awaiting inputs,
   input-unavailable, invalid, ready, waiting for dependency readiness,
   waiting for resources, waiting for batch, running, paused/deferred,
   retryable failed, terminal failed, and completed. Completed 2026-05-23 by
   replacing the old intent-required queue contract in source/tests with
   phase-aware task-state records and transitions. No compatibility aliases or
   shims remain for the old queue record symbols.
3. Add task-state read models and diagnostics projection for graph editor and
   run inspection. Read models must join immutable task definition facts with
   scheduler-owned lifecycle state and must allow model/task-intent fields to
   be unknown before materialization. Initial path-free workflow-service
   read-model projection and dedicated active-run query boundary completed
   2026-05-23, and the replacement task-state slice moved those read models to
   the phase-aware state contract with optional pre-intent fields. A later
   2026-05-23 slice joined immutable task graph facts with task-state records
   and made mismatched graph/state reads fail closed. The broader
   diagnostics/timing/attempt projection remains open for the orchestrator and
   ledger slices.
4. Align graph-visible scheduler constraints before relying on them in
   materialization or admission. The workflow-service task graph already
   models optional hard `runtime` and `device` constraints. Completed
   2026-05-23 by adding the optional typed `device` input to the canonical
   inference node descriptor, projecting `device` through workflow-service
   task graph tests, and replacing frontend/backend port-option provider
   context fields with `requestedRuntimeId` and `requestedDeviceId` so
   `backend_key` and `runtime_variant_id` are not graph-visible scheduler
   policy.
5. Add typed task-result materialization contracts and active-run result
   storage. Use option 2 now with option 3 discipline: store typed task outputs
   in active-run state for the first orchestrator slices, but design the schema
   and validation so durable diagnostics-ledger persistence/replay can replace
   that storage later without changing graph, node-engine, scheduler, or
   runtime-host contracts. Completed 2026-05-23 as a versioned and validated
   workflow-service `WorkflowSchedulerTaskResult` contract plus focused staged
   active-run scheduler task-result storage. The contract is path-free and
   launch-free, rejects unknown path metadata, carries typed values and bounded
   diagnostics, and leaves diagnostics-ledger replay as a storage replacement
   rather than a second execution path.
6. Add dependency-to-input binding resolution from materialized results. A
   runtime inference task becomes scheduler-admissible only when required
   typed values have been materialized into a valid `SchedulableTaskIntent`.
   Missing, wrong-type, unavailable, or invalid inputs must become typed
   diagnostics and scheduler task state, not graph-path fallback. Completed
   2026-05-23 as a focused workflow-service binding-resolution module plus
   path-free task intent templates. The resolver materializes upstream
   `PumasModelRef` task outputs into validated schedulable intents and returns
   ready, blocked, unavailable, or invalid typed outcomes without consulting
   graph paths, reduced execution plans, runtime handoff, or node-engine
   output demand. A 2026-05-23 follow-up generalized the resolver so every
   connected input binding must have a completed materialized upstream task
   result before the runtime intent is returned; `pumas_model_ref` remains the
   specialized value that is inserted into `SchedulableTaskIntent`.
7. Move runtime-host execution contracts and dispatcher out of
   `pantograph-embedded-runtime` into a lower-level shared contract crate
   before orchestrator implementation. `pantograph-workflow-service` must
   depend on the shared crate and call the runtime-host port from the
   orchestrator; `pantograph-embedded-runtime` must depend on the shared crate
   and implement the port. Remove the embedded-runtime-owned contract
   definitions after the move rather than preserving parallel DTOs or
   compatibility shims.
8. Add the scheduler task orchestrator with a synchronous policy core and an
   async shell for dependency readiness, runtime-host dispatch, ledger writes,
   cancellation, retries, and shutdown. Initial task-state creation from
   `WorkflowSchedulerTaskGraph` completed 2026-05-23; production store wiring,
   dependency readiness, dispatch lifecycle, ledger writes, bounded queues,
   cancellation, retry/defer, and panic handling remain open. The orchestrator
   active-run persistence method was added 2026-05-23; production session
   execution now calls it after queue admission and task graph extraction in
   a 2026-05-23 vertical slice. The remaining production cutover must move
   task progression into a dedicated scheduler-task execution path instead of
   continuing whole-run node-engine output demand, then consume or delete the
   staged dead-code allowances and old scheduler-managed inference launch
   path.
9. Add a dedicated scheduler-task execution entrypoint and non-runtime
   single-task execution through node-engine using materialized
   scheduler-owned inputs and task results. This slice intentionally executes
   only non-runtime graph tasks; runtime inference tasks must remain blocked
   or fail closed with typed scheduler diagnostics until actual
   scheduler-selected runtime handoff dispatch is wired. Do not wrap
   `workflow_run_internal`, do not call output-node demand from the new
   entrypoint, and do not use this adapter for runtime inference nodes.
   Classification prerequisite completed 2026-05-23: workflow-service now
   projects a schema-versioned `WorkflowSchedulerTaskExecutionClass` from
   immutable node type plus canonical node-contract facts, so later entrypoint
   and adapter code must consume that class instead of rechecking scattered
   node-type strings.
   Initial scheduler task-state creation began consuming this class on
   2026-05-23: first-stage no-dependency non-runtime tasks enter
   `Ready(NonRuntime)`, dependent non-runtime tasks await inputs, Pumas
   materialization awaits its dedicated boundary, and unsupported tasks become
   invalid instead of waiting indefinitely.
10. Add runtime task dispatch from actual dispatch-selected
   `SchedulerRuntimeHandoff` into the shared runtime-host execution port.
11. Replace session execution so the scheduler task orchestrator, not
   node-engine output demand, advances workflow progress. The replacement path
   must delegate from `run_workflow_execution_session` into a focused
   scheduler-task execution entrypoint after admission and task graph
   extraction, then remove or make unreachable the old scheduler-managed
   inference launch path. Revised replan decision accepted 2026-05-24: add a
   dedicated workflow-service scheduler-task session runner and keep
   `run_workflow_execution_session` as the admission/terminal wrapper. The
   runner must materialize request inputs into scheduler task results, read a
   run-class summary before runtime preflight/load, execute non-runtime-only
   runs without runtime admission/load, route runtime tasks only through
   scheduler-selected runtime-host handoff or typed fail-closed diagnostics,
   project task results into requested run outputs, and remove the legacy
   whole-run output-demand path instead of preserving it as fallback.
   Revised replan decision accepted 2026-05-29: implement option 2 first with
   option 3 discipline. The next source slice must extract the existing
   non-runtime-only progression loop into a dedicated workflow-service
   scheduler session runner without changing behavior. The following slice may
   add runtime-containing progression through the same runner: materialize
   request source inputs, execute allowlisted non-runtime upstream tasks,
   record typed task results, call runtime input advancement for dependent
   runtime tasks, and then fail closed before runtime-host dispatch until the
   dispatch-selected handoff slice is wired. The full option 3 durable runner
   remains the target architecture and must not be invalidated by the option 2
   extraction. Extraction-only slice completed 2026-05-29: the existing
   non-runtime-only progression loop now lives in
   `workflow/session_scheduler_runner.rs`; `session_execution_api.rs` remains
   the admission, timeout, terminal-event, and run-finalization wrapper. No
   runtime-containing progression was added in that slice.
   Runtime-containing runner progression completed 2026-05-29: the same
   runner now materializes request source inputs, executes allowlisted
   non-runtime upstream tasks, advances runtime inference tasks to
   `WaitingDependencyReadiness`, verifies the dispatch boundary, and fails
   closed before runtime-host dispatch. This preserves the no-fallback rule:
   there is still no runtime-host call, no node-engine whole-run output demand,
   no runtime `Ready` detour, and no reduced-plan handoff synthesis. Remaining
   task-level work is dependency readiness admission, scheduler dispatch
   selection, and runtime-host request construction from the actual
   dispatch-selected `SchedulerRuntimeHandoff`.
   2026-05-30 update: workflow-service now maps validated runtime dispatch
   candidate fact bundles into scheduler candidates for the dispatch-selection
   request. This only consumes already-canonical selected runtime/device/model,
   reservation, and resource-fit facts. Production embedded-runtime provider
   wiring remains blocked on explicit runtime-registry reservation
   release/retention ownership for all selected and unselected candidate
   outcomes. The selected re-plan path is a shared reservation lifecycle
   contract: workflow-service emits typed dispatch/session outcomes, and
   embedded-runtime implements runtime-registry release/reconcile behavior
   before real resource-backed candidate bundles are wired.
12. Remove planned-inference launch ownership and legacy resolver/path
   successful branches once task orchestration and runtime-host dispatch are
   wired.
13. Add recovery, replay, cancellation, duplicate-dispatch prevention,
   reservation release, and retry/defer idempotency tests.
14. Add multi-workflow acceptance coverage proving a workflow can pause between
    tasks while another user's compatible task runs or batches.

## Session Scheduler Runner Replan

The current whole-run session API is too broad to absorb runtime input
progression directly. `session_execution_api.rs` owns admission, queue waiting,
run started/terminal diagnostics, timeout wrapping, run finalization, legacy
runtime fail-closed behavior, and non-runtime task execution. Adding runtime
task progression inline would mix lifecycle handling, persistence, task
execution, diagnostics, and runtime dispatch policy in one large file.

Selected path: option 2 with option 3 discipline.

- **Option 2 immediate implementation:** add a focused workflow-service
  scheduler session runner module. `run_workflow_execution_session` remains the
  admission, timeout, terminal-event, and run-finalization wrapper. The runner
  owns active-run task progression only: source input materialization,
  non-runtime upstream execution, typed task-result recording, dependent task
  unblocking, requested-output projection, and fail-closed runtime diagnostics
  while runtime dispatch is not wired.
- **Option 3 target architecture:** evolve the runner into the durable
  task-level executor with task leases, replay/recovery, batching,
  retry/defer/cancel policy, bounded queues, worker lifecycle, and
  multi-workflow scheduling hooks. Option 2 must keep APIs and module
  boundaries narrow enough that option 3 can replace internals without
  changing graph, node-engine, scheduler, runtime-host, or Tauri/frontend
  contracts.

Standards gates for the option 2 runner:

- Extract before extending. The first slice must move the existing
  non-runtime-only progression loop into the runner and prove behavior is
  unchanged with focused session tests. Do not add runtime behavior in the
  extraction slice.
- Keep responsibility boundaries explicit. The runner must not own queue
  admission, graph validation, executable validation snapshot lookup, run
  terminal event recording, runtime/device policy, dependency readiness policy,
  runtime-host load-target resolution, or frontend/Tauri transport concerns.
- Preserve async safety. Do not hold the active-run store lock across awaited
  node-engine or runtime-host work. Use existing start/execute/complete/fail
  operations so durable result/state updates stay atomic under one store lock.
  Timeouts and cancellation remain owned by the session API wrapper until the
  option 3 durable runner introduces explicit lease/cancel records.
- Preserve no-fallback/no-legacy behavior. Runtime tasks must never route
  through node-engine output demand, `PlannedInferenceExecutionHost`,
  `ModelRefV2`, `ModelDependencyRequest`, graph `model_path`, reduced
  execution-plan handoff synthesis, or a temporary `Ready` detour.
- Keep runtime dispatch fail-closed until its own slice. Runtime-containing
  progression may advance a task to `WaitingDependencyReadiness`; it must then
  return typed diagnostics before runtime-host dispatch unless the task has
  gone through canonical readiness admission, scheduler dispatch selection, and
  validated runtime-host request construction.

Required verification for the option 2 runner:

- Focused session tests proving the extracted non-runtime-only runner preserves
  current output projection, queue/finalization behavior, timeout handling, and
  no calls to legacy whole-run host execution.
- Runtime-containing session tests proving source inputs and non-runtime
  upstream results can materialize, dependent runtime tasks can advance to
  `WaitingDependencyReadiness`, and execution then fails closed before
  runtime-host dispatch without legacy planned-inference launch.
- Targeted searches over touched workflow-service files for
  `PlannedInferenceExecutionHost`, `ModelDependencyRequest`, `ModelRefV2`,
  `model_path`, `modelPath`, reduced execution-plan handoff synthesis, and
  graph path repair helpers.
- Default, all-features, and no-default-features `cargo check` for
  `pantograph-workflow-service`, focused session/orchestrator tests, `cargo
  fmt` checks, and `git diff --check`.

## Attempt/Lease Lifecycle Replan

The next lifecycle hardening boundary is too broad to implement as one
source slice. Cancellation, retry/defer idempotency, duplicate-dispatch
prevention, reservation release, replay/recovery, and attempt/timing facts
touch scheduler task state, workflow-service orchestration, runtime-host
dispatch, reservation lifecycle, diagnostics/ledger timing, and async
shutdown behavior. The selected path is Option 2: implement the
workflow-service attempt/lease state core first.

Option 2 immediate implementation:

- Add a scheduler-owned active-run attempt/lease contract in
  workflow-service for claim/start/complete/fail/cancel transitions.
- Store attempt id, task id, workflow/run/session correlation, expected prior
  state, started/completed timestamps when available, and reservation release
  intent without adding retry policy or replay workers in the same slice.
- Require every runtime-host dispatch start, completion, failure, and cancel
  update to present the current attempt id. Stale, missing, duplicate, or
  mismatched attempts must fail closed with typed diagnostics and must not
  mutate task results or terminal state.
- Keep the active-run store mutation synchronous and atomic. Runtime-host
  awaits, reservation-port calls, ledger writes, and future worker-pool I/O
  must happen outside store locks and return to explicit completion/failure
  transitions.
- Add reservation release hooks as typed intent emitted by the state
  transition boundary; the next slice may wire the actual reservation lifecycle
  port. Do not silently drop reservations, and do not let Tauri/frontend own
  release policy.

Completed sub-slice on 2026-06-03:

- Workflow-service active-run store now creates a task attempt id when a
  ready runtime or non-runtime task starts.
- Runtime and non-runtime completion/failure paths now require the matching
  attempt id before task-state/result mutation.
- Duplicate starts and stale completions fail closed without recording
  additional task results.
- Started runtime-host dispatch errors now terminally fail through the
  matching attempt id, and the obsolete broad active-run
  `runtime-dispatch-not-wired` helper was removed.
- Workflow-service active-run task attempts now carry typed reservation lease
  and selected-candidate metadata. Completion, failure, and explicit cancel
  terminal store mutations return typed reservation-release intent for leased
  attempts. Duplicate reservation binding and stale reservation/cancel attempts
  fail closed before task-state/result mutation.
- Runtime dispatch now follows the explicit selected-dispatch sequence:
  start attempt, select dispatch, bind the selected reservation to the attempt,
  dispatch runtime-host work, terminally mutate task state/results, then apply
  reservation lifecycle release events outside the store lock from the typed
  terminal mutation release intent.
- The remaining lifecycle work is retry/defer idempotency, replay/recovery,
  durable duplicate-dispatch prevention, worker supervision, cancellation
  tokens for in-flight runtime-host work, and timing/attempt ledger facts.

Selected next path for cancellation/reservation release:

- Use Option 2. Extend the workflow-service active-run attempt record with
  selected runtime reservation metadata after dispatch selection succeeds:
  reservation lease id, selected candidate id when present, and the task/run
  correlation already owned by the active attempt.
- Add synchronous store transitions for terminal attempt outcomes that return
  typed reservation-release intent when the attempt has an attached lease.
  Completion, failure, and cancel must all use the same matching-attempt
  validation before task-state/result mutation.
- Represent cancellation in the current scheduler task-state contract as a
  terminal task-state diagnostic with explicit cancellation messaging. Do not
  add a separate `Cancelled` state in this slice; that belongs to a future
  shared scheduler-contract expansion if the semantics need to be consumed
  outside workflow-service.
- Apply reservation lifecycle events from the async orchestrator/session runner
  after the store transition returns release intent. Do not hold the store lock
  while calling the reservation lifecycle port.
- Completed store-side on 2026-06-03: reservation metadata binding, typed
  release-intent emission for completion/failure/cancel, explicit cancel
  transition support, stale/mismatched attempt rejection, and duplicate binding
  rejection.
- Completed async application on 2026-06-03: selected runtime dispatch is split
  so the session runner binds the reservation to the active attempt before the
  runtime-host await, and terminal lifecycle release events are emitted only
  after the matching terminal store mutation returns release intent.
- Keep retry/defer idempotency, replay/recovery, worker supervision,
  cancellation tokens for in-flight runtime-host work, and diagnostics ledger
  attempt/timing facts out of this slice.

Required verification for this next path:

- Store tests for reservation metadata binding to the active attempt, stale or
  mismatched attempt rejection, duplicate binding rejection, cancel transition,
  and release-intent emission only for leased terminal attempts.
- Orchestrator/session-runner tests proving reservation lifecycle events are
  applied after the store transition and that stale cancel/terminal attempts do
  not emit release events or mutate results.
- Focused deleted-symbol/no-fallback searches proving no graph path fallback,
  reduced-plan launch, node-engine runtime launch, compatibility retry branch,
  or Tauri/frontend lifecycle policy was added.
- `cargo fmt -p pantograph-workflow-service -- --check`, focused
  workflow-service scheduler tests, `cargo check -p pantograph-workflow-service`,
  and `git diff --check`.

Deferred Option 3 target:

- Add a single lifecycle owner for bounded workers, cancellation tokens,
  retry/defer policy, duplicate-dispatch prevention across restarts,
  reservation release execution, replay/recovery, diagnostics ledger writes,
  task panic handling, and shutdown.
- Promote attempt/timing facts into diagnostics read models only after the
  attempt/lease state core is validated.

Rejected or deferred alternatives:

- Option 1 minimal in-memory guardrails is rejected as the primary path
  because it does not establish durable attempt identity, stale-attempt
  rejection, or the state owner needed before removing more legacy launch
  paths.
- Option 4 contract-first scheduler crate extraction is deferred until there
  is a proven shared owner outside workflow-service. Extracting now would add
  abstraction before the workflow-service active-run lifecycle shape is
  validated.

Standards gates for the attempt/lease core:

- Keep sync-core/async-shell separation: store transitions are synchronous;
  async runtime-host, reservation, ledger, and worker lifecycle work stays at
  orchestration boundaries.
- Keep one lifecycle/state owner: workflow-service active-run orchestration
  owns task attempt state; Tauri and frontend only forward commands or render
  diagnostics.
- Keep no-fallback/no-legacy behavior: do not reintroduce whole-run output
  demand, reduced execution-plan launch, `ModelRefV2`,
  `ModelDependencyRequest`, graph `model_path`, node-engine runtime launch,
  or a compatibility retry branch.
- Keep decomposition simple: attempt contract/state transitions, runtime-host
  dispatch awaits, reservation execution, retry policy, replay/recovery, and
  diagnostics timing must stay separately owned slices.

Required verification for the attempt/lease core:

- Focused active-run store tests for successful claim/start/complete/fail/
  cancel, stale attempt rejection, duplicate start rejection, duplicate
  completion rejection, wrong task/run/session rejection, and no partial
  result/state persistence.
- Orchestrator tests proving runtime-host dispatch uses an attempt id, drops
  store locks before awaiting the port, completes/fails only with the matching
  attempt, and fails closed for stale attempts.
- Reservation hook tests proving release intent is produced on terminal
  completion/failure/cancel and not produced for stale rejected attempts.
- Targeted source searches proving no legacy runtime launch, graph path,
  reduced-plan handoff, or Tauri/frontend policy path was added.
- `cargo fmt`, focused workflow-service scheduler tests, default/all-features/
  no-default-features `cargo check` for workflow-service when public feature
  contracts change, and `git diff --check`.

## Runtime-Host Contract Crate Replan

The scheduler task orchestrator belongs in `pantograph-workflow-service`, but
the current runtime-host execution request/response/port/dispatcher types live
in `pantograph-embedded-runtime`. Since `pantograph-embedded-runtime` already
depends on `pantograph-workflow-service`, importing those types from
workflow-service would create a crate dependency cycle. The chosen option is
to move the shared runtime-host execution contract into a lower-level crate,
such as `pantograph-runtime-host-contracts`, before implementing the
orchestrator.

The shared crate should own:

- `RuntimeHostExecutionRequest`, `ValidatedRuntimeHostExecutionRequest`, and
  the execution contract version.
- `RuntimeHostExecutionResponse`, `ValidatedRuntimeHostExecutionResponse`,
  execution state, diagnostics, and contract errors.
- `RuntimeHostExecutionPort`, `SchedulerRuntimeHostDispatcher`, typed
  execution-port errors, dispatch errors, and response-correlation validation.

The shared crate may depend on `pantograph-scheduler`, `serde`,
`async-trait`, and `thiserror`. It must not depend on
`pantograph-workflow-service`, `pantograph-embedded-runtime`, node-engine,
Pumas Library, or inference runtime crates. After the move,
workflow-service consumes the shared port trait from the orchestrator, and
embedded-runtime implements that port and owns runtime-specific Pumas
load-target resolution.

Standards guardrails for the shared crate:

- Keep the crate role narrow. It is a contract/boundary crate for DTOs,
  validation wrappers, the runtime-host port trait, and response-correlation
  helper logic only. It must not own scheduler policy, workflow orchestration,
  runtime loading, Pumas load-target resolution, node-engine execution,
  process management, or concrete I/O.
- Keep the async surface at the boundary. The port trait may be async because
  runtime execution is external I/O, but validation, request construction, and
  response-correlation checks must remain synchronous. The shared crate must
  not create Tokio runtimes, spawn tasks, hold locks across `.await`, or own
  background lifecycle.
- Use correct-by-construction Rust APIs: crate-level `//!` docs, public
  re-exports from `lib.rs`, typed error enums with `thiserror`, validated
  wrappers with `TryFrom`, `#[must_use]` on validated wrappers/dispatcher
  results, and `#[non_exhaustive]` where runtime-host states, diagnostics, or
  errors will evolve.
- Preserve executable boundary contracts. Keep JSON fixture coverage for
  dispatch-selected request/response shapes, readiness-only rejection,
  unknown-field/path-field rejection, response-correlation validation, and
  failed/rejected response diagnostic requirements.
- Follow dependency ownership rules. Before adding the crate, confirm direct
  dependency ownership and use workspace dependency inheritance for shared
  crates already in the root manifest. Do not add third-party dependencies
  beyond `serde`, `async-trait`, and `thiserror` unless a new standards
  justification is recorded.
- Keep documentation traceability. Add a source-directory README and crate
  docs for the new crate, update `pantograph-embedded-runtime` README to name
  itself as implementation owner only, and update workflow-service README when
  it begins orchestrating through the shared port.
- Verify the move as a replacement, not an additive shim. The implementation
  slice must remove the old embedded-runtime-owned DTO/port/dispatcher modules
  or convert them into imports from the shared owner; no aliases, mirrored
  types, compatibility modules, or alternate successful request paths may
  remain.
- Required verification for the move: focused shared-crate contract tests,
  embedded-runtime runtime-host dispatch/load-target tests, workflow-service
  compile checks proving it can depend on the shared port without a cycle,
  default/all-features/no-default-features checks for touched crates, and
  `git diff --check`.
- 2026-05-23 implementation status: the shared
  `pantograph-runtime-host-contracts` crate now owns the runtime-host
  execution DTOs, validation wrappers, diagnostics, execution port,
  dispatcher, typed errors, README, and JSON fixture tests. Embedded-runtime
  imports the shared request wrapper for host-only Pumas load-target
  resolution, and its old runtime-host DTO/dispatcher source, tests, and
  fixtures were removed rather than retained as shims. Workflow-service
  consumer wiring remains with the orchestrator slice so the dependency is
  introduced only when it owns a real application-layer use.
- 2026-05-23 implementation status: workflow-service now has a focused
  `scheduler/task_orchestrator.rs` async shell that consumes the shared
  runtime-host dispatcher and proves dispatch-selected handoff reaches a
  fake runtime-host port while readiness-only handoff fails before the port.
  This is the first orchestrator boundary slice only; production task
  progression, dependency readiness, runtime selection, ledger writes,
  cancellation, retries, and queue lifecycle remain staged follow-ups.
- 2026-05-23 implementation status: `WorkflowService` now owns a configured
  `WorkflowSchedulerTaskOrchestrator`, exposes
  `with_runtime_host_execution_port` for production runtime-host wiring, and
  initializes active-run scheduler task state from the path-free task graph
  after queue admission in `run_workflow_execution_session`. The default
  runtime-host port is typed-unavailable so missing production wiring fails
  closed. This is not the full execution cutover: dependency readiness,
  runtime-host dispatch lifecycle, task result progression, ledger writes,
  bounded workers, cancellation, retry/defer, and removal of the whole-run
  node-engine output-demand path remain open.

Rejected alternatives:

- Put runtime execution on `WorkflowHost`: rejected because it broadens the
  workflow host with runtime execution ownership and makes task dispatch
  harder to reason about.
- Put the orchestrator in `pantograph-embedded-runtime`: rejected because
  workflow-service owns workflow state, task state, diagnostics, and run
  progression.
- Mirror DTOs in workflow-service: rejected because parallel runtime-host
  contracts would drift and violate the no-legacy/no-fallback rule.

## Production Orchestrator Ownership Replan

The next production cutover slice must decide ownership before touching
session execution. The selected design is service-owned orchestrator injection
plus a dedicated scheduler-task execution entrypoint.

`WorkflowService` must own or be configured with
`WorkflowSchedulerTaskOrchestrator` and the orchestrator's
`SchedulerRuntimeHostDispatcher`. This keeps runtime inference dispatch on one
canonical path:

1. `WorkflowSchedulerTaskGraph` and scheduler task-state records define task
   progress.
2. Scheduler policy produces dependency readiness, resource admission,
   dispatch decisions, and `SchedulerRuntimeHandoff`.
3. The workflow-service orchestrator calls the shared runtime-host execution
   port.

`run_workflow_execution_session` must not construct the orchestrator locally or
assemble runtime-host dispatch plumbing ad hoc. After queue admission and task
graph extraction it should delegate to a focused scheduler-task execution
entrypoint that initializes active-run task state and later advances
dependency readiness, dispatch, materialized results, ledger writes, retry /
defer, cancellation, and completion.

Production cutover standards gates:

- Wire the orchestrator through the workflow-service composition path. The
  production `WorkflowService` constructor/configuration must receive the
  orchestrator and shared runtime-host dispatcher as explicit dependencies or
  build them in one documented service-owned composition boundary. Do not use
  globals, lazy singletons, hidden default construction, or local construction
  inside `run_workflow_execution_session`.
- Keep `run_workflow_execution_session` narrow. It may own admission,
  graph/topology extraction, task graph extraction, and delegation, but task
  progression belongs in a focused scheduler-task execution entrypoint. If the
  current function would grow another responsibility, split before continuing.
- Preserve sync-core/async-shell separation. Scheduler readiness, resource,
  batching, runtime selection, retry/defer, and terminal-state policy must
  stay synchronous and pure. Async work is limited to workflow-service shell
  boundaries: store access, dependency readiness I/O, runtime-host dispatch,
  ledger writes, node-engine non-runtime adapter calls, timers, and shutdown.
- Keep lock and transaction scopes bounded. Session store, active-run result
  storage, task-state storage, and ledger handles must not be held across
  dependency readiness, runtime-host dispatch, node-engine calls, timers, or
  other `.await` points. Copy immutable snapshots out of locked state, release
  the guard, then perform I/O. Durable state changes must be transactional,
  idempotent, or explicitly compensating at cancellation points.
- Own task lifecycle explicitly. Any background workers, queues, or spawned
  tasks added by the cutover must be bounded, tracked, cancellable, and drained
  or aborted during shutdown. Task panics must map to typed task diagnostics
  and lifecycle-owner logs; unobserved fire-and-forget tasks are not allowed.
- Use typed contracts at every boundary. Production code must return typed
  errors/diagnostics for missing dependencies, unavailable runtime-host
  dispatch, stale state versions, duplicate dispatch, failed cancellation, and
  terminal closure. Do not add `Result<T, String>`, string parsing, untyped
  metadata maps, `unwrap`, or `expect` in production orchestration paths.
- Remove staged compatibility and dead-code allowances as the cutover consumes
  them. The implementation slice must either use or delete staged orchestrator
  APIs and retire the old scheduler-managed inference launch path. Do not
  leave both paths selectable behind config, feature flags, or fallback
  branches.
- Keep dependency ownership narrow. The production cutover should not require
  new third-party crates. If a crate is genuinely needed, stop and record the
  owning crate, transitive cost, feature-contract impact, lockfile impact, and
  verification commands before changing manifests.
- Update documentation where ownership changes. Workflow-service README/crate
  docs must describe the orchestrator dependency, task execution entrypoint,
  lifecycle/shutdown behavior, error semantics, no-fallback runtime-host
  dispatch boundary, and rejected old execution paths.
- Verification must include a vertical production-entrypoint test proving a
  submitted workflow reaches the scheduler-task execution entrypoint, creates
  task state, and does not call node-engine output demand for runtime
  inference. Focused tests must cover fake dependency readiness, fake
  runtime-host dispatch, fake non-runtime node-engine adapter, fake ledger
  writes, cancellation, retry/defer, duplicate dispatch/idempotency, terminal
  closure, panic handling when workers are introduced, and no lock-held-across
  I/O behavior where testable. Run deletion searches for old successful
  planned-inference launch branches and feature-matrix checks for touched
  crates.

Rejected alternatives:

- Construct the orchestrator inside `run_workflow_execution_session`: rejected
  because it spreads runtime-host ownership across session execution and makes
  scheduler policy easier to fragment.
- Continue running scheduler-managed inference through whole-workflow
  node-engine output demand while adding task-state observation: rejected
  because it preserves the legacy launch path instead of replacing it.
- Keep both old and new execution paths behind a compatibility branch:
  rejected because Milestone 5c requires direct replacement and removal of
  superseded runtime inference execution behavior.

## Non-Runtime Adapter First Replan

The selected next slice is option 2: introduce the dedicated scheduler-task
execution entrypoint and a narrow non-runtime node-engine single-task adapter
before wiring runtime inference dispatch. This is a staged replacement path,
not a compatibility wrapper around whole-workflow execution.

The slice must add a workflow-service-owned entrypoint that accepts an
admitted run's task graph/state context and advances one or more scheduler
tasks using materialized scheduler-owned inputs. Before that entrypoint can
complete real non-runtime work, the scheduler task-state contract must support
non-runtime executable states without `SchedulableTaskIntent`. The entrypoint
may execute only task kinds that are explicitly non-runtime, allowlisted, and
already have validated typed inputs/outputs. Runtime inference task kinds must
not call node-engine planned inference, `PlannedInferenceExecutionHost`,
`workflow_run_internal`, or output-node demand. Until Milestone 5c item 10
wires real `SchedulerRuntimeHandoff` dispatch, runtime inference tasks must
stay in typed blocked/deferred/failed scheduler state with diagnostics that
name the missing runtime-dispatch capability.

### Session Execution Cutover Replan

2026-05-24 revised decision: the production session cutover must be a focused
replacement runner, not additional inline orchestration inside
`run_workflow_execution_session` and not a compatibility wrapper around
`workflow_run_internal`.

Planned ownership:

- `run_workflow_execution_session` owns request validation, queue admission,
  creation of the path-free scheduler task graph, active-run scheduler
  task-state initialization, terminal run closure, reservation cleanup,
  artifact event projection, and response assembly.
- A new workflow-service-owned scheduler-task session runner owns active-run
  task progression after admission. It calls the existing scheduler task
  orchestrator split APIs, awaits node-engine non-runtime work outside store
  locks, dispatches runtime work only through the runtime-host execution port,
  and converts terminal task state into run-level success or typed failure.
- The runner is the only production owner of session task-loop policy. Do not
  duplicate ready-task scans, execution-class checks, materialized-input
  checks, output projection, or runtime-dispatch fallback decisions in
  `run_workflow_execution_session`, node-engine, frontend adapters, or runtime
  adapters.

Cutover sequence:

1. After queue admission and scheduler task-state initialization, compute a
   run-class summary from the immutable `WorkflowSchedulerTaskGraph` and
   active-run scheduler task-state records. The summary answers whether the
   run has runtime-inference tasks, non-runtime-only tasks, Pumas
   materialization tasks, unsupported tasks, or terminal invalid diagnostics.
   It must consume `WorkflowSchedulerTaskExecutionClass`; it must not re-check
   raw node-type strings.
2. Materialize request `WorkflowPortBinding` inputs into scheduler-owned task
   results for matching source/input tasks. This must be a typed conversion
   boundary that validates node id, port id, expected template/class, value
   type, duplicate input behavior, and result correlation. It must not mutate
   graph node data, stash arbitrary `serde_json`, or expose Pumas paths/load
   targets.
3. If the summary is non-runtime-only, bypass runtime admission, runtime
   preflight, runtime load, and runtime reservation records. Advance tasks
   through scheduler task state until completion, failure, or blocked
   diagnostics; then project completed scheduler results into the requested
   workflow outputs.
4. If the summary contains runtime-inference tasks, runtime work may become
   executable only after all required connected upstream inputs are
   materialized. Runtime tasks dispatch only from actual
   `SchedulerRuntimeHandoff` values into the shared runtime-host execution
   port. If runtime-host dispatch is not wired in the current slice, fail
   closed with typed scheduler/workflow diagnostics that name the unsupported
   runtime-dispatch capability.
5. Project completed scheduler task results to `WorkflowPortBinding` outputs
   through one typed converter, then call the existing requested-output
   validation. Unsupported output value kinds, missing requested ports, wrong
   value types, diagnostic-only results, or ambiguous producers fail closed
   with typed diagnostics.
6. Remove or make unreachable the legacy scheduler-managed session launch
   path. The replacement must not call `workflow_run_internal`,
   `DemandEngine::demand`, output-node demand, `PlannedInferenceExecutionHost`,
   node-engine workflow sessions, node-engine core `execute_puma_lib`, or any
   graph-path/model-path resolver as a successful branch.

Standards constraints:

- Keep sync policy decisions in pure helpers: run-class summary,
  external-input materialization validation, task-result output projection,
  and terminal-state selection should be synchronous and unit-testable.
- Keep async at I/O boundaries only: non-runtime adapter calls, runtime-host
  execution-port calls, ledger writes, and cleanup. Do not hold active-run
  store, queue, task-state, ledger, or reservation locks across `.await`.
- Preserve backend authority. The graph editor provides typed user constraints
  and displays backend read models; it does not know executable Pumas paths,
  runtime load targets, or scheduler resource reservations.
- Treat unfinished runtime dispatch as unavailable capability, not fallback.
  A runtime-containing run that cannot dispatch through the scheduler path
  fails with explicit diagnostics instead of using whole-run node-engine
  execution.
- Keep this replacement small enough to review. If the runner needs durable
  replay, worker pools, lease tokens, batching, or retry/cancellation policy
  beyond the existing atomic completion operation, stop and plan that as the
  later execution-lease/transaction slice rather than mixing it into the
  cutover.

Required tests for the cutover:

- Non-runtime-only session run with request `WorkflowPortBinding` inputs
  completes through scheduler task results, returns requested outputs, and
  proves the runtime admission/load path and legacy host run path were not
  called.
- Runtime-containing session run with runtime-host dispatch unavailable fails
  closed with typed scheduler/workflow diagnostics and does not call whole-run
  node-engine output demand.
- External input materialization covers valid scalar input, missing source
  task, wrong port, wrong value type, duplicate input, and unsupported source
  node cases.
- Output projection covers valid scalar output, missing requested output,
  unsupported value kind, diagnostic-only result, wrong type, and ambiguous
  producer cases.
- Search/deletion checks prove the new session runner does not call
  `workflow_run_internal`, output-node demand, `DemandEngine::demand`,
  `PlannedInferenceExecutionHost`, node-engine workflow sessions, or
  node-engine core `execute_puma_lib`.

2026-05-24 implementation status: workflow-service now has the first
external-input materialization boundary for the session cutover. It converts
request `WorkflowPortBinding` values into completed
`WorkflowSchedulerTaskResult` values for explicitly supported text and boolean
source/input tasks, rejects unknown nodes, duplicates, wrong value types, and
unsupported source tasks, and does not mutate graph node data or pass paths
through the scheduler task loop. The module is staged behind a scoped
`dead_code` allowance until the dedicated scheduler-task session runner
consumes it; that runner must remove the allowance in the next cutover slice.

2026-05-24 implementation status: workflow-service now also has the staged
run-class summary helper for the session cutover. It summarizes immutable
`WorkflowSchedulerTaskExecutionClass` values plus active scheduler task-state
records so session execution can decide whether runtime admission/load is
needed before touching runtime preflight. It rejects missing or unexpected
task-state records and remains staged behind a scoped `dead_code` allowance
until the scheduler-task session runner consumes it.

2026-05-24 implementation status: workflow-service now has the staged
task-result output projection boundary for the session cutover. It projects
completed `WorkflowSchedulerTaskResult` values into requested
`WorkflowPortBinding` outputs, rejects missing, ambiguous, non-completed, and
unsupported output values, and avoids output-node demand or legacy workflow
execution. It remains staged behind a scoped `dead_code` allowance until the
scheduler-task session runner consumes it before requested-output validation.

2026-05-24 source-input lifecycle replan decision: implement option 3 now and
record option 4 as the later target. Source/input nodes whose values are
provided by a run request must be represented as explicit source-input
scheduler tasks, not graph-data-backed node-engine execution templates. Task
graph projection must add a separate typed source-input contract field, such as
`WorkflowSchedulerSourceInputTemplate`, instead of adding request-bound variants
to `WorkflowSchedulerNonRuntimeTaskTemplate`. The non-runtime template enum
remains reserved for tasks that the non-runtime node-engine adapter is allowed
to execute.

Task classification must make the execution boundary explicit. Source-input
tasks should use a distinct path-free scheduler execution class, such as
`SourceInput`, so run summaries, read models, and orchestration do not describe
request materialization as `NonRuntimeNodeEngine` work. If implementation finds
a lower-blast equivalent, it must still preserve that separation in the typed
contract and adapter guardrails; request-bound source-input tasks must never be
accepted by `execute_non_runtime_scheduler_task`.

For the immediate allowlist, projection may create typed source-input templates
for `text-input.text` and `boolean-input.value` when the canonical node contract
exposes those typed input ports. Projection must not read request payloads,
mutate graph node data, or treat missing graph-stored values as
projection-invalid for source-input tasks. Existing graph-data-backed
`TextInput` and `BooleanInput` non-runtime execution must be retired for
scheduler-managed session runs or converted into the same source-input
materialization contract before the runner cutover; it must not remain as a
parallel successful path.

Session initialization must materialize matching request `WorkflowPortBinding`
values into completed `WorkflowSchedulerTaskResult` records and advance the
matching source-input task state through a store-owned atomic materialization
operation before dependent tasks are advanced. The operation must not fake a
`Running` node-engine execution state to reuse `complete_active_run_scheduler_task`;
it needs an explicit expected-state transition for source-input materialization.
Missing request inputs, wrong types, duplicate inputs, unsupported source nodes,
and source/task correlation mismatches become typed scheduler/workflow
diagnostics and terminal or blocked task state according to scheduler policy.
They must not fall back to `workflow_run_internal`, output-node demand, or
graph-node data mutation.

The existing `external_input_materialization` helper may remain only as the
converter-owned materialization boundary after it consumes the typed
source-input template instead of raw `node_type`/`port_id` checks. The helper
must reject tasks without a source-input template even if their node type looks
like a source node, preventing scattered allowlists and keeping task graph
projection as the single classifier.

Option 4 remains the target for future extensibility: replace the interim
allowlist source-input templates with a generic typed port-value source contract
derived from canonical node contracts so user-authored nodes and future typed
source nodes do not require one enum variant per node. Option 4 must still be
typed and converter-owned; it must not introduce arbitrary JSON passthrough or
parallel successful execution paths.

### Task Classification And Input Readiness Replan

2026-05-23 standards/blast-radius update: the non-runtime adapter slice must
not hard-code today's incidental runtime boundary as scattered string checks.
Before workflow-service executes any scheduler-owned task, it must add a
focused task-classification and typed-input-readiness boundary.

Planning names may change during implementation, but the boundary must preserve
these semantics:

- `WorkflowSchedulerTaskExecutionClass` or equivalent workflow-service type:
  classifies each immutable task-graph task as `RuntimeInference`,
  `NonRuntimeNodeEngine`, `PumasMaterialization`, or `Unsupported`.
- The classifier is the only workflow-service place where node type plus
  canonical node-contract facts become scheduler execution class. Do not spread
  `node_type == "llm-inference"` or allowlist string checks across task graph,
  orchestrator, adapter, and read-model modules.
- `llm-inference` is the only currently supported runtime-inference graph node,
  but that is a current contract fact, not a design limit. Future model types,
  model families, or runtime-backed node descriptors must be added through this
  classifier from canonical node contracts and inference task contracts.
- If a canonical contract identifies a task as inference/runtime-backed but the
  scheduler path does not yet support it, the task becomes `Unsupported` or
  blocked/invalid with typed diagnostics. It must not fall through to the
  node-engine non-runtime adapter.
- Pumas model selection/materialization is a dedicated boundary. `puma-lib`
  must not be executed through generic node-engine single-task execution.

The task graph remains the immutable task-definition owner. It may carry a
classification result and typed non-runtime task template, but it must not carry
raw arbitrary node data as executable input. For the first non-runtime slice,
only validated source/task settings needed by allowlisted nodes may be stored,
for example text input value and boolean input value. Each stored setting must
be a typed field or a validated `WorkflowSchedulerTaskResultValue`-compatible
template, not an unbounded JSON blob, graph-local path, Pumas load target, or
frontend-owned scheduler fact.

2026-05-24 decision: use the immediate option 2 template contract for the next
implementation slice because it gets scheduler-owned non-runtime execution
running without designing the full generic plugin/user-node execution contract
yet. This means:

- Add a schema-versioned `WorkflowSchedulerNonRuntimeTaskTemplate` or
  equivalent task-definition field with concrete variants for the current
  allowlist only: `TextInput { value: String }`,
  `BooleanInput { value: bool }`, and a no-static-data `TextOutput` template
  whose required upstream text value must come from materialized task results.
- The task-graph projection is the only place allowed to read graph node data
  for these concrete templates. The scheduler-task entrypoint and
  non-runtime adapter consume only immutable typed task-definition templates
  plus materialized `WorkflowSchedulerTaskResult` values; they must not read
  raw graph/editor node data, arbitrary `serde_json`, `_data`, graph-local
  model paths, Pumas paths, or frontend-owned scheduler facts.
- Each newly supported non-runtime node during option 2 requires an explicit
  template variant, converter, focused tests, and README/plan update. This is
  deliberate: user-authored or external nodes remain `Unsupported` with typed
  diagnostics until they either receive an explicit concrete template variant
  or the later generic typed execution contract replaces this interim shape.

2026-05-24 source-input follow-up: the option 2 `TextInput` and `BooleanInput`
non-runtime templates were useful staging work, but source input values now have
a stricter owner. Before scheduler-managed session runs consume the staged
runner, source input nodes must move to the separate source-input task contract
described above. Keeping graph-data-backed `TextInput`/`BooleanInput` execution
beside request materialization would violate the no-legacy/no-fallback rule and
make it unclear whether source values came from the scheduler request, graph
node data, or node-engine execution.
- Keep the later generic typed port-value execution/source contract as the
  extensibility target: it must be derived from canonical node contracts,
  suitable for user-authored nodes and new model/runtime families without adding
  a concrete enum variant per node. That later contract must still use typed
  values and converters; it must not reintroduce raw JSON passthrough or a
  parallel successful execution path.

Initial scheduler state creation must use the classification and readiness
facts instead of treating "no runtime intent" as automatically awaiting inputs:

- Supported runtime-inference task with a complete validated intent and all
  required connected inputs materialized: `Ready(Runtime)`.
- Supported runtime-inference task missing required connected inputs:
  `AwaitingInputs` with typed waiting diagnostics as needed.
- Supported non-runtime task with no required upstream inputs and a complete
  typed non-runtime template: `Ready(NonRuntime)`.
- Supported non-runtime task with required upstream inputs:
  `AwaitingInputs` until the materialized input resolver validates those
  inputs.
- Unsupported, excluded, stale, or ambiguous task class: `Invalid` or another
  explicit scheduler state with typed diagnostics. Do not silently defer forever
  and do not call the old whole-run path.

Binding resolution must become a general typed materialized-input resolver
before runtime dispatch and before node-engine non-runtime execution. The
existing `pumas_model_ref` resolver is only the first specialization. The next
resolver shape must validate every required connected input for the selected
class, including prompt/text/media/options inputs for inference and the
allowlisted scalar inputs for non-runtime nodes. If a required value cannot be
represented by the current `WorkflowSchedulerTaskResultValue` variants, stop
and plan the explicit value contract before adding that node or port to the
allowlist.

This keeps the graph editor and node engine simple: the graph editor provides
typed graph inputs and displays backend facts, node-engine executes only
approved non-runtime tasks from already-materialized values, and the scheduler
remains the only authority that decides when inference is ready to run and what
runtime/device handoff is executable.

### Node-Engine Single-Task API Replan

The selected boundary is option 1: add a narrow node-engine-owned
single-task API before implementing the workflow-service adapter. Node-engine
must own the execution mechanics required by its public `TaskExecutor`
contract, including `graph_flow::Context` creation and `ExecutorExtensions`
setup. Workflow-service must not add a direct `graph-flow` dependency, must
not re-export or construct graph-flow internals, and must not duplicate
node-engine node behavior.

Planned node-engine API shape, subject to local naming during implementation:

- `NodeEngineSingleTaskRequest`: task id, explicit node type, and typed
  `serde_json` inputs already converted by the workflow-service adapter.
  The request must not carry graph edges, output targets, demand state,
  runtime handoff, Pumas load targets, local paths, or scheduler policy.
- `NodeEngineSingleTaskResponse`: raw node-engine output map only. It remains
  node-engine-owned output data; workflow-service converts it immediately into
  `WorkflowSchedulerTaskResult` variants at the adapter boundary.
- `execute_core_task_once(request)`: creates a local `graph_flow::Context`,
  creates empty `ExecutorExtensions`, injects the explicit node type into the
  node-engine input shape, executes one task through `CoreTaskExecutor`, and
  verifies the core executor resolved the same explicit node type before
  returning the response. It must not rely on task-id suffix node-type
  inference and must not call `DemandEngine`, output-node demand, workflow
  sessions, planned-inference host extensions, runtime host dispatch, or
  `workflow_run_internal`.

The node-engine API is intentionally not the scheduler allowlist. It is an
execution-mechanics boundary. Workflow-service still owns the scheduler-task
adapter and must reject runtime inference, `puma-lib`, `model-provider`, file
I/O, arbitrary JSON, unknown task kinds, and unsupported non-runtime kinds
before constructing a node-engine single-task request.

Node-engine write set for this replan:

- Add one focused node-engine module for the single-task API and tests.
- Update `crates/node-engine/src/lib.rs` exports only for that API.
- Update `crates/node-engine/src/README.md` to document that this API is
  one-task execution plumbing, not demand execution, runtime inference, or
  scheduler policy.
- Do not edit node-engine inference, planned-inference, `DemandEngine`,
  workflow-session, registry, or `puma-lib` implementation files unless
  implementation proves the API cannot be added without doing so; if that
  happens, stop and replan again.

The non-runtime adapter must be narrow and path-free:

- Input: one scheduler task id, validated materialized task inputs, immutable
  task-definition facts needed by node-engine, and execution correlation ids.
- Output: `WorkflowSchedulerTaskResult` values and typed diagnostics only.
- Forbidden data: Pumas local paths, executable load targets, runtime-host
  handoff, reduced execution-plan nodes, graph-local model paths, worker
  launch metadata, or frontend-derived scheduler facts.
- Runtime inference guard: inference/runtime task kinds are rejected before
  the adapter can call node-engine, and the rejection becomes scheduler task
  state plus diagnostics rather than a fallback launch.
- Node-type authority: the adapter must derive the node type from immutable
  `WorkflowSchedulerTaskGraph` task-definition facts, inject that value into
  node-engine inputs when needed, and fail closed if node-engine resolution
  would disagree. It must not trust caller-provided `_data.node_type` or infer
  execution authority from task id suffixes.

Initial adapter allowlist:

- Allowed first-stage nodes: `text-input`, `text-output`, and `boolean-input`
  only, plus focused test fakes that use the same typed conversion surface.
  Each allowed node must have a complete mapping from materialized scheduler
  inputs to node-engine inputs and from node-engine outputs to
  `WorkflowSchedulerTaskResultValue`.
- Excluded until a specific typed contract exists: `number-input` and numeric
  settings that need floating-point values; `selection-input` with arbitrary
  option payloads; `vector-input`/`vector-output`; image/audio input and output
  nodes; `expand-settings`; file I/O nodes; human/tool nodes; `model-provider`;
  `puma-lib`; and any node that emits arbitrary JSON, executable paths, Pumas
  load targets, backend decisions, runtime ids, worker launch facts, or hidden
  dependency facts.
- `puma-lib` is a dedicated Pumas selector/materialization boundary, not a
  generic node-engine task. The stale node-engine core `puma-lib` implementation
  that emits `model_path` is a deletion or replacement target and must not be
  called by the scheduler-task adapter.

Legacy cleanup required before or with this slice:

- Update stale workflow-service graph registry tests that still expect
  `puma-lib.model_path`; the canonical output is `pumas_model_ref`.
  Completed 2026-05-23.
- Remove graph-persistence behavior and tests that preserve successful
  `puma-lib` `modelPath`/`model_path` values without canonical Pumas identity,
  or replace those cases with typed stale/invalid diagnostics. Do not keep a
  successful legacy path branch. Completed 2026-05-23.

Implementation order for the next slice:

1. Extend the scheduler task-state contract so ready/running/completed
   non-runtime tasks carry a non-runtime execution intent instead of
   `SchedulableTaskIntent`; add transition tests proving no fake runtime intent
   is required.
2. Remove or update stale `puma-lib.model_path` test/persistence behavior that
   would conflict with the path-free Pumas model-reference boundary.
   Completed 2026-05-23.
3. Add the node-engine-owned single-task API that hides `graph_flow::Context`
   and executes one explicit core task without demand execution, planned
   inference, runtime host dispatch, or scheduler policy.
4. Add the workflow-service task-classification and typed materialized-input
   readiness boundary. This must classify runtime inference, non-runtime
   node-engine, Pumas materialization, and unsupported tasks in one focused
   module, then validate all required connected inputs before any task becomes
   executable.
5. Add the immediate option 2 typed non-runtime task-template contract to the
   immutable scheduler task graph. Populate concrete first-stage templates
   only during graph projection, cover `text-input`, `boolean-input`, and
   `text-output`, and fail closed with typed diagnostics for missing, malformed,
   unsupported, or ambiguous source values. Do not pass raw node data to the
   adapter.
6. Add the scheduler-task execution entrypoint contract in workflow-service
   behind the existing service-owned orchestrator boundary. Completed
   2026-05-24 for ready non-runtime tasks.
7. Add the non-runtime single-task adapter fake/trait boundary and focused
   tests using materialized inputs and typed task results. Completed
   2026-05-24.
8. Add an active-run store completion operation that records a terminal
   `WorkflowSchedulerTaskResult` and advances the corresponding scheduler task
   state to completed under one active-run store lock. Do not add separate
   successful "persist result" and "complete task" paths. Completed
   2026-05-24.
9. Wire the entrypoint to execute a simple allowlisted non-runtime task and
   persist its `WorkflowSchedulerTaskResult`. Completed 2026-05-24, then
   refined into start/execute/complete/fail calls so production session
   execution can drop the active-run store lock before awaiting node-engine
   work.
10. Add a negative test proving runtime inference tasks do not call
   node-engine output demand or `PlannedInferenceExecutionHost` and instead
   produce typed scheduler diagnostics. Completed 2026-05-24 for the
   non-runtime entrypoint boundary.
11. Add dependent non-runtime readiness advancement from `AwaitingInputs`.
   Completed 2026-05-24 for materialized active-run task results: missing
   upstream input remains blocked, valid text input advances to
   `Ready(NonRuntime)`, and wrong-type or unavailable upstream values become
   typed scheduler diagnostics without calling graph output demand.
12. Update README and plan notes, then run focused node-engine and
   workflow-service checks. Completed 2026-05-24 for the ready non-runtime
   entrypoint slice.

Standards gates for this slice:

- Keep the implementation in focused workflow-service modules, such as a
  scheduler-task execution entrypoint module plus a non-runtime adapter module.
  Do not grow `session_execution_api.rs`, `workflow_run_api.rs`, or
  `task_orchestrator.rs` beyond narrow delegation/import changes.
- Keep the node-engine implementation focused on the single-task API. Do not
  change manifests or add a workflow-service `graph-flow` dependency. Do not
  re-export `graph_flow::Context` as the chosen boundary. Do not duplicate
  allowed node behavior in workflow-service.
- Use the node-engine single-task API only behind a workflow-service adapter
  that guards task kind first. The adapter must use a positive allowlist of
  scheduler task kinds/node families that are explicitly non-runtime and whose
  output values are representable in `WorkflowSchedulerTaskResultValue`. It
  must reject `llm-inference`, image-generation/inference, llama.cpp,
  audio-generation, planned-inference, `puma-lib`, `model-provider`,
  arbitrary-JSON nodes, file I/O nodes, and any unknown task kind before
  constructing node-engine inputs or calling the node-engine single-task API.
- Centralize task classification. Add one workflow-service classifier or
  equivalent focused module that converts immutable task graph facts plus
  canonical node-contract facts into runtime-inference, non-runtime
  node-engine, Pumas-materialization, or unsupported classes. Runtime and
  non-runtime code must consume that typed class rather than repeating raw
  node-type string predicates.
- Make non-runtime readiness explicit. Supported no-dependency non-runtime
  tasks may become `Ready(NonRuntime)` only after their typed task template is
  validated. Supported dependent non-runtime tasks remain `AwaitingInputs`
  until the materialized-input resolver validates every required upstream
  value. Unsupported or excluded kinds produce typed diagnostics and must not
  be deferred indefinitely.
- Keep option 2 concrete and replaceable. The immediate task-template enum is
  a narrow contract for the current allowlist, not a generic user-node system.
  New supported nodes require explicit variants and tests until option 3
  replaces this with a canonical typed port-value execution template. Do not
  add catch-all `Json`, `Map`, `Any`, or metadata variants to avoid planning
  the option 3 contract.
- Convert values explicitly at the adapter boundary. Materialized
  `WorkflowSchedulerTaskResult` values must be mapped into the node-engine
  input shape through typed conversion helpers, and node-engine outputs must
  be mapped back into `WorkflowSchedulerTaskResult` variants. Do not pass raw
  serde blobs through as an implicit compatibility format, and do not accept
  path-like fields as successful values. If a node requires a value variant
  that does not exist yet, such as finite `f64`, typed option payloads, vector
  values, or bounded JSON settings, stop and plan the explicit value contract
  before adding the node to the allowlist.
- Generalize input materialization before runtime dispatch. The first
  `pumas_model_ref` resolver must be extended or replaced by a focused
  materialized-input resolver that validates all required connected ports for
  the selected task class, including prompts/text/media/options for inference
  and allowlisted scalar inputs for non-runtime nodes. Runtime inference must
  not become executable merely because model identity exists while another
  connected upstream input is still missing.
- Keep the async shell narrow. The entrypoint may await the non-runtime
  adapter because node-engine's `TaskExecutor` is async, but pure task
  selection, allowlist validation, state transition choice, and result
  conversion must stay synchronous. Do not spawn background tasks, create
  Tokio runtimes, or hold session-store/task-state/ledger locks across the
  adapter `.await`.
- Keep task completion atomic inside the active-run store. The entrypoint may
  transition a ready task to running before awaiting the non-runtime adapter,
  but successful terminal completion must call one store-owned operation that
  validates the active run id, workflow id, task id, node id, expected running
  state, terminal transition, and result correlation before it stores the
  `WorkflowSchedulerTaskResult` and marks the task completed. The operation
  must fail closed for stale state, wrong task/run correlation, duplicate
  successful results, or mismatched terminal status. Adapter failure before a
  valid result must transition to typed retryable or terminal failure without
  recording a successful result.
- Use typed production errors and diagnostics. Runtime-task rejection,
  unsupported non-runtime kind, missing materialized input, wrong input type,
  adapter execution failure, output conversion failure, stale task state, and
  terminal-state closure must map to typed scheduler/workflow diagnostics and
  state transitions. Do not add `Result<T, String>`, string parsing, `unwrap`,
  or `expect` in production task execution paths.
- Keep dependency ownership unchanged. The next slice should use existing
  workspace crates (`node-engine`, workflow-service, scheduler contracts) and
  must not change manifests or lockfiles. If a dependency appears necessary,
  stop and record dependency ownership, transitive cost, feature impact, and
  verification before editing manifests.
- Verification must run with normal Rust test parallelism and include:
  focused node-engine single-task API tests proving explicit `text-input`,
  `text-output`, and `boolean-input` execution works without `DemandEngine`,
  output-node demand, workflow sessions, `PlannedInferenceExecutionHost`, or
  graph-flow exposure to workflow-service, and proving task-id suffix fallback
  cannot override the explicit node type;
  task-classification tests proving `llm-inference` is runtime-owned,
  allowlisted scalar nodes are non-runtime-owned, `puma-lib` is a dedicated
  Pumas materialization boundary, and unknown or future inference-shaped nodes
  fail closed instead of reaching the non-runtime adapter;
  materialized-input readiness tests proving runtime inference waits for every
  required connected input, not only `pumas_model_ref`;
  active-run store tests proving result persistence and completed-state
  transition happen through one operation, reject stale running state, reject
  wrong run/task/node correlation, and do not leave completed-without-result or
  result-without-completed state;
  scheduler state transition coverage proving non-runtime completed tasks do
  not carry `SchedulableTaskIntent`, focused positive allowlisted non-runtime
  task execution, runtime-task rejection before node-engine call,
  `puma-lib`/`model-provider` rejection before node-engine call,
  unsupported-kind rejection, typed input conversion failure, typed output
  conversion failure, stale `puma-lib.model_path` test cleanup, no
  lock-held-across-await review where testable, `cargo check -p
  node-engine`, `cargo check -p pantograph-workflow-service`, and `cargo
  check -p pantograph-scheduler` in default, all-features, and
  no-default-features modes for touched crates, `cargo fmt -p node-engine -p
  pantograph-workflow-service -p pantograph-scheduler`, `git diff --check`,
  and targeted searches proving the new entrypoint does not call
  `workflow_run_internal`, `DemandEngine::demand`, output-node demand,
  `PlannedInferenceExecutionHost`, or node-engine core `execute_puma_lib`.
- Update documentation in the touched source directories. At minimum,
  node-engine README/docs must describe the single-task API boundary, and
  workflow-service README/docs must describe the scheduler-task execution
  entrypoint, non-runtime adapter scope, runtime inference rejection behavior,
  and remaining runtime-host dispatch follow-up.

Rejected options for this boundary:

- Re-export `graph_flow::Context` from node-engine: rejected because it leaks
  node-engine execution plumbing into workflow-service and makes scheduler
  orchestration harder to reason about.
- Add `graph-flow` as a direct workflow-service dependency: rejected because
  workflow-service should not construct node-engine internals to run a task.
- Reimplement allowed node behavior in workflow-service: rejected because it
  duplicates node-engine ownership and will drift as node semantics evolve.
- Minimal wrapper around `workflow_run_internal`: rejected because it would
  rename the legacy output-demand path and preserve the old successful
  execution behavior.
- Runtime dispatch first: deferred because dispatch depends on the task
  entrypoint, materialized inputs/results, diagnostics, cancellation, and host
  wiring. It remains the following milestone item after the non-runtime
  adapter proves the scheduler-task execution path.
- Full cutover in one slice: rejected as too large for validated vertical
  implementation and too likely to obscure legacy removal mistakes.

2026-05-23 implementation status: the scheduler task-state contract now uses
`SchedulerTaskExecutionIntent` for executable states. The runtime variant wraps
`SchedulableTaskIntent`; the non-runtime variant wraps
`SchedulerNonRuntimeTaskIntent` with workflow/run/node/task correlation and a
validated non-runtime task kind. Existing `task_intent()` access now returns
only runtime intents, so readiness, resource, batching, dispatch, and handoff
policy cannot accidentally consume non-runtime node-engine work as runtime
work. The non-runtime node-engine adapter itself remains open, as do dedicated
task execution entrypoint wiring and runtime-host dispatch cutover.

2026-05-23 implementation status: stale successful `puma-lib.model_path`
persistence has been removed. Graph persistence now strips legacy
`modelPath`/`model_path` derived facts from `puma-lib` nodes regardless of
whether a canonical model id is present; focused save/load tests prove
path-only `puma-lib` state is not preserved as model identity. The graph
registry options-provider regression now asserts canonical `pumas_model_ref`.
The dedicated scheduler-task execution entrypoint, non-runtime adapter, and
runtime-host dispatch cutover remain open.

2026-05-23 implementation status: scheduler task-state read models now expose
state diagnostics and a path-free execution category. Runtime execution states
show runtime task facts; non-runtime execution states show only the
non-runtime task kind. Pre-intent states still show unknown model/runtime/device
facts without fabricating intent. Timing and attempt counters remain open until
the retry/defer/ledger lifecycle slice adds typed scheduler facts for them.

2026-05-23 implementation status: node-engine now owns the focused
`single_task` API planned for the non-runtime adapter boundary. The API exposes
validated `NodeEngineSingleTaskRequest` / `NodeEngineSingleTaskResponse`
contracts, creates local `graph_flow::Context` and empty `ExecutorExtensions`,
injects explicit node-type authority from the request, runs one
`CoreTaskExecutor` task, and fails closed for malformed `_data` or task-id
suffix fallback attempts. Focused tests cover `text-input`, `text-output`,
`boolean-input`, caller-supplied `_data.node_type` override, and blank request
fields. Workflow-service task classification, generalized materialized-input
readiness, scheduler-task execution entrypoint, non-runtime adapter conversion,
and runtime-task rejection remain open.

2026-05-24 implementation status: workflow-service task graph projection now
owns the immediate option 2 typed non-runtime template contract. Task graph
schema version 3 adds `WorkflowSchedulerNonRuntimeTaskTemplate` with concrete
`TextInput`, `BooleanInput`, and `TextOutput` variants. Projection is the only
layer that reads graph node data for these templates; it accepts canonical
`text-input.text`, canonical `boolean-input.value`, and `text-output` with an
upstream `text` binding, while malformed/missing/stale values produce typed
projection diagnostics. Orchestrator initialization rejects non-runtime tasks
that do not carry a validated template, so source non-runtime tasks cannot
become ready from raw graph data or incidental adapter behavior. Scheduler-task
execution entrypoint, non-runtime adapter conversion, and runtime-task
rejection remain open.

2026-05-24 implementation status: workflow-service now has the narrow
non-runtime adapter conversion module. It executes only tasks classified as
`NonRuntimeNodeEngine` with a typed template, converts `TextInput`,
`BooleanInput`, and `TextOutput` inputs into node-engine `single_task`
requests, converts node-engine outputs back into path-free
`WorkflowSchedulerTaskResult` values, and rejects runtime tasks before calling
node-engine. This slice intentionally does not update scheduler task state,
persist results, or advance active runs. The adapter has a temporary
module-scoped dead-code allowance until the next scheduler-task entrypoint
slice calls it; that allowance is a removal target for the entrypoint commit.

2026-05-24 replan decision: the scheduler-task entrypoint must not persist a
successful task result and mark the task completed through two independent
store calls. The immediate implementation uses option 2: add a focused
active-run store completion operation that commits result storage and the
terminal completed-state transition together under one store lock, after the
entrypoint has awaited the non-runtime adapter outside that lock. This keeps
the current staged active-run store coherent without introducing the larger
option 3 execution lease/transaction command yet. Option 3 remains the later
target for retries, duplicate dispatch prevention, cancellation, worker pools,
and attempt-token ownership.

2026-05-24 implementation status: workflow-service active-run storage now has
`complete_active_run_scheduler_task` as the atomic success boundary for
scheduler-task completion. The method validates the active run, task result,
completed transition, running current state, duplicate result absence, and
workflow/run/node/task correlation before storing the
`WorkflowSchedulerTaskResult` and completed task-state record together. Focused
tests prove successful completion, stale non-running state rejection, wrong
node correlation rejection, duplicate success rejection, and non-completed
result rejection without leaving completed-without-result or
result-without-completed state. The scheduler-task entrypoint wiring remains
open and must consume this method rather than separate result/state store calls.

2026-05-24 implementation status: workflow-service now has a ready
non-runtime scheduler-task execution entrypoint on
`WorkflowSchedulerTaskOrchestrator`. It reads the active-run task graph and
task state, rejects non-non-runtime tasks before the node-engine adapter,
transitions ready non-runtime tasks to running, reads materialized active-run
results, awaits `execute_non_runtime_scheduler_task` outside store mutation
calls, commits success through `complete_active_run_scheduler_task`, and moves
adapter failures to terminal failed without recording a successful result. The
temporary module-level dead-code allowance on the non-runtime adapter was
removed. Full session-execution cutover, dependent-task readiness advancement,
runtime-host dispatch wiring, cancellation/retry/defer idempotency, and legacy
output-demand launch removal remain open Milestone 5c work.

2026-05-24 implementation status: workflow-service task graph schema version 4
now separates request-provided source inputs from node-engine non-runtime
execution. `WorkflowSchedulerTaskExecutionClass::SourceInput` and
`WorkflowSchedulerSourceInputTemplate` represent canonical source ports such as
`text-input.text` and `boolean-input.value`; projection no longer reads
graph-local source values or marks missing graph data projection-invalid for
source tasks. `WorkflowSchedulerNonRuntimeTaskTemplate` now contains only
node-engine-executable non-runtime work for this slice, currently
`TextOutput`; the non-runtime adapter rejects source-input tasks before
node-engine. External input materialization consumes the typed source-input
template rather than raw node type checks and produces typed
`WorkflowSchedulerTaskResult` values. Run summaries and task-state read models
now report source inputs separately from non-runtime node-engine work.

2026-05-24 implementation status: the scheduler and workflow-service active
run store now have the canonical source-input materialization boundary. The
scheduler task-state contract adds `SchedulerSourceInputTaskIntent`, and
source-input materialization may transition directly from `AwaitingInputs` to
`Completed` only with source-input intent rather than runtime or non-runtime
execution intent. Workflow-service adds
`materialize_active_run_source_input_task`, which validates the active-run task
graph, requires `WorkflowSchedulerTaskExecutionClass::SourceInput` plus a typed
source-input template, validates completed task-result correlation, applies the
source-input transition, and stores the completed task result and completed
task-state record in one mutation. This removes the need to fake a node-engine
`Running` state for request inputs. Remaining session cutover work is runner
integration: call the orchestrator source-input materialization method,
advance dependent tasks, project completed task results to requested outputs,
and remove the remaining staged dead-code allowances when the runner consumes
the task-loop helpers.

2026-05-24 implementation status: the scheduler task orchestrator now consumes
the external-input converter and the atomic source-input store operation
through `materialize_external_inputs_for_active_run`. The method reads the
active immutable task graph and task-state records, converts request
`WorkflowPortBinding` values into typed task results, builds source-input
`AwaitingInputs -> Completed` transitions with `SchedulerSourceInputTaskIntent`,
and records each result/state pair through the store-owned atomic boundary.
This removes source-input transition construction from the future session
runner and removes the `external_input_materialization` module and
source-input store operation from the staged dead-code set. Remaining session
cutover work is to call this orchestrator method from the dedicated runner,
advance dependent task readiness, execute ready non-runtime tasks, project
completed scheduler results to requested outputs, and remove the orchestrator
staging allowances after production consumption.

2026-05-24 implementation status: non-runtime-only session runs now use a
dedicated scheduler-task session path. `run_workflow_execution_session`
precomputes the immutable scheduler task graph and initial task-state records
before runtime admission, summarizes the run class, skips runtime
admission/preflight/load for non-runtime-only graphs, materializes request
source inputs through the orchestrator, advances dependent non-runtime task
readiness, executes ready non-runtime tasks through the node-engine single-task
adapter, projects requested outputs from completed scheduler task results, and
finishes the active run without calling `workflow_run_internal`. Runtime
containing runs now also avoid runtime admission/preflight/load and
`workflow_run_internal` while dispatch-selected runtime-host handoff is not
wired: workflow-service marks runtime scheduler tasks terminal failed with
typed `SchedulerPolicyError` diagnostics and returns a capability-violation
workflow error. Remaining cutover work: replace that fail-closed branch with
actual scheduler-selected runtime-host dispatch, handle
Pumas-materialization-only and unsupported task-class terminal behavior,
update legacy session tests that intentionally expect whole-run host/runtime
behavior to use runtime-task graphs or new scheduler diagnostics, and remove
the remaining staged orchestrator `dead_code` allowances when runtime dispatch
is consumed.

2026-05-29 implementation status: the dedicated session scheduler runner now
owns runtime-containing active-run progression up to, but not beyond, the
runtime dispatch boundary. Runtime session runs with saved executable
validation snapshots materialize source inputs, progress allowlisted upstream
non-runtime tasks, advance runtime tasks from `AwaitingInputs` to
`WaitingDependencyReadiness`, and then return the scheduler dispatch
capability violation while host runtime load and legacy whole-run host
execution remain untouched. The focused test uses the current path-free
executable validation snapshot fixture. A realistic `prompt ->
inference.prompt` edge remains blocked by graph submit validation because
model-specific inference ports are still not applied to the static
`llm-inference` contract; that is an inference-interface validation/admission
follow-up, not a reason to add a static compatibility prompt port here.

2026-05-24 implementation status: the session runner no longer has a
successful legacy whole-run branch for Pumas-materialization-only or
unsupported scheduler task classes. Those classes now terminal-fail through
scheduler-validated task-state transitions with typed scheduler diagnostics.
This exposes the next no-legacy cleanup boundary: retired runtime-load
admission helpers, execution-plan admission helpers, runtime-reservation event
helpers, queue runtime-admission fields, and unused media artifactization
helpers now show as dead code and must be deleted or reconnected only through
canonical scheduler/runtime-host paths.

## Legacy Surface Cleanup Replan

2026-05-24 decision: use option 2 as a cleanup gate before continuing the
runtime-dispatch implementation. The next slice must classify each exposed
legacy surface, then either delete it, reattach it through the canonical
scheduler/runtime-host path, or convert it to scheduler task-result/output
ownership. The classification is part of the implementation checklist, not a
compatibility period.

Classification rules:

- **Delete:** code whose only remaining purpose is whole-run execution,
  output-node demand, reduced execution-plan launch/admission, graph-local
  model-path identity, old queue runtime-admission waits, or old runtime
  preflight/load ownership.
- **Reattach:** code that still represents a valid concept but is currently
  owned by the retired path. Reattachment is allowed only through the
  dispatch-selected scheduler/runtime-host path, with names and ownership
  updated so it cannot be called from session admission or node-engine output
  demand.
- **Convert:** code that still represents user-visible task output behavior
  but belongs under scheduler task-result/output projection. Conversion must
  consume `WorkflowSchedulerTaskResult` values and must not preserve the old
  whole-run artifactization path.

Initial classification targets:

- `workflow_run_internal`, `DemandEngine` output-demand launch helpers,
  node-engine workflow-session runtime launch, and
  `PlannedInferenceExecutionHost` successful branches are deletion targets
  for scheduler-managed runs.
- Runtime-load/session-admission helpers,
  `session_runtime_load_lifecycle`, and runtime-reservation event helpers are
  deletion targets unless the runtime handoff slice reattaches a narrow part
  as scheduler-selected runtime-host lifecycle diagnostics.
- `execution_plan_admission` helpers and exports are deletion targets if they
  only support reduced execution-plan admission or launch. Runtime execution
  must use task graph/state plus `SchedulerRuntimeHandoff`.
- Queue runtime-admission/preflight fields and helper methods are deletion
  targets unless an active scheduler read model or policy still consumes them
  without reintroducing runtime admission/load.
- `artifact_output_conversion` and `media_conversion_executor` are not active
  scheduler/runtime-host handoff surfaces. The old workflow-service
  artifactization route and host-injected conversion executor have been
  removed; the remaining unused Tauri managed media conversion adapter must be
  deleted rather than kept as dormant business logic.
- Future media conversion belongs behind a backend-owned, host-agnostic service
  boundary consumed by scheduler task-result materialization or runtime-host
  output projection. Tauri may provide platform infrastructure only, not
  conversion policy, planning, or workflow ownership.

Verification for this gate must include targeted usage searches for each
retired symbol, focused tests for any reattached or converted surface, crate
checks for every touched crate without new dead-code warnings, and
`git diff --check`. If a surface cannot be deleted in the current slice, the
plan must record the owner, the canonical path it will attach to, and the
specific follow-up before implementation continues.

2026-05-24 implementation status: the reduced execution-plan admission deletion
slice is complete. Workflow-service no longer declares
`execution_plan_admission`, no longer re-exports
`build_workflow_execution_plan_from_admission`, no longer documents that helper
as an active workflow module, and no longer keeps contract tests that prove
technical-fit admission can synthesize a reduced executable run plan. Targeted
source searches for `build_workflow_execution_plan_from_admission`,
`execution_plan_admission`, and `workflow_execution_plan_admission` are clean.
The remaining legacy cleanup work is now narrowed to queue runtime-admission
storage/helpers, retired runtime-load/session-admission diagnostics helpers,
`session_runtime_load_lifecycle`, `workflow_run_internal`, and the old media
artifactization conversion boundary.

2026-05-24 implementation status: unused queue prediction/update helper
deletion is complete. `queued_run_is_admission_candidate` and
`set_queue_decision_reason_if_present` were removed from the scheduler store,
leaving actual queue admission under `begin_queued_run` and scheduler policy.
Targeted source search for both helper names is clean. Remaining queue cleanup
is limited to stale fields and record payloads that are still present but no
active scheduler policy/read model consumes.

2026-05-24 implementation status: queued-run `timeout_ms` was reattached to the
canonical scheduler-task session runner. Non-runtime scheduler-task execution
now runs under the queued timeout and returns a typed `RuntimeTimeout` if the
duration is exceeded. This removes the `timeout_ms` dead-code surface without
keeping the old whole-run launch path alive. Runtime-host dispatch timeouts,
cancellation, attempt timing, and ledger-backed duration history remain later
scheduler/runtime-host lifecycle work.

2026-05-24 implementation status: session preflight cache cleanup is complete.
`WorkflowExecutionSessionPreflightCache` no longer stores capability models or
technical-fit decisions that no active scheduler path reads. Runtime-facing
preflight responses still compute technical-fit facts at the preflight API
boundary; the session cache is now limited to readiness invalidation and
blocking issue reuse. Tests were updated to verify that cache behavior directly
instead of using whole-run session execution as a proxy.

2026-05-24 implementation status: stale dequeued/finish-state payload cleanup
is complete. `WorkflowExecutionSessionDequeuedRun` no longer copies
`required_backends` or `required_models` from session state, and
`WorkflowExecutionSessionRunFinishState` no longer returns the redundant
`workflow_id`. The canonical session affinity facts remain on
session/preflight state and admission/placement projections; the runner handoff
now carries only fields consumed by active scheduler-task execution.

2026-05-24 implementation status: retired runtime-load lifecycle cleanup is
complete. The unused `session_runtime_load_lifecycle` module and its private
model-lifecycle event request/helper were removed instead of being preserved as
an inactive compatibility diagnostic path. Runtime load lifecycle diagnostics
remain a later scheduler/runtime-host lifecycle responsibility so they can
carry task/runtime handoff correlation rather than old session-admission facts.
The broader `workflow::tests::session_capacity` suite still fails against the
current cutover state because it expects legacy runtime/session capacity
behavior; those tests must be converted or deleted with the remaining lifecycle
and source-input materialization work.

2026-05-24 implementation status: retired session-admission diagnostics helper
cleanup is complete. `session_execution_api.rs` no longer carries unused
scheduler delay/admitted/reservation writers, runtime-load error record
builders, retry timestamp helpers, queued graph-settings decoding, or
technical-fit trace mapping reachable only from those retired writers. The
remaining warning surfaces are narrowed to active execution-plan storage, the
old whole-run `workflow_run_internal` path, and the old media artifactization
conversion boundary.

2026-05-24 implementation status: old whole-run execution cleanup is complete.
Workflow-service no longer declares `workflow_run_api` or
`artifact_output_conversion`, no longer exposes host-injected media conversion
configuration, no longer depends on `pantograph-media-conversion`, and no
longer has private tests for direct whole-run host execution. Artifact/media
output handling is now a future scheduler-task result materialization and
runtime-host output projection concern. The remaining warning surface is the
active execution-plan storage read by embedded-runtime planned inference; that
is a cross-crate bridge replacement boundary, not a local workflow-service
dead-code deletion.

2026-05-31 media conversion ownership decision: delete the unused
`src-tauri/src/workflow/managed_media_conversion.rs` adapter and its module
export in the next cleanup slice. Keeping the adapter behind Tauri would
preserve business logic in the desktop shell after workflow-service removed the
canonical injection point. A future media conversion feature must be planned as
a backend-owned service boundary with typed request/result/diagnostic
contracts, then attached to scheduler task-result materialization or
runtime-host output projection. That future service may receive Tauri-provided
process/tooling infrastructure, but Tauri must not own conversion semantics or
act as the workflow conversion policy surface.

## Scheduler Runtime Handoff Replacement Replan

2026-05-25 decision: use option 3. Replace the active execution-plan
planned-inference bridge with scheduler-owned runtime handoff dispatch, then
delete the legacy bridge. This is a cross-crate replacement, not a compatibility
period.

Current conflict:

- Workflow-service still stores an active `WorkflowExecutionPlan` for a run.
- `pantograph-embedded-runtime` planned inference reads that stored plan,
  projects a node decision into an inference backend decision, resolves Pumas
  package facts and artifact load target, then calls the inference gateway.
- That makes the embedded runtime a secondary runtime-decision path and keeps a
  whole-run plan bridge alive after scheduler-task execution has become the
  canonical workflow progress owner.

Replacement contract:

- Workflow-service scheduler-task execution consumes the existing
  `SchedulerRuntimeHandoff`, `RuntimeHostExecutionRequest`,
  `RuntimeHostExecutionPort`, and `SchedulerRuntimeHostDispatcher` contracts
  rather than adding parallel handoff or runtime-host DTOs.
- The handoff is built only after upstream scheduler task inputs are
  materialized, binding resolution produces a valid `SchedulableTaskIntent`,
  dependency readiness admits the task, and scheduler policy produces a
  dispatch decision.
- The handoff must remain path-free. It carries workflow/run/node/task
  correlation, typed task intent, readiness proof, dependency environment,
  bounded diagnostics, and a scheduler dispatch decision with selected
  runtime/device/model facts. It must not carry executable Pumas load targets,
  local graph model paths, frontend `modelPath` values, reduced execution-plan
  projections, node-engine internals, worker launch data, or an
  executor-selected runtime.
- Runtime-host execution resolves final Pumas load-target readiness from the
  scheduler-selected `PumasModelRef` inside the dispatch decision. Executable
  artifact paths remain host-local facts and must not be projected back into the
  scheduler, workflow-service task graph, node engine, or graph editor.
- Runtime selection from graph nodes is a hard scheduler constraint only when
  explicitly provided; otherwise the scheduler policy selects the runtime.
  Either way, the inference crate receives the selected runtime only through
  the scheduler-built handoff.

Staged implementation:

1. Consume the existing workflow-service task-result, active-run result
   storage, binding-resolution, scheduler handoff, and runtime-host dispatch
   contracts. Remove staged `dead_code` allowances as each integration point
   starts using the existing boundary.
2. Add or wire the production scheduler dispatch-decision selector inside
   `pantograph-scheduler`. Workflow-service may request dispatch selection and
   build the validated handoff envelope, but it must not choose runtime policy,
   device policy, batching, or historical-score behavior locally.
   2026-05-25 replan decision: use option 2 with option 3 discipline. Add the
   minimal scheduler-owned dispatch-selection request/response now, while
   shaping it as the future provider-consumed candidate boundary. The selector
   request must carry a validated task intent, readiness proof, environment ref,
   typed dispatch candidates, and bounded candidate-source diagnostics that
   explain evidence quality but are not accepted as scheduling facts. Each
   candidate must be path-free and must include only facts scheduler policy can
   legally rank or select: candidate id, runtime id, optional runtime variant,
   selected device ids, selected `PumasModelRef`, runtime trait settings,
   readiness/resource/residency/history summary facts that already exist as
   typed contracts, and explicit reservation/batching facts only when they have
   been produced by their canonical owners. The new dispatch candidate is a
   stricter scheduler contract; do not reuse `RuntimeTechnicalFitCandidate`
   because that shape allows optional executable selection facts, derived ids,
   compatibility-path metadata, and candidate-id ordering behavior. Do not
   reuse `SchedulerBatchCandidate` because that shape is specific to an already
   selected batching group. If a required reservation, resource, or candidate
   fact is unavailable, the selector returns a typed diagnostic response and no
   `SchedulerDispatchDecision`; do not synthesize placeholder reservation ids,
   generic runtime candidates, CPU/auto fallbacks, or candidate-id tie-break
   selection as compatibility behavior.
   The response must either contain one validated `SchedulerDispatchDecision`
   or typed diagnostics explaining no valid dispatch. Explicit graph runtime or
   device inputs are hard requirements filtered by scheduler policy; omitted
   inputs leave the scheduler free to select among valid candidates. Equal or
   otherwise unrankable candidates must fail closed unless the request includes
   a deliberate scheduler policy reason such as exploration mode with a stable
   policy seed. Selection must stay synchronous and pure: it may validate and
   rank supplied typed facts, but it must not query Pumas, inspect runtime
   hosts, read history stores, hold workflow-service locks, or perform I/O.
   This keeps the immediate API small but prevents workflow-service or
   embedded-runtime from becoming the long-term candidate/provider owner.
3. Extend the runtime-host response contract with typed, path-free output
   values that workflow-service can map into `WorkflowSchedulerTaskResult`.
   Runtime-host contracts own the runtime response DTO; workflow-service owns
   the mapping into workflow task results. Do not make runtime-host contracts
   depend on workflow-service. The response extension must use a stable contract
   version, explicit output value enum variants, bounded outputs/diagnostics,
   `serde(deny_unknown_fields)`, `TryFrom` validation, typed error enums, and
   `#[non_exhaustive]` public enums where future runtime outputs are expected.
   If a runtime output cannot be represented by a typed variant, stop and plan
   the contract extension rather than adding `serde_json::Value`, arbitrary
   metadata maps, or stringified values.
4. Wire runtime task readiness through the existing
   `workflow_scheduler_resolve_task_intent` binding-resolution boundary so
   connected Pumas refs and other materialized inputs become a validated
   `SchedulableTaskIntent` before dispatch. Do not duplicate input
   materialization inside the runtime dispatch path.
   2026-05-29 Milestone 5d split decision: this runtime-dispatch milestone owns
   final runtime-host input materialization after scheduler input readiness and
   dispatch selection. It must consume upstream task results, graph literals,
   descriptor defaults, and scheduler-selected handoff facts through typed
   contracts; it must not derive executable inputs from raw graph
   `task_kind`/runtime/device fields, frontend/Tauri payloads, model paths,
   `inference_settings`, or retired whole-run runtime execution.
   The same dispatch-owned slice must align the runtime-host input request with
   descriptor materialization and run pre-dispatch descriptor revalidation. If
   Pumas facts, selected artifact state, runtime capability, or descriptor
   fingerprint changed since executable publish/admission validation, dispatch
   selection must stop with typed diagnostics rather than using stale ports or
   inventing replacement inputs.
5. Implement the embedded-runtime `RuntimeHostExecutionPort` by accepting a
   validated dispatch-selected runtime-host request, resolving the Pumas artifact
   load target from the dispatch-selected `PumasModelRef`, calling the inference
   gateway, and returning a completed or failed typed runtime-host response.
   This implementation must not query workflow-service active execution plans.
6. Wire runtime scheduler tasks through `SchedulerRuntimeHostDispatcher` from
   the dedicated scheduler-task session runner. The runner must transition or
   claim task state under the store lock, drop the lock before awaiting
   runtime-host execution, then reacquire the store lock to persist terminal
   state and `WorkflowSchedulerTaskResult`.
7. Delete `set_active_run_execution_plan`, `active_run_execution_plan`,
   `workflow_execution_session_active_execution_plan`, active execution-plan
   store tests, embedded-runtime active-plan lookup/projection helpers,
   `workflow_execution_plan_projection`, `EmbeddedPlannedInferenceExecutionHost`,
   node-engine `PlannedInferenceExecutionHost`, and the image-generation success
   branch that reconstructs dispatch from a whole-run execution plan.
8. Update crate READMEs and this plan to state that runtime dispatch has one
   source of truth: scheduler task state plus dispatch-selected
   `SchedulerRuntimeHandoff`.

Verification for the staged replacement:

- Start each cross-layer slice with a focused failing contract or acceptance test
  that exercises the externally meaningful input/output for that slice before
  implementation expands shared layers.
- Targeted `rg` proves no production code reads active execution plans for
  runtime dispatch after the deletion stage.
- Workflow-service tests cover binding-resolution-driven runtime readiness,
  handoff construction from scheduler task state and scheduler dispatch
  decisions, runtime-host response to `WorkflowSchedulerTaskResult` mapping,
  missing/ambiguous runtime diagnostics, explicit runtime constraint
  enforcement, and no graph-local path fallback.
- Scheduler tests cover production dispatch-decision creation, including graph
  runtime constraints as hard requirements, scheduler-selected runtime behavior
  when the graph leaves runtime implicit, and no workflow-service-local policy
  selection.
- Embedded-runtime tests cover handoff-to-gateway execution and Pumas
  unavailable/invalid/stale load-target diagnostics with runtime/task
  correlation.
- Runtime-host contract tests cover output payload validation, response/request
  correlation, and rejection of path-shaped or executable load-target response
  data.
- Runtime-dispatch orchestration tests cover lock boundaries and idempotency:
  no active-run store lock is held across runtime-host awaits, stale state or
  duplicate terminal completion fails closed, and cancellation after dispatch
  cannot leave a completed-without-result or result-without-completed state.
- Session execution tests cover runtime tasks staying fail-closed before
  handoff wiring, executing only through the handoff after wiring, persisting
  typed task results, and deleting the old planned-inference success path.
- `cargo fmt`, focused tests for touched crates, default/no-default/all-feature
  checks for touched crates, and `git diff --check` must pass for each slice.

## Task Result Materialization Plan

The option 2 materialization boundary is now staged in code and must be consumed
by the scheduler-task runner:

- `WorkflowSchedulerTaskResult`, active-run result storage, source-input
  materialization, non-runtime result conversion, output projection, and
  dependency binding resolution already exist as focused workflow-service
  boundaries. The next slices must wire these into the dedicated session runner
  and remove their scoped staging `dead_code` allowances.
- Runtime-host execution still needs a typed, path-free output payload that can
  be mapped into `WorkflowSchedulerTaskResult`. The payload may carry output
  port ids, scalar values, media/artifact refs, diagnostics, and terminal
  metadata, but must not contain local model paths, executable Pumas load
  targets, worker launch details, runtime handoff internals, or raw node-engine
  values.
- Floating-point generation settings remain blocked until the scheduler trait
  value contract grows an explicit float variant; do not stringify floats
  silently. If a materialization or trait slice needs guidance scale, denoise
  strength, or other float-like generation settings, stop and add the typed
  scheduler/workflow contract extension first.
- Binding resolution consumes `WorkflowSchedulerTaskGraph` input bindings and
  materialized upstream results. It emits valid scheduler intent only after all
  required typed values exist and validate.
- Unresolved, wrong-type, unavailable, invalid, or ambiguous materialized inputs
  must become typed diagnostics and scheduler task states. Do not fall back to
  graph fields, `ModelRefV2`, `model_path`, frontend `modelPath`, active
  execution plans, or whole-workflow output-node demand.
- Keep durable event-sourced task-result ledger replay as the later option 3
  objective. The active-run contract must be serializable and replay-safe so
  the later durable implementation changes storage, not semantics.

## Materialization Standards Guardrails

- Keep new materialization code in focused workflow-service modules rather
  than growing existing scheduler, store, workflow, or runtime adapter files.
  New modules should stay below the repository decomposition threshold, and
  existing oversized modules may only receive narrow integration calls or
  deletion work needed by the slice. The expected workflow-service modules are
  separate task-result contracts, active-run task-result storage, and
  dependency binding resolution; do not add this logic to existing queue,
  session execution, node-engine inference, or planned-inference host files.
- Public Rust contracts must be typed and validated at construction or
  conversion boundaries. Use stable schema constants, typed ids/status enums,
  explicit value variants, `serde` field names that match existing contracts,
  and public enum extension discipline where future runtime/model families are
  expected. Production code must return typed errors instead of `String`
  errors and must not use `unwrap` or `expect`.
- Materialized values must be explicit contract variants, not incidental
  metadata maps. Do not encode unsupported values as strings. If a required
  value type is missing from scheduler or workflow-service contracts, stop the
  slice and plan the typed extension before implementation.
- Workflow-service owns task-result materialization, active-run result
  storage, dependency binding resolution, and user-visible diagnostics.
  Scheduler owns queue policy and task state transitions. Node-engine owns
  graph validation and non-runtime single-task execution from already
  materialized inputs. Runtime host owns Pumas load-target resolution and
  executable runtime dispatch.
- Runtime-host response payloads are runtime contracts, not workflow-service
  task results. Map them at the workflow-service boundary into
  `WorkflowSchedulerTaskResult` so the runtime host never depends on
  workflow-service and workflow-service never receives executable load-target
  state.
- Runtime-host response output variants should be an intentionally mapped subset
  or sibling contract, not a copied workflow-service DTO. Keep the mapping in a
  focused workflow-service module with exhaustive tests so future runtime output
  variants fail closed until the workflow task-result contract deliberately
  supports them.
- Active-run result storage is staged state, not the final durability claim.
  The result DTO must carry enough workflow/run/node/task correlation,
  versioning, status, and diagnostics for later diagnostics-ledger replay
  without changing graph editor, node-engine, scheduler, or runtime-host
  semantics. Durable ledger replay is a later replacement for this storage,
  not a second successful execution path.
- Frontend task-state and option-provider work must use focused DTO,
  presenter, and query modules. Do not grow large existing type, scheduler
  page, I/O inspector, or presenter files except for narrow integration calls.
  Keep graph editor context aligned to typed scheduler constraints and display
  disabled/unavailable capability reasons without owning runtime policy.
- Diagnostics-ledger task-result durability must be decomposed into focused
  task-result event, projection, and sqlite modules when that later slice is
  implemented. Existing large ledger event/sqlite/test files may only receive
  narrow dispatch or re-export hooks.
- Legacy execution files and successful branches are deletion targets after
  replacement. Do not expand `PlannedInferenceExecutionHost`, reduced
  execution-plan launch projection, `ModelRefV2`, `model_path`, or frontend
  `modelPath` paths while implementing materialization or orchestration.
- Async code must keep a small shell around synchronous policy. Do not hold
  session-store or result-store locks across await points; copy immutable
  records out of locks before dependency readiness, runtime-host dispatch,
  ledger writes, or node-engine calls. Use bounded queues and explicit
  cancellation/task state rather than shared mutable control flags.
- Shared contracts, generated DTOs, lockfiles, saved workflow fixtures, README
  updates, and plan files remain serial integration-owner work. Sub-agents may
  only take non-overlapping adapter or test slices with a report path.
- Each slice must include focused contract/storage/binding tests plus any
  README updates for new modules. Cross-layer slices need vertical acceptance
  coverage proving no graph-local paths, no reduced-plan handoff synthesis, and
  no node-engine output-demand runtime fallback. Verification must include
  relevant Rust format/check/test commands, default/all-features/no-default
  feature checks when public feature contracts change, and `git diff --check`.
- New or ownership-changing source directories must update README files in the
  same slice. Contract crates and host-facing modules must document API consumer
  contracts, structured producer contracts, lifecycle ordering, error semantics,
  compatibility/versioning, and rejected alternatives instead of relying only on
  type names.

2026-05-25 standards pass: the replan has been reviewed against the local plan,
architecture, Rust API, async/concurrency, testing, tooling, and documentation
standards. Implementation remains standards-compliant only if the runtime-host
response extension stays typed/validated/path-free, scheduler policy stays in
`pantograph-scheduler`, workflow-service acts as the async orchestration shell,
runtime-host executable load targets remain host-local, and every retired
planned-inference/active-plan surface is deleted rather than retained as a
fallback.

2026-05-25 implementation update: the first runtime-host response contract
slice is complete. `pantograph-runtime-host-contracts` now exposes typed,
bounded, path-free response output values and optional terminal metadata, with
fixture-backed validation for completed outputs and path-shaped rejection. This
unblocks the later workflow-service mapping slice without making the runtime
host depend on workflow-service task-result DTOs. Runtime execution is still
not wired; runtime tasks must remain fail-closed until scheduler-selected
handoff dispatch and workflow-service result mapping are implemented.

2026-05-25 implementation update: the workflow-service mapping slice is now
complete. A focused mapper converts validated terminal runtime-host responses
into `WorkflowSchedulerTaskResult` values, rejects accepted non-terminal
responses, and fails closed for unsupported future runtime-host variants.
The staged orchestrator handoff method now maps dispatcher responses through
that boundary, but production session progression still does not dispatch
runtime tasks until the scheduler-selected handoff wiring slice is implemented.

2026-05-25 production dispatch-selection replan: the next runtime-handoff
implementation boundary is scheduler-owned creation of `SchedulerDispatchDecision`.
The repository has a dispatch decision DTO and validation, but not a production
selector input that provides valid runtime/device/model candidates without
fallback behavior. The next code slices should first add
`SchedulerDispatchSelectionRequest`, `SchedulerDispatchCandidate`, and
`SchedulerDispatchSelectionDecision` or equivalent names in `pantograph-scheduler`,
with contract tests proving explicit runtime/device constraints are hard
requirements, missing candidates fail closed, unrankable ambiguity fails closed,
and a single valid candidate produces a validated `SchedulerDispatchDecision`.
Later option 3 work can replace the caller-supplied candidate list with runtime
registry/resource observer/Pumas provider composition without changing scheduler
selection semantics.
Codebase review tightened the slice: the new candidate type must not alias
runtime-registry technical-fit candidates or scheduler batching candidates, and
the no-selection response should carry explicit scheduler diagnostic codes such
as no candidates, incompatible explicit runtime/device requirement, missing
reservation, missing resource fit, invalid candidate evidence, duplicate
candidate id, and ambiguous ranking. Reservation lease ids in a successful
dispatch decision must come from a real resource/reservation owner fact; a
placeholder lease is a contract violation. Workflow-service may orchestrate
fact collection, but it must not rank candidates or construct the successful
decision directly.
Standards gate for this slice: implement the selector in a focused scheduler
module such as `dispatch_selection.rs`, re-export only the public contract from
`lib.rs`, and update `crates/pantograph-scheduler/README.md` plus
`src/README.md` in the same slice. Public DTOs must use explicit contract
versions, typed ids, typed diagnostic enums, `serde(deny_unknown_fields)`,
`TryFrom` validated wrappers, `#[must_use]` on validation/selection results,
and `#[non_exhaustive]` where future selection states or diagnostics may grow.
Do not use `serde_json::Value`, arbitrary string maps, `Result<T, String>`,
panics, or `unwrap`/`expect` in production selector paths. Keep the module under
the repository decomposition threshold or split public contract, policy, and
validation modules before continuing.
Do not add dependencies or manifest/lockfile changes for this selector unless a
standards note first records the owner crate, transitive cost, feature impact,
and verification plan. Verification must include public fixture/serde tests for
the new contract, focused selector unit tests, `cargo fmt`, default/all-features
and no-default-features `cargo check` for touched public crates, targeted
legacy/path-field searches, and `git diff --check`.

2026-05-25 dispatch-selection contract implementation completed in
`pantograph-scheduler`. The scheduler now owns strict path-free dispatch
selection DTOs and a pure synchronous selector that produces a validated
`SchedulerDispatchDecision` only when exactly one candidate satisfies explicit
runtime/device requirements and carries real reservation/resource-fit facts.
Missing candidates, duplicate ids, missing required facts, incompatible hard
constraints, rejected resource fits, and ambiguous eligible candidates return
typed no-selection diagnostics. Workflow-service still must wire production
runtime tasks by collecting canonical candidate facts and calling the scheduler
selector before runtime-host dispatch; graph editor and node-engine remain
path-free and do not choose runtime/device policy.

2026-05-25 workflow-service dispatch-selected handoff bridge completed.
`WorkflowSchedulerTaskOrchestrator` can now consume a validated scheduler
dispatch-selection request, call `pantograph-scheduler` selection, convert only
a selected `SchedulerDispatchDecision` into `SchedulerRuntimeHandoff`, and pass
that handoff through the shared runtime-host dispatcher. If scheduler selection
returns no-selection diagnostics, workflow-service stops before runtime-host
execution. This keeps graph editor and node-engine abstracted away from runtime
paths and keeps workflow-service as the async orchestration shell rather than
the runtime/device policy owner. Production session execution still needs the
next slice to assemble validated dispatch-selection requests from canonical
runtime/resource/Pumas facts for ready runtime tasks.

2026-05-25 dependent runtime task readiness initialization completed. Runtime
inference tasks with upstream dependencies now initialize as `AwaitingInputs`
instead of `Ready`, even when the graph already provides a valid
schedulable intent. This keeps runtime dispatch behind scheduler-owned
input-readiness materialization and prevents inference nodes from running
before connected upstream task results are available.

2026-05-29 session runner dependency-readiness admission wiring completed.
The workflow-service session runner now builds validated dependency readiness
requests for runtime tasks that reached `WaitingDependencyReadiness`, resolves
the configured readiness provider outside the session-store lock, applies
scheduler readiness admission, and only then checks for dispatch readiness. The
default provider is the no-I/O not-implemented dependency-environment service,
so runtime-containing session runs still fail closed before runtime-host
dispatch rather than using old runtime load, node-engine output demand, reduced
execution-plan handoff synthesis, or path-shaped dependency preflight. The
remaining production orchestration work is to provide real dependency
readiness evidence, collect canonical dispatch candidates, call scheduler
dispatch selection, and construct runtime-host requests from the
dispatch-selected `SchedulerRuntimeHandoff`.

2026-05-29 dependency-environment provider composition hook completed.
`WorkflowService` can now be configured with a canonical
`DependencyEnvironmentProvider` for scheduler-owned dependency readiness
admission. Focused session coverage proves the default not-implemented
provider still stops before dispatch, while an injected ready provider admits
the runtime task to the dispatch boundary and still fails closed because
runtime-host dispatch selection/request construction is not wired. This is not
a production dependency resolver and does not introduce legacy preflight,
runtime path, or readiness-proof injection surfaces; it only gives the backend
composition root a typed provider seam for the next production provider slice.

2026-05-29 production dependency-readiness source re-plan selected. The next
implementation direction is a snapshot-backed dependency-environment provider
owned by backend composition. The provider reads validated, path-free,
backend-owned readiness snapshots and returns canonical
`DependencyEnvironmentResult` values to the existing workflow-service
readiness lifecycle. It must not perform blocking package/runtime probes in the
session runner path, create a Tokio runtime, spawn untracked tasks, or derive
readiness from `ModelDependencyRequest`, `ModelRefV2`, technical-fit previews,
reduced execution plans, graph node data, Tauri/frontend payloads, or local
paths. Embedded-runtime or another infrastructure owner may later populate the
snapshots through an async lifecycle with tracked tasks, cancellation,
shutdown, retries, and tracing; that producer is a separate slice and must not
move scheduler policy or readiness proof authority into runtime adapters. The
current injected-ready provider remains test/dev scaffolding only and must not
be treated as production readiness authority while dispatch selection and
runtime-host request construction are wired.

2026-06-03 dependency-readiness lifecycle refinement: the earlier
synchronous-provider first-run option is superseded. The current session runner
can now enqueue dependency-readiness work, record deferred scheduler state, keep
the active run open, and return `RuntimeNotReady` before dispatch when no fresh
proof exists. The next complete production inference-session slice is an
explicit backend-owned resume command for an existing active `session_id` plus
`workflow_run_id`. It must validate that the run is still active and
dependency-readiness pending, use scheduler task state as the source of truth,
retry dependency-readiness admission from fresh canonical backend facts, and
continue toward dispatch or return typed pending/fail-closed diagnostics. This
still does not permit graph-path, Tauri/frontend, `ModelDependencyRequest`,
`ModelRefV2`, reduced-plan, selector-summary, static-ready, synchronous-probe,
client-rerun, or caller-supplied-proof fallback. After the first complete
production inference path is proven through that explicit backend resume path,
add the composition-root-owned backend worker/listener lifecycle that resumes
waiting tasks when readiness facts arrive and owns freshness, timeout,
cancellation, retry, reservation release, shutdown, overlap prevention, and
observability.

2026-06-03 manual Scheduler proof update: the explicit backend resume path now
has an operator-visible read-model and UI command surface. The typed
`workflow_execution_session_resume_state` field is appended to diagnostics
run-list/run-detail records but is populated by workflow-service from live
active-run scheduler task state rather than derived from durable ledger strings
or frontend status heuristics. The Scheduler page action only displays that
typed backend fact and forwards `session_id` plus `workflow_run_id` to the
existing backend command. The next production lifecycle slice remains the
composition-root-owned event-driven backend readiness worker/listener.

2026-06-03 auto-resume lifecycle re-plan resolution: choose the
embedded-runtime lifecycle-handle path for the next implementation slice. The
handle belongs beside the existing dependency-readiness snapshot producer and
owns a bounded poll/resume loop over workflow-service resume candidates. It
must call the existing backend resume API with the embedded backend host,
prevent concurrent resumes for the same active run, treat still-pending
runtime-not-ready results as non-terminal, log typed failures at the lifecycle
owner, and shut down idempotently. Rejected: a Tauri-owned loop, because that
complects transport/startup with scheduler readiness policy and violates the
thin binding/composition-root standards. Deferred improvement: add a typed
snapshot notification channel and convert the loop from polling to event-first
delivery once the working lifecycle is in place.

2026-05-30 runtime dispatch-selection boundary slice completed. Workflow-service
now has a focused runtime dispatch candidate provider seam and a path-free
request assembly helper that combines an admitted runtime task, the readiness
proof produced during admission, and provider-supplied scheduler dispatch
candidates into a validated `SchedulerDispatchSelectionRequest`. Session
execution now reaches scheduler dispatch selection for runtime tasks that pass
input readiness and dependency readiness. The default provider intentionally
returns no candidates, so the scheduler selector returns typed no-selection
diagnostics and workflow-service fails closed before runtime-host dispatch
instead of reporting a generic "dispatch not wired" branch or using legacy
graph/node-engine/reduced-plan execution. The slice also records the important
state-lifecycle invariant discovered during implementation: readiness proof
must be carried forward from the admission step because a task in `Ready` state
no longer has the `WaitingDependencyReadiness` context needed to rebuild the
readiness request safely.

Remaining production runtime dispatch work at that point:

- Add the production provider that gathers canonical runtime/resource/Pumas
  dispatch candidates without ranking them.
- Add the runtime-task running/completion state transitions and result
  persistence path before allowing non-empty candidate sets to reach
  runtime-host dispatch from session execution.
- Replace the temporary fail-closed terminal transition label/message used for
  no-selection with scheduler dispatch-selection diagnostics in task state.
- Keep the successful runtime-host execution path blocked until the selected
  handoff, runtime-host response mapping, and scheduler task-result persistence
  are one validated vertical slice.

2026-05-30 runtime task persistence helper slice completed. The
workflow-service scheduler task orchestrator now has focused helpers to move a
ready runtime inference task to `Running` and to atomically persist a terminal
runtime-host task result with the `Completed` task-state transition. This
keeps runtime task state/result persistence in the same backend-owned store
path used by non-runtime tasks and avoids making session execution or
runtime-host adapters mutate scheduler state directly. The helpers are not yet
used by session execution for successful runtime dispatch; that remains
blocked until production candidate collection, dispatch-selected handoff
execution, and result persistence are wired in one validated slice.

2026-05-30 injected runtime dispatch completion slice completed. Session
execution now uses the runtime task start/completion helpers around
`pantograph-scheduler` dispatch selection and the shared runtime-host execution
port, then projects completed scheduler task results into requested workflow
outputs. This removes the previous artificial non-empty-candidate blocker from
the session runner, but does not add a production candidate source. The default
runtime dispatch candidate provider still returns no candidates, so production
runtime-containing runs fail closed through scheduler no-selection before any
runtime-host call unless a real canonical provider is explicitly configured.
The successful-path coverage uses a test-only provider with explicit typed
runtime, device, model, reservation, and resource-fit facts from the saved
executable validation snapshot and scheduler task intent. That test scaffolding
must not become production policy, runtime discovery, or fallback synthesis.

Remaining production runtime dispatch work after this slice:

- Add the production provider that gathers canonical runtime/resource/Pumas
  dispatch candidates without ranking them or reading graph paths, frontend
  state, reduced execution plans, `ModelRefV2`, or runtime-host load targets.
- Add durable recovery/replay/cancellation/duplicate-dispatch prevention and
  reservation-release behavior before real multi-run inference workloads rely
  on this in-memory completion path.

2026-05-30 scheduler no-selection task diagnostics slice completed. When
scheduler dispatch selection returns no-selection for a started runtime task,
workflow-service now persists the scheduler dispatch-selection diagnostics on
the runtime task's terminal failure state instead of using the generic
runtime-dispatch-not-wired diagnostic. This keeps the default no-candidate
production path fail-closed before runtime-host dispatch while preserving the
typed scheduler reason, such as `NoCandidates`, for graph editor/run
inspection consumers. Non-selection still does not create candidates, choose a
fallback runtime, call runtime-host execution, or route through node-engine.

Remaining production runtime dispatch work after this slice:

- Add the production provider that gathers canonical runtime/resource/Pumas
  dispatch candidates without ranking them or reading graph paths, frontend
  state, reduced execution plans, `ModelRefV2`, or runtime-host load targets.
- Add durable recovery/replay/cancellation/duplicate-dispatch prevention and
  reservation-release behavior before real multi-run inference workloads rely
  on this in-memory completion path.

2026-05-30 Pumas image package-facts bridge re-plan selected. Investigation of
the current Pumas-Library checkout confirmed that Pumas now exposes the image
model facts Pantograph needs as factual package evidence: versioned `PumasModelRef`,
artifact kind/storage/validation facts, task modalities, Diffusers pipeline
class/version/name, image-generation family evidence, Diffusers component
roles, generation defaults, custom-code evidence, backend hints, summaries, and
cache/update-feed support. The next production runtime-dispatch work is
therefore not to create new Pumas image facts or to promote test candidate
facts into production. It is to add a Pantograph production bridge that consumes
Pumas owner-API full package facts, validates and projects only the path-free
facts needed by inference planning/runtime compatibility, and returns typed
diagnostics when those facts are unavailable, stale, ambiguous, path-carrying,
or only available through a selector summary.

The first production bridge slice must preserve these boundaries:

- Pumas remains the factual model/package producer. Pantograph must not infer
  image family, supported task, selected artifact, or backend hints from graph
  paths, display names, reduced execution plans, `ModelRefV2`, frontend/Tauri
  state, or runtime-host load targets.
- `pantograph-embedded-runtime` is the initial concrete owner for Pumas
  owner-API access because it already depends on `pumas-library`.
  `pantograph-workflow-service` must stay host-agnostic and receive injected
  providers/projections rather than adding direct Pumas API ownership.
- Full package facts are owner-API-only for the first slice. Current
  local-client/read-only selector access explicitly cannot return full package
  facts; those modes must fail closed with typed diagnostics instead of
  falling back to summaries or cached display rows as executable authority.
- Pumas facts are not scheduler dispatch candidates by themselves. Candidate
  creation remains blocked until the same provider path can combine validated
  package facts with runtime-registry capability facts plus real
  scheduler/resource-owner reservation and resource-fit facts.
- `PumasModelRef.selected_artifact_path`, artifact entry paths, and Pumas load
  targets must be rejected or stripped before scheduler dispatch. Runtime-host
  load-target resolution remains the host-boundary responsibility after a
  scheduler-selected handoff exists.

Next thin implementation slice: add the production Pumas package-facts bridge
contract and owner-API implementation in the embedded-runtime composition
layer, with workflow-service tests proving that missing/unavailable bridge
facts produce typed no-candidate diagnostics and do not fabricate scheduler
candidates. Do not produce non-empty production dispatch candidates until real
reservation/resource-fit ownership is implemented.

2026-05-30 owner-API Pumas package-facts bridge slice completed. The
embedded-runtime composition layer now has a staged Pumas dispatch package-facts
bridge that resolves full package facts only from owner selector access,
normalizes Pumas' versioned model-ref field into Pantograph's dependency
planning contract, rejects path-carrying input model refs, strips path fields
from projected facts, and returns typed diagnostics for missing access,
unsupported local-client/read-only full-fact access, lookup/decode failures,
stale package-facts contracts, selected-artifact mismatches, and stripped path
facts. Focused tests prove owner API Diffusers facts project into a path-free
bridge shape, read-only access does not promote summaries, and path-carrying
model refs fail before Pumas lookup.

This slice intentionally does not create scheduler dispatch candidates. The
bridge remains staged until the next dispatch-provider slice combines these
package facts with runtime-registry capability facts and a real
resource-owner reservation/resource-fit source. The temporary module-level
dead-code allowance is limited to this staged bridge and must be removed when
the production candidate provider consumes it.

2026-05-30 dispatch source-diagnostics propagation slice completed. The
workflow-service runtime dispatch candidate provider boundary now returns a
candidate set containing typed candidates plus bounded source diagnostics, and
workflow-service forwards those diagnostics into
`SchedulerDispatchSelectionRequest`. Scheduler dispatch selection preserves
request-level source diagnostics on no-selection decisions and still adds a
typed `NoCandidates` diagnostic when the candidate set is empty. This lets the
next production provider report missing Pumas package facts, runtime capability
facts, reservation facts, or resource-fit facts without fabricating a scheduler
candidate or hiding the reason behind a generic no-candidates result.

Re-plan boundary: production candidate fact collection cannot be wired directly
through the current synchronous workflow-service provider without deciding async
ownership. Owner-API Pumas package-fact resolution is async, and real
reservation/resource-fit facts must come from backend resource owners rather
than scheduler policy or workflow-service inference. Before implementing the
provider, choose whether fact collection becomes an async provider boundary, a
precomputed validated candidate-fact snapshot, or an async source-provider
composition that hands a validated path-free bundle to the existing synchronous
scheduler selector. Do not create candidates from summaries, cached display
rows, graph paths, or blocking ad hoc Pumas calls while this boundary is open.

2026-05-30 re-plan decision: use async source-provider composition for the
production dispatch candidate provider. The immediate implementation target is
not a precomputed persisted snapshot. Instead, the composition layer will call
explicit async source providers for Pumas package facts, runtime-registry
capability facts, and resource-owner reservation/resource-fit facts, then build
a validated path-free candidate-fact bundle for the existing synchronous
scheduler selector. Scheduler policy stays pure and synchronous; source
providers own infrastructure access and lifecycle freshness.

The candidate-fact bundle is the immediate contract boundary. It must contain
only facts that are legal to project into `SchedulerDispatchCandidate`: runtime
id and optional runtime variant, selected device ids, selected `PumasModelRef`,
runtime trait settings, dependency environment ref, real reservation lease,
real resource-fit assessment, optional batching fact when owned by the batching
source, and bounded source diagnostics. It must reject executable Pumas load
targets, local paths, selector summaries, display rows, graph-authored fallback
values, runtime-host internals, and arbitrary JSON.

Implementation order:

1. Define the validated final candidate-fact bundle and candidate-provider
   injection point without emitting non-empty production candidates.
   2026-05-30 slice completed: workflow-service now has the staged validated
   path-free candidate-fact bundle. The bundle validates typed facts and
   rejects path-carrying model refs or duplicate candidate ids, but its only
   current conversion into the dispatch candidate set preserves diagnostics
   and emits zero candidates until all required source adapters are wired.
   2026-05-30 API boundary follow-up completed: the final bundle/provider
   contracts and the runtime dispatch candidate-provider injection point are
   now public workflow-service API so embedded-runtime composition can inject
   a concrete final provider without moving infrastructure access into
   workflow-service.
   2026-05-30 source ownership correction: concrete async source-provider
   traits belong in embedded-runtime composition, not workflow-service API. A
   Pumas-only source cannot truthfully return the final bundle because final
   candidate facts require runtime capability plus real reservation/resource
   fit evidence from other owners.
2. Adapt the staged Pumas owner-API package-facts bridge into an
   embedded-runtime Pumas source provider and preserve
   missing/unsupported/stale facts as source diagnostics.
   2026-05-30 slice completed: embedded-runtime now has a narrow
   `PumasDispatchPackageFactsSource` that owns optional Pumas selector access,
   delegates to the owner-API bridge, and preserves path-free projected facts
   or typed source diagnostics. It does not create workflow-service final
   candidate bundle rows or scheduler candidates.
3. Add runtime-registry capability source projection.
   2026-05-30 slice completed: embedded-runtime now has a staged
   `RuntimeDispatchCapabilityFactsSource` that snapshots the shared runtime
   registry into path-free source facts and typed diagnostics. It does not
   join with Pumas facts or create final dispatch candidates.
4. Add real resource-owner reservation/resource-fit source projection.
   2026-05-30 re-plan boundary: this cannot be implemented as a truthful
   candidate source until the resource-owner contract is selected. Scheduler
   dispatch candidates require real scheduler-shaped reservation and fit facts,
   while the current runtime-registry lease alone does not expose selected
   scheduler device id, resource kind, and reserved bytes in a way the final
   provider can safely project without a lifecycle/release owner. Choose
   between fail-closed diagnostics, a runtime-registry-owned reservation source
   with explicit selected device/resource requirements and release ownership,
   or a dedicated scheduler resource-reservation service. Do not emit
   placeholder reservations from dry-run checks or snapshots.
   2026-05-30 decision: use runtime-registry as the immediate resource owner,
   but only through an embedded-runtime resource source that receives explicit
   selected runtime, selected scheduler device, workflow/run/task ids, typed
   resource requirements, and reservation owner id from final-provider
   assembly. The source must acquire/reuse a real lease, map only owned claims
   into scheduler reservation rows, return typed fit diagnostics on admission
   failure, and wire session-runner release/retention cleanup before any
   non-empty dispatch candidates can be emitted. A dedicated scheduler
   resource-reservation service remains the long-term target when resource
   policy needs to move beyond runtime-registry admission.
   2026-05-30 source slice completed: embedded-runtime now has a staged
   `RuntimeDispatchResourceFactsSource` that accepts explicit selected
   runtime/device/task/resource requirements, acquires or reuses real
   runtime-registry reservations, projects owned claims into scheduler
   reservation facts, and returns scheduler fit diagnostics on admission
   failure. It still does not join Pumas/runtime capability facts or emit
   scheduler candidates until release/retention cleanup ownership is wired.
5. Map the validated bundle into `SchedulerDispatchCandidate` values only when
   every required source fact is present and valid.

Precomputed validated candidate-fact snapshots remain the likely future
durability/replay layer, but they are deferred until the option 3 bundle exists
and real inference workloads expose concrete replay, recovery, and freshness
requirements. Until then, missing source facts must produce typed
no-candidate diagnostics rather than synthetic candidates, blocking calls in
scheduler policy, or summary/cache-row execution authority.

## Effects On Existing Systems

### Scheduler

The scheduler moves from contract-only dynamic dispatch plus session-level
admission into the active owner of task progress. It must expose stable
contracts for task graph state, task transitions, readiness, dispatch, resource
leases, batching groups, retry/defer decisions, and terminal diagnostics.

The policy core should remain small and replaceable. Runtime ranking,
batching, memory fit, and history weighting can change without modifying graph
editor, node-engine, runtime host, or inference adapters.

### Node-Engine

Node-engine becomes simpler at the runtime boundary. It validates graph shape,
produces or supports path-free task intent, and executes non-runtime graph
tasks from materialized inputs. Runtime inference nodes no longer call
`PlannedInferenceExecutionHost`; they wait for scheduler-owned task state and
consume task results.

Missing scheduler task state is a closed failure with typed diagnostics, not a
fallback to local dependency preflight or model paths.

### Graph Editor

The graph editor gains clearer backend-owned state. It shows optional runtime
and device inputs as user constraints, displays capability options and disabled
reasons, and renders task waiting/running/failure state from backend read
models. It does not need Pumas paths, dependency environments, resource
reservations, or runtime load targets.

### Runtime Host And Inference

Runtime host execution becomes reachable only through scheduler-dispatched
runtime-host requests. The inference crate receives selected runtime/device and
host-owned executable facts from that boundary, not graph inputs or node-engine
shortcuts.

## Verification Strategy

- Contract tests for task graph extraction and task-state payload validation.
- Contract and descriptor tests proving optional scheduler constraints are
  consistent across workflow-nodes, workflow-service projection, read models,
  and frontend option-provider context before materialization relies on
  `device`.
- Store transition tests for every scheduler task state and illegal transition.
- Recovery tests proving replay does not duplicate dispatch, leak
  reservations, or re-run completed tasks.
- Node-engine tests proving runtime inference nodes do not call planned
  inference and fail closed without scheduler task state.
- Runtime-host dispatch tests proving requests are built from actual
  dispatch-selected `SchedulerRuntimeHandoff` values.
- Session cutover tests proving non-runtime-only runs bypass runtime
  admission/load and legacy whole-run output demand, runtime-containing runs
  use scheduler-selected runtime-host dispatch or typed fail-closed
  diagnostics, request inputs are materialized into scheduler task results
  without graph mutation, and completed task results project through one typed
  output converter before requested-output validation.
- Multi-workflow tests covering task interleaving, batching compatibility, and
  fairness.
- Resource tests using deterministic observers and reservation leases before
  platform-specific collectors are trusted.
- Frontend/read-model tests proving task status is rendered from backend facts
  and not inferred optimistically.
- Search/deletion checks proving old successful branches are removed after the
  replacement path is wired.
- Size/decomposition checks during review for any touched oversized modules;
  new materialization, binding, frontend task-state, and ledger durability work
  must stay in focused files.

## No-Fallback Requirements

- Do not synthesize `SchedulerRuntimeHandoff` from reduced workflow execution
  plans, backend projections, graph inputs, or node-engine request context.
- Do not preserve whole-workflow output-node demand as the runtime inference
  launch path after scheduler task orchestration is wired.
- Do not let node-engine dependency preflight, `ModelRefV2`, `model_path`,
  or frontend `modelPath` become successful runtime identity.
- Do not expose executable Pumas load targets outside the runtime host.
- Do not place scheduler policy in graph editor, frontend adapters,
  node-engine, runtime adapters, Tauri commands, or inference worker code.
- Do not add compatibility shims for retired runtime execution contracts.

## Re-Plan Triggers

- Task graph extraction requires executable paths or full Pumas package facts.
- Graph-visible scheduler constraints cannot be represented consistently by
  workflow-node descriptors, workflow-service task graph projection, read
  models, and frontend option-provider context.
- Materialization needs a value type, such as floating-point generation
  settings, that is not present in typed scheduler/workflow contracts.
- Scheduler task orchestration reaches a point where runtime inference task
  intent depends on upstream materialized outputs that are not available at run
  admission.
- Node-engine cannot execute non-runtime tasks without owning scheduler policy.
- Runtime-host dispatch cannot be triggered from a dispatch-selected handoff.
- Durable task state cannot represent pause, retry, cancellation, or replay
  without whole-workflow static execution.
- Batching or multi-device resource admission requires graph/editor-visible
  execution facts.
- Scheduler policy requires async I/O inside the ranking/admission core rather
  than through an async shell.
