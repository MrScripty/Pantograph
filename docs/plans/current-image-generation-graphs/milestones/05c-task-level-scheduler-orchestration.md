# Milestone 5c: Task-Level Scheduler Orchestration

**Goal:** Implement option 4 by making scheduler-owned task state the driver of
workflow progress. Workflow session execution must advance by admitting and
executing ready task units, not by asking node-engine to demand output nodes as
one uninterrupted workflow.

This milestone is inserted after Milestone 5a contracts and before the
remaining Milestone 5b production runtime-host wiring. Milestone 5b already
contains completed runtime-host contract and load-target slices; those remain
valid. The next runtime wiring work must wait until this milestone creates the
durable task orchestration path.

**Tasks:**

- [ ] Add a path-free run-scoped scheduler task graph projection from validated
  workflow topology and graph inputs. It must carry workflow/run/node/task
  correlation, dependencies, task kind, Pumas model refs, optional hard
  runtime/device constraints, typed trait settings, and estimate hints without
  local paths or executable Pumas load targets.
- [ ] Add durable workflow scheduler task records and transition APIs for
  pending, ready, blocked, waiting for dependency readiness, waiting for
  resources, waiting for batch, running, paused/deferred, retryable failed,
  terminal failed, and completed.
- [ ] Add scheduler task-state read models for graph editor, run inspection,
  and diagnostics views. Read models may expose typed state, waiting reasons,
  timings, attempts, and diagnostics; they must not expose scheduler internals
  or executable load targets.
- [ ] Add the scheduler task orchestrator as the workflow-service application
  layer async shell around the synchronous scheduler policy core. It owns
  dependency readiness calls, runtime-host dispatch calls, ledger writes,
  cancellation, retries, shutdown, bounded queues, and task panic handling.
- [ ] Add task result materialization for node outputs, media/artifact refs,
  scalar values, and diagnostics so dependent tasks can resume without relying
  on whole-workflow output-node demand.
- [ ] Add a narrow node-engine single-task execution adapter for non-runtime
  graph tasks using materialized scheduler-owned inputs. Runtime inference
  nodes must not launch through this adapter.
- [ ] Wire runtime inference tasks through actual dispatch-selected
  `SchedulerRuntimeHandoff` values and the runtime-host execution port added
  in Milestone 5b. Do not build handoff from reduced execution plans or
  backend projections.
- [ ] Replace workflow/session run execution so scheduler task orchestration,
  not node-engine output demand, advances workflow progress. A workflow must
  be able to pause between tasks while another workflow's compatible task runs
  or batches.
- [ ] Add cancellation, retry/defer idempotency, duplicate-dispatch
  prevention, reservation release, replay, and recovery behavior before
  removing legacy launch paths.
- [ ] Update README/crate documentation for task orchestration ownership,
  lifecycle, task-state contracts, node-engine adapter scope, runtime-host
  dispatch scope, and no-fallback removal boundaries.

**Verification:**

- Contract tests for scheduler task graph extraction, task records, task
  result materialization, and task-state read models.
- Store transition tests for every legal task state transition and focused
  negative tests for illegal transitions.
- Scheduler orchestration tests using deterministic dependency readiness,
  resource observer, runtime-host port, and node-engine task adapter fakes.
- Multi-workflow acceptance test proving task interleaving across two workflow
  runs and at least one defer/resume path.
- Batching acceptance test proving compatible ready tasks can share a batch
  without exposing batching groups to graph inputs.
- Runtime-host dispatch test proving runtime inference tasks use actual
  dispatch-selected `SchedulerRuntimeHandoff` and reject reduced execution-plan
  projections as launch input.
- Node-engine tests proving runtime inference nodes do not call
  `PlannedInferenceExecutionHost` and fail closed when scheduler task state is
  missing.
- Recovery tests proving replay does not duplicate dispatch, leak resource
  reservations, or re-run completed tasks.
- Frontend/read-model tests proving task state is displayed from backend facts
  without optimistic updates or local inference over scheduler state.
- Focused crate checks for every touched Rust crate, including default,
  all-features, and no-default-features checks when public feature contracts
  change.

**No-Fallback Requirements:**

- Do not preserve whole-workflow output-node demand as the successful runtime
  inference launch path after task orchestration is wired.
- Do not synthesize `SchedulerRuntimeHandoff` from `WorkflowExecutionPlan`,
  `WorkflowExecutionPlanNodeDecision`, backend execution projections, graph
  inputs, or node-engine request context.
- Do not let node-engine dependency preflight, `ModelRefV2`, `model_path`, or
  frontend `modelPath` become successful runtime identity.
- Do not let graph editor, frontend adapters, Tauri commands, node-engine,
  runtime adapters, or inference workers own scheduler policy.
- Do not expose executable Pumas load targets outside runtime host execution.
- Do not add compatibility shims for retired runtime execution contracts.

**Status:**

- [ ] Planned.
- 2026-05-23: Created as the option 4 re-plan target after implementation
  found that workflow-service session execution currently produces only a
  reduced `WorkflowExecutionPlan` and does not produce or store actual
  dispatch-selected `SchedulerRuntimeHandoff` values at the task boundary.
  This milestone must be completed before the remaining Milestone 5b
  production runtime-host wiring and legacy execution deletion continue.
