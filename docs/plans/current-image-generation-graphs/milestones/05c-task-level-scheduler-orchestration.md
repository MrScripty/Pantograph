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

- [x] Add a path-free run-scoped scheduler task graph projection from validated
  workflow topology and graph inputs. It must carry workflow/run/node/task
  correlation, dependencies, task kind, Pumas model refs, optional hard
  runtime/device constraints, typed trait settings, and estimate hints without
  local paths or executable Pumas load targets.
- [x] Replace the current intent-required `SchedulerQueueTaskRecord` and
  `SchedulerQueueTransition` contracts with phase-aware scheduler task-state
  records and transition APIs. The new scheduler-owned state contract must
  represent pre-intent tasks such as awaiting materialized inputs, invalid
  graph projection, and input-unavailable states, while carrying
  `SchedulableTaskIntent` only in schedulable phases such as ready, waiting
  for dependency readiness, waiting for resources, waiting for batch, running,
  paused/deferred, retryable failed, and completed. Old queue record types are
  replacement/removal targets, not compatibility surfaces.
- [x] Close the non-runtime executable-state gap before running node-engine
  tasks from the scheduler path. The current phase-aware state contract still
  requires `SchedulableTaskIntent` on ready/running/completed executable
  states, which is correct for runtime tasks but wrong for pure non-runtime
  tasks. Replace that payload with `SchedulerTaskExecutionIntent` or an
  equivalent state-specific enum where runtime states carry
  `SchedulableTaskIntent` and non-runtime states carry only a validated
  non-runtime task intent. Do not fabricate model refs, synthetic runtime task
  types, or dummy schedulable intents for non-runtime completion. Completed
  2026-05-23 with scheduler-owned `SchedulerTaskExecutionIntent` plus
  `SchedulerNonRuntimeTaskIntent`; runtime policy consumers still see
  `SchedulableTaskIntent` only through the runtime variant.
- [ ] Add scheduler task-state read models for graph editor, run inspection,
  and diagnostics views. Read models must join immutable
  `WorkflowSchedulerTaskGraph` definition facts with scheduler-owned lifecycle
  state, expose optional/unknown model and task-intent fields before
  materialization, and show typed state, waiting reasons, timings, attempts,
  and diagnostics without exposing scheduler internals or executable load
  targets. Partial 2026-05-23 status: workflow-service read models already
  join immutable task-graph facts with validated scheduler task-state records
  for active runs, hide transition ids/state versions/handoffs/load targets,
  expose pre-intent unknown model/runtime/device facts, and now project
  scheduler state diagnostics plus runtime/non-runtime execution category.
  2026-06-05 active-attempt read-model update: the active-run query now joins
  scheduler-owned active attempt id and attempt start time from the active-run
  store into schema version 2 task-state read models while an attempt is
  running. Historical counters, retry/defer decisions, replay outcomes, worker
  lifecycle diagnostics, and additional timing summaries remain open and must
  come from scheduler lifecycle/ledger facts rather than inferred state
  versions.
- [x] Align graph-visible scheduler constraints before materialization relies
  on them. The workflow-service task graph currently models optional hard
  `runtime` and `device` constraints, while the canonical inference node
  descriptor exposes `runtime` and not `device`. Add the optional typed
  `device` input and matching option/read-model tests, or remove `device`
  from the current task-result/admission scope until the graph contract can
  provide it. Frontend option-provider context must use scheduler constraint
  vocabulary and must not revive `backend_key` or `runtime_variant_id` as
  graph-visible scheduler policy.
- [x] Add the typed task result materialization contract and active-run result
  storage before implementing the orchestrator. Results must represent
  scheduler-owned task outputs as typed values, including `PumasModelRef`,
  scalar settings, media/artifact refs, diagnostics, invalid/unavailable
  states, and no executable paths. The active-run storage is an implementation
  stage only; the contract must be shaped so diagnostics-ledger persistence and
  replay can replace the storage later without changing graph, node-engine, or
  runtime-host semantics.
- [x] Add dependency-to-input binding resolution from materialized task
  results. Resolution must produce a valid `SchedulableTaskIntent` only after
  required canonical values are materialized; missing, wrong-type,
  unavailable, or invalid values must emit typed diagnostics and keep the task
  blocked/deferred/failed according to scheduler policy rather than falling
  back to graph-local paths or whole-workflow node-engine demand.
- [x] Move runtime-host execution contracts, response contracts, execution
  port, dispatcher, and typed dispatch errors out of
  `pantograph-embedded-runtime` into a lower-level shared contract crate before
  implementing the orchestrator. `pantograph-workflow-service` must consume the
  shared port from the orchestrator, and `pantograph-embedded-runtime` must
  implement it. Remove the embedded-runtime-owned contract definitions after
  the move; do not keep parallel DTOs, aliases, compatibility shims, or a
  `WorkflowHost` runtime-execution bypass.
- [ ] Add the scheduler task orchestrator as a workflow-service-owned
  dependency and application-layer async shell around the synchronous
  scheduler policy core. `WorkflowService` construction/configuration must
  provide the orchestrator and its runtime-host dispatcher; production
  `run_workflow_execution_session` must call a dedicated scheduler-task
  execution path instead of constructing ad hoc dispatch plumbing locally. The
  orchestrator owns dependency readiness calls, runtime-host dispatch calls,
  ledger writes, cancellation, retries, shutdown, bounded queues, and task
  panic handling. Partial 2026-05-23 status: `WorkflowService` now owns the
  orchestrator, provides a runtime-host execution-port configuration hook,
  and initializes active-run scheduler task state after queue admission. The
  dedicated task progression path, dispatch lifecycle, ledger writes,
  cancellation, retry/defer, bounded workers, panic handling, and legacy
  output-demand removal remain open.
- [ ] Add a narrow node-engine single-task execution adapter for non-runtime
  graph tasks using materialized scheduler-owned inputs. Runtime inference
  nodes must not launch through this adapter. The next implementation slice
  must first add a narrow node-engine-owned single-task API that hides
  `graph_flow::Context`, executes one explicit core task without demand
  execution, planned inference, runtime host dispatch, or scheduler policy,
  and fails closed if task-id suffix fallback would disagree with the explicit
  node type. Workflow-service must then add one focused task-classification
  boundary that maps immutable task graph facts plus canonical node-contract
  facts into runtime-inference, non-runtime node-engine, Pumas-materialization,
  or unsupported classes. The dedicated scheduler-task execution entrypoint may
  execute only tasks classified as explicitly non-runtime and whose values are
  representable in `WorkflowSchedulerTaskResultValue`. Runtime inference tasks
  remain scheduler-owned: they must wait until every required connected input
  is materialized, then run only through actual scheduler-selected runtime
  handoff dispatch once that slice is wired. The adapter must use a positive
  first-stage allowlist of output-compatible nodes such as `text-input`,
  `text-output`, and `boolean-input`; explicit typed input/output conversion
  between `WorkflowSchedulerTaskResult` and node-engine values; node-type
  authority from immutable `WorkflowSchedulerTaskGraph`; bounded lock scopes
  around awaits; and targeted deletion/usage searches proving the new path does
  not call `workflow_run_internal`, `DemandEngine` output demand,
  `PlannedInferenceExecutionHost`, node-engine workflow sessions, or
  node-engine core `execute_puma_lib`.
  Exclude `puma-lib`, `model-provider`, `expand-settings`, arbitrary-JSON
  nodes, floating-point/vector nodes, file I/O, image/audio blob nodes,
  human/tool nodes, and unknown kinds until each has an explicit typed
  contract. Partial 2026-05-23 status: node-engine now owns a focused
  `single_task` API that validates one explicit task id/node type, injects the
  explicit node type into core inputs, creates local graph-flow context and
  empty executor extensions, executes one `CoreTaskExecutor` task, and fails
  closed for malformed `_data` or task-id suffix authority. Workflow-service
  now also owns a focused task-classification boundary and schema-versioned
  task-graph execution class that maps canonical node contracts into
  runtime-inference, first-stage non-runtime node-engine, Pumas-materialization,
  or unsupported classes before any adapter can choose a path. Generalized
  runtime connected-input readiness now blocks scheduler intent materialization
  until every upstream binding has a completed materialized task result, while
  still specializing `pumas_model_ref` into the scheduler intent model ref.
  Initial scheduler task-state creation now consumes the execution class:
  source-input tasks become `AwaitingInputs` until request values are
  materialized, dependent non-runtime tasks await inputs, Pumas materialization
  awaits its dedicated boundary, and unsupported classes become invalid with
  typed diagnostics.
  2026-05-24 replan decision: before the adapter conversion slice, add the
  immediate option 2 typed non-runtime task-template contract to the immutable
  task graph. The projection may create concrete templates only for
  `text-input`, `boolean-input`, and `text-output`; the entrypoint and adapter
  must consume those templates plus materialized task results and must not
  read raw graph/editor node data. User-authored or external nodes remain
  unsupported with typed diagnostics until an explicit concrete template is
  added or the later generic typed port-value execution/source contract replaces
  the interim enum. Completed 2026-05-24 with a superseded staging schema v3
  contract, then replaced 2026-05-24 by schema version 4: `text-input.text`
  and `boolean-input.value` are now `WorkflowSchedulerSourceInputTemplate`
  source-input tasks, while `WorkflowSchedulerNonRuntimeTaskTemplate` keeps
  only node-engine-executable non-runtime tasks such as `text-output`. Source
  input projection does not read graph-local values; source inputs stay
  `AwaitingInputs` until request materialization produces typed task results.
  Scheduler-task session-runner wiring and the store-owned source-input
  materialization operation remain open. Non-runtime adapter conversion
  completed
  2026-05-24 with a workflow-service adapter that consumes typed templates
  plus materialized task results, calls node-engine `single_task`, converts
  outputs back into `WorkflowSchedulerTaskResult`, and rejects runtime tasks
  before node-engine execution. The adapter has a temporary module-scoped
  dead-code allowance because the scheduler-task entrypoint wiring is the next
  slice; remove that allowance when the entrypoint calls the adapter.
  2026-05-24 entrypoint consistency replan decision: before wiring the
  entrypoint, add a focused active-run store completion operation that records
  a successful `WorkflowSchedulerTaskResult` and applies the terminal
  completed-state transition together under one active-run store lock. The
  entrypoint may transition ready tasks to running and await the adapter
  outside store locks, but it must not use separate successful result-store
  and complete-task calls. Stale state, wrong run/task/node correlation,
  duplicate success, or mismatched terminal status must fail closed with typed
  diagnostics. The broader option 3 execution lease/transaction command with
  attempt tokens remains the later target for retries, duplicate dispatch,
  cancellation, and worker-pool ownership. Completed 2026-05-24 with
  `complete_active_run_scheduler_task`; focused tests cover successful
  result-plus-completed state commit, stale non-running state rejection, wrong
  run/task/node correlation rejection, duplicate successful completion
  rejection, and non-completed result rejection without half-state persistence.
  Ready non-runtime scheduler-task entrypoint wiring completed 2026-05-24:
  `WorkflowSchedulerTaskOrchestrator` now transitions ready non-runtime tasks
  to running, awaits the non-runtime adapter outside store mutation calls,
  commits success through `complete_active_run_scheduler_task`, rejects runtime
  tasks before node-engine execution, and moves adapter failures to terminal
  failed without storing a successful result. Full session-execution cutover,
  runtime-host dispatch wiring, cancellation/retry/defer idempotency, and
  legacy output-demand removal remain open. Dependent non-runtime readiness
  advancement completed 2026-05-24: the orchestrator now validates
  materialized active-run task results for dependent non-runtime tasks, leaves
  missing inputs blocked in `AwaitingInputs`, advances valid text input
  bindings to `Ready(NonRuntime)`, and maps unavailable or invalid materialized
  inputs to typed scheduler task-state diagnostics without graph output demand.
  Store-lock-safe non-runtime entrypoint split completed 2026-05-24: the
  previous single async helper was replaced by start/execute/complete/fail
  methods so production can transition to running and read materialized inputs
  under a store lock, drop the lock before awaiting node-engine work, then
  complete or fail the running task under a new store lock. The scoped
  `dead_code` allowances on this staging API must be removed when
  `run_workflow_execution_session` consumes the split calls.
- [x] Remove stale `puma-lib.model_path` compatibility surfaces before they can
  conflict with scheduler-task execution. Update workflow-service graph
  registry tests to assert the canonical `pumas_model_ref` options-provider
  boundary, and remove or replace graph-persistence behavior/tests that
  preserve successful `puma-lib` `modelPath`/`model_path` values without
  canonical Pumas identity. Stale path-shaped graphs must become typed
  diagnostics, not successful execution inputs. Completed 2026-05-23 by
  stripping `modelPath`/`model_path` for all persisted `puma-lib` nodes,
  updating graph persistence tests so path-only `puma-lib` state is not
  preserved as successful identity, updating the registry options-provider
  test to assert `pumas_model_ref`, and documenting the graph persistence
  invariant.
