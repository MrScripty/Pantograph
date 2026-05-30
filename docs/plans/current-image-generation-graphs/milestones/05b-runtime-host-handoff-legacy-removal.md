# Milestone 5b: Runtime Host Handoff And Legacy Execution Removal

**Goal:** Replace successful `model_path`/`ModelRefV2` runtime execution with
direct scheduler-to-runtime-host execution that consumes scheduler-owned
handoff facts and resolves Pumas-approved load targets only at the host
boundary.

This milestone is split out of Milestone 5a because replacing node-engine
dependency preflight alone would preserve the legacy successful execution path.
Milestone 5a owns scheduler contracts and dynamic dispatch. Milestone 5b owns
runtime-host handoff wiring and deletion of the old resolver/path contracts.
Milestone 5b is a hard gate before real image-generation execution slices can
depend on runtime loading: the canonical runtime-host request/response,
host-owned Pumas load-target resolution, and direct scheduler dispatch caller
must exist before old successful `ModelRefV2`/`model_path` paths are deleted.

Selected re-plan direction as of 2026-05-23: plan for option 4 before the
remaining production runtime-host wiring. Runtime execution must be initiated
by scheduler task dispatch with the actual dispatch-selected
`SchedulerRuntimeHandoff`. Workflow progress must be driven by durable
scheduler task state, not whole-workflow node-engine output demand. Do not
synthesize handoff from `WorkflowExecutionPlanNodeDecision`, and do not keep
planned inference as an alternate successful launch path.

Selected re-plan refinement as of 2026-05-29: the next runtime/dependency
cleanup uses the contract-first readiness/handoff replacement path. The
remaining work crosses scheduler readiness, workflow-service materialization,
runtime-host handoff, node-engine preflight retirement, and legacy fixture
deletion. It must not be implemented as another graph-authoring cleanup slice
or by adapting canonical readiness/handoff data into `ModelDependencyRequest`,
`ModelRefV2`, or `model_path` success behavior.

Selected scheduler state-machine re-plan as of 2026-05-29: use the direct
scheduler-contract transition path. Runtime inference tasks with upstream graph
inputs must progress from `AwaitingInputs` to `WaitingDependencyReadiness`
after their connected inputs materialize, then proceed through canonical
dependency readiness admission and dispatch selection. Do not route runtime
tasks through `Ready` as a temporary compatibility detour, and do not encode
this transition only in workflow-service; the legal lifecycle belongs in the
`pantograph-scheduler` task-state contract.

**Tasks:**

- [x] Define the runtime-host execution request/response contract first. It must
  consume `SchedulerRuntimeHandoff`, scheduler dispatch decision, dependency
  environment ref, and Pumas model/artifact identity without exposing
  `ModelRefV2`, `model_path`, executable load targets, reservations, batching
  groups, or worker launch internals to graph/node-engine contracts.
- [ ] Consume the canonical `DependencyReadinessProofEnvelope` through the
  remaining production runtime-host dispatch and legacy-retirement path. The
  proof must come from backend validation summaries, dependency-planning facts,
  selected runtime/device facts, descriptor fingerprints, explicit user
  constraints, and typed availability evidence. It must carry
  freshness/correlation ids, selected environment identity when one exists,
  and bounded diagnostics without graph-visible paths, executable load targets,
  scheduler policy in Tauri/frontend code, or a second adapter-local readiness
  proof type.
- [x] Add the host-owned Pumas load-target resolution service. It must resolve
  executable load targets only from scheduler-selected Pumas refs/artifact
  identity at runtime dispatch, and return typed unavailable/stale/invalid
  diagnostics instead of falling back to paths.
- [x] Add the scheduler-owned runtime-host execution dispatch port. It must
  accept only `RuntimeHostExecutionRequest`, return
  `RuntimeHostExecutionResponse`, and keep request/cancellation/retry
  correlation in scheduler/application orchestration rather than runtime
  adapters.
- [x] Update the scheduler task-state transition contract to allow runtime
  inference tasks with materialized upstream inputs to transition directly from
  `AwaitingInputs` to `WaitingDependencyReadiness`. The scheduler contract
  must require a runtime execution intent for that state, keep non-runtime
  input advancement on the existing `AwaitingInputs` to `Ready` path, and
  reject any attempt to use this transition as a generic shortcut around
  dependency readiness.
- [ ] Complete Milestone 5c task-level scheduler orchestration before
  continuing production runtime-host wiring. The scheduler must own durable
  task graph/state/result progression so runtime-host dispatch receives actual
  dispatch-selected handoff from task state rather than a reduced workflow
  execution-plan projection.
