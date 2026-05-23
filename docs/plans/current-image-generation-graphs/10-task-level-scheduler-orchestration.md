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
- `SchedulerTaskStateRecord` and `SchedulerTaskTransition`: scheduler-owned
  durable lifecycle contracts that replace the current intent-required
  `SchedulerQueueTaskRecord` and `SchedulerQueueTransition` shapes. The record
  carries workflow/run/node/task correlation, state version, transition
  correlation, bounded diagnostics, and a state-specific payload. Pre-intent
  states such as awaiting materialized inputs, invalid graph projection, or
  input-unavailable must not carry `SchedulableTaskIntent`; schedulable states
  carry a validated intent as part of the state payload.
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
- `RuntimeHostExecutionRequest` / `RuntimeHostExecutionResponse` /
  `RuntimeHostExecutionPort`: shared runtime-host execution contracts moved
  out of `pantograph-embedded-runtime` into a lower-level contract crate. The
  request must carry only a dispatch-selected `SchedulerRuntimeHandoff`; it
  must reject readiness-only handoffs, reduced execution-plan projections,
  graph paths, local Pumas load targets, and worker launch metadata.
- `SchedulerTaskStateReadModel`: backend-owned status for graph editor,
  run-inspection, and diagnostics views. It exposes waiting/running/failed
  facts, not scheduler internals or executable load targets.

The names are planning names. Implementation may choose shorter local names,
but the ownership and data boundaries must remain explicit.

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
workflow-service binding resolution has validated materialized inputs. Scheduler
readiness, resource admission, batching, dispatch, and runtime handoff policy
must operate only on state variants that carry a validated intent.

Rejected approaches:

- Lazy-create scheduler records only after intent materialization. This hides
  blocked tasks from scheduler state and user-visible run status.
- Add `Option<SchedulableTaskIntent>` to the current record shape. This makes
  invalid state combinations representable and forces every policy consumer to
  rediscover whether a task is actually schedulable.
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
  equivalent typed payload structure where only schedulable states can contain
  `SchedulableTaskIntent`. Use stable contract/schema versions, typed ids,
  bounded diagnostics, `serde(deny_unknown_fields)`, `TryFrom` validated
  wrappers for raw persisted/IPC values, typed error enums, `#[must_use]` on
  validation/transition results, and `#[non_exhaustive]` where public states or
  diagnostics are expected to evolve.
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
   retryable failed, terminal failed, and completed. Queue-state transition
   coverage completed 2026-05-23 against the old queue contract, but the next
   implementation must replace that contract rather than preserving it as a
   compatibility layer.
3. Add task-state read models and diagnostics projection for graph editor and
   run inspection. Read models must join immutable task definition facts with
   scheduler-owned lifecycle state and must allow model/task-intent fields to
   be unknown before materialization. Initial path-free workflow-service
   read-model projection and dedicated active-run query boundary completed
   2026-05-23 against the old queue record; replacement read-model wiring must
   move to the phase-aware state contract.
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
   output demand.
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
   cancellation, retries, and shutdown.
9. Add non-runtime single-task execution through node-engine using
   materialized scheduler-owned inputs and task results. Do not use output-node
   demand to drive runtime inference.
10. Add runtime task dispatch from actual dispatch-selected
   `SchedulerRuntimeHandoff` into the shared runtime-host execution port.
11. Replace session execution so the scheduler task orchestrator, not
   node-engine output demand, advances workflow progress.
12. Remove planned-inference launch ownership and legacy resolver/path
   successful branches once task orchestration and runtime-host dispatch are
   wired.
13. Add recovery, replay, cancellation, duplicate-dispatch prevention,
   reservation release, and retry/defer idempotency tests.
14. Add multi-workflow acceptance coverage proving a workflow can pause between
    tasks while another user's compatible task runs or batches.

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

Rejected alternatives:

- Put runtime execution on `WorkflowHost`: rejected because it broadens the
  workflow host with runtime execution ownership and makes task dispatch
  harder to reason about.
- Put the orchestrator in `pantograph-embedded-runtime`: rejected because
  workflow-service owns workflow state, task state, diagnostics, and run
  progression.
- Mirror DTOs in workflow-service: rejected because parallel runtime-host
  contracts would drift and violate the no-legacy/no-fallback rule.

## Task Result Materialization Plan

The next implementation target is the option 2 materialization boundary:

- Add `WorkflowSchedulerTaskResult` as a workflow-service contract, not a
  scheduler policy contract. It records what a workflow task produced so the
  orchestrator can resolve downstream task inputs.
- Include stable schema version, workflow id, workflow run id, node id, task
  id, result status, typed output values, bounded diagnostics, and optional
  terminal metadata. It must not contain local model paths, executable Pumas
  load targets, worker launch details, runtime handoff, or raw node-engine
  internals.
- Use typed value variants for at least `PumasModelRef`, scalar strings,
  booleans, signed/unsigned integers, media/artifact refs, and diagnostic-only
  outputs. Floating-point generation settings remain blocked until the
  scheduler trait value contract grows an explicit float variant; do not
  stringify floats silently. If a materialization or trait slice needs
  guidance scale, denoise strength, or other float-like generation settings,
  stop and add the typed scheduler/workflow contract extension first.
- Add active-run result storage and read APIs in workflow-service as the first
  stage. Mark this as staged storage, not the final durability story, and do
  not let the current whole-run `active_run` shape become the long-term task
  scheduler state model for concurrent users, batching, or multi-device
  orchestration.
- Add binding resolution that consumes `WorkflowSchedulerTaskGraph` input
  bindings and materialized upstream results. It emits valid scheduler intent
  only after all required typed values exist and validate.
- Map unresolved, wrong-type, unavailable, invalid, or ambiguous materialized
  inputs into typed diagnostics and queue states. Do not fall back to graph
  fields, `ModelRefV2`, `model_path`, frontend `modelPath`, or whole-workflow
  output-node demand.
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