- [x] Wire runtime inference tasks through actual dispatch-selected
  `SchedulerRuntimeHandoff` values and the runtime-host execution port added
  in Milestone 5b. Do not build handoff from reduced execution plans or
  backend projections.
  Partial 2026-05-24 status: runtime-containing session runs no longer enter
  runtime admission, runtime preflight/load, or the legacy whole-run
  node-engine host launch while the runtime handoff cutover is incomplete.
  Workflow-service now marks active runtime scheduler tasks terminal failed
  with typed `SchedulerPolicyError` diagnostics and returns a
  capability-violation workflow error. Actual dispatch-selected
  `SchedulerRuntimeHandoff` construction/execution remains open.
  2026-05-25 replan decision refined after codebase review: use option 3 and
  replace the active-execution-plan planned-inference bridge with scheduler-owned
  runtime handoff dispatch by consuming the existing shared contracts, not by
  adding parallel DTOs. `SchedulerRuntimeHandoff` must remain path-free and must
  never carry executable Pumas load targets, local paths, worker launch details,
  or reduced execution-plan projections. The replacement must happen in staged
  vertical slices:
  1. consume the existing `WorkflowSchedulerTaskResult`,
     active-run result storage, binding-resolution, `SchedulerRuntimeHandoff`,
     `RuntimeHostExecutionRequest`, `RuntimeHostExecutionPort`, and
     `SchedulerRuntimeHostDispatcher` boundaries;
  2. add or wire production `pantograph-scheduler` dispatch-decision selection
     so workflow-service does not own runtime/device/batching policy;
  3. extend `RuntimeHostExecutionResponse` with typed, path-free output values
     that workflow-service can map into `WorkflowSchedulerTaskResult`. The
     response extension must use a stable contract version, explicit output
     value enum variants, bounded outputs/diagnostics,
     `serde(deny_unknown_fields)`, `TryFrom` validation, typed error enums, and
     `#[non_exhaustive]` public enums where future runtime output families are
     expected. Runtime-host contracts must not depend on workflow-service
     result DTOs; workflow-service owns the focused mapping module and exhaustive
     tests from runtime outputs to task results;
  4. run runtime readiness through
     `workflow_scheduler_resolve_task_intent` so connected Pumas refs and other
     upstream values materialize before dispatch;
  5. implement embedded-runtime `RuntimeHostExecutionPort` by resolving the
     Pumas load target from the scheduler-selected `PumasModelRef`, calling the
     inference gateway, and returning a completed or failed runtime-host
     response without reading workflow-service execution plans;
  6. wire the dedicated session runner to transition/claim state under the store
     lock, drop the lock for runtime-host awaits, then persist terminal state and
     `WorkflowSchedulerTaskResult` under a new store lock;
  7. delete `active_run_execution_plan`,
     `workflow_execution_session_active_execution_plan`, active execution-plan
     store tests, embedded-runtime active-plan lookup/projection helpers,
     `workflow_execution_plan_projection`, `EmbeddedPlannedInferenceExecutionHost`,
     node-engine `PlannedInferenceExecutionHost`, and the image-generation
     success branch that reconstructs dispatch from a whole-run plan.
     2026-05-31 progress: `active_run_execution_plan`,
     `workflow_execution_session_active_execution_plan`, active execution-plan
     store tests, and the embedded-runtime
     `workflow_execution_plan_projection` helper/tests were deleted after the
     session runtime path had scheduler-selected runtime-host dispatch
     coverage. The public `WorkflowExecutionPlan` DTO remains only for current
     validation/diagnostic contracts and must not be used as runtime launch
     input.
  Until the handoff execution slice is wired, runtime tasks must continue to fail
  closed with typed scheduler/workflow diagnostics; do not re-enable a successful
  active-plan or node-engine planned-inference compatibility route.
  2026-05-25 implementation status: the runtime-host response contract now
  carries typed, bounded, path-free output values plus optional path-free
  terminal metadata. `RuntimeHostExecutionOutputValue` currently supports
  string, bool, signed/unsigned integer, media/artifact refs, and diagnostic-only
  values; unsupported output families must add an explicit enum variant and
  workflow-service mapping slice before use. Focused fixtures and tests prove
  completed responses with outputs validate, path-shaped output fields are
  rejected, output counts are bounded, outputs on non-completed responses are
  rejected, and dispatcher response correlation still works. This slice does
  not wire runtime execution and does not revive any active-plan, reduced-plan,
  graph-path, or node-engine planned-inference fallback.
  2026-05-25 implementation status: workflow-service now owns the focused
  runtime-host response to `WorkflowSchedulerTaskResult` mapper. The mapper
  consumes only validated terminal runtime-host responses, maps typed path-free
  scalar and media/artifact outputs into scheduler task-result values, maps
  runtime-host diagnostics into task-result diagnostics, rejects accepted
  non-terminal responses, and fails closed for unsupported future runtime-host
  states, outputs, or diagnostic variants. The staged orchestrator runtime
  handoff method now consumes the mapper after shared dispatcher response
  validation; runtime execution is still not wired into production session
  progression.
  2026-05-31 implementation status: removed stale staging-only `dead_code`
  suppressions from the scheduler runtime-dispatch orchestration type and
  methods now consumed by the production session runner:
  `StartedRuntimeTaskExecution`, `dispatch_runtime_handoff`,
  `select_and_dispatch_runtime_task`, `start_ready_runtime_task`,
  `complete_started_runtime_task`, `advance_awaiting_runtime_task_inputs`, and
  `apply_runtime_dependency_readiness_admission`. This keeps the standards
  signal clean for the canonical scheduler to runtime-host handoff path instead
  of hiding it as unused staging code. Remaining runtime follow-ups are still
  production proof producers, complete image-generation runtime-host execution,
  retry/defer/cancellation leases, and stale test cleanup.
  2026-05-31 sequencing re-plan decision: complete the minimal production
  inference path before building the full producer and lease lifecycle. The next
  implementation slice should use the already-composed scheduler session runner,
  dispatch selection, runtime-host port, embedded image execution, artifact
  sink, and runtime-host response mapper to persist a completed
  `WorkflowSchedulerTaskResult` and project requested outputs. It may depend on
  pre-existing canonical readiness/load proof facts, but missing or invalid
  facts must remain typed fail-closed diagnostics. Do not combine this with
  dependency-readiness producer loops, runtime-load proof producer loops,
  durable execution leases, retry/defer, cancellation, replay, or recovery in
  the same slice; those follow after the complete path proves the canonical
  boundaries work.
  2026-05-25 replan decision for production dispatch selection: use option 2
  with option 3 discipline. The next scheduler slice must add a minimal
  scheduler-owned dispatch-selection contract rather than letting
  workflow-service, embedded-runtime, node-engine, or runtime-host code build
  `SchedulerDispatchDecision` directly. The contract should introduce
  scheduler-owned request/candidate/decision DTOs in `pantograph-scheduler`
  for a validated `SchedulableTaskIntent`, readiness proof, environment ref,
  typed path-free dispatch candidates, and bounded candidate-source diagnostics
  that explain evidence quality without becoming scheduler policy facts.
  Candidates may contain candidate id, runtime id, optional runtime variant,
  selected device ids, selected `PumasModelRef`, runtime trait settings, and
  already-typed resource/residency/history facts. Reservation or batching facts
  may appear only when their canonical owners produce them; otherwise dispatch
  selection must return typed diagnostics and no selected decision. The
  dispatch candidate type must be new and stricter than the existing
  runtime-registry technical-fit candidate or scheduler batch candidate shapes:
  do not alias `RuntimeTechnicalFitCandidate`, because it permits optional
  executable selection facts, derived candidate ids, compatibility-path
  metadata, and candidate-id ordering behavior; do not alias
  `SchedulerBatchCandidate`, because it represents an already selected batching
  group rather than a general dispatch option. Explicit graph runtime/device
  inputs are hard requirements; omitted inputs leave scheduler policy free to
  select among valid candidates. Missing candidates, incompatible explicit
  constraints, missing required reservation/resource facts, duplicate candidate
  ids, and unrankable ambiguity must fail closed with typed no-selection
  diagnostics. Do not synthesize generic runtime candidates, placeholder
  reservation leases, CPU/auto fallbacks, candidate-id tie-break winners, or
  compatibility aliases. Selection must remain synchronous and pure over
  supplied typed facts; Pumas calls, runtime-host probes, history queries,
  workflow-service store locks, and other I/O stay in orchestration/provider
  layers. Later option 3 work can replace the request's caller-supplied
  candidate list with provider composition from runtime registry, resource
  observer, Pumas, and history services without moving selection semantics out
  of the scheduler crate. 2026-06-03 reconciliation: later
  scheduler/runtime-host slices completed the dispatch-selected runtime
  handoff path. Workflow-service session execution now resolves runtime
  readiness through canonical facts, selects dispatch through
  `pantograph-scheduler`, sends `RuntimeHostExecutionRequest` to the shared
  runtime-host port, maps typed runtime-host responses into
  `WorkflowSchedulerTaskResult`, and projects requested outputs without
  reduced execution-plan launch.
- [x] Replace workflow/session run execution so the dedicated scheduler-task
  execution path, not node-engine output demand, advances workflow progress.
  A workflow must be able to pause between tasks while another workflow's
  compatible task runs or batches. The cutover must call orchestrator
  initialization after task graph extraction and then remove or make
  unreachable the old scheduler-managed inference launch path rather than
  preserving it as a compatibility branch.
  2026-05-24 revised replan decision: replace the legacy whole-run launch with
  a dedicated workflow-service scheduler-task session runner instead of
  inserting task progression directly into `run_workflow_execution_session`.
  `run_workflow_execution_session` remains the admission/terminal wrapper; the
  runner owns active-run task progression, external-input materialization,
  non-runtime adapter calls, runtime handoff/fail-closed decisions, task-result
  projection, and output validation. Before any runtime preflight/load, the
  cutover must read a run-class summary from the immutable scheduler task graph
  and scheduler task-state records. Non-runtime-only runs bypass runtime
  admission/load and finish from scheduler task results. Runtime-containing
  runs dispatch only through actual scheduler-selected runtime-host handoff; if
  that slice is not wired, they fail closed with typed "runtime dispatch not
  wired" scheduler/workflow diagnostics. Request `WorkflowPortBinding` inputs
  must be converted into scheduler-owned materialized task results for matching
  source/input tasks without mutating graph node data. Completed scheduler task
  results must be projected through one typed output converter and then checked
  with existing requested-output validation. Do not keep the old whole-run
  node-engine path, output demand, or `workflow_run_internal` as a compatibility
  branch for tasks handled by the scheduler loop.
  2026-05-24 implementation status: runtime-containing runs now take the
  fail-closed scheduler-task branch before any runtime admission/load or
  `workflow_run_internal` call. Focused tests assert no runtime load and no
  legacy host execution. Non-runtime-only and runtime-containing runs are no
  longer successful legacy branches; remaining session-runner gaps are
  Pumas-materialization-only or unsupported task classes, plus the actual
  runtime handoff dispatch slice.
  2026-05-24 implementation status: Pumas-materialization-only and unsupported
  scheduler task classes now also fail closed through terminal scheduler task
  diagnostics instead of entering runtime admission/load or
  `workflow_run_internal`. This removes the remaining successful session-runner
  fallback. Discovered issue: removing that branch exposes retired private
  runtime-load/session-admission helpers, execution-plan admission helpers,
  queue runtime-admission fields, and media artifactization code as compiler
  dead-code warnings. The next cleanup slice must delete or reconnect those
  systems through canonical scheduler/runtime-host paths; do not silence the
  warnings with compatibility allowances.
  2026-05-24 cleanup replan decision: use option 2 before the next runtime
  dispatch implementation slice. First classify every newly exposed
  dead/legacy surface by owner and action, then implement deletion or
  canonical reattachment in small vertical commits. Valid actions are:
  delete, reattach only through dispatch-selected scheduler/runtime-host
  ownership, or convert to scheduler task-result/output ownership. No
  compatibility shim, feature flag, alias, `allow(dead_code)`, or retained
  alternate successful execution branch is allowed. Initial classification:
  `workflow_run_internal` and whole-run output-demand helpers are deletion
  targets; retired runtime-load/session-admission helpers and
  `session_runtime_load_lifecycle` may be reattached only if the runtime
  handoff slice consumes them as scheduler-selected runtime-host lifecycle
  diagnostics; execution-plan admission helpers and re-exports are deletion
  targets if they only feed reduced-plan launch/admission; queue
  runtime-admission/preflight fields and helper methods are deletion targets
  unless an active scheduler read model still consumes them; media
  artifactization helpers and `media_conversion_executor` are conversion
  targets for a later scheduler task-result artifactization boundary, not a
  reason to keep the old whole-run artifact path alive. Verification for this
  gate must include targeted `rg` searches for every retired symbol, focused
  compile/test commands for touched crates, `git diff --check`, and a clean
  decision recorded here for any surface that cannot be deleted immediately.
  2026-05-24 cleanup implementation status: the first deletion slice removed
  the reduced execution-plan admission helper, its workflow-service export,
  its README entry, and the tests that asserted technical-fit admission could
  still produce an executable reduced run plan. Source searches now show no
  `build_workflow_execution_plan_from_admission`,
  `execution_plan_admission`, or `workflow_execution_plan_admission` source
  references. Remaining cleanup targets include old queue runtime-admission
  fields/helpers, retired runtime-load/session-admission diagnostics helpers,
  `session_runtime_load_lifecycle`, `workflow_run_internal`, and the media
  artifactization conversion boundary.
  2026-05-24 cleanup implementation status: the second deletion slice removed
  the unused queue prediction/update helpers
  `queued_run_is_admission_candidate` and
  `set_queue_decision_reason_if_present`. Actual admission remains owned by
  `begin_queued_run` and scheduler policy; the removed helpers no longer
  provide a side path for predicted admission or queue-decision mutation.
  Remaining queue cleanup targets are the stale queued/dequeued/preflight
  fields that no active scheduler read model or policy consumes.
  2026-05-24 cleanup implementation status: `timeout_ms` was classified as a
  canonical reattachment target, not a deletion target. The scheduler-task
  session runner now applies the queued run timeout around non-runtime
  scheduler-task execution and returns a typed `RuntimeTimeout` error when the
  run exceeds the requested duration. This preserves the user-visible timeout
  contract without reviving `workflow_run_internal`, runtime admission/load,
  or node-engine output demand. The current scope covers non-runtime
  scheduler-task runs after queue admission; runtime dispatch timeout,
  cancellation, and durable attempt timing remain owned by the later
  scheduler/runtime-host lifecycle slice.
  2026-05-24 cleanup implementation status: stale preflight-cache payloads
  `capability_models` and `technical_fit_decision` were deleted from
  `WorkflowExecutionSessionPreflightCache`. The cache now carries only the
  invalidation fingerprints, normalized override selection, runtime affinity
  requirements, and blocking runtime issues still consumed by the active
  session/runtime readiness path. Preflight cache tests now call the preflight
  boundary directly instead of depending on a successful whole-run session
  launch, preserving the no-legacy cutover.
  2026-05-24 cleanup implementation status: stale dequeued/finish-state
  payload cleanup removed `required_backends` and `required_models` from
  `WorkflowExecutionSessionDequeuedRun` and removed the redundant `workflow_id`
  from `WorkflowExecutionSessionRunFinishState`. Session affinity remains on
  session/preflight state and admission/placement projections; the dequeued
  run handoff carries only queue timing, decision reason, workflow id, and the
  queued request, while finish state carries only the active runner's
  `unload_runtime` decision.
  2026-05-24 cleanup implementation status: the retired
  `session_runtime_load_lifecycle` module was deleted together with its
  private scheduler model-lifecycle event request/helper entry point. Runtime
  load lifecycle diagnostics are not kept as an unused compatibility surface;
  the later scheduler/runtime-host lifecycle owner must reintroduce typed
  runtime-load, dispatch, retry, cancellation, and attempt timing events with
  task/runtime handoff correlation. A broader existing
  `workflow::tests::session_capacity` suite still fails because it expects
  legacy runtime/session capacity behavior while current scheduler-task runs
  block source-input runs or fail closed; convert or delete those tests when
  the scheduler/runtime-host lifecycle and source-input materialization slices
  replace that behavior.
  2026-05-24 cleanup implementation status: retired session-admission
  diagnostics helpers were deleted from `session_execution_api.rs`, including
  unused scheduler delay/admitted/reservation event writers, runtime-load
  error record builders, retry timestamp arithmetic, queued graph-settings
  decoding, and technical-fit trace mapping used only by those retired
  writers. Future diagnostics for these phases must be introduced through the
  scheduler-task lifecycle owner with task ids, attempt ids, and runtime
  handoff correlation rather than old whole-session admission facts.
  2026-05-24 cleanup implementation status: old whole-run execution cleanup
  removed private `workflow_run_internal`, its test-only caller module, the
  old `artifact_output_conversion` module, host-injected media conversion
  configuration, and the workflow-service `pantograph-media-conversion`
  dependency. Workflow-service no longer keeps a private node-engine
  whole-run launch or whole-run artifactization compatibility route. Future
  artifact/media output handling must be introduced through scheduler-task
  result materialization and runtime-host output projection. 2026-05-31
  update: the active execution-plan storage/projection bridge was later
  deleted after scheduler-selected runtime-host dispatch coverage existed for
  session runtime tasks; workflow progress must continue through scheduler
  task state/results rather than whole-run execution-plan storage.
  2026-05-31 media conversion ownership decision: delete the unused Tauri
  managed media conversion adapter in the next cleanup slice instead of
  preserving dormant desktop-owned business logic. Future conversion must be a
  backend-owned, host-agnostic service boundary with typed request, result,
  attribution, and diagnostic contracts, consumed by scheduler task-result
  materialization or runtime-host output projection. Tauri may supply only
  process/tool lease infrastructure and event transport.
  2026-05-25 replan decision: the cross-crate replacement must preserve the
  task scheduler as the only source of runtime truth. Runtime selection from a
  graph node remains a scheduler input/constraint, not an executor decision.
  The node engine and graph editor may expose typed inference inputs and
  option-provider hints, but they must not receive local model paths,
  executable Pumas load targets, backend execution decisions, or direct runtime
  launch details. The runtime-host port receives only a scheduler-built
  handoff for a ready runtime task after upstream scheduler task inputs are
  materialized. 2026-06-03 reconciliation: workflow-service no longer keeps a
  private `workflow_run_internal` whole-run node-engine launch or whole-run
  artifactization route, and the active execution-plan storage/projection
  bridge was deleted after session runtime tasks had scheduler-selected
  runtime-host dispatch coverage. Session progress now proceeds through
  scheduler task state/results; remaining 5c work is lifecycle hardening,
  retry/defer/cancellation/replay/reservation behavior, attempt/timing facts,
  and documentation cleanup.