- [ ] Wire scheduler dispatch to call runtime-host execution directly from the
  actual dispatch-selected `SchedulerRuntimeHandoff`. The reduced
  `WorkflowExecutionPlan` may remain an inspection/diagnostics projection but
  must not be used to launch inference or build handoff. Runtime requests must
  include the canonical dependency readiness proof and workflow-service-owned
  materialized runtime inputs derived from validated upstream task results.
- [x] Wire the session/runtime runner to call workflow-service runtime input
  advancement after upstream task results are recorded. Selected re-plan:
  implement option 2 first with option 3 discipline. First extract the existing
  non-runtime-only progression loop into a dedicated workflow-service scheduler
  session runner with no behavior change, then add runtime-containing
  progression that materializes source inputs, executes allowlisted
  non-runtime upstream tasks, advances dependent runtime tasks to
  `WaitingDependencyReadiness`, and fails closed before runtime-host dispatch
  until the dispatch slice is wired. Option 3 remains the target: a durable
  task runner with leases, replay, batching, retry/defer, cancellation, and
  multi-workflow scheduling hooks. Direct wiring through the existing
  fail-closed runtime session branch is not allowed because it changes
  legacy/session expectations and risks reintroducing broad compatibility
  behavior. The runner must keep graph editing, validation, dependency
  readiness, runtime input materialization, and runtime-host dispatch as
  separate boundaries.
- [ ] Retire node-engine planned-inference launch ownership for runtime
  inference nodes. Affected nodes must submit or reference scheduler task
  intent and consume scheduler task state/results; missing scheduler task state
  must fail closed with typed diagnostics.
- [ ] Replace PyTorch execution so successful model loading consumes
  scheduler-dispatched runtime-host requests plus host-owned executable facts
  and no longer reads graph `model_path`, reduced execution-plan projections,
  or emits `ModelRefV2`.
- [ ] Replace llama.cpp execution so successful model loading consumes
  scheduler-dispatched runtime-host requests plus host-owned executable facts
  and no longer reads graph `model_path`, reduced execution-plan projections,
  or emits `ModelRefV2`.
- [ ] Replace audio execution so successful model loading consumes host-owned
  executable facts and no longer reads graph `model_path`, reduced
  execution-plan projections, or emits `ModelRefV2`.
- [ ] Replace node-engine dependency preflight output with typed readiness or
  scheduler task-state facts after scheduler-to-runtime-host dispatch exists.
  Missing scheduler task state must fail closed with typed diagnostics, not
  repair old inputs. Any old dependency/preflight command reached before its
  replacement must be diagnostic-only and must not successfully produce
  `ModelDependencyRequest`, `ModelRefV2`, path-shaped dependency payloads, or
  executable launch inputs.
- [ ] Remove embedded-runtime `ModelDependencyResolver`/`ModelRefV2` resolution
  paths after runtime host load-target resolution is wired.
- [ ] Remove retired node-engine contracts and helpers:
  `ModelDependencyRequest`, `ModelDependencyResolver`, `ModelRefV2`,
  `build_model_ref_v2`, `PlannedInferenceExecutionHost`, path repair helpers,
  and successful `model_path` test fixtures only after scheduler-to-runtime-host
  dispatch and load-target resolution are wired.
- [ ] Remove frontend/Tauri dependency actions keyed by `modelPath` or
  `model_path` after backend capability and task diagnostics cover the
  replacement user-visible state.
- [ ] Update README/crate documentation for every new host-facing contract,
  Pumas load-target boundary, runtime migration, deleted legacy path, and
  fixture replacement.

**Verification:**

- Contract tests and JSON fixtures for runtime-host execution input/output and
  Pumas load-target diagnostics.
- Contract tests and JSON fixtures for dependency readiness proof success,
  stale descriptor fingerprint, missing dependency availability, invalid
  explicit runtime/device constraint, unavailable environment, and selected
  environment identity.
- Boundary tests proving graph, node-engine, saved-workflow, scheduler hint,
  and scheduler handoff payloads reject executable path fields.
- Runtime-host tests proving Pumas load targets are resolved only at the host
  boundary and unavailable states produce typed diagnostics.
- Queue admission and scheduler dispatch tests proving readiness proof
  freshness is checked before enqueue/materialization and again before
  runtime-host dispatch, and that stale/missing proof fails closed before any
  runtime-host call.
- Scheduler dispatch tests proving runtime-host requests are created only from
  dispatch-selected `SchedulerRuntimeHandoff`, not from
  `WorkflowExecutionPlanNodeDecision` or backend-decision projections.