- [ ] Add cancellation, retry/defer idempotency, duplicate-dispatch
  prevention, reservation release, replay, and recovery behavior before
  removing legacy launch paths. 2026-06-03 re-plan decision: implement Option
  2 first, the attempt/lease state core. The next source slice must add a
  scheduler-owned active-run attempt/lease contract and store transitions for
  claim/start/complete/fail/cancel, stale-attempt rejection,
  duplicate-dispatch prevention, and reservation release hooks. It must keep
  runtime-host awaits outside store locks and fail closed for stale or
  mismatched attempt ids. Durable retry/defer policy, replay/recovery, worker
  supervision, and timing/attempt ledger facts remain follow-up slices after
  this state core exists. Option 1 is rejected because minimal in-memory
  guardrails are not enough to complete the lifecycle boundary. Option 3
  remains the final durable lifecycle supervisor target. Option 4 is deferred
  unless the attempt/lease contract must be shared outside workflow-service.
  2026-06-03 source update: the first attempt-core slice added
  workflow-service active-run task attempt ids for runtime and non-runtime
  task starts. Completion/failure paths now require the matching attempt id
  before task-state/result mutation, duplicate starts fail closed, and stale
  completions cannot record a second result. Remaining follow-up inside this
  item: explicit cancel transition, reservation-release intent wiring,
  retry/defer idempotency, replay/recovery, worker supervision, and
  timing/attempt ledger facts. 2026-06-03 follow-up source update: started
  runtime-host dispatch errors now terminally fail through the matching task
  attempt id, and the obsolete broad `runtime-dispatch-not-wired` active-run
  helper was deleted. 2026-06-03 cancellation/reservation re-plan decision:
  use Option 2 next. Bind selected reservation lease/candidate metadata to the
  active runtime task attempt, add a matching-attempt cancel transition, and
  return typed reservation-release intent from synchronous terminal store
  transitions while applying reservation lifecycle events outside the store
  lock. Keep retry/defer idempotency, replay/recovery, worker supervision,
  ledger timing facts, and any distinct scheduler cancelled-state expansion as
  follow-up work. 2026-06-03 source update: workflow-service active-run task
  attempts now store typed reservation lease/candidate metadata, completion/
  failure/cancel terminal mutations return typed release intent for leased
  attempts, explicit cancel uses matching-attempt validation, and stale or
  duplicate reservation/cancel attempts fail closed. Remaining immediate
  follow-up: wire async reservation lifecycle application from the orchestrator
  or session runner after the store mutation releases the lock, then continue
  retry/defer idempotency, replay/recovery, worker supervision, and ledger
  timing facts in later slices. 2026-06-03 async application update: runtime
  dispatch now selects, binds the selected reservation to the started attempt,
  dispatches runtime-host work without holding the store lock, terminally
  mutates task state/results, and applies reservation lifecycle release events
  from the returned release intent after the store mutation completes.
  Remaining follow-up: retry/defer idempotency, replay/recovery, durable
  duplicate-dispatch prevention, worker supervision/cancellation tokens, and
  diagnostics-ledger timing/attempt facts. 2026-06-04 re-plan decision: use
  Option 3 next. The attempt/lease core is validated enough to introduce a
  workflow-service durable lifecycle supervisor in thin slices: lifecycle
  manager skeleton, durable duplicate-dispatch/task lease guardrails,
  cancellation/shutdown token ownership, retry/defer policy, replay/bootstrap
  recovery, and diagnostics-ledger attempt/timing facts. Each slice must keep
  one lifecycle owner, sync-core/async-shell separation, no Tauri/frontend
  business logic, and no graph-path/reduced-plan/node-engine runtime fallback.
  2026-06-04 lifecycle manager skeleton update: the first option 3 slice added
  the workflow-service task lifecycle manager skeleton and focused tests for
  handle ownership, duplicate/stale handle failure, and shutdown idempotency.
  Remaining immediate follow-up: wire durable duplicate-dispatch/task lease
  guardrails through this owner without adding retry, replay, ledger writes,
  runtime-host API changes, or adapter-owned lifecycle policy.
  2026-06-04 duplicate-dispatch guardrail update: the orchestrator now claims
  and releases lifecycle handles around runtime and non-runtime started task
  attempts. Attempt ids are generated before store start transitions, failed
  starts roll back the lifecycle claim, and matching terminal mutations release
  the handle after the store update. Remaining immediate follow-up:
  cancellation/shutdown token ownership through the same lifecycle owner.
  2026-06-04 cancellation/shutdown re-plan decision: the next slice must first
  add a cancellable runtime-host execution contract so workflow-service can
  propagate typed cancellation/shutdown to in-flight runtime tasks while
  retaining lifecycle/business ownership. A workflow-service-only new-work gate
  is insufficient for in-flight runtime tasks, and adapter-owned cancellation
  is rejected. After the cancellable contract exists, implement the full
  workflow-service task supervisor with tracked handles, child cancellation
  tokens, shutdown draining, timeout/abort behavior, and panic observation.
  2026-06-05 diagnostics-ledger attempt/timing update: scheduler attempt
  lifecycle events now cover started, redispatched, completed, failed, and
  cancelled transitions with scheduler-owned task/attempt identity, execution
  class, start/end/duration timing, selected runtime/backend/device/network
  facts, and reservation id. The scheduler timeline projection now drains
  those events and exposes typed read-model fields, and Scheduler page display
  consumes those fields through presenter rows without payload parsing.
  The active task-state read model also exposes the scheduler-owned active
  attempt id and started-at time from active-run store facts while an attempt
  is running. Remaining work in this broader lifecycle item is retry/defer
  idempotency, replay/recovery outcomes, worker lifecycle diagnostics, any
  historical task-state attempt counters/timing summaries, and terminal
  cooperative runtime/worker cancellation observation. 2026-06-05 active
  cancellation response update: the active runtime task cancellation API now
  returns typed `cancellation_requested` status with scheduler-owned task id,
  active attempt id, and a message that terminal cancellation is reported only
  after runtime observation. 2026-06-05 worker lifecycle diagnostics re-plan:
  use Option 3 as the target. Full worker diagnostics must be backed by the
  scheduler `SchedulerLifecycleOwnerSnapshot` contract with real
  workflow-service lifecycle ownership for queue worker,
  dependency-readiness action, resource observation loop, runtime-host
  dispatch, retry loop, and reservation cleanup. The next source slices must
  add or wire component ownership first, then expose public snapshots and
  diagnostics-ledger worker lifecycle events. Do not fabricate missing
  component state in a projection, and do not ship a task-supervisor-only
  snapshot as the final worker lifecycle model. 2026-06-05 scheduler
  lifecycle registry update: the first typed workflow-service component
  registry now owns explicit `NotStarted` records for queue worker,
  dependency-readiness action, resource observation loop, runtime-host
  dispatch, retry loop, and reservation cleanup. Remaining work: attach real
  runtime-host dispatch/task-supervisor state, then attach the other component
  owners before public snapshot/query or diagnostics-ledger lifecycle events.
  2026-06-05 runtime-host dispatch attachment update: runtime-host dispatch
  component state is now attached to real task-supervisor ownership in the
  task lifecycle manager. Remaining component attachments: dependency
  readiness, resource observation, retry, queue, and reservation cleanup.
- [x] Update README/crate documentation for task orchestration ownership,
  lifecycle, task-state contracts, node-engine adapter scope, runtime-host
  dispatch scope, and no-fallback removal boundaries. 2026-06-03
  reconciliation: scheduler, workflow-service scheduler/workflow, and
  node-engine READMEs now document phase-aware task state, scheduler-owned
  dispatch selection, source-input materialization, non-runtime
  `single_task` adapter scope, runtime-host dispatch through shared contracts,
  reservation lifecycle ownership, no whole-run output-demand fallback, and
  the still-open retry/defer/cancellation/replay lifecycle hardening boundary.

**Verification:**

- Contract tests for scheduler task graph extraction, task records, task
  result materialization, and task-state read models.
- Descriptor/projection tests proving optional scheduler constraints are
  consistent across workflow-nodes, workflow-service task graph extraction,
  task-state read models, and frontend option-provider context before `device`
  is used for admission.
- Store transition tests for every legal task state transition and focused
  negative tests for illegal transitions.
- Scheduler orchestration tests using deterministic dependency readiness,
  resource observer, runtime-host port, and node-engine task adapter fakes.
- Focused non-runtime adapter tests proving the scheduler-task execution
  entrypoint executes a simple non-runtime task from materialized inputs,
  persists a typed `WorkflowSchedulerTaskResult`, and rejects runtime
  inference task kinds before node-engine planned-inference or output-demand
  paths can run.
- Session cutover tests proving request inputs become typed scheduler task
  results without mutating graph node data, non-runtime-only runs bypass
  runtime admission/load and legacy whole-run output demand, runtime-containing
  runs dispatch only through scheduler-selected runtime-host handoff or typed
  fail-closed diagnostics, and completed scheduler task results project through
  one output converter before requested-output validation.
- Active-run store tests proving scheduler task completion persists the
  `WorkflowSchedulerTaskResult` and completed-state transition atomically,
  rejects stale running state, rejects wrong active-run/workflow/task/node
  correlation, rejects duplicate successful completion, and cannot leave
  completed-without-result or result-without-completed active-run state.
- Focused task-template projection tests proving first-stage `text-input` and
  `boolean-input` produce schema-versioned typed source-input templates while
  `text-output` produces a typed non-runtime template; unsupported,
  arbitrary-JSON, or user-authored node data fails closed with typed
  diagnostics instead of reaching the adapter.
- Scheduler state transition tests proving non-runtime ready/running/completed
  tasks do not carry `SchedulableTaskIntent`, while runtime readiness,
  resource, batching, dispatch, and handoff policy still reject non-runtime
  execution intents.
- Focused stale-contract cleanup tests/searches proving `puma-lib.model_path`
  and graph-persisted `modelPath`/`model_path` no longer remain successful
  paths after the cleanup slice.
- Standards verification for the non-runtime adapter slice must include
  default/all-features/no-default-features workflow-service checks, normal
  parallel Rust test execution, `git diff --check`, README updates for touched
  source directories, and targeted searches proving no new successful path
  through `workflow_run_internal`, output-node demand, reduced execution-plan
  handoff synthesis, graph-local model paths, or `PlannedInferenceExecutionHost`.
- Multi-workflow acceptance test proving task interleaving across two workflow
  runs and at least one defer/resume path.
- Batching acceptance test proving compatible ready tasks can share a batch
  without exposing batching groups to graph inputs.
- Runtime-host dispatch test proving runtime inference tasks use actual
  dispatch-selected `SchedulerRuntimeHandoff` and reject reduced execution-plan
  projections as launch input. The workflow-service bridge from validated
  scheduler dispatch selection to runtime-host handoff completed 2026-05-25;
  production session request assembly remains open.
- Scheduler dispatch-selection contract tests proving a single valid candidate
  produces a validated `SchedulerDispatchDecision`, explicit runtime/device
  constraints are hard requirements, no-candidate and missing required fact
  inputs fail closed with typed diagnostics, ambiguous/unrankable candidates do
  not fall back to candidate-id ordering, and no local paths/load targets are
  accepted in request or candidate payloads. Completed 2026-05-25 in
  `pantograph-scheduler`.
- Scheduler dispatch-selection negative tests proving
  `RuntimeTechnicalFitCandidate` and `SchedulerBatchCandidate` are not accepted
  as the dispatch candidate contract, duplicate candidate ids fail closed,
  reservation lease ids must be backed by a real supplied reservation fact, and
  candidate-source diagnostics cannot be treated as ranking facts. Completed
  2026-05-25 in `pantograph-scheduler`.
- Scheduler dispatch-selection standards tests and checks proving public DTOs
  validate through `TryFrom` wrappers, reject unknown fields and path-shaped
  payloads, expose typed diagnostic enums instead of stringly reason fields,
  serialize stable fixture shapes, and compile across default, all-features, and
  no-default-features modes for touched public crates. Completed 2026-05-25 in
  `pantograph-scheduler`.
- Runtime-host output contract tests proving typed output payload validation,
  bounded diagnostics, response/request correlation, unknown/path-field
  rejection, and fail-closed behavior for output variants not yet mapped to
  `WorkflowSchedulerTaskResult`.
- Runtime-dispatch orchestration tests proving no active-run store lock is held
  across runtime-host awaits, duplicate dispatch or duplicate terminal
  completion fails closed, cancellation cannot leave completed-without-result or
  result-without-completed state, and stale task state cannot be completed by a
  late runtime-host response.
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
- Legacy cleanup classification verification proving every newly exposed
  dead-code surface from the session-runner cutover is deleted, canonically
  reattached, or converted to scheduler task-result/output ownership. The
  verification must include targeted usage searches for `workflow_run_internal`,
  old runtime-load/session-admission helpers, `session_runtime_load_lifecycle`,
  execution-plan admission helpers, queue runtime-admission/preflight fields,
  `artifact_output_conversion`, `media_conversion_executor`, and the Tauri
  managed media conversion adapter, plus compile checks without new dead-code
  warnings for the touched crates.

**No-Fallback Requirements:**

- Do not preserve whole-workflow output-node demand as the successful runtime
  inference launch path after task orchestration is wired.
- Do not fabricate placeholder `SchedulableTaskIntent` values, dummy Pumas
  model refs, or synthetic task types to satisfy scheduler task-state storage.
  Missing materialized inputs must remain typed task state and diagnostics
  until they resolve or fail.
- Do not synthesize `SchedulerRuntimeHandoff` from `WorkflowExecutionPlan`,
  `WorkflowExecutionPlanNodeDecision`, backend execution projections, graph
  inputs, or node-engine request context.
- Do not let node-engine dependency preflight, `ModelRefV2`, `model_path`, or
  frontend `modelPath` become successful runtime identity.
- Do not execute `puma-lib` through node-engine core. Pumas model-reference
  materialization is a dedicated Pumas selector/host boundary, not a generic
  non-runtime adapter task.
- Do not let graph editor, frontend adapters, Tauri commands, node-engine,
  runtime adapters, or inference workers own scheduler policy.
- Do not expose executable Pumas load targets outside runtime host execution.
- Do not add compatibility shims for retired runtime execution contracts.
- Do not silence newly exposed retired-code warnings. A warning from a
  superseded launch, admission, runtime-load, reservation, execution-plan, or
  whole-run artifactization surface is a cleanup blocker until the surface is
  deleted, reattached through canonical scheduler/runtime-host dispatch, or
  converted to scheduler task-result/output ownership.

**Standards Guardrails:**

- New task-result materialization and binding-resolution code must be split
  into focused workflow-service modules and tests. Do not grow existing large
  scheduler/store/workflow files except for narrow integration calls or
  removal of retired code. Expected workflow-service ownership should be split
  across focused task-result contract, active-run task-result storage, and
  dependency binding-resolution modules rather than added to queue, session
  execution, node-engine inference, or planned-inference host files.
- `WorkflowSchedulerTaskResult` and related DTOs must be typed, versioned,
  serializable, and validated. Use explicit value variants and typed
  diagnostics; do not use incidental metadata maps, stringified unsupported
  values, or executable path fields. If materialization needs floating-point
  generation settings, add the explicit scheduler/workflow value contract
  first instead of encoding those values as strings.
- Active-run result storage is a staged implementation boundary. It must be
  documented and implemented so durable diagnostics-ledger persistence/replay
  can replace storage later without changing graph editor, node-engine,
  scheduler, or runtime-host semantics. Do not generalize the current
  whole-run `active_run` shape into the long-term task scheduler truth for
  concurrent users, batching, or multi-device orchestration.
- Binding resolution must consume materialized typed outputs and produce
  scheduler-admissible intent only after validation. Missing, wrong-type,
  unavailable, invalid, or ambiguous inputs must produce typed diagnostics and
  task state, not compatibility behavior.
- Separate task definition from task lifecycle state. `WorkflowSchedulerTaskGraph`
  is the immutable run-scoped task definition owner for dependency edges,
  input bindings, and intent templates. The scheduler crate owns mutable task
  lifecycle state and transitions. Workflow-service may orchestrate and join
  those facts for read models, but it must not create a second scheduler state
  machine or move graph binding ownership into scheduler policy.
- The replacement scheduler task-state contract must be correct by
  construction. Do not add `Option<SchedulableTaskIntent>` to a single record
  shape where invalid combinations are possible. Use state-specific payloads
  so pre-intent states cannot accidentally look schedulable, and so scheduler
  readiness, resource, batching, dispatch, and runtime handoff policy consume
  only states that actually carry a validated `SchedulableTaskIntent`.
- Orchestrator initialization must create scheduler task-state records for
  every task in the `WorkflowSchedulerTaskGraph`, including tasks that are
  awaiting materialized inputs or have projection diagnostics. A task may
  transition into a schedulable state only after workflow-service binding
  resolution produces a validated `SchedulableTaskIntent`.
- The task-state replacement must be implemented in focused scheduler modules
  and tests, with narrow workflow-service integration updates for active-run
  storage, read models, and orchestrator consumption. Do not grow unrelated
  dispatch, readiness, batching, handoff, session execution, or frontend files
  except for narrow imports, call sites, or deletion of retired paths.
- Public task-state APIs must follow Rust API standards: stable contract
  versioning, typed ids and diagnostics, `serde(deny_unknown_fields)`,
  `TryFrom` validated wrappers for raw persisted/IPC values, typed error
  enums, `#[must_use]` transition/validation results, and `#[non_exhaustive]`
  public state/diagnostic enums where future scheduler phases may be added.
- Scheduler lifecycle policy must remain synchronous. Store access,
  dependency-readiness I/O, runtime-host dispatch, diagnostics-ledger writes,
  task spawning, cancellation, and shutdown stay in the workflow-service async
  shell and must not leak async I/O into scheduler ranking/admission policy.
- The replacement must not add new third-party dependencies unless a new
  standards note first records the owner crate, reason, transitive cost,
  feature-contract impact, and verification plan.
- Old queue-shaped persisted artifacts, fixtures, and tests are contract
  artifacts. Because this plan does not preserve legacy compatibility, they
  must be regenerated to the phase-aware contract or rejected with typed
  diagnostics. Do not add silent migration, best-effort parsing, aliases, or
  compatibility adapters for old queue shapes.
- Verification for the replacement must include vertical coverage proving a
  pre-intent task is created, displayed in read models, resolved after
  materialized inputs arrive, admitted by scheduler policy, and dispatched
  through runtime host without node-engine output demand or reduced-plan
  handoff synthesis.
- Keep async orchestration as a shell around synchronous scheduler policy. Do
  not hold store locks across await points, and use bounded queues plus typed
  cancellation/task state for concurrent execution.
- Production orchestration must be wired through a workflow-service-owned
  composition boundary, not constructed ad hoc inside
  `run_workflow_execution_session`. The session function should delegate to a
  focused scheduler-task execution entrypoint after admission and task graph
  extraction, and the entrypoint must own state advancement, dependency
  readiness, dispatch, result materialization, ledger writes, retry/defer,
  cancellation, and completion.
- Any orchestrator background work must have explicit lifecycle ownership:
  bounded queues, tracked task handles, cancellation propagation, graceful
  shutdown/drain behavior, and typed diagnostics for task panics or aborted
  work. Fire-and-forget tasks, unbounded worker queues, hidden globals, lazy
  singleton dispatchers, and lock guards held across runtime-host or ledger
  awaits are not allowed.
- The production cutover must consume or delete staged dead-code allowances and
  retire the old scheduler-managed inference launch path. Do not leave old and
  new paths selectable through feature flags, config switches, compatibility
  branches, aliases, or fallback dispatch.
- Production cutover verification must include a vertical
  `run_workflow_execution_session` path test plus focused fake-port tests for
  dependency readiness, runtime-host dispatch, non-runtime node-engine task
  execution, ledger writes, duplicate dispatch, retry/defer, cancellation,
  terminal closure, and worker panic handling when background workers are
  introduced.