- Workflow/session execution tests proving scheduler task dispatch calls the
  runtime-host execution port, records typed responses, preserves
  cancellation/retry correlation, and does not launch inference through
  node-engine.
- Task-level orchestration tests proving workflow progress can pause and
  resume between tasks, admit work from another workflow while one run is
  waiting, and dispatch runtime tasks from scheduler task state rather than
  whole-workflow output demand.
- Scheduler task-state contract tests proving `AwaitingInputs` can advance
  directly to `WaitingDependencyReadiness` only with a runtime execution
  intent; non-runtime tasks still advance to `Ready`; invalid, unavailable, or
  missing inputs fail closed without readiness admission or runtime-host
  dispatch.
- PyTorch, llama.cpp, and audio tests proving successful execution no longer
  reads graph `model_path`, launches from reduced execution-plan projections,
  or emits `ModelRefV2`.
- Node-engine tests proving affected runtime nodes fail closed when scheduler
  task state is missing and do not call `ModelDependencyResolver` or
  `PlannedInferenceExecutionHost`.
- Deletion/search checks proving successful production paths no longer contain
  `ModelDependencyResolver`, `ModelDependencyRequest`, `ModelRefV2`,
  `build_model_ref_v2`, `PlannedInferenceExecutionHost`, frontend `modelPath`
  dependency actions, direct old runtime task success fixtures, or path-shaped
  success fixtures.
- Focused crate checks for every touched Rust crate, including default,
  all-features, and no-default-features checks when public feature contracts
  change.

**No-Fallback Requirements:**

- Do not adapt scheduler readiness or handoff facts back into `ModelRefV2`.
- Do not adapt canonical dependency readiness proof back into
  `ModelDependencyRequest` or path-shaped dependency requests.
- Do not synthesize scheduler handoff from reduced workflow execution-plan
  facts or backend execution projections.
- Do not route runtime inference tasks through `Ready` as a workaround for
  missing scheduler lifecycle support before dependency readiness admission.
- Do not preserve `model_path`/`modelPath` as successful runtime execution
  identity.
- Do not leave old resolver calls as alternate successful execution branches.
- Do not leave `PlannedInferenceExecutionHost` as an alternate successful
  inference launch branch after direct scheduler dispatch is wired.
- Do not let node-engine, runtime adapters, frontend actions, or Tauri commands
  choose scheduler runtime/device/dependency policy.
- Do not expose executable Pumas load targets outside the runtime host
  boundary.

**Status:**

- [ ] In progress.
- 2026-05-22: Created from the Milestone 5a node-engine legacy boundary
  re-plan. Decision: use Option 3 planning structure with Option 1
  implementation direction. Milestone 5b owns runtime-host handoff and legacy
  execution removal; Milestone 5a continues scheduler-owned dynamic dispatch.
- 2026-05-22: Milestone 5a closeout decision recorded. Option 2 selected:
  close Milestone 5a as scheduler-contract complete and keep actual legacy
  deletion in this milestone as the hard gate. Implementation must begin with
  the runtime-host execution request/response contract, then host-owned Pumas
  load-target resolution, then runtime migrations and deletion. Do not create
  a scheduler-handoff-to-`ModelRefV2` adapter or a path-repair bridge while
  crossing this boundary.
- 2026-05-22 runtime-host request/response contract slice completed. Smallest
  useful vertical slice: add the embedded-runtime host-facing execution
  request/response DTOs, validated wrappers, typed diagnostics, and JSON
  fixtures without resolving Pumas load targets or launching runtimes. Allowed
  write set: `crates/pantograph-embedded-runtime/`, this milestone file, and
  execution notes. No-fallback confirmation: the request consumes a
  dispatch-selected `SchedulerRuntimeHandoff` and rejects readiness-only
  handoff; request/response contracts expose no executable load target, local
  path, `ModelDependencyRequest`, `ModelRefV2`, graph `model_path`, frontend
  `modelPath`, path repair, reservation/batching internals, or worker launch
  details. Verification passed: `cargo fmt -p pantograph-embedded-runtime`,
  `cargo test -p pantograph-embedded-runtime runtime_host_execution`,
  `cargo check -p pantograph-embedded-runtime`,
  `cargo check -p pantograph-embedded-runtime --all-features`,
  `cargo check -p pantograph-embedded-runtime --no-default-features`,
  `cargo fmt -p pantograph-embedded-runtime -- --check`, `git diff --check`,
  README coverage review, source/test fixture directory coverage review, and
  file-size standards check for new runtime-host source/test files. Remaining
  follow-up: add host-owned Pumas load-target resolution service.