- Future frontend task-state work must use focused DTO, presenter, and query
  modules rather than growing large scheduler page, I/O inspector, or shared
  type files. Future diagnostics-ledger durability work must use focused
  task-result event/projection/sqlite modules rather than adding task-result
  replay logic directly to existing large ledger files.
- `PlannedInferenceExecutionHost`, reduced execution-plan launch projection,
  `ModelRefV2`, `model_path`, and frontend `modelPath` successful branches
  are deletion targets after replacement. Do not expand them while
  implementing task-result materialization or orchestration.
- Runtime-host execution request/response/port/dispatcher contracts are owned
  by a lower-level shared contract crate, not by embedded-runtime or
  workflow-service. Workflow-service may orchestrate against the shared port,
  and embedded-runtime may implement it, but neither crate may define a
  parallel DTO shape.
- Runtime-host response output values must be a typed runtime-host contract with
  an explicit workflow-service mapping boundary. Do not reuse
  `WorkflowSchedulerTaskResult` inside the shared runtime-host crate, do not use
  arbitrary `serde_json::Value` payloads, and do not stringify unsupported
  values. If a runtime needs a new output kind, add the explicit runtime-host
  variant, workflow-service mapping, focused tests, and README updates in the
  same slice.
- The shared runtime-host crate must remain a boundary/contract crate. It may
  define DTOs, validated wrappers, typed errors, the async execution port
  trait, and synchronous request/response validation helpers. It must not own
  scheduler policy, workflow orchestration, runtime loading, Pumas
  load-target resolution, node-engine execution, concrete I/O, spawned task
  lifecycle, or Tokio runtime creation.
- Runtime-host contract migration must be replacement work. The slice must
  remove the embedded-runtime-owned DTO/port/dispatcher definitions or replace
  them with imports from the shared owner; aliases, mirrored types,
  compatibility modules, and alternate successful launch paths are not allowed.
- The shared crate must include crate-level docs, a source-directory README,
  public re-exports from `lib.rs`, typed error enums, `TryFrom` validated
  wrappers, and executable JSON fixture tests for dispatch-selected request
  validation, readiness-only rejection, unknown/path-field rejection,
  response-correlation validation, and failed/rejected response diagnostics.
- Runtime-host contract-crate verification must include focused shared-crate
  tests, embedded-runtime runtime-host dispatch/load-target tests,
  workflow-service compile checks proving the dependency cycle is gone,
  default/all-features/no-default-features checks for touched crates, and
  `git diff --check`.
- Shared contracts, generated DTOs, saved workflow fixtures, lockfiles, README
  files, and plan files remain serial integration-owner work. Any sub-agent
  slice must have a non-overlapping write set and a report path.
- Every slice must update module documentation where ownership changes and run
  focused tests plus relevant Rust format/check/test commands, including
  default/all-features/no-default checks when public feature contracts change.
- Start cross-layer runtime-dispatch slices with a failing contract or
  acceptance test that proves the intended externally meaningful input/output
  before broadening shared layers. README updates for new or ownership-changing
  source directories must include API consumer contracts, structured producer
  contracts, lifecycle/error semantics, compatibility/versioning notes, and
  rejected alternatives.
- The dispatch-selection implementation must be a focused scheduler module,
  expected as `dispatch_selection.rs` or an equivalently narrow name. Public
  exports belong in `pantograph-scheduler/src/lib.rs`; source and crate READMEs
  must describe the contract, ownership boundary, lifecycle/error semantics,
  no-selection diagnostics, producer/consumer fields, and rejected aliases to
  technical-fit or batching candidates.
- Public dispatch-selection DTOs must follow Rust API standards: explicit
  contract version constants, typed ids, typed diagnostics, `serde` stable
  field names, `serde(deny_unknown_fields)`, `TryFrom` validated wrappers,
  specific error enums, `#[must_use]` validation/selection results, and
  `#[non_exhaustive]` public enums where future runtime families or diagnostic
  codes are likely. Do not use `serde_json::Value`, arbitrary maps,
  `Result<T, String>`, panics, or `unwrap`/`expect` in production selector
  paths.
- The selector must remain a synchronous pure policy function over supplied
  validated facts. Workflow-service, embedded-runtime, or future provider code
  may perform async Pumas, resource observer, runtime-host, history, or store
  operations before calling the selector, but scheduler selection must not own
  those I/O calls, spawned tasks, background workers, or lock lifecycles.
- The dispatch-selection slice must not add third-party dependencies or edit
  manifests/lockfiles unless the plan is updated first with dependency owner,
  transitive cost, feature-contract impact, and verification commands. Keep new
  files below the decomposition threshold; if the selector grows beyond one
  responsibility, split candidate validation, diagnostics, and ranking into
  focused modules before continuing.

**Status:**

- [ ] Planned.
- 2026-05-23: Created as the option 4 re-plan target after implementation
  found that workflow-service session execution currently produces only a
  reduced `WorkflowExecutionPlan` and does not produce or store actual
  dispatch-selected `SchedulerRuntimeHandoff` values at the task boundary.
  This milestone must be completed before the remaining Milestone 5b
  production runtime-host wiring and legacy execution deletion continue.
- 2026-05-23: First implementation slice completed. `pantograph-workflow-service`
  now exposes `workflow_scheduler_task_graph` plus path-free
  `WorkflowSchedulerTaskGraph` DTOs. The projection uses validated workflow
  topology, preserves dependency/input bindings, parses scheduler-owned
  workflow/run/node/task ids, emits typed projection diagnostics for missing
  canonical inference inputs, and only creates `SchedulableTaskIntent` when
  canonical `pumas_model_ref` and explicit `task_kind` are valid. Legacy
  `model_ref` and `model_path` are not accepted as scheduler identity.
  Verification passed: `cargo fmt -p pantograph-workflow-service`,
  `cargo test -p pantograph-workflow-service workflow::tests::task_graph`,
  `cargo check -p pantograph-workflow-service`,
  `cargo check -p pantograph-workflow-service --all-features`, and
  `cargo check -p pantograph-workflow-service --no-default-features`.
  Discovered issue for later task-trait slices: the current scheduler
  `SchedulerTraitValue` supports string, bool, signed integer, and unsigned
  integer values only. Floating-point generation traits such as guidance scale
  need an explicit typed contract extension before they can be projected
  without loss.
- 2026-05-23: Second implementation slice started the durable task-state
  boundary by adding active-run scheduler task record storage and transition
  application to the workflow-service scheduler store. The store validates
  records and transitions through `pantograph-scheduler` queue contracts and
  rejects records whose `workflow_run_id` does not match the active run. This
  does not complete the durable-record milestone item yet because replay,
  diagnostics-ledger persistence, and orchestrator consumption remain to be
  wired. Verification passed:
  `cargo test -p pantograph-workflow-service scheduler::store::tests` and
  `cargo check -p pantograph-workflow-service`, including all-features and
  no-default-features checks. Deviation/follow-up:
  `#[allow(dead_code)]` is scoped to the staged task-state bridge until the
  Milestone 5c orchestrator slice consumes these store APIs in production.
- 2026-05-23: Third implementation slice strengthened the canonical
  scheduler queue-state contract coverage. `pantograph-scheduler` now has an
  exhaustive public integration test matrix proving each declared
  `SchedulerQueueTaskState` transition is accepted or rejected by
  `apply_scheduler_queue_transition`, including initial-state creation,
  terminal-state closure, stale expected-state rejection, and idempotent replay.
  This keeps the state machine in the scheduler crate and does not add any
  workflow-service or node-engine fallback behavior. Verification passed:
  `cargo test -p pantograph-scheduler --test queue_state`,
  `cargo check -p pantograph-scheduler`, `cargo check -p pantograph-scheduler
  --all-features`, and `cargo check -p pantograph-scheduler
  --no-default-features`.
- 2026-05-23: Fourth implementation slice started the task-state read-model
  boundary. `pantograph-workflow-service` now exposes
  `workflow_scheduler_task_state_read_models`, a path-free projection from
  validated `SchedulerQueueTaskRecord` values to presentation-neutral graph
  editor/run-inspection facts. The projection exposes workflow/run/node/task
  correlation, task type, model id, queue state, optional requested
  runtime/device constraints, and typed trait settings while intentionally
  hiding transition ids, state versions, scheduler runtime handoff, executable
  load targets, worker launch details, and raw task intent. Verification
  passed: `cargo test -p pantograph-workflow-service
  workflow::tests::task_state_read_model`, `cargo check -p
  pantograph-workflow-service`, `cargo check -p pantograph-workflow-service
  --all-features`, and `cargo check -p pantograph-workflow-service
  --no-default-features`.
- 2026-05-23: Fifth implementation slice added the dedicated workflow-service
  query boundary for active-run scheduler task-state read models.
  `workflow_get_scheduler_task_state_read_models` validates session/run input,
  reads canonical active-run task records from the scheduler store, and returns
  the same path-free read models without modifying queue item or scheduler
  snapshot DTOs. This keeps task progress display facts separate from
  admission/snapshot facts and does not expose executable handoff or Pumas load
  targets. Verification passed: `cargo test -p pantograph-workflow-service
  workflow::tests::task_state_read_model`, `cargo check -p
  pantograph-workflow-service`, `cargo check -p pantograph-workflow-service
  --all-features`, `cargo check -p pantograph-workflow-service
  --no-default-features`, and `cargo fmt -p pantograph-workflow-service
  -- --check`.
- 2026-05-23: Re-plan boundary reached before the scheduler task orchestrator
  implementation slice. The current plan defines the target ownership, but the
  next code slice must first choose the task-result materialization contract
  used by the orchestrator. Runtime inference tasks cannot always be admitted
  with complete `SchedulableTaskIntent` at run admission because upstream
  non-runtime graph tasks can produce required values such as `PumasModelRef`,
  scalar generation inputs, media refs, or diagnostics after the workflow has
  already started. Implementing the orchestrator without that contract would
  either preserve whole-workflow node-engine output demand or force scheduler
  task intent from incomplete graph inputs, both of which violate this
  milestone's no-fallback rule. The next planning pass must define:
  materialized task-result DTOs, dependency-to-input binding resolution,
  closed diagnostics for missing materialized inputs, persistence/replay
  ownership, and the handoff point where a materialized inference task becomes
  a scheduler-admissible `SchedulableTaskIntent`.
- 2026-05-23: Re-plan direction chosen. Implement option 2 now with option 3
  discipline: add a typed `WorkflowSchedulerTaskResult` contract plus active-run
  result storage/resolution first, designed as if it will later be persisted
  through the diagnostics ledger. The next code slices must not introduce
  incidental metadata maps, node-engine output-demand fallback, reduced
  execution-plan handoff synthesis, or Pumas path exposure. Durable
  event-sourced task-result replay remains the later objective after the
  orchestrator can run against the typed contract.
- 2026-05-23: Standards and blast-radius review updated the milestone before
  the next code slice. Findings captured here: graph-visible `device`
  constraints must be aligned with the inference descriptor before admission
  depends on them; materialization must be added in focused workflow-service
  modules rather than oversized queue/session/node-engine files; active-run
  result storage is staged and must not become the long-term scheduler state
  model; floating-point generation settings require a typed contract extension
  before use; frontend task-state and diagnostics-ledger durability work must
  be decomposed into focused files; and legacy planned-inference/model-path
  execution branches remain deletion targets, not extension points.
- 2026-05-23: Graph-visible scheduler constraint alignment slice completed.
  `workflow-nodes` now exposes optional typed `runtime` and `device` inputs on
  the canonical `llm-inference` descriptor. `pantograph-workflow-service`
  task-graph tests now prove graph `device` values project into
  `SchedulerRuntimeDeviceConstraints`. `node-engine` port-option context and
  frontend selection-input/cache contracts now carry `requestedRuntimeId` and
  `requestedDeviceId` rather than graph-visible `backendId` or
  `runtimeVariantId`; frontend mocks and UniFFI smoke fixtures were updated to
  use the same context. This preserves the no-fallback rule because the graph
  can provide typed scheduler constraints without exposing executable paths,
  Pumas load targets, backend policy, or runtime-host launch details.
  Verification passed: `cargo fmt -p workflow-nodes -p node-engine -p
  pantograph-workflow-service -p pantograph-uniffi`, `cargo test -p
  workflow-nodes processing::inference --lib`, `cargo test -p
  pantograph-workflow-service workflow::tests::task_graph --lib`, `cargo test
  -p node-engine port_options --lib`, `node --experimental-strip-types --test
  src/components/nodes/workflow/selectionInputProviderOptions.test.ts
  src/services/workflow/portOptionsCache.test.ts
  src/services/workflow/WorkflowService.commands.test.ts`, `npm run
  typecheck`, `cargo check -p node-engine -p workflow-nodes -p
  pantograph-workflow-service -p pantograph-uniffi`, `cargo check -p
  node-engine -p workflow-nodes -p pantograph-workflow-service -p
  pantograph-uniffi --all-features`, `cargo check -p node-engine -p
  workflow-nodes -p pantograph-workflow-service -p pantograph-uniffi
  --no-default-features`, and `git diff --check`.
- 2026-05-23: Typed task-result materialization and active-run storage slice
  completed. `pantograph-workflow-service` now exposes the versioned,
  validated `WorkflowSchedulerTaskResult` contract with typed output values
  for `PumasModelRef`, scalar values, media artifact refs, diagnostic-only
  outputs, bounded diagnostics, terminal metadata, and closed invalid or
  unavailable status. The contract rejects unknown path or launch metadata and
  contains no local model paths, Pumas load targets, runtime handoff, worker
  launch details, or node-engine internals. The scheduler store now has a
  focused staged active-run result storage module that validates result
  correlation against the active workflow run and can be replaced by later
  diagnostics-ledger replay without changing graph, node-engine, scheduler, or
  runtime-host semantics. This preserves the no-fallback rule because
  materialized outputs are explicit typed scheduler-owned facts, not
  compatibility routes from graph-local paths or whole-workflow output demand.
  Verification passed: `cargo test -p pantograph-workflow-service
  workflow::tests::task_result_contracts --lib`, `cargo test -p
  pantograph-workflow-service active_run_scheduler_task_results --lib`,
  `cargo fmt -p pantograph-workflow-service -- --check`, `cargo check -p
  pantograph-workflow-service`, `cargo check -p pantograph-workflow-service
  --all-features`, `cargo check -p pantograph-workflow-service
  --no-default-features`, and `git diff --check`. Verification deviation
  fixed during the slice: the first focused compile used a non-existent
  `QueueItemNotRunning` error variant; the store now uses the existing
  `QueueItemNotFound` contract for no-active-run and wrong-active-run reads.
  Remaining follow-up: implement dependency-to-input binding resolution from
  materialized task results before the scheduler task orchestrator can admit
  downstream runtime tasks.
- 2026-05-23: Dependency-to-input binding resolution slice completed.
  `pantograph-workflow-service` now carries a path-free
  `WorkflowSchedulerTaskIntentTemplate` on scheduler task graph records when
  graph-supplied runtime/device/task-type/trait settings are valid but a
  canonical model reference is expected from an upstream task. The new focused
  binding-resolution module consumes the task template plus materialized
  `WorkflowSchedulerTaskResult` values and produces a validated
  `SchedulableTaskIntent` only when the required `PumasModelRef` output is
  present, completed, and correctly typed. Missing materialized output keeps
  the task blocked; unavailable, invalid, failed, or wrong-type upstream
  results return typed diagnostics instead of falling back to graph-local
  paths, reduced execution plans, node-engine output demand, or Pumas load
  targets. Verification passed: `cargo test -p pantograph-workflow-service
  workflow::tests::task_binding_resolution --lib`, `cargo test -p
  pantograph-workflow-service workflow::tests::task_graph --lib`, `cargo fmt
  -p pantograph-workflow-service -- --check`, `cargo check -p
  pantograph-workflow-service`, `cargo check -p pantograph-workflow-service
  --all-features`, `cargo check -p pantograph-workflow-service
  --no-default-features`, and `git diff --check`. Verification deviation
  fixed during the slice: the first binding resolver tests used an
  unregistered synthetic model-selector node; the fixture now uses registered
  `puma-lib` behavior so topology validation covers the real graph boundary.
  Remaining follow-up: implement the scheduler task orchestrator application
  shell that consumes these ready/blocked binding outcomes.
- 2026-05-23: Re-plan direction chosen for the runtime-host execution contract
  boundary before the orchestrator slice. The orchestrator belongs in
  `pantograph-workflow-service`, but the current runtime-host execution
  contracts and dispatcher live in `pantograph-embedded-runtime`, which already
  depends on workflow-service. Importing those contracts from workflow-service
  would create a crate dependency cycle. Use option 1: move
  `RuntimeHostExecutionRequest`, `RuntimeHostExecutionResponse`, validated
  wrappers, diagnostics, `RuntimeHostExecutionPort`,
  `SchedulerRuntimeHostDispatcher`, and typed port/dispatch errors into a
  lower-level shared contract crate before implementing orchestrator dispatch.
  Workflow-service will depend on that shared crate and call the port;
  embedded-runtime will depend on it and implement the port with
  runtime-specific Pumas load-target resolution. Rejected alternatives:
  adding runtime execution to `WorkflowHost`, moving the orchestrator into
  embedded-runtime, or mirroring DTOs in workflow-service. This preserves the
  no-fallback rule because runtime inference still launches only from an
  actual dispatch-selected `SchedulerRuntimeHandoff`, and the move removes the
  embedded-runtime-owned duplicate contract boundary instead of preserving
  legacy or compatibility shapes.
- 2026-05-23: Standards iteration for the runtime-host contract-crate re-plan
  completed. The plan now records the shared crate as a narrow
  boundary/contract crate, not an orchestration or runtime implementation
  crate. It requires dependency ownership checks, crate-level docs, README
  traceability, typed Rust APIs, executable JSON contract fixtures, async only
  at the port boundary, no spawned task/runtime lifecycle in the shared crate,
  replacement/removal of embedded-runtime-owned definitions, feature-matrix
  checks for touched crates, and explicit rejection of aliases or mirrored
  DTOs. No source implementation changed in this standards pass.
- 2026-05-23 runtime-host shared contract crate slice completed. Smallest
  useful vertical slice: move `RuntimeHostExecutionRequest`,
  `RuntimeHostExecutionResponse`, validated wrappers, diagnostics,
  `RuntimeHostExecutionPort`, `SchedulerRuntimeHostDispatcher`, and typed
  port/dispatch errors into the new `pantograph-runtime-host-contracts`
  crate, update embedded-runtime load-target resolution to consume that crate,
  and delete the embedded-runtime-owned DTO/dispatcher modules and fixtures.
  Allowed write set: workspace manifest, new
  `crates/pantograph-runtime-host-contracts` contract crate, embedded-runtime
  manifest/imports/docs/load-target tests, and Milestone 5c plan notes.
  No-fallback confirmation: no compatibility aliases, mirrored DTOs, old
  embedded-runtime contract modules, or alternate runtime launch paths were
  retained. Runtime-host Pumas load-target resolution still consumes only the
  validated shared request and keeps executable paths host-only. Verification
  passed: `cargo test -p pantograph-runtime-host-contracts`, `cargo test -p
  pantograph-embedded-runtime runtime_host_load_target --lib`, `cargo check -p
  pantograph-runtime-host-contracts`, `cargo check -p
  pantograph-embedded-runtime`, `cargo check -p pantograph-workflow-service`,
  `cargo check -p pantograph-runtime-host-contracts --all-features`, `cargo
  check -p pantograph-runtime-host-contracts --no-default-features`, `cargo
  check -p pantograph-embedded-runtime --all-features`, `cargo check -p
  pantograph-embedded-runtime --no-default-features`, `cargo check -p
  pantograph-workflow-service --all-features`, `cargo check -p
  pantograph-workflow-service --no-default-features`, `cargo fmt -p
  pantograph-runtime-host-contracts -p pantograph-embedded-runtime --
  --check`, `git diff --check`, targeted `rg` deletion checks for
  embedded-runtime-owned runtime-host modules, and file-size review for the
  new/touched runtime-host files. Verification deviation: workflow-service
  compile checks prove the workspace remains cycle-free after the contract
  extraction, but workflow-service does not yet depend on the shared port in
  this slice because adding an unused dependency would violate dependency
  ownership standards. The actual workflow-service consumer wiring remains in
  the next orchestrator slice.
- 2026-05-23 workflow-service orchestrator runtime-host boundary slice
  completed. Smallest useful vertical slice: add a focused
  `scheduler/task_orchestrator.rs` async shell in workflow-service that
  depends on the shared runtime-host contract crate and dispatches
  scheduler-provided `SchedulerRuntimeHandoff` values through
  `SchedulerRuntimeHostDispatcher`. Allowed write set: workflow-service
  manifest, scheduler module wiring, new orchestrator module/tests,
  workflow-service README, lockfile update, and Milestone 5c plan notes.
  No-fallback confirmation: the shell accepts only scheduler-owned handoff
  input and delegates validation/port calls to the shared runtime-host
  dispatcher; it does not import embedded-runtime, synthesize handoff from
  reduced plans, resolve Pumas paths, call node-engine, or create alternate
  launch paths. Verification passed: `cargo test -p
  pantograph-workflow-service scheduler::task_orchestrator --lib`, `cargo
  fmt -p pantograph-workflow-service -- --check`, `cargo check -p
  pantograph-workflow-service`, `cargo check -p pantograph-workflow-service
  --all-features`, `cargo check -p pantograph-workflow-service
  --no-default-features`, `git diff --check`, and file-size review for the
  new orchestrator files. Deviation/remaining follow-up: the orchestrator
  type is explicitly staged with scoped `dead_code` allowances until the next
  production session-execution slice calls it. The full orchestrator checklist
  item remains open because dependency readiness calls, task-state
  transitions, ledger writes, bounded queues, cancellation, retry/defer,
  panic handling, and production runtime-host dispatch are not wired yet.
- 2026-05-23 task-definition/task-state re-plan direction selected. Codebase
  review found that the current `SchedulerQueueTaskRecord` and
  `SchedulerQueueTransition` require a complete `SchedulableTaskIntent`, while
  `WorkflowSchedulerTaskGraph` can validly contain tasks that only have input
  bindings and `WorkflowSchedulerTaskIntentTemplate` until upstream task
  results materialize. The agreed replacement is a phase-aware scheduler
  task-state contract that keeps `WorkflowSchedulerTaskGraph` as the immutable
  workflow-service task definition and keeps mutable lifecycle transitions in
  the scheduler crate. `SchedulableTaskIntent` remains strict and is carried
  only by schedulable state variants. Rejected approaches: lazy-creating
  scheduler records only after intent materialization because blocked tasks
  disappear from scheduler state; adding `Option<SchedulableTaskIntent>` to
  the existing record because it permits invalid combinations; and moving
  workflow graph bindings/templates into scheduler because that couples
  scheduler policy to graph composition. No source implementation changed in
  this planning update, and no commit was created.
- 2026-05-23 phase-aware scheduler task-state replacement slice completed.
  Smallest useful vertical slice: replace the scheduler crate's
  intent-required durable queue record and transition contract with
  `SchedulerTaskStateRecord`, `SchedulerTaskStateTransition`, and
  `SchedulerTaskState` variants that distinguish pre-intent, schedulable, and
  terminal diagnostic states. Allowed write set: `pantograph-scheduler`
  task-state contract, lifecycle import/tests, scheduler README/fixtures;
  workflow-service active-run task-state storage/read-model tests; and this
  plan. No-fallback confirmation: old `SchedulerQueueTaskRecord`,
  `SchedulerQueueTransition`, `SchedulerQueueTaskState`,
  `ValidatedSchedulerQueue*`, `SCHEDULER_QUEUE_STATE_CONTRACT_VERSION`, and
  `apply_scheduler_queue_transition` symbols were removed from source/tests
  instead of being shimmed. Focused tests/fixtures now cover pre-intent states
  without `SchedulableTaskIntent`, required diagnostics for invalid and
  unavailable states, idempotent replay, terminal closure, and stale previous
  state rejection. Workflow-service read models now accept pre-intent records
  and expose optional task/model/runtime/device fields until a schedulable
  intent exists. Verification passed: `cargo test -p pantograph-scheduler
  --test queue_state`, `cargo test -p pantograph-scheduler --test
  task_lifecycle`, `cargo test -p pantograph-workflow-service
  scheduler::store::tests --lib`, `cargo test -p pantograph-workflow-service
  workflow::tests::task_state_read_model --lib`, `cargo check -p
  pantograph-scheduler`, `cargo check -p pantograph-scheduler
  --all-features`, `cargo check -p pantograph-scheduler
  --no-default-features`, `cargo check -p pantograph-workflow-service`,
  `cargo check -p pantograph-workflow-service --all-features`, and `cargo
  check -p pantograph-workflow-service --no-default-features`. Remaining
  follow-up: the broader graph-editor diagnostics read model item stays open
  because immutable task definition joins, timing/attempt fields, waiting
  reasons, and ledger durability still need the orchestrator integration.
- 2026-05-23 joined scheduler task-state read-model slice completed. Smallest
  useful vertical slice: replace records-only read-model projection with a
  join over immutable `WorkflowSchedulerTaskGraph` facts and mutable
  `SchedulerTaskStateRecord` values. Allowed write set:
  workflow-service active-run scheduler task-state storage/read-model modules,
  tests, README export notes, and this plan. No-fallback confirmation: the
  graph editor/run-inspection read model now receives backend-owned definition
  facts plus scheduler lifecycle state; it does not infer dependencies from
  the frontend, expose Pumas paths, synthesize task intent for pre-intent
  states, or preserve the old records-only projection as a compatibility
  path. Implementation notes: active-run storage now keeps the scheduler task
  graph with task-state records, the read model exposes node type, dependency
  task ids, input bindings, projection diagnostics, and optional
  task/model/runtime/device fields, and mismatched graph/state joins fail
  closed. Verification passed: `cargo test -p pantograph-workflow-service
  scheduler::store::tests --lib`, `cargo test -p
  pantograph-workflow-service workflow::tests::task_state_read_model --lib`,
  `cargo check -p pantograph-workflow-service`, `cargo check -p
  pantograph-workflow-service --all-features`, and `cargo check -p
  pantograph-workflow-service --no-default-features`. Remaining follow-up:
  this does not close the broader read-model checklist item because waiting
  reasons, timings, attempts, and ledger-backed diagnostics still depend on
  orchestrator lifecycle integration.
- 2026-05-23 orchestrator initialization slice completed. Smallest useful
  vertical slice: add the workflow-service orchestrator method that converts
  immutable `WorkflowSchedulerTaskGraph` tasks into initial
  `SchedulerTaskStateRecord` values before dependency readiness or runtime
  dispatch begins. Allowed write set: workflow-service task orchestrator module
  and tests plus this plan. No-fallback confirmation: initialization does not
  call node-engine output demand, synthesize model paths, create dummy Pumas
  refs, or launch runtime inference. Tasks with complete validated
  `SchedulableTaskIntent` start as `Ready`; tasks with templates or unresolved
  inputs start as `AwaitingInputs`; tasks with projection diagnostics start as
  `Invalid` with typed scheduler diagnostics. Verification passed: `cargo
  test -p pantograph-workflow-service scheduler::task_orchestrator --lib`,
  `cargo check -p pantograph-workflow-service`, `cargo check -p
  pantograph-workflow-service --all-features`, and `cargo check -p
  pantograph-workflow-service --no-default-features`. Remaining follow-up:
  the orchestrator checklist item stays open because state persistence
  initialization, dependency readiness calls, runtime-host dispatch lifecycle,
  ledger writes, bounded queues, cancellation, retry/defer, and panic handling
  are not wired into production session execution yet.
- 2026-05-23 orchestrator active-run state persistence slice completed.
  Smallest useful vertical slice: add a workflow-service orchestrator method
  that derives initial task-state records from `WorkflowSchedulerTaskGraph` and
  stores the graph plus records together on the active run. Allowed write set:
  workflow-service task orchestrator module/tests and this plan. No-fallback
  confirmation: the persistence method uses the canonical active-run
  scheduler task-state store and does not create a parallel queue, call
  node-engine output demand, launch runtime inference, or preserve a
  records-only compatibility path. Verification passed: `cargo test -p
  pantograph-workflow-service scheduler::task_orchestrator --lib`, `cargo
  check -p pantograph-workflow-service`, `cargo check -p
  pantograph-workflow-service --all-features`, and `cargo check -p
  pantograph-workflow-service --no-default-features`. Remaining follow-up:
  production session execution still needs to call this initialization after
  task graph extraction, then advance task-state transitions through
  dependency readiness, dispatch, materialization, ledger writes, and retry /
  cancellation policy.
- 2026-05-23 production orchestrator ownership re-plan selected. The agreed
  design combines service-owned orchestrator injection with a dedicated
  scheduler-task execution entrypoint. `WorkflowService` must own or be
  configured with the `WorkflowSchedulerTaskOrchestrator` and its
  `SchedulerRuntimeHostDispatcher` so runtime inference has one canonical
  path: scheduler task graph/state -> scheduler decision/handoff -> shared
  runtime-host execution port. `run_workflow_execution_session` should
  delegate to a focused scheduler-task execution path after admission and task
  graph extraction, then later slices advance tasks through dependency
  readiness, dispatch, materialization, and completion. Rejected options:
  constructing the orchestrator locally inside `run_workflow_execution_session`,
  because it spreads ownership and encourages adapter-local policy; and
  continuing to run scheduler-managed inference through whole-workflow
  node-engine output demand, because that preserves the legacy launch path. No
  source changed in this planning update, and per instruction this update is
  not committed yet.
- 2026-05-24 external session-input materialization slice completed. Smallest
  useful vertical slice: add the workflow-service conversion boundary that
  turns request `WorkflowPortBinding` inputs into completed
  `WorkflowSchedulerTaskResult` values for explicitly supported source/input
  tasks. Allowed write set: workflow-service external-input materialization
  module/tests, workflow module registration, and this plan. No-fallback
  confirmation: the helper reads only immutable scheduler task graph facts and
  request bindings; it does not mutate graph node data, accept Pumas paths or
  executable load targets, call node-engine, call `workflow_run_internal`,
  launch runtime inference, or pass raw arbitrary graph/editor data into the
  scheduler task loop. Focused tests cover valid text/boolean inputs, unknown
  input nodes, duplicate input bindings, wrong value types, and unsupported
  source task types. Verification passed: `cargo fmt -p
  pantograph-workflow-service`, `cargo test -p pantograph-workflow-service
  external_input_materialization`, and `git diff --check`. Deviation/remaining
  follow-up: the module has a scoped staging `dead_code` allowance until the
  dedicated scheduler-task session runner consumes it. The next cutover slice
  must call this helper before task progression and remove the allowance.
- 2026-05-24 scheduler task run-summary slice completed. Smallest useful
  vertical slice: add the workflow-service helper that summarizes an active
  run's immutable scheduler task graph plus scheduler task-state records before
  runtime admission/load. Allowed write set: workflow-service run-summary
  module/tests, workflow module registration, and this plan. No-fallback
  confirmation: the helper consumes `WorkflowSchedulerTaskExecutionClass` and
  scheduler task-state records; it does not inspect raw node-type strings,
  synthesize runtime handoff, call node-engine, call `workflow_run_internal`,
  or choose a legacy execution path. Focused tests cover non-runtime-only
  summaries, mixed runtime summaries, unsupported/invalid state counts,
  missing task-state records, and unexpected task-state records. Verification
  passed: `cargo fmt -p pantograph-workflow-service`, `cargo test -p
  pantograph-workflow-service task_run_summary`, and `git diff --check`.
  Deviation/remaining follow-up: the module has a scoped staging `dead_code`
  allowance until the dedicated scheduler-task session runner consumes it. The
  next cutover slice must use this summary before runtime preflight/load and
  remove the allowance.
- 2026-05-24 scheduler task output-projection slice completed. Smallest useful
  vertical slice: add the workflow-service converter that projects completed
  `WorkflowSchedulerTaskResult` values into requested `WorkflowPortBinding`
  outputs. Allowed write set: workflow-service task-result output projection
  module/tests, workflow module registration, and this plan. No-fallback
  confirmation: the converter reads only immutable scheduler task graph facts,
  requested output targets, and completed scheduler task results; it does not
  call output-node demand, node-engine workflow sessions, `workflow_run_internal`,
  runtime dispatch, Pumas path resolution, or graph/editor metadata. Focused
  tests cover completed scalar output projection, missing requested output,
  non-completed result rejection, unsupported Pumas model-ref output values,
  and ambiguous producer results. Verification passed: `cargo fmt -p
  pantograph-workflow-service`, `cargo test -p pantograph-workflow-service
  task_result_output_projection`, and `git diff --check`. Deviation/remaining
  follow-up: the module has a scoped staging `dead_code` allowance until the
  dedicated scheduler-task session runner consumes it. The session runner must
  call this converter before existing requested-output validation and remove
  the allowance.
- 2026-05-24 session-runner implementation replan boundary reached. Codebase
  review before wiring the dedicated runner found that current task graph
  projection treats source `text-input` and `boolean-input` nodes without
  graph-stored values as projection-invalid, while the revised cutover requires
  request `WorkflowPortBinding` inputs to materialize those source tasks
  without mutating graph node data. If the session runner is wired before this
  is resolved, non-runtime-only session runs with request-provided inputs would
  either keep invalid scheduler task-state records or require a legacy
  whole-run fallback. The next plan decision must define the canonical source
  input lifecycle: whether projection represents request-bound source inputs
  as awaiting external input, whether active-run initialization accepts
  pre-materialized external results and starts matching source tasks as
  completed, and how missing request inputs become typed scheduler diagnostics.
  Do not make graph node data, `workflow_run_internal`, or output-node demand a
  successful compatibility path for this gap.
- 2026-05-24 source-input lifecycle replan decision selected option 3. The
  next implementation slice must replace graph-data-backed source input
  execution with an explicit source-input scheduler task contract for the
  current typed allowlist. Add a separate typed contract field, such as
  `WorkflowSchedulerSourceInputTemplate`, instead of adding request-bound
  variants to `WorkflowSchedulerNonRuntimeTaskTemplate`; the non-runtime
  template enum remains only for tasks that the node-engine adapter may execute.
  Source-input tasks should use a distinct path-free execution class, such as
  `SourceInput`, so read models and run summaries do not describe request
  materialization as `NonRuntimeNodeEngine` work.