- 2026-05-22 host-owned Pumas load-target resolution slice completed.
  Smallest useful vertical slice: add the embedded-runtime host-only load
  target resolver module that builds Pumas requests from validated
  runtime-host execution requests and maps ready/unavailable Pumas responses
  into host-owned results without wiring runtime execution. Allowed write set:
  `crates/pantograph-embedded-runtime/`, this milestone file, and execution
  notes. No-fallback confirmation: the resolver uses scheduler-selected Pumas
  model/artifact identity and Pumas typed resolver states only; it does not
  accept graph `model_path`, frontend `modelPath`, `ModelDependencyRequest`,
  `ModelRefV2`, path repair, package-fact scraping, or alternate successful
  resolver branches. Verification passed: `cargo fmt -p
  pantograph-embedded-runtime`, `cargo test -p pantograph-embedded-runtime
  runtime_host_load_target`, `cargo test -p pantograph-embedded-runtime
  runtime_host_execution`, crate check matrix, fmt check, diff checks, README
  coverage review, and file-size standards check. Remaining follow-up before
  PyTorch migration: wire scheduler dispatch to call runtime-host execution
  directly from the actual dispatch-selected scheduler handoff.
- 2026-05-23: Re-plan boundary recorded before the PyTorch migration slice.
  Initial direction was direct scheduler-owned dispatch before runtime
  migration. A later option 4 re-plan in this file supersedes that ordering by
  requiring task-level scheduler orchestration before production runtime-host
  wiring continues. The existing `EmbeddedPlannedInferenceExecutionHost`
  launches from a reduced `WorkflowExecutionPlanNodeDecision`; that projection
  cannot be the source of truth for `RuntimeHostExecutionRequest`. No
  scheduler-handoff synthesis from reduced plans, no backend-decision bridge,
  and no alternate planned-inference successful branch are allowed.
- 2026-05-23 scheduler-owned runtime-host execution dispatch port slice
  completed. Smallest useful vertical slice: add the embedded-runtime
  scheduler dispatch port and dispatcher that builds
  `RuntimeHostExecutionRequest` only from an actual
  `SchedulerRuntimeHandoff`, validates dispatch-selected request shape before
  calling the runtime-host port, and validates runtime-host response
  correlation before returning a validated response. Allowed write set:
  `crates/pantograph-embedded-runtime/`, this milestone file, and execution
  notes. No-fallback confirmation: the dispatcher exposes no constructor from
  `WorkflowExecutionPlan`, `WorkflowExecutionPlanNodeDecision`,
  `BackendExecutionDecision`, graph inputs, `ModelRefV2`, or `model_path`; it
  rejects readiness-only handoff before the runtime-host port is called and
  does not wire or preserve planned-inference launch behavior. Verification
  passed: `cargo fmt -p pantograph-embedded-runtime`, `cargo test -p
  pantograph-embedded-runtime runtime_host_dispatch`, `cargo check -p
  pantograph-embedded-runtime`, `cargo check -p pantograph-embedded-runtime
  --all-features`, `cargo check -p pantograph-embedded-runtime
  --no-default-features`, `cargo fmt -p pantograph-embedded-runtime --
  --check`, `git diff --check`, README coverage review, and file-size
  standards check. Remaining follow-up: wire scheduler dispatch to call the
  runtime-host execution port from the actual dispatch-selected scheduler
  handoff.
- 2026-05-23 option 4 re-plan recorded. The next production wiring step must
  first implement Milestone 5c task-level scheduler orchestration because
  workflow-service session execution currently stores only a reduced execution
  plan and advances the whole workflow through node-engine output demand.
  Direct runtime-host dispatch must come from durable scheduler task state and
  actual dispatch-selected handoff; no reduced-plan handoff synthesis,
  node-engine planned-inference launch, or `ModelRefV2` bridge is allowed.
- 2026-05-29 contract-first readiness/handoff replacement re-plan recorded.
  The remaining runtime/dependency cleanup is a scheduler/workflow-service/
  runtime-host boundary change, not a graph cleanup. The selected path is to
  consume the existing canonical `DependencyReadinessProofEnvelope` in queue
  admission and dispatch, pass it into runtime-host requests with materialized
  runtime inputs, make old dependency/preflight paths fail closed if reached,
  then delete `ModelDependencyRequest`, `ModelRefV2`, model-path success
  behavior, planned-inference launch ownership, direct old runtime task success
  fixtures, and frontend/Tauri path-shaped dependency actions. No second
  readiness proof type or adapter-local compatibility proof is allowed.
- 2026-05-29 runtime-host materialized input contract slice completed.
  Smallest useful vertical slice: require shared runtime-host execution
  requests to carry explicit typed `materialized_inputs`, update scheduler
  dispatcher/workflow-service orchestrator calls to pass those values, and
  remove the lingering nested `selected_artifact_path` from the runtime-host
  request fixture. Allowed write set: `pantograph-runtime-host-contracts`,
  workflow-service task orchestrator call sites/tests, this milestone file,
  and execution notes. No-fallback confirmation: materialized inputs are
  path-free runtime-host contract values; missing input field, path-shaped
  input fields, oversized input lists, readiness-only handoff, and old path/
  model-ref names are rejected or absent. Verification passed: `cargo fmt -p
  pantograph-runtime-host-contracts -p pantograph-workflow-service`; `cargo
  test -p pantograph-runtime-host-contracts -- --nocapture`; `cargo test -p
  pantograph-workflow-service task_orchestrator --lib -- --nocapture`; `cargo
  check -p pantograph-runtime-host-contracts`; `cargo check -p
  pantograph-runtime-host-contracts --all-features`; `cargo check -p
  pantograph-runtime-host-contracts --no-default-features`; `cargo check -p
  pantograph-workflow-service`; `cargo fmt -p
  pantograph-runtime-host-contracts -p pantograph-workflow-service -- --check`;
  targeted source search over the runtime-host contract and touched
  workflow-service orchestrator files for retired path/model-ref terms; and
  `git diff --check`. Existing caveat: `cargo check -p
  pantograph-workflow-service` still emits the known unused
  `set_active_run_execution_plan` warning.
- 2026-05-29 workflow-service runtime-host task input materialization slice
  completed. Smallest useful vertical slice: add the workflow-service-owned
  mapper from completed scheduler task results into typed
  `RuntimeHostExecutionInput` values and wire selected scheduler dispatch to
  use it before the runtime-host port is called. Allowed write set:
  workflow-service runtime-host task input mapping, task orchestrator tests/
  README, this milestone file, and execution notes. No-fallback confirmation:
  the mapper consumes only validated completed upstream task results, rejects
  missing, unavailable, failed, invalid, and unsupported materialized values,
  and skips Pumas model-ref bindings only on explicit model-ref target ports
  because model identity remains in scheduler handoff/readiness facts rather
  than runtime materialized inputs. It does not read graph paths, synthesize
  model paths, call legacy dependency resolvers, or adapt results into
  `ModelRefV2`. Verification passed: `cargo fmt -p
  pantograph-workflow-service`; `cargo test -p pantograph-workflow-service
  runtime_host_task_input_mapping --lib -- --nocapture`; `cargo test -p
  pantograph-workflow-service task_orchestrator --lib -- --nocapture`;
  `cargo check -p pantograph-workflow-service`; `cargo fmt -p
  pantograph-workflow-service -- --check`; targeted source search over the
  touched mapper/orchestrator/README files for retired path/model-ref terms;
  and `git diff --check`. Existing caveat: `cargo check -p
  pantograph-workflow-service` still emits the known unused
  `set_active_run_execution_plan` warning.
- 2026-05-29 scheduler state-machine re-plan recorded. Re-plan boundary:
  workflow-service can now materialize runtime-host inputs, but dependent
  runtime inference tasks cannot advance from `AwaitingInputs` to
  `WaitingDependencyReadiness` because the scheduler transition contract does
  not permit that edge. Selected option: update `pantograph-scheduler` first
  so the domain state machine owns the legal transition and requires a runtime
  execution intent. Rejected options: route through `Ready` as a temporary
  bridge, add a new runtime-specific state before it is needed, or put the
  lifecycle exception only in workflow-service. Standards alignment: this keeps
  lifecycle policy in the scheduler core contract, preserves correct-by-
  construction state transitions, avoids compatibility/fallback behavior, and
  requires focused scheduler contract tests before workflow-service runtime
  input advancement is retried.
- 2026-05-29 scheduler runtime-input transition contract slice completed.
  Smallest useful vertical slice: update the `pantograph-scheduler` task-state
  transition matrix so runtime inference tasks can advance directly from
  `AwaitingInputs` to `WaitingDependencyReadiness`, and enforce that
  `WaitingDependencyReadiness` carries a runtime execution intent. Allowed
  write set: `crates/pantograph-scheduler/src/queue.rs`,
  `crates/pantograph-scheduler/tests/queue_state.rs`, this milestone file, and
  execution notes. No-fallback confirmation: the slice does not dispatch
  runtime work, does not route runtime tasks through `Ready`, does not
  synthesize scheduler handoff, and does not introduce `ModelRefV2`,
  `ModelDependencyRequest`, graph paths, or executable load targets.
  Verification passed: `cargo fmt -p pantograph-scheduler`; `cargo test -p
  pantograph-scheduler --test queue_state -- --nocapture`; `cargo check -p
  pantograph-scheduler`; `cargo fmt -p pantograph-scheduler -- --check`;
  `cargo check -p pantograph-scheduler --all-features`; `cargo check -p
  pantograph-scheduler --no-default-features`; targeted source search over
  touched scheduler files for retired path/model-ref terms; and `git diff
  --check`. Search caveat: allowed hits remain in the negative
  path-shaped-field rejection test and the existing typed Pumas fixture value.
  Remaining follow-up: retry workflow-service runtime input advancement on top
  of the scheduler-owned transition contract.