- `text-input.text` and `boolean-input.value` may project as source-input tasks
  when the canonical node contract exposes those typed ports. Projection must
  not read request payloads, mutate graph node data, or mark missing
  graph-stored values projection-invalid for source-input tasks. Existing
  graph-data-backed `TextInput` and `BooleanInput` non-runtime execution must
  be retired for scheduler-managed session runs or converted into the same
  source-input materialization contract before the runner cutover; it must not
  remain as a parallel successful path.
- The dedicated session runner or initialization helper must materialize
  matching request `WorkflowPortBinding` values into completed
  `WorkflowSchedulerTaskResult` records and complete those source tasks through
  a store-owned atomic materialization operation before dependent tasks advance.
  That operation must not fake a `Running` node-engine execution state to reuse
  `complete_active_run_scheduler_task`; it needs an explicit expected-state
  transition for source-input materialization. Missing request inputs, wrong
  value types, duplicate bindings, unsupported source nodes, and correlation
  mismatches must produce typed diagnostics and blocked or terminal task states;
  they must not mutate graph node data, call `workflow_run_internal`, execute
  through the non-runtime adapter, or use output-node demand.
- The staged `external_input_materialization` helper may remain only as the
  converter-owned source-input materialization boundary after it consumes typed
  source-input templates instead of raw `node_type`/`port_id` checks. Later
  option 4 remains the target: a generic typed port-value source contract
  derived from canonical node contracts for user-authored and future source
  nodes without one enum variant per node.
- 2026-05-24 source-input scheduler contract slice completed. Smallest useful
  vertical slice: replace graph-data-backed `text-input`/`boolean-input`
  non-runtime execution with schema-versioned source-input task projection,
  orchestration state, read-model, summary, and external-input materialization
  contracts. Allowed write set: workflow-service task graph contracts,
  classification/projection, external-input materialization,
  non-runtime adapter, task orchestrator/read-model/summary code and focused
  tests, workflow-service README, plan files, and the execution log.
  No-fallback/no-legacy confirmation: `WorkflowSchedulerNonRuntimeTaskTemplate`
  no longer contains `TextInput` or `BooleanInput`; source input nodes project
  as `WorkflowSchedulerTaskExecutionClass::SourceInput` with
  `WorkflowSchedulerSourceInputTemplate`; the non-runtime adapter rejects
  source-input tasks before node-engine; projection does not read graph-local
  source values; and no branch calls `workflow_run_internal`, output-node
  demand, runtime dispatch, or Pumas path resolution to materialize source
  inputs. Verification passed: `cargo fmt -p
  pantograph-workflow-service`; `cargo test -p pantograph-workflow-service
  workflow::tests::task_graph --lib`; `cargo test -p
  pantograph-workflow-service workflow::external_input_materialization --lib`;
  `cargo test -p pantograph-workflow-service
  workflow::non_runtime_task_adapter --lib`; `cargo test -p
  pantograph-workflow-service workflow::task_run_summary --lib`; `cargo test
  -p pantograph-workflow-service workflow::tests::task_state_read_model --lib`;
  `cargo test -p pantograph-workflow-service scheduler::task_orchestrator
  --lib`; `cargo test -p pantograph-workflow-service scheduler::store --lib`;
  `cargo test -p pantograph-workflow-service
  workflow::task_result_output_projection --lib`; `cargo check -p
  pantograph-workflow-service`; `cargo check -p
  pantograph-workflow-service --no-default-features`; and `cargo check -p
  pantograph-workflow-service --all-features`.
- Deviation/discovered issue resolved in-slice: focused test compilation
  exposed stale `create_session` call sites that lacked the canonical
  attribution argument; the touched test fixtures were updated to the current
  six-argument session-store API instead of preserving an old helper shape.
  Remaining follow-up: the session runner still needs the store-owned atomic
  source-input materialization operation that records completed source-input
  task results and advances dependents without faking a node-engine `Running`
  state. The staged `external_input_materialization` helper remains
  `dead_code` until that runner integration consumes it.
- 2026-05-24 source-input materialization store slice completed. Smallest
  useful vertical slice: add the shared scheduler source-input task-state
  intent and the workflow-service active-run store operation that atomically
  records a completed source-input task result with the completed source-input
  task-state transition. Allowed write set: `pantograph-scheduler` task-state
  contracts/tests/README, workflow-service scheduler store task-result
  module/tests/README, task-state read-model exhaustiveness, plan files, and
  the execution log. No-fallback/no-legacy confirmation: source inputs now
  materialize through `SchedulerSourceInputTaskIntent` and
  `materialize_active_run_source_input_task`; the operation requires
  `AwaitingInputs -> Completed`, `WorkflowSchedulerTaskExecutionClass::SourceInput`,
  a typed source-input template, completed task-result correlation, and one
  active-run store mutation. It does not fake a node-engine `Running` state,
  execute source inputs through runtime/non-runtime adapters, mutate graph node
  data, call output-node demand, or call `workflow_run_internal`. Verification
  passed: `cargo fmt -p pantograph-scheduler -p
  pantograph-workflow-service -- --check`; `cargo test -p pantograph-scheduler
  --test queue_state`; `cargo test -p pantograph-workflow-service
  scheduler::store::store_task_results --lib`; `cargo test -p
  pantograph-workflow-service scheduler::store --lib`; `cargo test -p
  pantograph-workflow-service workflow::tests::task_state_read_model --lib`;
  `cargo check -p pantograph-scheduler`; `cargo check -p pantograph-scheduler
  --no-default-features`; `cargo check -p pantograph-scheduler --all-features`;
  `cargo check -p pantograph-workflow-service`; `cargo check -p
  pantograph-workflow-service --no-default-features`; `cargo check -p
  pantograph-workflow-service --all-features`; and `git diff --check`.
  Remaining follow-up: wire the dedicated session runner to call
  `materialize_external_workflow_inputs`, complete each source-input task via
  this store operation, advance dependents, and remove the staged dead-code
  allowances after consumption.
- 2026-05-24 orchestrator source-input materialization slice completed.
  Smallest useful vertical slice: make `WorkflowSchedulerTaskOrchestrator`
  consume the typed external-input materialization helper and atomic
  source-input active-run store operation through one orchestrator method.
  Allowed write set: workflow-service scheduler orchestrator/tests/README,
  workflow-service module exports, Milestone 5c/task-level orchestration plan
  notes, and this execution log. Existing unrelated Pumas proposal Markdown
  changes remain ignored. No-fallback/no-legacy confirmation: the slice does
  not execute source inputs through node-engine, runtime dispatch, output
  demand, or `workflow_run_internal`; it does not mutate graph node data or
  write source values directly into store maps outside the atomic source-input
  boundary. Verification passed: `cargo fmt -p pantograph-workflow-service
  -- --check`; `cargo test -p pantograph-workflow-service
  scheduler::task_orchestrator --lib`; `cargo test -p
  pantograph-workflow-service workflow::external_input_materialization --lib`;
  `cargo test -p pantograph-workflow-service
  scheduler::store::store_task_results --lib`; `cargo check -p
  pantograph-workflow-service`; `cargo check -p pantograph-workflow-service
  --no-default-features`; `cargo check -p pantograph-workflow-service
  --all-features`; and `git diff --check`. Remaining follow-up: the dedicated
  session runner must call the new orchestrator method, advance dependent task
  readiness, execute ready non-runtime tasks, project scheduler task results to
  requested outputs, and remove the remaining orchestrator staging `dead_code`
  allowances.
- 2026-05-24 non-runtime-only session runner cutover slice completed.
  Smallest useful vertical slice: route non-runtime-only workflow session runs
  through scheduler task progression after queue admission while bypassing
  runtime admission/preflight/load and the legacy whole-run host path. Allowed
  write set: workflow-service session execution, scheduler orchestrator
  re-export/cfg cleanup, focused session execution tests, scheduler README,
  Milestone 5c/task-level orchestration plan notes, and this execution log.
  Existing unrelated Pumas proposal Markdown changes remain ignored.
  No-fallback/no-legacy confirmation: non-runtime-only runs now materialize
  source inputs through the orchestrator, advance dependent non-runtime tasks,
  execute only typed non-runtime templates through the node-engine single-task
  adapter, project outputs from scheduler task results, and finish without
  calling `workflow_run_internal`, output demand, runtime admission, runtime
  load, or runtime-host dispatch. Verification passed: `cargo fmt -p
  pantograph-workflow-service -- --check`; `cargo test -p
  pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_lifecycle_create_run_close
  --lib`; `cargo test -p pantograph-workflow-service
  scheduler::task_orchestrator --lib`; `cargo test -p
  pantograph-workflow-service workflow::task_result_output_projection --lib`;
  `cargo check -p pantograph-workflow-service`; and `cargo check -p
  pantograph-workflow-service --no-default-features`; `cargo check -p
  pantograph-workflow-service --all-features`; and `git diff --check`.
  Discovered issue: the broader
  `workflow::tests::session_execution` suite still contains legacy
  whole-run-host/runtime expectations against non-runtime text graphs; those
  tests need a later cleanup to use runtime-task graphs where runtime behavior
  is the subject, or to assert scheduler diagnostics for non-runtime runs.
  Remaining follow-up: runtime-containing session runs still need the
  scheduler-selected runtime-host dispatch cutover, and the remaining
  orchestrator staging `dead_code` allowances should be removed when that
  branch consumes the runtime handoff APIs.
- 2026-05-25 standards pass: reviewed the milestone against the local plan,
  architecture, Rust API, async/concurrency, testing, tooling, and documentation
  standards. The remaining runtime-handoff work is standards-compliant only if
  dispatch selection stays inside `pantograph-scheduler`, runtime-host output
  contracts stay typed/validated/path-free, workflow-service owns mapping and
  async orchestration without holding locks across awaits, graph editor and
  node-engine remain path-free/non-policy consumers, and every retired
  active-plan/planned-inference success path is deleted rather than preserved.
- 2026-05-25 implementation slice completed the runtime-host response output
  contract in `pantograph-runtime-host-contracts`. Verification passed:
  `cargo fmt -p pantograph-runtime-host-contracts -- --check`,
  `cargo test -p pantograph-runtime-host-contracts`,
  `cargo check -p pantograph-runtime-host-contracts`,
  `cargo check -p pantograph-runtime-host-contracts --all-features`, and
  `cargo check -p pantograph-runtime-host-contracts --no-default-features`.
  Downstream verification `cargo check -p pantograph-workflow-service` passed
  with the known active execution-plan dead-code warning for
  `set_active_run_execution_plan`; that warning remains the recorded
  cross-crate runtime handoff removal boundary and was not hidden.
- 2026-05-25 implementation slice completed the workflow-service
  runtime-host response to task-result mapping boundary. Verification passed:
  `cargo fmt -p pantograph-workflow-service -- --check`,
  `cargo test -p pantograph-workflow-service
  workflow::runtime_host_task_result_mapping --lib`,
  `cargo test -p pantograph-workflow-service
  orchestrator_dispatches_runtime_task_through_shared_runtime_host_port --lib`,
  `cargo check -p pantograph-workflow-service`,
  `cargo check -p pantograph-workflow-service --all-features`, and
  `cargo check -p pantograph-workflow-service --no-default-features`.
  Each `cargo check` still reports only the known
  `set_active_run_execution_plan` warning from the remaining active-plan
  runtime handoff removal boundary; no new mapper warnings remain.
- 2026-05-25 production dispatch-selection replan recorded: the next runtime
  handoff slice should add a minimal `pantograph-scheduler` dispatch-selection
  contract using typed path-free candidates and typed no-selection diagnostics,
  with option 3 discipline so provider-composed runtime/resource/Pumas/history
  inputs can replace the interim candidate list later without changing the
  scheduler-owned selection semantics.
- 2026-05-25 dispatch-selection blast-radius review tightened the plan:
  existing runtime-registry technical-fit candidates and scheduler batch
  candidates are input evidence or downstream policy contracts, not the new
  dispatch candidate shape. The next slice must introduce a stricter scheduler
  dispatch candidate, explicit no-selection diagnostic codes, real
  reservation/resource fact requirements, and pure synchronous selector
  semantics before workflow-service may wire production runtime handoff.
- 2026-05-25 standards iteration completed for the dispatch-selection replan:
  the next slice now has explicit Rust API, documentation, dependency,
  decomposition, async-shell/sync-core, fixture, feature-matrix, and no-legacy
  search requirements. No additional codebase standards blockers were found in
  the plan text after these constraints were added.
- 2026-05-25 implementation slice completed the scheduler dispatch-selection
  contract in `pantograph-scheduler`. The slice added strict path-free
  `SchedulerDispatchSelectionRequest`, `SchedulerDispatchCandidate`, typed
  diagnostics, validated wrappers, a pure synchronous
  `select_scheduler_dispatch` policy, a stable JSON fixture, and focused
  integration tests. The implementation was decomposed into public contract,
  policy, and validation modules so each file stays below the local source-size
  review threshold. Verification passed: `cargo fmt -p pantograph-scheduler --
  --check`; `cargo test -p pantograph-scheduler --test dispatch_selection`;
  `cargo test -p pantograph-scheduler`; `cargo check -p
  pantograph-scheduler`; `cargo check -p pantograph-scheduler --all-features`;
  `cargo check -p pantograph-scheduler --no-default-features`; targeted source
  search for legacy/path/stringly/panic patterns; `wc -l` decomposition check;
  and `git diff --check`. Remaining follow-up: production workflow-service
  runtime-handoff wiring must gather typed candidate facts from canonical
  owners and call this selector; provider-composed option 3 candidate assembly
  remains a later boundary.
- 2026-05-25 implementation slice completed the workflow-service
  dispatch-selected handoff bridge. `WorkflowSchedulerTaskOrchestrator` can now
  accept a validated `SchedulerDispatchSelectionRequest`, call
  `pantograph-scheduler::select_scheduler_dispatch`, build a
  dispatch-selected `SchedulerRuntimeHandoff` only from the returned
  `SchedulerDispatchDecision`, and dispatch through the shared runtime-host
  port. A no-selection scheduler result stops before runtime-host dispatch as a
  typed orchestrator error. Verification passed: `cargo fmt -p
  pantograph-workflow-service -- --check`; `cargo test -p
  pantograph-workflow-service scheduler::task_orchestrator --lib`; `cargo
  check -p pantograph-workflow-service`; `cargo check -p
  pantograph-workflow-service --all-features`; `cargo check -p
  pantograph-workflow-service --no-default-features`; and targeted source
  search for legacy/path/stringly/panic patterns. The workflow-service checks
  still report only the known `set_active_run_execution_plan` warning from the
  active-plan runtime handoff removal boundary. Remaining follow-up:
  production session execution still needs canonical dispatch-selection request
  assembly before runtime-containing runs can leave the fail-closed path.
- 2026-05-25 implementation slice completed dependent runtime task readiness
  initialization. Runtime inference tasks with upstream dependencies now start
  as `AwaitingInputs` even when they already have a valid schedulable intent,
  so scheduler-owned input readiness must materialize connected upstream
  results before runtime dispatch selection can run. Verification passed:
  `cargo fmt -p pantograph-workflow-service -- --check`; `cargo test -p
  pantograph-workflow-service scheduler::task_orchestrator --lib`; `cargo
  check -p pantograph-workflow-service`; `cargo check -p
  pantograph-workflow-service --all-features`; `cargo check -p
  pantograph-workflow-service --no-default-features`; targeted source search
  for legacy/path/stringly/panic patterns; and `git diff --check`. The
  workflow-service checks still report only the known
  `set_active_run_execution_plan` warning. Remaining follow-up: runtime
  input-readiness advancement and canonical dispatch-selection request assembly
  are still required before runtime-containing session runs can leave the
  fail-closed path.
- 2026-06-03 scheduler runtime orchestration staging-allowance cleanup slice
  completed. Smallest useful vertical slice: remove obsolete `dead_code`
  allowances from runtime dispatch/error, runtime input-readiness, and
  dependency-readiness transition helpers that are now consumed by production
  `WorkflowSchedulerTaskOrchestrator` methods and focused tests. No fallback or
  legacy branch was added; runtime execution still flows through
  scheduler-owned readiness, dispatch selection, runtime-host request/response,
  and task-result recording. Verification passed: `cargo fmt -p
  pantograph-workflow-service -- --check`; `cargo check -p
  pantograph-workflow-service`; `cargo test -p pantograph-workflow-service
  scheduler::task_orchestrator --lib`; and `git diff --check`. Remaining
  follow-up: durable cancellation, retry/defer idempotency, duplicate-dispatch
  prevention, reservation release, replay, recovery, and attempt/timing facts
  remain the open scheduler lifecycle hardening work.