- 2026-05-29 workflow-service runtime input advancement slice completed.
  Smallest useful vertical slice: add a workflow-service orchestrator method
  that advances a dependent runtime inference task from `AwaitingInputs` to
  `WaitingDependencyReadiness` only after all connected upstream scheduler task
  results have materialized. Allowed write set:
  `crates/pantograph-workflow-service/src/scheduler/task_orchestrator.rs`,
  `crates/pantograph-workflow-service/src/scheduler/task_orchestrator_tests.rs`,
  this milestone file, and execution notes. No-fallback confirmation: the
  slice does not dispatch runtime work, does not route runtime tasks through
  `Ready`, does not synthesize handoff, does not read graph paths or executable
  load targets, and does not adapt runtime readiness into `ModelRefV2` or
  `ModelDependencyRequest`. Blocked inputs return no transition; unavailable
  or invalid materialized inputs move through typed scheduler diagnostics.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
  test -p pantograph-workflow-service task_orchestrator --lib --
  --nocapture`; `cargo check -p pantograph-workflow-service`; `cargo fmt -p
  pantograph-workflow-service -- --check`; `cargo check -p
  pantograph-workflow-service --all-features`; `cargo check -p
  pantograph-workflow-service --no-default-features`; targeted retired
  path/model-ref source search over touched workflow-service scheduler files
  and the session API; and `git diff --check`. Verification caveat: `cargo
  check -p pantograph-workflow-service` still emits the known unused
  `set_active_run_execution_plan` warning. Deviation recorded: session-runner
  wiring was attempted and reverted because it widened existing session
  behavior before the dedicated runtime runner boundary was planned and tested.
  Remaining follow-up: implement the dedicated session/runtime runner slice so
  upstream result recording invokes this advancement path without reviving old
  planned-inference launch behavior.
- 2026-05-29 session scheduler runner extraction slice completed. Smallest
  useful vertical slice: extract the existing non-runtime-only session
  progression loop from `session_execution_api.rs` into
  `workflow/session_scheduler_runner.rs` with no runtime-containing behavior
  change. Allowed write set: `crates/pantograph-workflow-service/src/workflow.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_scheduler_runner.rs`,
  `crates/pantograph-workflow-service/src/workflow/README.md`, this milestone
  file, `10-task-level-scheduler-orchestration.md`, and execution notes.
  No-fallback confirmation: the slice only moves existing non-runtime source
  materialization, non-runtime task execution, task-result projection, and
  no-legacy whole-run host avoidance into a dedicated runner. It does not add
  runtime progression, runtime-host dispatch, node-engine output demand,
  planned-inference launch, reduced-plan handoff synthesis, `ModelRefV2`,
  `ModelDependencyRequest`, graph paths, or a runtime `Ready` detour.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
  test -p pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_lifecycle_create_run_close
  --lib -- --nocapture`; `cargo test -p pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_timeout_applies_to_scheduler_task_runner
  --lib -- --nocapture`; `cargo test -p pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_runtime_run_fails_closed_before_legacy_launch
  --lib -- --nocapture`; `cargo check -p pantograph-workflow-service`;
  `cargo fmt -p pantograph-workflow-service -- --check`; `cargo check -p
  pantograph-workflow-service --all-features`; `cargo check -p
  pantograph-workflow-service --no-default-features`; targeted retired
  path/model-ref source search over touched session runner/API files; and
  `git diff --check`. Verification caveat: `cargo check -p
  pantograph-workflow-service` still emits the known unused
  `set_active_run_execution_plan` warning. Verification deviation: the broad
  `cargo test -p pantograph-workflow-service workflow::tests::session_execution
  --lib -- --nocapture` suite still has legacy expectation failures around old
  runtime load/whole-run host behavior and missing executable validation
  snapshots; this extraction slice did not widen scope to rewrite those tests.
  Remaining follow-up: add runtime-containing progression through the runner
  in the next slice.
- 2026-05-29 runtime-containing session runner progression slice completed.
  Smallest useful vertical slice: extend `workflow/session_scheduler_runner.rs`
  so runtime-containing session runs materialize request source inputs, advance
  allowlisted non-runtime upstream tasks through typed scheduler task results,
  advance runtime inference tasks from `AwaitingInputs` to
  `WaitingDependencyReadiness`, verify that the active run reached the runtime
  dispatch boundary, and then fail closed before runtime-host dispatch.
  Allowed write set: `crates/pantograph-workflow-service/src/workflow/session_scheduler_runner.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
  this milestone file, `10-task-level-scheduler-orchestration.md`, workflow
  README notes, and execution notes. No-fallback confirmation: the slice does
  not call runtime-host execution, does not call node-engine whole-run output
  demand, does not route runtime tasks through `Ready`, does not synthesize
  scheduler handoff from reduced execution-plan projections, does not load
  runtime sessions, and does not adapt runtime readiness into `ModelRefV2`,
  `ModelDependencyRequest`, graph paths, or executable load targets. Focused
  test coverage stores a path-free executable validation snapshot before queue
  admission and proves runtime-containing runs fail with the scheduler
  dispatch capability violation after reaching the runtime dispatch boundary,
  while host runtime-load and whole-run execution attempts remain zero.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
  test -p pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_runtime_run_advances_to_dispatch_boundary_before_fail_closed
  --lib -- --nocapture`; `cargo test -p pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_runtime_run_fails_closed_before_legacy_launch
  --lib -- --nocapture`; `cargo test -p pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_lifecycle_create_run_close
  --lib -- --nocapture`; `cargo test -p pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_timeout_applies_to_scheduler_task_runner
  --lib -- --nocapture`; `cargo check -p pantograph-workflow-service`;
  `cargo fmt -p pantograph-workflow-service -- --check`; `cargo check -p
  pantograph-workflow-service --all-features`; `cargo check -p
  pantograph-workflow-service --no-default-features`; `git diff --check`;
  and targeted retired path/model-ref source search over touched session
  runner/API/README/test files. Search caveat: allowed pre-existing hits
  remain in documentation negative-path text and an unrelated legacy runtime
  state fixture in `session_execution.rs`.
  Verification caveat: `cargo check -p pantograph-workflow-service` still
  emits the known unused `set_active_run_execution_plan` warning. Discovered
  issue recorded: the current static `llm-inference` contract does not expose
  model-specific ports such as `prompt`, so a realistic
  `prompt -> inference.prompt` runtime-input edge is still rejected by graph
  submit validation until the inference-interface descriptor ports are applied
  to saved graph validation/admission. This slice intentionally did not add a
  fake static prompt port or any compatibility shim. Remaining follow-up:
  wire dependency readiness admission, scheduler dispatch selection, and
  runtime-host request construction from the actual dispatch-selected
  `SchedulerRuntimeHandoff`.