- 2026-06-04 runtime-host cancellation contract foundation slice completed.
  Smallest useful vertical slice: extend the runtime-host execution contract so
  every request carries required workflow-service-owned cancellation context
  and every runtime-host port call receives a live cancellation handle. Allowed
  write set used: runtime-host contracts/fixtures/tests, workflow-service
  runtime-host port call sites and focused scheduler/session tests,
  embedded-runtime port implementations/tests, and this plan set.
  No-fallback/no-legacy confirmation: the slice does not add adapter-owned
  lifecycle policy, Tauri/frontend cancellation behavior, runtime launch
  fallbacks, graph-path execution, retry/defer policy, replay/bootstrap,
  diagnostics-ledger writes, Pumas model-fact changes, lockfile changes, or
  saved workflow fixture rewrites. Missing serialized cancellation context is
  a typed contract validation failure.
  Implementation summary: `RuntimeHostExecutionRequest` is now contract
  version 2 and requires `RuntimeHostExecutionCancellationContext`.
  `RuntimeHostExecutionPort` receives a
  `RuntimeHostExecutionCancellationHandle` beside the request.
  `RuntimeHostDispatcher::dispatch` creates a running workflow-service context
  for existing callers, and `dispatch_with_cancellation` exposes the explicit
  supervisor entrypoint for the next slice. Concrete workflow-service and
  embedded-runtime ports compile against the new boundary.
  Verification passed: `cargo fmt -p pantograph-runtime-host-contracts`;
  `cargo fmt -p pantograph-workflow-service`; `cargo fmt -p
  pantograph-embedded-runtime`; `cargo test -p
  pantograph-runtime-host-contracts runtime_host_execution --lib`; `cargo
  test -p pantograph-runtime-host-contracts runtime_host_dispatch --lib`;
  `cargo test -p pantograph-workflow-service scheduler::task_orchestrator
  --lib`; `cargo test -p pantograph-workflow-service
  scheduler::readiness_lifecycle --lib`; `cargo test -p
  pantograph-embedded-runtime runtime_host_execution_port --lib`; `cargo check
  -p pantograph-runtime-host-contracts`; `cargo check -p
  pantograph-workflow-service`; and `cargo check -p
  pantograph-embedded-runtime`.
  Verification note: `git diff --check` passed. Targeted no-fallback/
  deleted-symbol search over touched runtime-host, workflow-service, and
  embedded-runtime files found only existing negative tests/README guardrails,
  path-rejection tests, and a Pumas test fixture path helper; no production
  fallback or legacy execution branch was introduced.
  Deviation/discovered issue: this is the contract foundation only. The
  dispatcher currently supplies a running static cancellation snapshot for
  existing callers so the contract is mandatory without introducing a second
  execution path. The next supervisor slice must replace that with a
  workflow-service-owned live signal for in-flight runtime tasks. Remaining
  follow-up: workflow-service task supervisor with real child cancellation/
  shutdown signal, adapter observation of cancellation/shutdown, await/abort
  and panic handling, then retry/defer idempotency, replay/bootstrap, and
  diagnostics-ledger attempt/timing facts.
- 2026-06-04 lifecycle-owned runtime cancellation signal slice completed.
  Smallest useful vertical slice: move the live runtime-host cancellation
  signal from the dispatcher default into workflow-service task lifecycle
  ownership for started runtime task attempts. Allowed write set used:
  `scheduler/task_lifecycle.rs`, `scheduler/task_lifecycle_tests.rs`,
  `scheduler/task_orchestrator.rs`, `scheduler/task_orchestrator_tests.rs`,
  `workflow/session_scheduler_runner.rs`, and this plan set.
  No-fallback/no-legacy confirmation: the slice validates the active
  task/attempt before creating the runtime-host cancellation handle and routes
  production started runtime dispatch through `dispatch_with_cancellation`.
  It does not add adapter-owned lifecycle policy, Tauri/frontend cancellation
  behavior, graph-path execution, reduced-plan launch, compatibility shims,
  retry/defer policy, replay/bootstrap, diagnostics-ledger writes, Pumas fact
  changes, lockfile changes, or saved workflow fixture edits.
  Implementation summary: `WorkflowSchedulerTaskLifecycleManager` now creates
  `RuntimeHostExecutionCancellationHandle` values backed by a live
  workflow-service signal, updates active signals to `CancellationRequested`
  or `ShutdownRequested`, and rejects stale-attempt signal creation.
  `WorkflowSchedulerTaskOrchestrator::dispatch_started_runtime_task` uses that
  signal after input materialization and before awaiting runtime-host
  execution. The session runner now uses this lifecycle-aware dispatch path
  for started runtime tasks.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo fmt
  -p pantograph-workflow-service -- --check`; `cargo test -p
  pantograph-workflow-service task_lifecycle --lib`; `cargo test -p
  pantograph-workflow-service scheduler::task_orchestrator --lib`; `cargo
  test -p pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_dispatches_ready_runtime_task_through_scheduler_selection
  --lib`; `cargo test -p pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_resume_consumes_fresh_dependency_readiness_snapshot_and_dispatches_active_run
  --lib`; `cargo check -p pantograph-workflow-service`; `git diff --check`;
  and targeted no-fallback search over touched files.
  Verification deviation/discovered issue: the broad
  `cargo test -p pantograph-workflow-service
  workflow::tests::session_execution --lib` command still fails pre-existing
  legacy/non-runtime session expectations and missing saved executable
  snapshot fixtures. The runtime-dispatch session tests that cover this slice
  passed and are listed above.
  Remaining follow-up: make runtime-host adapters observe the live cancellation
  handle, add workflow-service await/abort behavior and panic observation for
  supervised task handles, then continue with retry/defer idempotency,
  replay/bootstrap, and diagnostics-ledger attempt/timing facts.
- 2026-06-04 embedded runtime-host cancellation observation slice completed.
  Smallest useful vertical slice: make the embedded runtime-host execution
  port observe workflow-service-owned cancellation/shutdown at its current
  async boundaries. Allowed write set used:
  `crates/pantograph-runtime-host-contracts/src/runtime_host_execution.rs`,
  `crates/pantograph-embedded-runtime/src/runtime_host_execution_port.rs`, and
  this plan set.
  No-fallback/no-legacy confirmation: the adapter only observes the live
  cancellation handle supplied by workflow-service. It does not own lifecycle
  policy, add Tauri/frontend cancellation behavior, restore graph-path or
  reduced-plan execution, add compatibility shims, introduce retry/defer,
  alter Pumas facts, edit lockfiles, or update saved workflow fixtures.
  Mismatched or invalid cancellation snapshots fail closed as port errors.
  Implementation summary: runtime-host diagnostics now include
  `CancellationRequested` and `ShutdownRequested`; the embedded runtime-host
  port validates cancellation snapshots, rejects mismatched contexts, and
  checks cancellation before dependency resolution, after load-target
  resolution, after package-facts resolution, and before gateway execution.
  Cancellation and shutdown produce typed rejected runtime-host responses with
  stable diagnostic hints.
  Verification passed:
  `cargo fmt -p pantograph-runtime-host-contracts -- --check`;
  `cargo fmt -p pantograph-embedded-runtime -- --check`;
  `cargo test -p pantograph-runtime-host-contracts runtime_host_execution --lib`;
  `cargo test -p pantograph-embedded-runtime runtime_host_execution_port --lib`;
  `cargo test -p pantograph-workflow-service scheduler::task_orchestrator --lib`;
  `cargo check -p pantograph-runtime-host-contracts`;
  `cargo check -p pantograph-embedded-runtime`;
  `cargo check -p pantograph-workflow-service`; and `git diff --check`.
  Verification note: targeted no-fallback/deleted-symbol search over touched
  source found only a Pumas test fixture path helper, not a production
  fallback, legacy execution branch, Tauri/frontend policy, reduced-plan
  launch, or retired runtime handoff.
  Remaining follow-up: the embedded runtime-host port now observes
  cancellation before and between existing async steps, but the image gateway
  still needs cooperative cancellation for mid-inference interruption. Continue
  with workflow-service await/abort and panic observation, deeper runtime
  gateway/provider cancellation, retry/defer idempotency, replay/bootstrap,
  and diagnostics-ledger attempt/timing facts.
- 2026-06-04 active supervisor path re-plan decision recorded. Option 3 is now
  the active next implementation path because option 2's cancellable
  runtime-host contract foundation and embedded-runtime observation are in
  place. Standards alignment: lifecycle/business policy remains owned by
  workflow-service; Tauri and bindings may only forward requests; runtime
  adapters may only observe cancellation; sync task-state/idempotency checks
  stay in the store/lifecycle core; async awaits, shutdown drain/abort, and
  panic observation stay in the workflow-service supervisor shell; failures
  return typed diagnostics without graph-path, reduced-plan, node-engine, or
  compatibility fallback.
  Thin-slice sequence:
  1. Add a workflow-service-owned active task cancellation intent API that
     validates the active task/attempt and updates the lifecycle-owned
     cancellation signal. It must not terminally mutate the task or release
     reservations until the supervised execution owner observes completion or
     cancellation.
  2. Move runtime task execution under a workflow-service supervisor shell with
     tracked async handles, await outside store locks, typed panic/join
     diagnostics, and terminal mutation through the existing store boundary.
  3. Add lifecycle-owner shutdown drain and bounded timeout/abort behavior.
  4. Push cooperative cancellation into long-running image gateway/provider
     calls.
  5. Resume retry/defer idempotency, replay/bootstrap, and diagnostics-ledger
     attempt/timing facts only after the supervisor boundary is stable.
  Rejected path remains adapter/Tauri-owned cancellation because it splits
  lifecycle ownership and moves business policy outside workflow-service.
- 2026-06-04 active cancellation lifecycle-core slice completed. Smallest
  useful vertical slice: preserve workflow-service-owned cancellation/shutdown
  intent on active task handles before runtime-host cancellation handles exist,
  and prove the pending state is projected into later runtime-host dispatch
  without terminal task mutation or reservation release. Allowed write set
  used: `scheduler/task_lifecycle.rs`, `scheduler/task_lifecycle_tests.rs`,
  `scheduler/task_orchestrator.rs`, `scheduler/task_orchestrator_tests.rs`,
  and this plan set.
  No-fallback/no-legacy confirmation: the slice keeps lifecycle/business state
  in workflow-service, changes no Tauri/frontend code, adds no adapter-owned
  policy, does not terminally mutate cancelled tasks, does not release
  reservations early, and does not restore graph-path, reduced-plan,
  node-engine, or compatibility execution branches.
  Implementation summary: `WorkflowSchedulerTaskLifecycleHandleRecord` now
  stores pending runtime-host cancellation state and reason. Cancellation and
  shutdown update that pending state before a signal exists, and
  `runtime_host_cancellation` initializes the later signal from the pending
  state. Existing live signals still update immediately. The orchestrator has
  a test-only harness proving a started runtime task can record cancellation
  before dispatch, remain `Running`, keep its lifecycle handle, and pass
  `CancellationRequested` into runtime-host dispatch.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo fmt
  -p pantograph-workflow-service -- --check`; `cargo test -p
  pantograph-workflow-service task_lifecycle --lib`; `cargo test -p
  pantograph-workflow-service scheduler::task_orchestrator --lib`; `cargo
  check -p pantograph-workflow-service`; `git diff --check`; and targeted
  no-fallback search over touched source. The only search match was an
  existing negative legacy-bridge test name.
  Deviation/discovered issue: the initially planned production active
  cancellation API would have been unused by production code in this slice.
  To stay standards-compliant and avoid dead staging code, the production
  workflow-service active cancellation surface is deferred to the next slice,
  where it must be wired to an actual backend command/API caller while keeping
  Tauri/frontend as forwarding-only layers.
  Remaining follow-up: add the workflow-service-owned active cancellation
  surface, then move runtime task execution under supervised handles with
  typed join/panic diagnostics, shutdown drain/abort, deeper gateway/provider
  cooperative cancellation, retry/defer idempotency, replay/bootstrap, and
  diagnostics-ledger attempt/timing facts.
- 2026-06-04 inference gateway cooperative cancellation slice completed.
  Smallest useful vertical slice: project the workflow-service-owned
  runtime-host cancellation signal through embedded-runtime into inference
  gateway/backend execution context. Allowed write set used:
  inference telemetry/gateway/backend source and tests,
  embedded-runtime runtime-host execution port, and this plan set.
  No-fallback/no-legacy confirmation: cancellation remains on the canonical
  workflow-service to runtime-host to embedded-runtime to inference path and
  does not add Tauri/frontend policy, adapter-owned lifecycle policy,
  graph-path launch, reduced-plan launch, node-engine runtime launch,
  compatibility shims, Pumas fact changes, lockfile edits, or saved workflow
  fixture rewrites.
  Implementation summary: inference now has local cancellation
  handle/snapshot/state DTOs carried by `BackendExecutionContext`; gateway
  image planning/dispatch entrypoints reject typed cancellation before backend
  execution; embedded-runtime bridges runtime-host cancellation into inference;
  and the PyTorch image backend checks the signal before entering its blocking
  Python worker call.
  Verification passed: `cargo fmt -p inference -p pantograph-embedded-runtime`;
  `cargo test -p inference
  test_generate_image_from_plan_with_cancellation_forwards_running_signal
  --lib`; `cargo test -p inference
  test_generate_image_from_planning_input_with_cancellation_rejects_before_backend
  --lib`; `cargo test -p pantograph-embedded-runtime
  port_completes_image_execution_with_sink_backed_media_ref --lib`; `cargo
  test -p pantograph-embedded-runtime
  port_rejects_cancelled_request_before_runtime_dependencies --lib`; `cargo
  check -p inference`; `cargo check -p pantograph-embedded-runtime`; `cargo
  fmt -p inference -p pantograph-embedded-runtime -- --check`; `git diff
  --check`; and targeted no-fallback/no-legacy search over touched source and
  plan files.
  Deviation/discovered issue: the Python worker contract is still a
  synchronous blocking call, so mid-call interruption requires a future
  worker-owned cooperative cancellation contract rather than forced thread
  interruption or Tauri/frontend policy. Remaining follow-up: Python worker
  mid-call cancellation if needed, then retry/defer idempotency,
  replay/bootstrap, and diagnostics-ledger attempt/timing facts.
- 2026-06-04 dependency-readiness retry idempotency slice completed. Smallest
  useful vertical slice: make workflow-service retry of deferred runtime
  dependency readiness idempotent after a task has already re-entered
  `WaitingDependencyReadiness`. Allowed write set used:
  `scheduler/task_orchestrator.rs`, `scheduler/task_orchestrator_tests.rs`,
  and this plan set.
  No-fallback/no-legacy confirmation: this only hardens scheduler task-state
  retry idempotency. It does not add adapter-owned retry policy, Tauri/frontend
  business logic, graph-path launch, reduced-plan launch, node-engine runtime
  launch, compatibility shims, Pumas fact changes, lockfile edits, generated
  files, build outputs, sqlite artifacts, workflow fixture rewrites,
  replay/bootstrap logic, or diagnostics-ledger writes.
  Implementation summary: repeated dependency-readiness retry calls now return
  the current `WaitingDependencyReadiness` record unchanged. Store-level
  `AlreadyApplied` transition results are accepted only by this retry path;
  terminal and other lifecycle transitions still use the strict duplicate
  rejection helper.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
  test -p pantograph-workflow-service
  orchestrator_dependency_readiness_retry_is_idempotent_after_waiting_state
  --lib`; `cargo test -p pantograph-workflow-service task_orchestrator --lib`;
  `cargo check -p pantograph-workflow-service`; `cargo fmt -p
  pantograph-workflow-service -- --check`; `git diff --check`; and targeted
  no-fallback/no-legacy source search over touched scheduler files.
  Remaining follow-up: replay/bootstrap recovery and diagnostics-ledger
  attempt/timing facts.
- 2026-06-04 bootstrap recovery classifier slice completed. Smallest useful
  vertical slice: classify existing active-run runtime scheduler task records
  into typed bootstrap recovery actions and route the existing
  dependency-readiness resume candidate query through that classifier, without
  replaying work or adding diagnostics-ledger writes. Allowed write set used:
  `crates/pantograph-workflow-service/src/scheduler/store.rs`,
  `crates/pantograph-workflow-service/src/scheduler/store_queue.rs`,
  `crates/pantograph-workflow-service/src/scheduler/store_tests.rs`, and this
  plan set.
  No-fallback/no-legacy confirmation: the classifier only reads canonical
  workflow-service active-run scheduler task graph/state. It does not infer
  recovery from graph paths, selector summaries, reduced plans, node-engine
  state, runtime adapters, Tauri/frontend state, Pumas package facts,
  lockfiles, generated files, saved workflow fixtures, or diagnostics-ledger
  records. Missing task-state records are represented as a typed recovery
  action instead of silently falling back.
  Implementation summary: `WorkflowExecutionSessionStore` now exposes an
  active-run bootstrap recovery snapshot for runtime scheduler tasks. The
  snapshot maps task state to `ResumeProgressLoop`,
  `RetryDependencyReadiness`, `RedispatchReadyRuntime`,
  `RuntimeRecoveryRequired`, `Completed`, `TerminalDiagnostic`, or
  `MissingTaskStateRecord`. The existing
  `active_run_dependency_readiness_resume_state` path now consumes this
  classifier, so `RetryableFailed` dependency-readiness tasks are treated
  consistently with other retryable readiness states.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
  test -p pantograph-workflow-service bootstrap_recovery --lib`; `cargo test
  -p pantograph-workflow-service
  dependency_readiness_resume_candidates_use_bootstrap_recovery_classifier
  --lib`; `cargo test -p pantograph-workflow-service scheduler::store --lib`;
  `cargo check -p pantograph-workflow-service`; `cargo fmt -p
  pantograph-workflow-service -- --check`; `git diff --check`; and targeted
  no-fallback/no-legacy source search over touched scheduler files. Matches
  were existing Pumas test helper imports and existing warm-compatibility
  naming only.
  Deviation/discovered issue: a broader aggregate bootstrap snapshot API was
  intentionally not kept in this slice because no production bootstrap owner
  consumes it yet, and leaving it unused would violate the standards bias
  against dead staging code.
  Remaining follow-up: add the workflow-service-owned bootstrap/replay runner
  that consumes the active-run recovery snapshot to reconcile incomplete
  attempts without duplicate dispatch, then add diagnostics-ledger
  attempt/timing facts after replay ordering is proven.