- 2026-05-29 session runner dependency-readiness admission slice completed.
  Smallest useful vertical slice: wire `WorkflowSchedulerSessionRunner` to
  build canonical dependency-readiness requests for runtime tasks after input
  materialization, resolve the configured provider outside the session-store
  lock, apply scheduler dependency-readiness admission, and stop before
  runtime-host dispatch unless the task becomes dispatch-ready. Allowed write
  set: workflow-service scheduler readiness lifecycle exports, workflow service
  configuration/default provider, session runner, focused session execution
  tests, scheduler/workflow README notes, this milestone file,
  `10-task-level-scheduler-orchestration.md`, and execution notes.
  No-fallback confirmation: the default provider is the no-I/O
  not-implemented dependency-environment service, so runtime runs fail closed
  through scheduler readiness admission before dispatch. The slice does not
  call runtime-host execution, does not load runtime sessions, does not call
  node-engine whole-run output demand, does not route runtime tasks through a
  compatibility `Ready` detour before admission, does not synthesize handoff
  from reduced execution-plan projections, and does not adapt readiness into
  `ModelRefV2`, `ModelDependencyRequest`, graph paths, or executable load
  targets. Focused coverage proves the host runtime load and whole-run
  execution attempts remain zero while the runtime task is rejected as not
  admitted for dispatch.
  Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
  test -p pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_runtime_run_requires_dependency_readiness_before_dispatch
  --lib -- --nocapture`; `cargo test -p pantograph-workflow-service
  scheduler::readiness_lifecycle --lib -- --nocapture`; `cargo test -p
  pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_runtime_run_fails_closed_before_legacy_launch
  --lib -- --nocapture`; `cargo test -p pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_lifecycle_create_run_close
  --lib -- --nocapture`; `cargo check -p pantograph-workflow-service`; and
  `cargo fmt -p pantograph-workflow-service -- --check`; `cargo check -p
  pantograph-workflow-service --all-features`; `cargo check -p
  pantograph-workflow-service --no-default-features`; targeted retired
  path/model-ref source search over touched source/README/test files; and
  `git diff --check`. Search caveat: allowed pre-existing hits remain in
  documentation negative-path text and an unrelated legacy runtime state
  fixture in `session_execution.rs`. Verification caveat: `cargo check -p
  pantograph-workflow-service` still emits the known unused
  `set_active_run_execution_plan` warning. Remaining follow-up: replace the
  default not-implemented readiness provider with production readiness
  evidence, then wire scheduler dispatch selection and runtime-host request
  construction from the actual dispatch-selected `SchedulerRuntimeHandoff`.
- 2026-05-29 dependency-environment provider composition hook slice completed.
  Smallest useful vertical slice: expose a workflow-service configuration hook
  that accepts the canonical `SharedDependencyEnvironmentProvider`, wraps it in
  `DependencyEnvironmentService`, and uses that service as the runtime
  dependency-readiness provider for session execution. Allowed write set:
  workflow service configuration, focused session execution tests, workflow
  README notes, this milestone file, `10-task-level-scheduler-orchestration.md`,
  and execution notes. No-fallback confirmation: the hook does not accept
  `DependencyPreflightResult`, readiness-proof JSON, `ModelDependencyRequest`,
  `ModelRefV2`, graph paths, runtime-host handoff, or executable load targets
  from callers. It only wires the canonical dependency-environment provider
  facade already owned by backend composition. Focused coverage proves an
  injected ready provider admits the runtime task past dependency readiness and
  still fails closed at the runtime dispatch-not-wired boundary with host
  runtime-load and whole-run execution attempts remaining zero.
  Verification passed: `cargo fmt -p
  pantograph-workflow-service`; `cargo test -p pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_ready_dependency_readiness_stops_at_dispatch_boundary
  --lib -- --nocapture`; and `cargo test -p pantograph-workflow-service
  workflow::tests::session_execution::workflow_execution_session_runtime_run_requires_dependency_readiness_before_dispatch
  --lib -- --nocapture`; `cargo check -p pantograph-workflow-service`; `cargo
  check -p pantograph-workflow-service --all-features`; `cargo check -p
  pantograph-workflow-service --no-default-features`; `cargo fmt -p
  pantograph-workflow-service -- --check`; targeted retired path/model-ref
  source search over touched source/README/test files; and `git diff --check`.
  Search caveat: allowed pre-existing retired-path hits remain in workflow
  README negative-path text and an unrelated legacy runtime state fixture in
  `session_execution.rs`. Verification caveat: `cargo check -p
  pantograph-workflow-service` still emits the known unused
  `set_active_run_execution_plan` warning. Remaining follow-up: implement the
  production dependency-environment provider/fact source and then wire
  scheduler dispatch selection/runtime-host request construction from the
  actual dispatch-selected `SchedulerRuntimeHandoff`.
- 2026-05-29 production dependency-readiness source re-plan selected. Decision:
  implement a snapshot-backed production dependency-environment provider first,
  with embedded-runtime or another infrastructure owner as the later snapshot
  producer. The provider must read validated backend-owned readiness snapshots
  and return canonical `DependencyEnvironmentResult` values through the
  existing `DependencyEnvironmentService` facade. It must remain path-free,
  synchronous at the provider/session-runner boundary, and fail closed when no
  fresh matching snapshot exists. It must not block on filesystem/process/
  package probes, create runtimes, spawn untracked tasks, or derive readiness
  from `ModelDependencyRequest`, `ModelRefV2`, technical-fit preview facts,
  reduced execution plans, graph node data, Tauri/frontend payloads, graph
  `model_path`/`modelPath`, runtime-host handoff, or executable load targets.
  Standards alignment: this keeps backend-owned data as the source of truth,
  preserves validated contract DTOs across crate boundaries, keeps scheduler
  policy synchronous, places future async probing in an owned infrastructure
  lifecycle, and supports deterministic tests. Next thin slice: define the
  snapshot DTO/store/provider contract in the smallest backend-owned module,
  add focused provider tests for ready/missing/stale/mismatched snapshots, and
  add a session acceptance test proving only a fresh matching snapshot admits a
  runtime task to the dispatch boundary. Deferred follow-up: implement the
  async snapshot producer with tracked handles, cancellation, shutdown,
  retries, and tracing before using real host package/runtime probes. Guardrail:
  the injected-ready provider remains test/dev scaffolding only and must not
  become production readiness authority.