- 2026-06-04 bootstrap recovery report API slice completed. Smallest useful
  vertical slice: expose the active-run recovery classifier through a
  workflow-service-owned read-only report API so future bootstrap/replay code
  and forwarding adapters consume backend DTOs instead of re-deriving
  scheduler policy. Allowed write set used:
  `crates/pantograph-workflow-service/src/scheduler/mod.rs`,
  `crates/pantograph-workflow-service/src/scheduler/store_queue.rs`,
  `crates/pantograph-workflow-service/src/workflow/contracts.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
  `crates/pantograph-workflow-service/src/lib.rs`, and this plan set.
  No-fallback/no-legacy confirmation: the API only reports classifications
  derived from canonical workflow-service active-run scheduler task
  graph/state. It performs no replay, task-state mutation, runtime-host call,
  diagnostics-ledger write, Tauri/frontend policy, graph-path inference,
  reduced-plan launch, node-engine runtime launch, compatibility shim, Pumas
  fact change, lockfile edit, generated file edit, or saved workflow fixture
  rewrite.
  Implementation summary: workflow contracts now include bootstrap recovery
  report/run/task/action DTOs. `WorkflowService` exposes
  `workflow_execution_session_bootstrap_recovery_report`, maps internal
  scheduler recovery actions into workflow DTOs, and aggregates active-run
  snapshots from the session store. The existing readiness-pending workflow
  test asserts the report for a real active run deferred at dependency
  readiness.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
  test -p pantograph-workflow-service
  workflow_execution_session_runtime_run_defers_pending_dependency_readiness_before_dispatch
  --lib`; `cargo test -p pantograph-workflow-service bootstrap_recovery
  --lib`; `cargo check -p pantograph-workflow-service`; `cargo fmt -p
  pantograph-workflow-service -- --check`; `git diff --check`; and targeted
  no-fallback/no-legacy source search over touched files. Matches were
  existing negative legacy tests, existing Pumas fixture setup, existing
  warm-compatibility naming, and existing diagnostics compatibility payload
  fields only.
  Remaining follow-up: implement the workflow-service-owned bootstrap/replay
  runner that consumes this report to reconcile incomplete attempts without
  duplicate dispatch, then add diagnostics-ledger attempt/timing facts after
  replay ordering is proven.
- 2026-06-04 bootstrap recovery planning slice completed. Smallest useful
  vertical slice: add a workflow-service-owned read-only recovery plan over the
  canonical bootstrap recovery report so replay/recovery owners can consume
  typed decisions without embedding policy in adapters. Allowed write set used:
  `workflow/contracts.rs`, `workflow/session_execution_api.rs`,
  `workflow/tests/session_execution.rs`, `lib.rs`, and the plan docs.
  No-fallback/no-legacy confirmation: no replay, task-state mutation,
  runtime-host call, reservation release, diagnostics-ledger write,
  Tauri/frontend policy, graph-path execution, node-engine path, reduced-plan
  path, Pumas/package fact change, lockfile change, generated rewrite, or
  workflow fixture rewrite was added.
  Implementation summary: workflow contracts now expose recovery decision and
  plan DTOs. `WorkflowService` exposes
  `workflow_execution_session_bootstrap_recovery_plan`; the pure planner maps
  dependency-readiness retry actions to deduplicated resume requests, blocks
  ready runtime redispatch with a persisted-readiness-proof and duplicate-dispatch guard diagnostic, and
  emits typed blocking decisions for runtime recovery required, terminal
  diagnostic, and missing task-state records.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
  test -p pantograph-workflow-service
  bootstrap_recovery_plan_blocks_ready_runtime_redispatch_without_recovery_state
  --lib`; `cargo test -p pantograph-workflow-service
  workflow_execution_session_runtime_run_defers_pending_dependency_readiness_before_dispatch
  --lib`; `cargo test -p pantograph-workflow-service bootstrap_recovery
  --lib`; `cargo check -p pantograph-workflow-service`; `cargo fmt -p
  pantograph-workflow-service -- --check`; `git diff --check`; and targeted
  no-fallback/no-legacy source search. Search matches were existing diagnostics
  compatibility payload fields, existing negative legacy tests, and existing
  Pumas test fixtures only.
  Remaining follow-up: implement the workflow-service-owned bootstrap/replay
  runner that applies safe plan decisions with durable
  duplicate-dispatch/idempotency protection, then add diagnostics-ledger
  attempt/timing facts after replay ordering is proven.
- 2026-06-04 bootstrap recovery apply slice completed. Smallest useful
  vertical slice: add the first mutating workflow-service recovery runner that
  consumes the backend recovery plan and applies only dependency-readiness
  resume requests through the existing canonical runtime dependency-readiness
  resume API. Allowed write set used: `workflow/contracts.rs`,
  `workflow/session_execution_api.rs`, `workflow/tests/session_execution.rs`,
  `lib.rs`, and the plan docs.
  No-fallback/no-legacy confirmation: the runner fails closed on blocking
  decisions, rejects unsupported nonblocking decisions such as
  progress-loop replay, and does not redispatch ready runtime tasks, invent a
  replay loop, mutate scheduler state directly, call Tauri/frontend code,
  change graph/node-engine/reduced-plan execution, edit Pumas/package facts,
  rewrite lockfiles/generated files/workflow fixtures, or write
  diagnostics-ledger attempt/timing facts.
  Implementation summary: workflow contracts now include
  `WorkflowExecutionSessionBootstrapRecoveryResult`. `WorkflowService`
  exposes `recover_workflow_execution_session_bootstrap`; it gates the plan,
  delegates safe dependency-readiness recovery to the existing resume API, and
  returns the applied plan plus resumed run responses.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
  test -p pantograph-workflow-service
  bootstrap_recovery_apply_gate_rejects_unimplemented_progress_loop_replay
  --lib`; `cargo test -p pantograph-workflow-service
  workflow_execution_session_bootstrap_recovery_applies_dependency_readiness_resume_plan
  --lib`; `cargo test -p pantograph-workflow-service bootstrap_recovery
  --lib`; `cargo check -p pantograph-workflow-service`; `cargo fmt -p
  pantograph-workflow-service -- --check`; `git diff --check`; and targeted
  no-fallback/no-legacy source search. Search matches were existing diagnostics
  compatibility payload fields, existing negative legacy tests, and existing
  Pumas test fixtures only.
  Remaining follow-up: implement explicit progress-loop replay, durable
  duplicate-dispatch/idempotency guard for ready runtime redispatch, and
  diagnostics-ledger attempt/timing facts.
- 2026-06-04 bootstrap progress-loop recovery slice completed. Smallest useful
  vertical slice: allow the workflow-service bootstrap recovery runner to
  apply `ResumeProgressLoop` decisions through the existing scheduler progress
  loop, then recompute the recovery plan before applying dependency-readiness
  resumes. Allowed write set used: `workflow/contracts.rs`,
  `workflow/session_execution_api.rs`, `workflow/session_scheduler_runner.rs`,
  `workflow/tests/session_execution.rs`, and the plan docs.
  No-fallback/no-legacy confirmation: the slice reuses canonical
  workflow-service scheduler progress behavior, does not redispatch ready
  runtime tasks, does not create graph/node-engine/reduced-plan execution, does
  not move policy to Tauri/frontend, does not edit Pumas/package facts,
  lockfiles, generated files, or workflow fixtures, and still blocks ready
  runtime redispatch through the recomputed runtime redispatch recovery-state decision.
  Implementation summary: `WorkflowSchedulerSessionRunner` exposes a narrow
  `resume_progress_loop` wrapper; bootstrap recovery deduplicates active runs
  needing progress-loop replay, applies progress, replans, gates the recomputed
  plan, and returns both initial and final recovery plans.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
  test -p pantograph-workflow-service
  bootstrap_recovery_progress_loop_requests_dedupe_by_active_run --lib`;
  `cargo test -p pantograph-workflow-service
  workflow_execution_session_bootstrap_recovery_applies_progress_loop_before_readiness_resume
  --lib`; `cargo test -p pantograph-workflow-service bootstrap_recovery
  --lib`; `cargo check -p pantograph-workflow-service`; `cargo fmt -p
  pantograph-workflow-service -- --check`; `git diff --check`; and targeted
  no-fallback/no-legacy source search. Search matches were existing diagnostics
  compatibility payload fields, existing negative legacy tests, and existing
  Pumas test fixtures only.
  Remaining follow-up: implement durable duplicate-dispatch/idempotency guard
  for ready runtime redispatch, then diagnostics-ledger attempt/timing facts.
- 2026-06-04 runtime redispatch recovery-state diagnostic slice completed.
  Smallest useful vertical slice: refine ready-runtime bootstrap redispatch
  from a duplicate-dispatch-only blocker into a typed
  `BlockedRuntimeRedispatchRecoveryStateRequired` decision that names both
  durable prerequisites: persisted readiness proof and
  duplicate-dispatch/idempotency state. Allowed write set used:
  `workflow/contracts.rs`, `workflow/session_execution_api.rs`, and the plan
  docs.
  No-fallback/no-legacy confirmation: no ready runtime task is redispatched,
  no graph/node-engine/reduced-plan execution path is added, no policy moves
  to Tauri/frontend, no Pumas/package facts, lockfiles, generated files, or
  workflow fixtures are edited, and old runtime behavior is not preserved.
  Discovered issue: canonical ready dispatch needs an admitted readiness proof,
  but the ready task record does not currently persist that proof for bootstrap
  replay. Ready redispatch must remain typed diagnostics until that proof and
  duplicate-dispatch guard state are durable.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
  test -p pantograph-workflow-service
  bootstrap_recovery_plan_blocks_ready_runtime_redispatch_without_recovery_state
  --lib`; `cargo test -p pantograph-workflow-service bootstrap_recovery
  --lib`; `cargo check -p pantograph-workflow-service`; `cargo fmt -p
  pantograph-workflow-service -- --check`; `git diff --check`; and targeted
  no-fallback/no-legacy source search. Search matches were existing diagnostics
  compatibility payload fields, existing negative legacy tests, and existing
  Pumas test fixtures only.
  Remaining follow-up: persist ready runtime dispatch recovery state needed for
  bootstrap redispatch, then diagnostics-ledger attempt/timing facts.
- 2026-06-04 ready runtime dispatch recovery proof persistence slice completed.
  Smallest useful vertical slice: persist the admitted dependency readiness
  proof when a runtime task reaches canonical `Ready` state, then project proof
  availability through bootstrap recovery report and plan DTOs. Allowed write
  set used: `scheduler/store.rs`, `scheduler/store_queue.rs`,
  `scheduler/store_task_results.rs`, `scheduler/task_orchestrator.rs`,
  `scheduler/store_tests.rs`, `scheduler/task_orchestrator_tests.rs`,
  `workflow/contracts.rs`, `workflow/session_execution_api.rs`, and the plan
  docs.
  No-fallback/no-legacy confirmation: no ready runtime redispatch is
  implemented yet, no graph/node-engine/reduced-plan execution is added, no
  policy moves to Tauri/frontend, no Pumas/package facts, lockfiles, generated
  files, or workflow fixtures are edited, and old runtime behavior is not
  preserved. Bootstrap recovery remains typed diagnostics until durable
  duplicate-dispatch/idempotency guard state exists.
  Implementation summary: workflow-service active-run state now stores
  `DependencyReadinessProofEnvelope` by scheduler task id. The orchestrator
  records the proof only after readiness admission applies `Ready`; bootstrap
  recovery snapshots and workflow DTOs expose
  `runtime_dispatch_recovery_state_available`, and the blocked redispatch
  diagnostic distinguishes missing proof from proof-present but missing guard
  state.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
  test -p pantograph-workflow-service
  orchestrator_persists_started_runtime_task_result --lib`; `cargo test -p
  pantograph-workflow-service
  active_run_bootstrap_recovery_snapshot_classifies_runtime_task_states
  --lib`; `cargo test -p pantograph-workflow-service
  bootstrap_recovery_plan_reports_persisted_proof_when_guard_state_missing
  --lib`; `cargo test -p pantograph-workflow-service bootstrap_recovery
  --lib`; `cargo check -p pantograph-workflow-service`; `cargo fmt -p
  pantograph-workflow-service -- --check`; `git diff --check`; and targeted
  no-fallback/no-legacy source search. Search matches were existing diagnostics
  compatibility payload fields, existing negative legacy tests, existing Pumas
  test fixtures, existing warm-compatibility naming, and existing node-engine
  non-runtime task diagnostics only.
  Remaining follow-up: implement durable duplicate-dispatch/idempotency guard
  state for ready runtime redispatch, then diagnostics-ledger attempt/timing
  facts.
- 2026-06-04 ready runtime redispatch recovery slice completed. Smallest
  useful vertical slice: make proof-present ready runtime bootstrap recovery
  actionable by routing it through the existing active-run resume shell and
  scheduler start-attempt guard, while proof-missing ready runtime tasks remain
  blocked with typed diagnostics. Allowed write set used:
  `workflow/contracts.rs`, `workflow/session_execution_api.rs`,
  `workflow/session_scheduler_runner.rs`, `workflow/tests/session_execution.rs`,
  and the plan docs.
  No-fallback/no-legacy confirmation: redispatch uses the canonical
  workflow-service runner, persisted dependency readiness proof, runtime
  dispatch source/candidate providers, scheduler dispatch selection,
  scheduler start-attempt mutation, runtime-host execution port, and
  reservation lifecycle path. It does not add graph/node-engine/reduced-plan
  runtime execution, Tauri/frontend policy, Pumas/package fact changes,
  lockfiles, generated files, workflow fixtures, or compatibility shims.
  Missing proof still fails closed.
  Discovered issue fixed: the progress loop previously attempted to start
  every ready task through the non-runtime node-engine path. Ready runtime
  tasks are now filtered out by immutable scheduler task execution class so
  runtime dispatch owns them.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
  test -p pantograph-workflow-service
  bootstrap_recovery_plan_accepts_ready_redispatch_with_recovery_state --lib`;
  `cargo test -p pantograph-workflow-service
  workflow_execution_session_bootstrap_recovery_redispatches_ready_runtime_task
  --lib`; and `cargo test -p pantograph-workflow-service bootstrap_recovery
  --lib`; `cargo check -p pantograph-workflow-service`; `cargo fmt -p
  pantograph-workflow-service -- --check`; `git diff --check`; and targeted
  no-fallback/no-legacy source search. Search matches were existing diagnostics
  compatibility payload fields, existing Pumas test fixtures, and existing
  node-engine non-runtime task diagnostics only.
  Remaining follow-up: add diagnostics-ledger attempt/timing facts.
  2026-06-04 diagnostics-ledger attempt/timing re-plan decision: use Option 1
  next. The next source slice must add a `pantograph-diagnostics-ledger`
  scheduler task-attempt event contract and focused validation tests before
  workflow-service emits any attempt events. The event contract must include
  scheduler task id, attempt id, execution class, lifecycle transition,
  started/ended/duration timing fields where applicable, and optional
  runtime/reservation facts. Do not reuse `WorkflowTimingObservation` for this
  contract because it is run/node timing-history data and does not own
  scheduler attempt lifecycle identity. 2026-06-04 follow-up adjustment:
  because the payload enum is a shared public contract, the same slice may
  update `workflow/diagnostics_api.rs` only to classify the new payload in the
  existing projection-refresh match so workflow-service remains compilable.
  Workflow-service event emission, scheduler lifecycle producer wiring, and
  query/read-model behavior remain later slices after the ledger-owned
  contract and consumer compile adaptation are frozen.
  2026-06-04 source update: the ledger-owned
  `scheduler.task_attempt_lifecycle_changed` contract is implemented with
  scheduler task id, attempt id, execution class, lifecycle transition,
  terminal timing validation, optional runtime/reservation facts, public
  exports, focused diagnostics-ledger tests, existing diagnostic event ledger
  persistence/projection-summary support, and one workflow-service
  projection-refresh classification test. The slice did not add
  workflow-service event emission or scheduler lifecycle producer wiring.
  Remaining follow-up: wire workflow-service start/terminal/cancel/redispatch
  emission through the diagnostics-ledger append helper, then add
  projection/read-model fields only after emitted event ordering is proven.
  2026-06-05 active-lane reconciliation: later committed slices have emitted
  started, terminal, cancelled, and redispatched attempt lifecycle events,
  projected them into the backend scheduler timeline, and displayed those
  typed facts on the Scheduler page. Remaining retry/defer decision facts,
  replay outcomes, worker lifecycle events, cooperative runtime cancellation
  response mapping, and additional task-state attempt counters are still
  Milestone 5c hardening work, but they are not blockers for returning to
  Milestone 5b legacy runtime deletion. If a 5b deletion slice needs a missing
  lifecycle fact to avoid preserving a legacy path, stop and re-plan that fact
  as a backend-owned scheduler/runtime-host contract extension.
