# Plan: Current Image Generation Graphs And Stale Graph Diagnostics

2026-06-05 worker system alignment re-plan decision: complete the
backend-owned worker systems for `queue_worker` and
`resource_observation_loop` before exposing public worker lifecycle snapshots
or diagnostics-ledger worker lifecycle events. The scheduler lifecycle
registry intentionally introduced worker-shaped vocabulary ahead of those
concrete owners; keeping that vocabulary now requires implementing real
workflow-service/composition-root-owned workers rather than attaching state to
request-scoped actions. Request APIs may remain as command/query surfaces, but
they must not own queue progression, resource polling, retry loops, shutdown,
or lifecycle business policy once the workers exist. This supersedes the
earlier active execution-lane instruction to immediately continue Milestone 5b
legacy runtime deletion wherever that would leave queue/resource lifecycle
terms backed only by request-scoped behavior. The next source lane is
Milestone 5c worker-system alignment: implement the queue worker and resource
observation worker in thin slices, then attach their real lifecycle state,
then add public lifecycle snapshots/ledger worker events only after ordering,
shutdown, and replay semantics are validated.

2026-06-05 queue worker owner update: workflow-service scheduler code now has
the first concrete backend-owned `queue_worker` owner. The worker owns bounded
wake and shutdown mechanics and drives the shared scheduler lifecycle
registry from explicit `Running` to `ShuttingDown`/`Shutdown`; it does not
move queue progression yet, expose public lifecycle snapshots, emit
diagnostics-ledger worker events, add retry/defer policy, observe resources,
or restore legacy runtime launch paths. The next worker-system slice must
migrate queue progression business behavior out of request-scoped session
execution paths so request APIs signal/query the worker instead of owning the
business loop.

2026-06-05 queue progression API re-plan decision: use Option 1,
worker-owned completion while preserving the existing blocking
`run_workflow_execution_session` response shape. The request path may still
validate, enqueue, and await `WorkflowRunResponse`, but it must await a
worker-owned completion channel/receiver rather than polling/admitting/
executing queue work itself. The queue worker must own `begin_queued_run`,
scheduler task-state setup, non-runtime/runtime progression, terminal
mutation, completion signaling, lifecycle state, and shutdown. Enqueue,
cancel, reprioritize, push-front, status, and inspection APIs remain
request-scoped command/query surfaces only. Rejected for the next slice:
enqueue-only public API migration and full durable event-driven queue, because
both require broader caller/contract/generated/frontend coordination than the
next thin worker migration slice allows. This decision is superseded for
execution/completion ownership by the Option 4 queue/task execution ownership
re-plan below; keep it as historical context for the completed queue
admission migration slices.

2026-06-05 queue admission owner update: workflow-service session execution
now submits admission polling through the queue worker owner module instead
of directly polling `begin_queued_run` in the request path. The slice added an
internal `WorkflowSchedulerQueueAdmissionCommand` and focused queue-worker
tests for immediate and blocked-then-unblocked admission. This does not yet
move scheduler task-state setup, non-runtime/runtime progression, terminal
mutation, timeout handling, or completion signaling behind the worker; those
remain the next Option 1 source slice and must be removed from the
request-scoped execution path without adding fallback branches.

2026-06-05 queue task-state owner update: admitted-run scheduler task-state
initialization now goes through the queue worker owner module via an internal
`WorkflowSchedulerQueueTaskStateCommand`. `run_workflow_execution_session`
still computes the task graph and initial records before admission because
that summary is needed for the existing blocking response path, but it no
longer writes active-run scheduler task state directly. Remaining queue
progression work: move non-runtime/runtime progression, timeout handling,
terminal mutation, and completion signaling behind the queue worker owner.

2026-06-05 queue non-runtime progression owner update: non-runtime-only
session execution now delegates started-event recording, scheduler
non-runtime progression, timeout handling, terminal mutation, terminal
diagnostics recording, and IO artifact event recording through an internal
`WorkflowSchedulerQueueWorker::run_non_runtime_to_completion` helper. This
preserves the blocking public response shape while removing the non-runtime
execution branch from direct request ownership. Remaining queue progression
work: move runtime dispatch-boundary progression, dependency-readiness deferral
handling, terminal mutation, and completion signaling behind the queue worker
owner.

2026-06-05 queue runtime progression owner update: runtime-containing session
execution now delegates started-event recording, runtime dispatch-boundary
progression, timeout handling, dependency-readiness pending deferral,
failure finish mutation, and terminal diagnostics through an internal
`WorkflowSchedulerQueueWorker::run_until_runtime_dispatch_boundary` helper.
This preserves the blocking public response shape while removing the runtime
execution branch from direct request ownership. Remaining queue progression
work: move the unhandled scheduler class fail-closed branch and final
completion signaling semantics behind the queue worker owner, then continue
to the resource observation worker slice.

2026-06-05 queue unhandled-class owner update: the unhandled scheduler class
fail-closed branch now delegates started-event recording, scheduler task
fail-closed transitions, failure finish mutation, and terminal diagnostics
through an internal
`WorkflowSchedulerQueueWorker::fail_unhandled_scheduler_classes_to_completion`
helper. The blocking request path now routes all admitted run branches through
queue-worker-owned helpers after admission and task-state initialization.
Remaining queue-worker follow-up: introduce explicit worker completion
signaling/channel semantics and worker-unavailable diagnostics before public
lifecycle snapshots or diagnostics-ledger worker lifecycle events.

2026-06-05 queue/task execution ownership re-plan decision: use Option 4
instead of turning `queue_worker` into the full workflow-run execution owner.
The queue worker owns queue lifecycle, wake/shutdown, admission, and handoff.
A separate backend task execution/scheduler owner must own admitted scheduler
task progression across runs, including non-runtime execution, runtime
dispatch-boundary progression, timeout handling, terminal mutation,
completion signaling, and typed unavailable/shutdown diagnostics for
execution. This better matches the coding standards' simplicity/complection
and single-owner rules and keeps the future path open for batching related
tasks from simultaneous workflow runs. The existing queue-worker progression
helpers are accepted only as an interim migration state; the next source
slices must define the task execution owner boundary, move branch progression
helpers out of `queue_worker` into that owner, and make `run_workflow_execution_session`
submit/await backend-owned task execution completion without preserving
request-owned fallback execution. Do not add fake oneshot wrappers around
direct helper calls, and do not expose public lifecycle snapshots or
diagnostics-ledger worker lifecycle events until the queue worker,
task-execution owner, and resource-observation owner all have real lifecycle
semantics.

2026-06-05 task execution owner boundary update: workflow-service now has an
internal `WorkflowTaskExecutionOwner` module and the non-runtime-only admitted
run branch routes through that task execution owner instead of
`WorkflowSchedulerQueueWorker`. This is the first Option 4 source slice: queue
worker ownership is narrowed toward queue lifecycle/admission/handoff, while
non-runtime scheduler progression, timeout handling, finish mutation, terminal
diagnostics, and IO artifact events are owned by the task execution owner. The
slice does not add public DTOs, public lifecycle snapshots, diagnostics-ledger
worker lifecycle events, request-owned fallback execution, or legacy runtime
launch behavior. Remaining Option 4 task-execution owner work: move runtime
dispatch-boundary progression and unhandled-class fail-closed progression out
of queue-worker helpers, then add real backend-owned completion signaling and
typed execution-unavailable/shutdown diagnostics.

2026-06-05 task execution owner runtime-boundary update: runtime-containing
admitted runs now route runtime dispatch-boundary progression through
`WorkflowTaskExecutionOwner` instead of `WorkflowSchedulerQueueWorker`. The
task execution owner now owns both non-runtime-only progression and
runtime-dispatch-boundary progression, including timeout handling,
dependency-readiness pending deferral, failure finish mutation, and terminal
diagnostics for runtime-containing runs. Remaining Option 4 task-execution
owner work: move the unhandled-class fail-closed branch out of the queue
worker helper, then add real backend-owned completion signaling and typed
execution-unavailable/shutdown diagnostics.

2026-06-05 task execution owner unhandled-branch update: unhandled scheduler
class fail-closed progression now routes through `WorkflowTaskExecutionOwner`
instead of `WorkflowSchedulerQueueWorker`. All admitted-run branch progression
helpers have now moved out of the queue worker into the task execution owner.
Queue worker ownership is queue lifecycle, wake/shutdown, admission, and
handoff; task execution owner ownership is branch progression, timeout
handling, terminal mutation, and terminal diagnostics. Remaining Option 4
work: add real backend-owned completion signaling and typed task-execution
unavailable/shutdown diagnostics, then continue the resource observation
worker before public lifecycle snapshots or diagnostics-ledger worker events.

2026-06-05 task execution completion ownership re-plan decision: use Option
2, reusing the existing workflow-service `WorkflowSchedulerTaskLifecycleManager`
as the canonical task-execution lifecycle/completion owner. `WorkflowTaskExecutionOwner`
must consume or wrap that real lifecycle manager instead of inventing a second
execution state machine, adding fake oneshot wrappers around direct helper
calls, or making request handlers own task completion policy. This matches the
coding standards' simplicity/complection rule because queue lifecycle,
task-execution lifecycle, and resource observation remain separate concerns;
it also matches the single-owner and sync-core/async-shell rules because
task handle state, shutdown, cancellation, stale completion rejection, and
typed lifecycle diagnostics stay in the existing synchronous lifecycle core
while async execution remains an application-layer shell. The next source
slices must thread the existing task lifecycle manager into
`WorkflowTaskExecutionOwner`, gate execution when it is shutting down, map
lifecycle errors to typed task-execution unavailable/shutdown diagnostics, and
complete lifecycle handles from real branch completion. Do not build a new
async task-execution worker or durable event stream until this existing owner
is integrated; those remain future batching/replay work after the blocking
inference path and resource observation worker are complete.

2026-06-05 task execution lifecycle availability gate update:
`WorkflowTaskExecutionOwner` now checks the existing
`WorkflowSchedulerTaskLifecycleManager` through the scheduler task orchestrator
before queue admission. If the task lifecycle owner is shutting down or shut
down, new execution fails closed with typed `CapabilityViolation` diagnostics
and the queued item is cancelled instead of admitting work that cannot be
owned by the backend lifecycle manager. The slice does not add request-owned
fallback execution, queue-worker branch progression, fake completion channels,
public lifecycle snapshots, diagnostics-ledger worker events, node-engine
whole-run launch, planned-inference launch, graph-path inference, or
frontend/Tauri policy. Remaining Option 2 work: register and complete real
task lifecycle handles for any branch that is moved behind a future async
worker loop, then continue the resource observation worker before exposing
public lifecycle snapshots or worker lifecycle ledger events. Discovery:
normal non-runtime, runtime dispatch-boundary, and unhandled-class terminal
paths already route through orchestrator start/terminal methods that track and
release task lifecycle handles; the remaining known timeout gap is runtime
dispatch timeout cleanup.

2026-06-05 task execution timeout cleanup update: non-runtime session
execution timeouts now ask the scheduler task orchestrator to cancel any
running tasks for the workflow run through active scheduler task state and to
release their lifecycle handles through the existing task lifecycle manager.
The public error remains typed `RuntimeTimeout`, and the cleanup does not add
request-owned execution, fallback completion, fake channels, queue-worker
branch progression, public lifecycle snapshots, or diagnostics-ledger worker
events. Remaining timeout follow-up: runtime dispatch timeout cleanup still
needs explicit supervisor cancellation and reservation release semantics; do
not solve that by dropping task handles without aborting/reconciling the
runtime supervisor.

2026-06-06 runtime dispatch timeout cleanup re-plan decision: use Option 3 as
the immediate implementation path. Add an orchestrator-owned runtime timeout
cancellation command that owns the whole cleanup sequence for in-flight
runtime dispatch: request runtime cancellation, abort/drain the tracked
supervisor with bounded timeout, terminally mutate the matching scheduler task
attempt, apply reservation release/reconcile, release the task lifecycle
handle, and return typed cleanup diagnostics. The request/task-execution
wrapper may only translate timeout into this backend command and return typed
`RuntimeTimeout`; it must not own supervisor lifecycle, task-state mutation,
reservation policy, retry, fallback completion, or legacy execution. Option 4
is recorded as the later durable worker/replay evolution after this immediate
cleanup is validated; do not implement the full event-driven task execution
worker in the next slice.

2026-06-06 runtime task-execution worker re-plan decision: supersede the
immediate Option 3 cleanup-command path and use Option 4 as the active next
implementation path. Build a backend-owned task-execution worker/event loop
that owns admitted task progression, runtime supervisor handles,
timeout/cancellation, reservation release/reconcile, lifecycle handle
completion, and typed diagnostics. This aligns terminology with the intended
worker architecture and gives the scheduler a task-level execution boundary
that can later batch related work across simultaneous workflow runs. The
request/session wrapper may only validate, enqueue, await the worker-owned
completion, and translate typed worker outcomes; it must not own runtime
supervisor lifecycle, task-state mutation, reservation policy, retry,
fallback completion, legacy execution, or worker shutdown. Option 3 remains
recorded as historical context only; do not implement the standalone runtime
timeout cleanup command before the worker path unless a new re-plan explicitly
restores it.

2026-06-06 task-execution worker contract slice: completed the first Option 4
source slice by adding an internal `task_execution_worker` module with
task-attempt-scoped worker commands, terminal/deferred/unavailable outcomes,
shutdown reasons, and typed diagnostics. The contracts are not public DTOs,
do not introduce a worker loop yet, and do not encode fallback, node-engine
whole-run, planned-inference, graph-path inference, frontend/Tauri, or
compatibility-shim execution paths. Verification:
`cargo fmt -p pantograph-workflow-service` and
`cargo test -p pantograph-workflow-service task_execution_worker --lib`.
Next Option 4 slice: make runtime supervisor ownership drainable and
lifecycle-owned before moving runtime dispatch execution behind the worker.

2026-06-06 runtime supervisor drainability slice: completed the second
Option 4 source slice by making `WorkflowSchedulerStartedRuntimeTaskSupervisor`
explicitly expose an abort handle, abort command, and `abort_and_join` drain
method. Runtime dispatch still runs through the existing scheduler
orchestrator/runtime-host path; the change only makes supervisor cleanup a
first-class lifecycle operation for the future worker. Verification:
`cargo fmt -p pantograph-workflow-service` and
`cargo test -p pantograph-workflow-service runtime_task_supervisor_ --lib`.
Next Option 4 slice: introduce the bounded backend task-execution worker loop
startup/shutdown shell without moving runtime dispatch behavior yet.

2026-06-06 bounded task-execution worker shell slice: completed the third
Option 4 source slice by adding a scheduler lifecycle
`task_execution_worker` component and a bounded internal worker shell with
startup, command-queue observation, shutdown signaling, idempotent shutdown,
and worker-unavailable diagnostics for queue send failures. The shell is not
wired into request execution and does not execute, complete, retry, or
fallback-run task attempts. Verification:
`cargo fmt -p pantograph-workflow-service`,
`cargo test -p pantograph-workflow-service task_execution_worker --lib`, and
`cargo test -p pantograph-workflow-service lifecycle_component_kinds_have_stable_snapshot_names --lib`.
Discovery handled in-slice: the worker module needed the scheduler lifecycle
types through crate re-exports instead of the private lifecycle module, and
the test-only lifecycle owner id re-export was added for sibling module tests.
Next Option 4 slice: wire `WorkflowTaskExecutionOwner` to enqueue and await a
worker-owned completion path for one execution branch without moving runtime
dispatch policy into request code.

2026-06-06 task-execution worker service-ownership sequencing update:
discovered before the branch enqueue/await slice that `WorkflowService` does
not yet own a task-execution worker instance. Wiring a request branch directly
to an ad hoc worker would make the worker request-scoped and violate the
lifecycle ownership standard. Insert a narrow service-owned worker accessor
slice before branch migration: add the worker handle to `WorkflowService`,
start it lazily from the backend lifecycle owner, and provide explicit
shutdown. After that slice is verified, continue with the planned single
branch enqueue/await migration.

2026-06-06 task-execution worker service-ownership slice: completed the
inserted ownership slice by adding an optional task-execution worker handle to
`WorkflowService`, lazy backend-lifecycle startup through the scheduler
lifecycle owner, and explicit idempotent worker shutdown. Task execution
behavior remains unchanged: requests still do not enqueue task attempts,
the worker does not complete attempts, and no fallback or legacy execution
path was added. Verification: `cargo fmt -p pantograph-workflow-service` and
`cargo test -p pantograph-workflow-service task_execution_worker --lib`.
Next Option 4 slice: migrate one request execution branch to enqueue and await
worker-owned completion using the service-owned worker.

2026-06-06 task-execution worker branch-migration re-plan trigger: stop
before migrating a request execution branch. The service-owned worker shell is
valid, but a real worker-owned completion path cannot be created by adding a
oneshot around the current direct request call; that would be the fake
completion-channel shim the plan forbids. The worker loop also cannot simply
own `Arc<WorkflowService>` while `WorkflowService` owns the worker without
introducing a self-referential ownership cycle. The next plan decision must
choose the worker execution context shape before source work continues:
extract a small non-self-referential backend execution context that the worker
owns, move the worker owner outside `WorkflowService` into the composition
root, or narrow the first migration to a worker-owned command whose required
dependencies are explicitly passed without capturing request-owned policy.
Do not migrate a branch until this is decided.

2026-06-06 task-execution worker ownership decision: use Option 2. Move
task-execution worker ownership out of `WorkflowService` and into a
composition-root backend runtime owner that can hold the workflow service,
task-execution worker, runtime host/registry, resource ledger, and lifecycle
shutdown ordering without self-reference. The composition root must keep
runtime-host and runtime-registry instances long-lived and shared across
workflow runs so model/runtime reuse remains possible; do not create a new
inference runtime per workflow run. Next source sequence: introduce the
composition-root owner as an internal backend lifecycle object, move the
service-owned worker handle into that owner, expose typed worker
unavailable/shutdown diagnostics to `WorkflowService`, then migrate one
request branch to enqueue and await worker-owned completion through that
owner. Do not proceed by wrapping direct request execution in a oneshot,
passing request policy through worker commands, or adding compatibility
fallbacks.

2026-06-06 composition-root task-execution owner slice: completed the first
Option 2 source slice by adding an internal `task_execution_runtime` owner
that holds `Arc<WorkflowService>` plus the task-execution worker lifecycle
without making `WorkflowService` self-referential. The task-execution worker
handle was removed from `WorkflowService`, so worker lifecycle ownership now
lives in the composition-root owner. Runtime dispatch behavior and request
execution behavior remain unchanged; no task branch has been migrated yet.
Verification: `cargo fmt -p pantograph-workflow-service`,
`cargo test -p pantograph-workflow-service task_execution_worker --lib`, and
`cargo test -p pantograph-workflow-service runtime_owner_holds_service_and_worker_without_service_self_reference --lib`.
Verification deviation: the broad filter
`cargo test -p pantograph-workflow-service task_execution_ --lib` also ran
unrelated task-classification tests and exposed an existing
`classifier_rejects_excluded_and_unknown_nodes` panic on missing
`expand-settings` contract facts; that is outside this slice and is recorded
as a follow-up. Next Option 2 slice: expose typed worker
unavailable/shutdown diagnostics from the composition-root owner to
`WorkflowService` without public lifecycle snapshots or diagnostics-ledger
worker events.

2026-06-06 task-execution runtime owner diagnostic slice: completed the next
Option 2 source slice by replacing the runtime owner's optional worker handle
with explicit `NotStarted`, `Running`, and `Shutdown` lifecycle states and by
adding typed enqueue diagnostics for not-started and shut-down workers. The
owner now returns `WorkerUnavailable` outcomes with `WorkerUnavailable` or
`ShutdownRequested` diagnostic codes instead of forcing future
`WorkflowService` request-facade code to infer lifecycle state from a missing
handle. Runtime dispatch behavior and request execution behavior remain
unchanged; no task branch has been migrated yet. Verification:
`cargo fmt -p pantograph-workflow-service` and
`cargo test -p pantograph-workflow-service runtime_owner_ --lib`. Next Option
2 slice: migrate one request branch to enqueue through the composition-root
owner and await real worker-owned completion without a direct-execution
oneshot, request-scoped worker, public lifecycle snapshot, diagnostics-ledger
worker event, or compatibility DTO.

2026-06-05 active execution lane re-plan decision: use a short
documentation-only reconciliation slice, then continue Milestone 5b legacy
runtime deletion/replacement. The minimal production image inference path has
already been proven through backend scheduler task state, the runtime-host
port, embedded image execution, artifact persistence, scheduler task-result
mapping, and path-free output projection. Remaining Milestone 5c lifecycle
hardening is still required, but it is not a reason to preserve successful
node-engine/planned-inference/model-path launch paths. The next source slice
for Milestone 5b, after the worker-system alignment slice sequence above,
must choose one classified legacy surface, prove its current owner and
callers, and delete or replace it with scheduler task state/results,
runtime-host responses, or typed fail-closed diagnostics. Do not start
Milestone 6 real PyTorch/diffusers execution until the remaining successful
legacy runtime paths are deleted or fail closed.

2026-06-05 scheduler task-state active-attempt read-model update: the
workflow-service active-run task-state query now exposes scheduler-owned
active attempt id and attempt started-at time from active-run store facts in
schema version 2 read models. The slice did not infer lifecycle state from
frontend, Tauri, runtime adapters, Pumas, graph paths, or diagnostics text.
Remaining Milestone 5c hardening is historical attempt counters/timing
summaries, retry/defer decisions, replay outcomes, worker lifecycle
diagnostics, and terminal cooperative runtime/worker cancellation observation.

2026-06-05 scheduler active cancellation response update: the
workflow-service active runtime task cancellation API now returns a typed
`cancellation_requested` response with scheduler-owned task id, active attempt
id, and message text clarifying that terminal cancellation is observed later
by the runtime task supervisor. This keeps cancellation business state in the
backend and does not add Tauri/frontend inference, runtime adapter policy,
graph-path fallback, or compatibility launch behavior.

2026-06-05 worker lifecycle diagnostics re-plan decision: use Option 3, the
full scheduler lifecycle owner snapshot path, instead of a narrow
task-supervisor-only snapshot. The next lifecycle work must first introduce or
wire real workflow-service lifecycle ownership for every component required by
`SchedulerLifecycleOwnerSnapshot`: queue worker, dependency-readiness action,
resource observation loop, runtime-host dispatch, retry loop, and reservation
cleanup. Components may report `not_started` only from an explicit lifecycle
owner record; do not fabricate component facts in a projection. Public
snapshot/query and diagnostics-ledger worker lifecycle events come after those
component states have real owners. This keeps lifecycle ownership,
diagnostics persistence, retry/defer policy, replay recovery, transport, and
UI display decomposed per the coding standards' simplicity/complection rule.

2026-06-05 scheduler lifecycle registry update: workflow-service now has the
first typed scheduler lifecycle component registry slice. It owns the required
component vocabulary for queue worker, dependency-readiness action, resource
observation loop, runtime-host dispatch, retry loop, and reservation cleanup,
with each component starting from an explicit `NotStarted` owner record. This
does not expose public worker snapshots, emit diagnostics-ledger lifecycle
events, infer missing component facts, or re-enable legacy runtime launch
paths. The next lifecycle slice must attach real runtime-host dispatch/task
supervisor state to this registry before any public snapshot/query work.

2026-06-05 runtime-host dispatch lifecycle attachment update: the task
lifecycle manager now updates the scheduler lifecycle registry's
`runtime_host_dispatch` component from real task-supervisor ownership. Plain
task handles leave the component at explicit `NotStarted`; tracking a runtime
task supervisor abort handle moves it to `Running`; completion returns it to
`NotStarted` while the owner is running; lifecycle shutdown moves it through
`ShuttingDown` to `Shutdown`. This remains backend-owned workflow-service
state only and does not add public snapshots, diagnostics-ledger events,
projection-inferred component facts, Tauri/frontend policy, or legacy runtime
launch behavior.

2026-06-05 shared scheduler lifecycle registry handle update: before attaching
dependency-readiness, resource, retry, queue, or reservation-cleanup component
states, the lifecycle registry now has a cloneable workflow-service owner
handle. The task lifecycle manager accepts that shared handle and updates the
same registry through it, so future component owners do not need to depend on
the task lifecycle manager as an unrelated state container. This resolves the
component-ownership complection risk discovered after the runtime-host
dispatch attachment slice without adding public snapshots, diagnostics-ledger
events, inferred component state, or fallback runtime behavior.

2026-06-05 dependency-readiness lifecycle attachment update: the
workflow-service dependency-readiness lifecycle now updates the shared
scheduler lifecycle registry's `dependency_readiness_action` component from
real provider activity. Requirements-seed and readiness-proof provider calls
mark the component `Running` only while the provider boundary is executing,
then return it to explicit `NotStarted`. This does not add public lifecycle
queries, diagnostics-ledger worker events, projection-inferred component
state, Tauri/frontend policy, or legacy runtime launch behavior.

2026-06-05 retry-loop lifecycle attachment update: the workflow-service
session scheduler runner now wraps its existing deferred runtime
dependency-readiness retry sweep in a scheduler-owned retry lifecycle helper.
The shared registry's `retry_loop` component is `Running` only while the retry
sweep re-enters deferred or retryable runtime dependency-readiness tasks, then
returns to explicit `NotStarted`. This does not add retry scheduling policy,
replay/bootstrap behavior, diagnostics-ledger worker events, public lifecycle
queries, Tauri/frontend policy, or legacy runtime launch behavior.

2026-06-05 reservation-cleanup lifecycle attachment update: the scheduler task
orchestrator now updates the shared scheduler lifecycle registry's
`reservation_cleanup` component from real terminal reservation-release cleanup
paths. Runtime-host completion, runtime-host failure/rejection, and
workflow-cancel release events mark cleanup `Running` only while the
reservation lifecycle port applies the cleanup event, then return it to
explicit `NotStarted`. Dispatch-started and candidate-selection lifecycle
events remain outside cleanup state. This does not add reservation policy,
public lifecycle queries, diagnostics-ledger worker events,
projection-inferred state, Tauri/frontend policy, or legacy runtime launch
behavior.

2026-06-05 verification gate note: broad `npm run lint:full` currently fails
on an unrelated existing `svelte/prefer-writable-derived` issue in
`src/components/nodes/workflow/PumaLibNode.svelte`. Until that is fixed in a
separate cleanup slice, frontend slices must run targeted ESLint for touched
files plus `npm run typecheck`/`npm run build` as relevant and record the
broad-lint deviation. The unrelated proposal document changes currently in the
worktree are not part of this plan slice.

2026-06-03 Tauri edit-session launcher deletion update: after the minimal
scheduler/runtime-host image inference path was proven, the unregistered
`run_workflow_execution_session` Tauri wrapper and desktop-local
`workflow_execution_runtime.rs` graph-snapshot launcher were deleted. The
app-facing TypeScript `runSession` boundary remains fail-closed and is covered
by a regression test that proves it throws before invoking a Tauri command.
Scheduler execution-session commands remain the only GUI workflow submission
path. Verification passed with Tauri fmt/check, frontend command tests,
typecheck, and deleted-symbol search; unrelated diagnostics/Pumas dead-code
warnings remain future cleanup candidates.

2026-06-03 Tauri event-adapter execution-graph hook deletion update: removed
the now-unused `TauriEventAdapter::with_execution_graph` builder and optional
adapter graph state left behind by the deleted edit-session launcher. This
does not add a graph snapshot fallback or scheduler adapter; graph snapshots
and diagnostics projection stay backend-owned. Verification passed with Tauri
fmt/check, event-adapter tests, and deleted-symbol search. The deletion exposes
`WorkflowDiagnosticsStore::set_execution_graph` as a separate dead diagnostics
helper cleanup candidate.

2026-06-03 Tauri diagnostics graph setter cleanup update:
`WorkflowDiagnosticsStore::set_execution_graph` is now test-only after the
production adapter caller was deleted. This removes a dead production graph
snapshot attachment API without adding any graph fallback or runtime launch
path, while preserving diagnostics graph-context projection coverage in tests.
Verification passed with Tauri fmt/check, diagnostics tests, event-adapter
tests, and diff hygiene.

2026-06-03 Tauri Pumas selector helper cleanup update: unused extension-based
Pumas selector helper wrappers in `puma_lib_commands.rs` are now test-only.
Production commands continue through backend-owned selector access and
access-based Pumas APIs; no owner-API fallback, path lookup, or runtime launch
branch was added. Verification passed with Tauri fmt/check, Pumas command
tests, and diff hygiene.

2026-06-03 Tauri runtime/scheduler snapshot event contract re-plan decision:
use Option 2, deleting the retired Tauri `WorkflowEvent::RuntimeSnapshot` and
`WorkflowEvent::SchedulerSnapshot` contract surface instead of quarantining
one helper or promoting a new backend event stream. The next implementation
slice must first inventory active consumers, then remove the event variants,
input DTOs, constructors, serialization/projection branches, and direct
`record_workflow_event` production helper only if they are proven unused by
active transport. Runtime and scheduler diagnostics snapshots must continue
through backend-owned diagnostics/headless snapshot APIs and store update/
record helpers. Tests that currently construct snapshot events should migrate
to the active diagnostics snapshot APIs. No Tauri business logic, graph
snapshot fallback, scheduler adapter, runtime launch branch, or compatibility
shim may be introduced. Option 1 is rejected because it leaves the dead event
contract in production; Option 3 remains deferred unless inspection finds a
live requirement for a backend-owned push snapshot event stream.

2026-06-03 Tauri runtime/scheduler snapshot event deletion update: the retired
Tauri event input DTOs, enum variants, constructors, serializer branches,
event-adapter ownership branches, diagnostics overlay event branches, and
trace projection branches were deleted. Diagnostics snapshot recording remains
backend-owned through diagnostics-store/headless record and update helpers;
test-only record helpers now write trace facts and snapshot state directly
instead of constructing Tauri events. Verification passed with Tauri
fmt/check, diagnostics tests, event-adapter tests, deleted-symbol search, and
diff hygiene. No graph snapshot fallback, scheduler adapter, runtime launch
branch, frontend-derived policy, or compatibility event shim was added.

2026-06-03 scheduler lifecycle hardening re-plan decision: use Option 2, the
attempt/lease state core path, before broader durable lifecycle work.
Workflow-service active-run orchestration must add scheduler-owned
attempt/lease transitions for claim/start/complete/fail/cancel, stale-attempt
rejection, duplicate-dispatch prevention, and reservation release hooks while
keeping runtime-host awaits outside store locks. This is the next thin
implementation path because it creates a single lifecycle/state owner without
folding retry policy, replay/recovery, worker supervision, and timing ledger
integration into one slice. Option 1, minimal in-memory guardrails, is
rejected as insufficient for the open lifecycle boundary. Option 3, a full
lifecycle supervisor, remains the target architecture after the attempt/lease
core exists. Option 4, moving contracts into `pantograph-scheduler`, is
deferred unless shared ownership is proven.

2026-06-03 scheduler attempt identity source update: workflow-service
active-run task execution now creates a scheduler-owned attempt id when a
ready runtime or non-runtime task starts. Completion and failure paths must
present the matching attempt id before mutating task state or task results;
duplicate starts and stale completions fail closed without recording results.
This keeps async task execution outside the store mutation boundary and adds no
graph path fallback, node-engine runtime launch, Tauri/frontend policy, or
compatibility shim. Remaining lifecycle work: explicit cancel attempt
transition, reservation-release intent wiring, retry/defer idempotency,
replay/recovery, worker supervision, and timing/attempt ledger facts.

2026-06-03 scheduler runtime dispatch error update: started runtime task
dispatch errors now terminally fail the matching active attempt instead of
using the retired broad active-run `runtime-dispatch-not-wired` helper. The
dead helper, transition builder, and direct test were removed. Runtime dispatch
selection no-candidate and runtime-host dispatch error paths both return to
attempt-aware terminal mutation without recording task results or adding any
legacy runtime launch branch.

2026-06-03 scheduler cancellation/reservation re-plan decision: use Option 2.
The next source slice must bind scheduler reservation metadata to the active
runtime task attempt after dispatch selection, add an explicit cancel
transition for the matching active attempt, and have synchronous store
transitions return typed reservation-release intent when a leased attempt ends
through cancel/failure/completion. The async orchestrator/session runner must
apply that intent through the reservation lifecycle port outside the store
lock. This keeps workflow-service as the single lifecycle/state owner without
moving business logic into Tauri/frontend or folding retry policy,
replay/recovery, worker supervision, and ledger timing facts into the same
slice. Cancellation may map to the existing terminal task-state diagnostic
until `pantograph-scheduler` grows a distinct cancelled state; that future
state expansion remains deferred unless shared scheduler semantics require it.

2026-06-03 scheduler reservation binding/cancel store update:
workflow-service active-run task attempts now carry typed reservation lease
metadata, terminal completion/failure/cancel store mutations return typed
reservation-release intent for leased attempts, and cancel uses the same
matching-attempt validation as completion/failure. Stale or duplicate
reservation binding fails closed. This does not apply release events yet; the
next slice must wire the async orchestrator/session runner to call the
reservation lifecycle port after the store mutation releases its lock. No
graph path fallback, reduced-plan launch, node-engine runtime launch,
Tauri/frontend policy, compatibility shim, or separate scheduler cancelled
state was added.

2026-06-03 scheduler reservation lifecycle application update: runtime
dispatch now uses an explicit selected-dispatch sequence: start the task
attempt, select dispatch, bind the selected reservation to that attempt,
dispatch through the runtime-host port, terminally mutate task state/results,
then apply reservation lifecycle release events outside the store lock.
Runtime-host success/failure release events now consume the terminal store
mutation's typed release intent instead of being emitted before the store
transition. Missing reservation lifecycle configuration still fails closed
before runtime-host dispatch. Remaining lifecycle work is retry/defer
idempotency, replay/recovery, durable duplicate-dispatch prevention, worker
supervision/cancellation tokens, and diagnostics-ledger attempt/timing facts.

2026-06-04 scheduler durable lifecycle supervisor re-plan decision: use Option
3 next. The attempt/lease core is now validated, so the next lifecycle work
must introduce a single workflow-service lifecycle supervisor through thin
slices: lifecycle manager skeleton, durable duplicate-dispatch/task lease
guardrails, cancellation/shutdown token ownership, retry/defer policy,
replay/bootstrap recovery, and diagnostics-ledger attempt/timing facts. These
must remain backend-owned and decomposed; do not move business logic into
Tauri/frontend, do not reintroduce graph-path or reduced-plan launch fallback,
and do not combine retry, replay, worker lifecycle, and ledger facts into one
source slice.

2026-06-04 scheduler lifecycle manager skeleton update: workflow-service now
has a synchronous task lifecycle owner skeleton for active task handles,
shutdown state, and typed lifecycle diagnostics. Focused tests cover manager
construction, handle tracking, duplicate handle rejection, stale completion
rejection, matching completion, shutdown idempotency, and active-handle
shutdown blocking. This slice intentionally does not add retry/defer policy,
replay/bootstrap, diagnostics-ledger writes, runtime-host dispatch changes,
Tauri/frontend lifecycle logic, or graph-path/reduced-plan fallback. The next
slice is durable duplicate-dispatch/task lease guardrails and runner
integration through the lifecycle owner.

2026-06-04 scheduler lifecycle duplicate-dispatch guardrail update:
`WorkflowSchedulerTaskOrchestrator` now owns a shared task lifecycle manager.
Runtime and non-runtime task starts generate the scheduler attempt id before
the running transition, claim the lifecycle handle, and roll the claim back if
the store transition fails. Matching terminal completion/failure releases the
handle only after the store mutation succeeds. This extends the attempt/lease
core with one lifecycle owner for overlapping runner calls without adding
retry/defer policy, replay/bootstrap, diagnostics-ledger writes,
runtime-host API changes, graph-path fallback, reduced-plan launch, or
Tauri/frontend lifecycle behavior. The next slice is cancellation/shutdown
token ownership through the same lifecycle owner.

2026-06-04 scheduler cancellation/shutdown contract re-plan decision: use the
cancellable runtime-host execution contract path next. The current
runtime-host execution port accepts only an execution request and cannot
observe workflow-service cancellation or shutdown, so task cancellation cannot
be standards-compliant if it is hidden in Tauri/frontend, runtime adapters, or
best-effort future dropping. The next thin slice must add backend-owned,
typed cancellation/shutdown propagation at the runtime-host contract boundary
while workflow-service remains the lifecycle/business owner. After that
contract exists, implement the full workflow-service task supervisor with
tracked handles, child cancellation tokens, shutdown draining, timeout/abort
behavior, and panic observation. A workflow-service-only gate may reject new
work but is insufficient for in-flight runtime tasks; adapter-owned
cancellation is rejected because it splits lifecycle/business ownership.

2026-06-04 scheduler runtime-host cancellation contract foundation update:
runtime-host execution contract v2 now requires a serialized workflow-service
cancellation context on every execution request and passes a live cancellation
handle beside the request through the runtime-host execution port. The
existing dispatcher creates a running workflow-service-owned context, while a
new explicit dispatch-with-cancellation entrypoint gives the upcoming
workflow-service supervisor a typed boundary for cooperative cancellation and
shutdown. Concrete workflow-service and embedded-runtime port implementations
compile against the new contract; adapters do not own lifecycle policy and no
fallback runtime launch, graph path, Tauri/frontend cancellation branch, or
compatibility shim was added. Remaining lifecycle work: wire a real
workflow-service supervisor signal into the handle, make adapters observe
cancellation/shutdown, add await/abort/panic handling, then continue with
retry/defer, replay/bootstrap, and diagnostics-ledger attempt/timing facts.

2026-06-04 scheduler lifecycle-owned runtime cancellation signal update:
workflow-service task lifecycle now owns the live runtime-host cancellation
signal for started runtime task attempts. The lifecycle manager validates the
matching active attempt before creating a runtime-host cancellation handle,
updates that handle for task cancellation and lifecycle-owner shutdown, and
the production session runner dispatches started runtime tasks through
`dispatch_with_cancellation`. This keeps cancellation state backend-owned and
does not add adapter, Tauri, frontend, graph-path, reduced-plan, retry,
replay, or diagnostics-ledger behavior. Remaining lifecycle work: runtime
adapters must observe cancellation/shutdown, and the workflow-service
supervisor still needs await/abort behavior, panic observation, retry/defer
idempotency, replay/bootstrap, and diagnostics-ledger attempt/timing facts.

2026-06-04 workflow-service active runtime cancellation API update:
workflow-service now exposes a backend-owned active runtime task cancellation
request/response contract and service method. The method validates the active
session/run/task, requires the task to be a running runtime inference task,
resolves the current active scheduler attempt id from the store, releases the
store lock, then records cancellation intent through the lifecycle owner. This
does not terminally mutate scheduler task state, release reservations, create
Tauri/frontend policy, or add graph-path/reduced-plan/runtime-launch fallback.
Focused verification passed with `cargo fmt -p pantograph-workflow-service --
--check`, `cargo test -p pantograph-workflow-service session_queue --lib`,
`cargo test -p pantograph-workflow-service task_lifecycle --lib`, `cargo test
-p pantograph-workflow-service task_orchestrator --lib`, and prior
`cargo check -p pantograph-workflow-service`. Remaining lifecycle work:
runtime adapters must observe the cancellation/shutdown signal, active runtime
supervision still needs await/abort/panic handling, and retry/defer,
replay/bootstrap, and diagnostics-ledger attempt/timing facts remain future
thin slices.

2026-06-04 workflow-service runtime dispatch supervisor shell update:
runtime task dispatch in the workflow-service session runner now executes
under a small supervised async join boundary. Runtime-host dispatch still uses
the existing orchestrator, lifecycle cancellation signal, terminal store
mutation, and reservation-release paths, but panic/cancelled join failures are
converted into typed orchestrator diagnostics instead of unwinding through the
session runner. This adds no retry policy, shutdown drain/abort behavior,
Tauri/frontend policy, graph-path/reduced-plan launch fallback, or adapter
lifecycle ownership. Focused verification passed with `cargo fmt -p
pantograph-workflow-service -- --check`, targeted runtime success/failure/
panic session-execution tests, `cargo test -p pantograph-workflow-service
task_orchestrator --lib`, `cargo test -p pantograph-workflow-service
task_lifecycle --lib`, `cargo check -p pantograph-workflow-service`, and
targeted no-fallback search. A broader exploratory `workflow_execution_session_records_`
filter also exposed unrelated retained-artifact/runtime-proof fixture failures
outside this slice; those are recorded as discovered issues, not fixed here.
Remaining lifecycle work: shutdown drain/abort, deeper image gateway/provider
cooperative cancellation, retry/defer idempotency, replay/bootstrap, and
diagnostics-ledger attempt/timing facts.

2026-06-04 workflow-service runtime shutdown drain/abort update:
runtime dispatch supervisor spawning is now owned by the workflow-service
orchestrator instead of the session runner, and each supervised runtime task
registers an abort handle with the scheduler task lifecycle owner. A new
backend-owned lifecycle shutdown method requests cooperative shutdown, waits
for a bounded drain, aborts still-active runtime dispatch supervisors, then
waits for the existing session-runner terminal mutation and reservation
release path to clear handles. This adds no Tauri/frontend policy, adapter
lifecycle ownership, retry/replay behavior, graph-path/reduced-plan launch
fallback, or compatibility shim. Focused verification passed for lifecycle
abort handles, full-path blocked runtime dispatch shutdown abort, runtime
success/failure/panic regressions, orchestrator/lifecycle suites, fmt/check,
and diff hygiene. Remaining lifecycle work: pass cooperative cancellation
deeper into long-running image gateway/provider calls, then retry/defer
idempotency, replay/bootstrap, and diagnostics-ledger attempt/timing facts.

2026-06-04 inference gateway cooperative cancellation update: the
workflow-service-owned runtime-host cancellation signal now projects through
embedded-runtime into inference-local gateway/backend execution context.
Image generation planning and gateway dispatch reject typed cancellation
before backend execution, and the PyTorch image backend checks the same signal
before entering its blocking Python worker call. This adds no Tauri/frontend
policy, adapter-owned lifecycle logic, graph-path/reduced-plan launch fallback,
runtime launch branch, Pumas fact change, lockfile edit, or saved workflow
fixture rewrite. Remaining lifecycle work: Python worker-contract support for
mid-call cooperative cancellation, then retry/defer idempotency,
replay/bootstrap, and diagnostics-ledger attempt/timing facts.

2026-06-04 scheduler dependency-readiness retry idempotency update:
workflow-service retry of deferred runtime dependency readiness is now
idempotent once the task has already re-entered `WaitingDependencyReadiness`.
Repeated retry calls return the current scheduler task-state record unchanged
instead of producing an invalid-request error, while other stale lifecycle and
terminal mutations remain strict. This adds no retry policy engine, replay
bootstrap, diagnostics-ledger writes, adapter/Tauri/frontend policy, graph-path
or reduced-plan launch fallback, Pumas fact change, lockfile edit, generated
file, or workflow fixture rewrite. Remaining lifecycle work: replay/bootstrap
recovery and diagnostics-ledger attempt/timing facts.

2026-06-04 scheduler attempt-start diagnostics update: workflow-service now
emits the ledger-owned `scheduler.task_attempt_lifecycle_changed` `Started`
event after runtime and non-runtime scheduler attempts are claimed. Scheduler
state remains the source of attempt identity and start time, while the session
runner records the diagnostic event through the existing workflow-service
append helper. This adds no terminal/cancel/redispatch policy, projection
schema/read-model change, graph-path or reduced-plan launch fallback,
node-engine runtime launch branch, Tauri/frontend behavior, runtime-host
adapter policy, Pumas fact change, lockfile edit, generated file, or workflow
fixture rewrite. Remaining lifecycle work: emit terminal/cancel/redispatch
attempt events with runtime/reservation/error facts after ordering is defined,
then add projection/read-model fields after emitted event ordering is proven.

2026-06-04 scheduler attempt lifecycle diagnostics re-plan decision: use the
decomposed producer path. The next immediate source slice is terminal
`Completed`/`Failed` attempt events from existing successful and failing
runtime and non-runtime terminal paths. Later slices must add `Cancelled`
events, then `Redispatched` events and recovery ordering, and only then add
projection/read-model fields after emitted event ordering is proven. This
keeps scheduler lifecycle state, diagnostics emission, recovery ordering, and
read-model schema changes as separate reasoning axes under the
simplicity/complection standards. Do not combine cancellation, redispatch,
replay, and projection schema work into the next slice, and do not introduce
graph-path, reduced-plan, node-engine runtime launch, Tauri/frontend policy,
runtime-host adapter policy, or diagnostics fallback behavior.

This plan is split into focused documents so each section stays readable while
still preserving the planning standards' required traceability, verification,
risk, lifecycle, and execution-management content.

## Plan Sections

1. [Objective And Scope](00-objective-scope.md)
   - Objective
   - In scope
   - Out of scope

2. [Inputs And Contracts](01-inputs-contracts.md)
   - Problem statement
   - Current codebase findings
   - Constraints
   - Assumptions
   - Dependencies
   - Affected contracts

3. [Image Generation Family Planner Design](02-image-generation-family-planner.md)
   - Planner contracts
   - Minimum Pumas package facts
   - Family requirements
   - Reference-repo findings
   - Concurrency and lifecycle review
   - Standards guardrails and compliance matrix
   - Affected persisted artifacts

4. [Device And Runtime Variant Selection](06-device-runtime-selection.md)
   - Device policy objective
   - Backend support notes
   - Backend adapter and scheduler boundary
   - Transformers-compatible canonical semantics
   - Canonical device/runtime contracts
   - llama.cpp runtime variant design
   - PyTorch/Transformers, vLLM, Candle, and MLX device implications
   - Scheduler-facing candidate and selected-decision facts

5. [Pumas Library Image Generation Facts](07-pumas-library-image-generation-facts.md)
   - Pumas/Pantograph ownership boundary
   - Required diffusers bundle facts
   - Required GGUF metadata facts
   - [Pumas package artifact size facts handoff](pumas-package-artifact-size-facts-handoff.md)
   - Snapshot/cache/update-feed behavior
   - Early P0-P1 producer-contract start after Pantograph Milestone 0
   - P2-P5 producer-fact completion gate before Pantograph Milestone 5a/5c/5b/6
   - Cross-repo fixtures and verification

6. [Scheduler-Owned Dynamic Task Dispatch](08-scheduler-owned-dynamic-task-dispatch.md)
   - Dynamic task dispatch objective
   - Graph editor and node-engine abstraction boundaries
   - Capability hint contract
   - Schedulable task intent
   - Scheduler queue state
   - Dispatch decision contract
   - Resource/residency and batching requirements
   - Legacy removal targets

7. [Runtime Host Handoff And Legacy Execution Removal](09-runtime-host-handoff-legacy-removal.md)
   - Runtime host handoff objective
   - Pumas load-target boundary
   - PyTorch, llama.cpp, and audio migration
   - Node-engine preflight replacement
   - Legacy resolver/path deletion sequence
   - 2026-06-03: dead node-engine dependency-preflight enforcement deleted;
     remaining node-engine preflight code is limited to retired input
     rejection and path-free projection helpers, with embedded-runtime
     Python-backed preflight tracked separately.
   - 2026-06-03: exported package Puma-Lib node now uses path-free
     `pumas_model_ref` option lookup and persists only canonical Pumas
     identity.

8. [Task-Level Scheduler Orchestration](10-task-level-scheduler-orchestration.md)
   - Option 4 target architecture
   - Current whole-run execution gap
   - Task graph, task state, and result materialization boundaries
   - Scheduler, node-engine, runtime-host, frontend, and ledger effects
   - Staged implementation and verification plan

9. [Inference Interface Resolution And Validation](11-inference-interface-resolution-and-validation.md)
   - Generic inference-node model interface resolution
   - Shared descriptor contract for port discovery and validation
   - Production inference facts provider and conservative resource estimate
     hints from Pumas static facts plus Pantograph runtime/device facts
   - Graph editor draft validation, executable publish/admission validation,
     and execution revalidation
   - Scheduler, node-engine, runtime-host, and Pumas ownership boundaries

10. [Risks And Definition Of Done](03-risks-and-definition-of-done.md)
   - Risk table
   - Definition of done

11. [Milestones](04-milestones.md)
   - Contract gate
   - Current Juggernaut graph slice
   - Retired node producer removal
   - Backend stale graph diagnostics
   - IO inspector stale graph presentation
   - Device and runtime variant selection
   - Scheduler-owned dynamic task dispatch
   - Task-level scheduler orchestration
   - Inference interface resolution and validation
   - Runtime host handoff and legacy execution removal
   - PyTorch/diffusers image generation execution slice
   - Candle guardrail
   - Release build and user validation

12. [Execution Management](05-execution-management.md)
   - Execution notes
   - Commit cadence
   - Optional worker assignment
   - Re-plan triggers
   - Recommendations
   - Completion summary

## Implementation Rule

Use `04-milestones.md` as the execution checklist. Update
`05-execution-management.md` after each validated slice with verification
results, deviations, follow-ups, and any standards concerns discovered during
implementation.

Pumas ordering rule: the Pumas plan is not implemented as the final Pantograph
step. Pumas P0-P1 starts immediately after Pantograph Milestone 0 freezes the
expected contract. Pumas P2-P5 may run in parallel with Pantograph Milestones
1-5, but must complete and be pinned before Pantograph Milestone 5a consumes
production model facts for scheduler dispatch, before Milestone 5c integrates
production task-level orchestration, before Milestone 5d resolves
model-specific inference interfaces, before Milestone 5b resolves runtime-host
load targets, and before Milestone 6 implements real PyTorch/diffusers image
execution.

2026-06-02 resource-estimate handoff update: the next Pantograph production
inference-facts provider slice is gated on the Pumas package artifact size facts
handoff in
`pumas-package-artifact-size-facts-handoff.md`. After Pumas publishes or pins
that contract, Pantograph should update its Pumas dependency and inference
package-facts DTOs, project logical artifact size facts through
embedded-runtime, and implement the conservative Pantograph-owned
model/load plus execution/context estimator. Until then, production dispatch
must continue to fail closed with typed missing-resource-estimate diagnostics
rather than deriving estimates from graph paths, selector summaries, UI state,
or legacy model-reference contracts.

2026-06-02 implementation note: the pushed untagged Pumas package-facts v3
commit is now pinned by SHA in Pantograph, and Pantograph mirrors/projects
logical artifact/file size facts through inference and embedded-runtime package
fact bridges. The remaining production inference-facts work is the
Pantograph-owned conservative estimator/provider and scheduler estimate-hint
population; no loaded-memory or admission policy was added in the DTO/projection
slice.

2026-06-02 estimator primitive update: embedded-runtime now has a pure
Pantograph-owned conservative estimator that transforms validated Pumas logical
size facts into inference resource-estimate DTOs and bounded scheduler
RAM/VRAM estimate hints. It uses checked arithmetic, rejects weak or missing
size evidence with typed unavailable estimates, and does not read graph paths,
selector summaries, UI state, Tauri policy, or runtime-registry state. The next
slice must wire a production `InferenceInterfaceFactsProvider` through backend
composition so resolver facts can receive these hints together with runtime,
device, task-shape, and proven-residency context.

2026-06-02 provider wiring update: resource-backed embedded-runtime workflow
service composition now installs a backend-owned
`InferenceInterfaceFactsProvider` before sharing the service. The provider
combines Pumas package facts, runtime-registry capability facts, and the
conservative logical-size estimator into workflow-service resolver facts. It
does not put business logic in Tauri or the frontend, and missing Pumas,
runtime, task, or estimate facts continue to fail closed through canonical
resolver/admission diagnostics instead of graph-path or selector-summary
fallbacks.

2026-06-03 dependency-readiness active-run lifecycle re-plan update: the
implemented first-run deferral path remains valid: workflow-service enqueues
dependency-readiness work, records typed deferred scheduler state, blocks
runtime dispatch, and returns typed runtime-not-ready diagnostics when no fresh
backend readiness proof exists. The public runtime session lifecycle now keeps
dependency-readiness pending as a non-terminal backend-owned active-run state,
not a finished workflow run. The explicit backend-owned resume API for an
existing active `session_id` plus `workflow_run_id` is now implemented in
workflow-service: it validates active runtime scheduler state, retries
dependency-readiness admission from fresh canonical backend facts, preserves
the active run when facts are still missing, and completes runtime dispatch
without requiring a replacement client-created workflow run. The selected
host-boundary forwarder is also implemented through embedded-runtime, the
Tauri command facade, and the TypeScript workflow command service; it forwards
only `session_id` and `workflow_run_id` and does not own retry, readiness,
resource, package, or scheduler policy. The manual-use re-plan is now
implemented: run-list and run-detail projections expose an optional typed
backend read-model field for dependency-readiness resume eligibility, populated
by workflow-service from live active-run scheduler task state. The Scheduler UI
only displays that backend fact and forwards `session_id` plus
`workflow_run_id` to the existing backend command; it does not infer
resumability from generic `running` status, scheduler reason strings, error
text, graph paths, selector summaries, or Tauri state. The next implementation
step is the deferred event-driven backend readiness lifecycle: a
composition-root-owned embedded-runtime lifecycle handle that coordinates the
existing snapshot producer, workflow-service resume-candidate query, embedded
backend host, and explicit backend resume API. It must track tasks, support
idempotent shutdown, prevent overlapping resumes for the same active run,
apply bounded retry/backoff behavior, and log lifecycle failures at the owner.
Tauri remains a thin composition/handle manager and must not own retry,
readiness, scheduler, package, or resource policy. Missing, stale, or
insufficient facts remain typed fail-closed diagnostics, not graph-path,
Tauri, frontend, selector-summary, synchronous probe, client rerun, or legacy
preflight fallback. Future improvement: replace bounded polling with a typed
snapshot notification/event channel after the lifecycle is working, if
responsiveness or idle overhead requires it.

2026-06-03 auto-resume lifecycle handle update: embedded-runtime now exports
the tracked `DependencyReadinessAutoResume` lifecycle component with focused
tests for shutdown, empty candidate polling, duplicate suppression, successful
resume attempts, pending-readiness preservation, and poll-interval validation.
The follow-up source slice wires a real embedded-runtime resume port through
hosted and standalone composition so production startup returns/manages this
handle beside the snapshot producer while Tauri remains a thin handle manager.

2026-06-03 auto-resume production wiring update: the real embedded-runtime
resume port is wired. It lists workflow-service resume candidates and calls
the existing backend resume API with an embedded workflow host. Standalone
runtime construction owns the handle, hosted Tauri startup creates the handle
through embedded-runtime host construction and only stores/shuts down the
returned handle, and shutdown stops auto-resume before tearing down runtime
resources. The remaining improvement is optional event-first notification
after the bounded polling path is proven under real inference workflow smoke.

2026-06-03 complete-path proof update: embedded-runtime now has session-level
coverage proving a workflow-service scheduler session can dispatch through the
production-composed `EmbeddedRuntimeHostExecutionPort`, resolve Pumas load
targets and package facts, execute image generation through the inference
gateway, persist media through the backend artifact writer, map the completed
runtime-host response into scheduler task results, and return the requested
workflow output as a path-free artifact reference. This completes the minimal
image inference path proof. Remaining work moves to deleting/replacing
retired node-engine/planned-inference launch paths, then continuing
dependency-readiness producer lifecycle and durable scheduler hardening in
separate validated slices.

2026-06-03 retired dependency contract diagnostic cleanup update: active
node-engine and embedded-runtime Rust source/tests no longer mention the
retired concrete contract names `ModelDependencyRequest`,
`ModelDependencyResolver`, `ModelRefV2`, `build_model_ref_v2`, or
`PlannedInferenceExecutionHost`. Fail-closed diagnostics now use generic
retired dependency/model-reference contract language while still blocking
before legacy request, resolver, model-reference, or adapter dispatch behavior.

2026-06-03 dependency activity path-key cleanup update: active dependency-
environment action/activity transport no longer keys user-visible dependency
activity by `modelPath` or `model_path`. The backend-owned diagnostic activity
event carries optional `target_node_id`, Tauri forwards the event without
policy, and the frontend matcher compares that id to the current
dependency-environment node. The activity log remains display/history state
only and cannot produce runtime launch facts, readiness proofs, scheduler
policy, or load authority.

2026-06-03 retired frontend renderer cleanup update: app-local direct runtime
node renderers for PyTorch, llama.cpp, and reranking were deleted, and the
unused shared-package llama.cpp renderer export was removed. Canonical app
workflows render through `LLMInferenceNode.svelte` and backend descriptors
instead of path-era direct runtime node components. The older exported package
`PumaLibNode.svelte` still carries path-shaped mock UI and is explicitly left
as a separate delete-or-rewrite follow-up because it is a package API surface,
not an app-local orphaned renderer.

2026-06-03 backend graph-mutation validation auto-trigger update:
workflow-service semantic graph mutations now cancel stale validation and start
one backend-owned validation task for the committed current graph revision.
Layout-only node-position edits, connection candidate lookup, and insertion
previews remain non-triggering. Validation freshness, task lifecycle, and
typed rejection behavior stay backend-owned; Tauri/frontend overlay consumption
remains a later presentation slice.

2026-06-03 validation projection read-model update: workflow-service now owns a
read-only current validation projection query that returns the stored backend
summary plus node projections for the caller-observed current graph revision
without starting validation. Stale or missing state returns typed summary
diagnostics and no projections. Tauri and frontend service code forward the
query by graph session/revision only, so the next presentation slice can
consume validation lifecycle events without re-running validation or deriving
descriptor overlays locally.

2026-06-03 toolbar lifecycle projection update: `WorkflowToolbar.svelte` now
consumes graph-validation lifecycle events through the backend current
validation projection read model. Lifecycle handling applies backend-provided
node overlays and summary state without calling validation refresh, deriving
descriptor overlays, or computing validation freshness in frontend code.

2026-06-04 embedded runtime-host cancellation observation update:
runtime-host diagnostics now include typed cancellation and shutdown codes,
and the embedded-runtime execution port observes the workflow-service-owned
live cancellation handle before and between its existing async dependency and
gateway steps. Cancellation or shutdown returns typed rejected runtime-host
responses; invalid or mismatched cancellation snapshots fail closed as port
errors. Remaining Milestone 5c lifecycle work is workflow-service await/abort
and panic observation, deeper cooperative cancellation inside long-running
image gateway/provider calls, retry/defer idempotency, replay/bootstrap, and
diagnostics-ledger attempt/timing facts.

2026-06-04 active supervisor path re-plan update: option 3 is the active next
path now that the cancellable runtime-host contract foundation and
embedded-runtime observation are in place. The next implementation slices must
keep lifecycle/business policy in workflow-service: active task cancellation
intent API, supervised runtime task handles with typed join/panic diagnostics,
shutdown drain plus bounded abort, deeper image gateway/provider cooperative
cancellation, then retry/replay/ledger facts. Tauri/frontend and runtime
adapters remain forwarding/observation layers only.

2026-06-04 active cancellation lifecycle-core update: workflow-service
lifecycle handles now retain pending runtime-host cancellation or shutdown
state before the runtime-host cancellation handle exists, and later dispatch
initializes the signal from that pending state. The slice intentionally avoids
early terminal task mutation, early reservation release, adapter-owned policy,
and Tauri/frontend changes. The production active cancellation API is the next
slice so it can be wired to a real backend caller instead of leaving dead
staging code.

2026-06-04 bootstrap recovery classifier update: workflow-service now exposes
an active-run scheduler bootstrap recovery snapshot for canonical runtime task
state and reuses that classifier in the existing dependency-readiness resume
candidate path. The classifier maps runtime task state to typed next actions:
resume progress loop, retry dependency readiness, redispatch ready runtime
task, require runtime recovery, completed, terminal diagnostic, or missing
task-state record. It does not replay work, mutate task state, infer from
legacy graph/backend/runtime paths, write diagnostics-ledger facts, add
Tauri/frontend policy, or change Pumas/package facts. Verification passed:
`cargo fmt -p pantograph-workflow-service`; `cargo test -p
pantograph-workflow-service bootstrap_recovery --lib`; `cargo test -p
pantograph-workflow-service
dependency_readiness_resume_candidates_use_bootstrap_recovery_classifier
--lib`; `cargo test -p pantograph-workflow-service scheduler::store --lib`;
`cargo check -p pantograph-workflow-service`; `cargo fmt -p
pantograph-workflow-service -- --check`; `git diff --check`; and targeted
no-fallback/no-legacy source search. Remaining lifecycle work: consume the
recovery snapshot from a workflow-service-owned bootstrap/replay runner that
reconciles incomplete attempts without duplicate dispatch, then add
diagnostics-ledger attempt/timing facts.

2026-06-04 bootstrap recovery report API update: workflow-service now exposes
a backend-owned `workflow_execution_session_bootstrap_recovery_report` API
that aggregates active-run runtime task recovery classifications into
workflow-level DTOs. The report is read-only and performs no replay, task-state
mutation, runtime-host call, diagnostics-ledger write, Tauri/frontend policy,
or legacy graph/backend/runtime inference. It gives the future bootstrap/replay
runner and forwarding adapters a canonical backend contract for recovery
decisions. Verification passed: `cargo fmt -p pantograph-workflow-service`;
`cargo test -p pantograph-workflow-service
workflow_execution_session_runtime_run_defers_pending_dependency_readiness_before_dispatch
--lib`; `cargo test -p pantograph-workflow-service bootstrap_recovery --lib`;
`cargo check -p pantograph-workflow-service`; `cargo fmt -p
pantograph-workflow-service -- --check`; `git diff --check`; and targeted
no-fallback/no-legacy source search. Remaining lifecycle work: implement the
workflow-service-owned bootstrap/replay runner that consumes this report and
reconciles incomplete attempts without duplicate dispatch, then add
diagnostics-ledger attempt/timing facts.

2026-06-04 bootstrap recovery planning update: workflow-service now exposes a
backend-owned `workflow_execution_session_bootstrap_recovery_plan` API that
turns the canonical recovery report into typed reconciliation decisions and
deduplicated dependency-readiness resume requests. The planner is read-only and
performs no replay, task-state mutation, runtime-host call, reservation
release, diagnostics-ledger write, Tauri/frontend policy, or graph-path/
node-engine/reduced-plan fallback. Ready runtime redispatch is intentionally
blocked with a persisted readiness proof and duplicate-dispatch guard diagnostic until durable dispatch
idempotency state is implemented; unsafe runtime recovery, terminal, and missing
task-state conditions are also typed blocking decisions. Verification passed:
`cargo fmt -p pantograph-workflow-service`; `cargo test -p
pantograph-workflow-service
bootstrap_recovery_plan_blocks_ready_runtime_redispatch_without_recovery_state --lib`;
`cargo test -p pantograph-workflow-service
workflow_execution_session_runtime_run_defers_pending_dependency_readiness_before_dispatch
--lib`; `cargo test -p pantograph-workflow-service bootstrap_recovery --lib`;
`cargo check -p pantograph-workflow-service`; `cargo fmt -p
pantograph-workflow-service -- --check`; `git diff --check`; and targeted
no-fallback/no-legacy source search. Remaining lifecycle work: implement the
workflow-service-owned bootstrap/replay runner that applies safe plan decisions
with durable duplicate-dispatch/idempotency protection, then add
diagnostics-ledger attempt/timing facts after replay ordering is proven.

2026-06-04 bootstrap recovery apply update: workflow-service now exposes
`recover_workflow_execution_session_bootstrap`, the first mutating recovery
runner slice. The runner consumes the backend recovery plan, fails closed when
the plan has blocking decisions, rejects unsupported nonblocking decisions such
as progress-loop replay, and applies only dependency-readiness resume requests
through the existing canonical runtime dependency-readiness resume path. The
slice added a recovery result DTO that returns the applied plan and resumed run
responses. It does not redispatch ready runtime tasks, invent a replay loop,
mutate scheduler state directly, call Tauri/frontend code, change graph/
node-engine/reduced-plan execution, edit Pumas/package facts, rewrite
lockfiles/generated files/workflow fixtures, or write diagnostics-ledger
attempt/timing facts. Verification passed: `cargo fmt -p
pantograph-workflow-service`; `cargo test -p pantograph-workflow-service
bootstrap_recovery_apply_gate_rejects_unimplemented_progress_loop_replay
--lib`; `cargo test -p pantograph-workflow-service
workflow_execution_session_bootstrap_recovery_applies_dependency_readiness_resume_plan
--lib`; `cargo test -p pantograph-workflow-service bootstrap_recovery --lib`;
`cargo check -p pantograph-workflow-service`; `cargo fmt -p
pantograph-workflow-service -- --check`; `git diff --check`; and targeted
no-fallback/no-legacy source search. Remaining lifecycle work: implement the
explicit progress-loop replay runner, durable duplicate-dispatch/idempotency
guard for ready runtime redispatch, and diagnostics-ledger attempt/timing facts.

2026-06-04 bootstrap progress-loop recovery update: the bootstrap recovery
runner now applies `ResumeProgressLoop` decisions by invoking the existing
workflow-service scheduler progress loop for the affected active run, then
recomputes the recovery plan before applying dependency-readiness resumes. The
recomputed gate preserves the no-fallback rule: if progress reaches ready
runtime dispatch, duplicate-dispatch protection still blocks redispatch until
that durable guard is implemented. The recovery result now returns both the
initial and final plans so replay effects remain inspectable. Verification
passed: `cargo fmt -p pantograph-workflow-service`; `cargo test -p
pantograph-workflow-service
bootstrap_recovery_progress_loop_requests_dedupe_by_active_run --lib`; `cargo
test -p pantograph-workflow-service
workflow_execution_session_bootstrap_recovery_applies_progress_loop_before_readiness_resume
--lib`; `cargo test -p pantograph-workflow-service bootstrap_recovery --lib`;
`cargo check -p pantograph-workflow-service`; `cargo fmt -p
pantograph-workflow-service -- --check`; `git diff --check`; and targeted
no-fallback/no-legacy source search. Remaining lifecycle work: implement the
durable duplicate-dispatch/idempotency guard for ready runtime redispatch and
diagnostics-ledger attempt/timing facts.

2026-06-04 runtime redispatch recovery-state diagnostic update: bootstrap
recovery now classifies ready runtime redispatch as
`BlockedRuntimeRedispatchRecoveryStateRequired` instead of implying that
duplicate-dispatch protection alone is sufficient. The diagnostic explicitly
requires persisted readiness proof plus duplicate-dispatch/idempotency state
before ready runtime tasks can be recovered through redispatch. This records
the discovered implementation constraint from the canonical dispatch path:
ready dispatch currently needs an admitted readiness proof that is held in the
runner flow, not durably persisted in the ready task record. Verification
passed: `cargo fmt -p pantograph-workflow-service`; `cargo test -p
pantograph-workflow-service
bootstrap_recovery_plan_blocks_ready_runtime_redispatch_without_recovery_state
--lib`; `cargo test -p pantograph-workflow-service bootstrap_recovery --lib`;
`cargo check -p pantograph-workflow-service`; `cargo fmt -p
pantograph-workflow-service -- --check`; `git diff --check`; and targeted
no-fallback/no-legacy source search. Remaining lifecycle work: persist the
ready runtime dispatch recovery state needed for bootstrap redispatch, then add
diagnostics-ledger attempt/timing facts.

2026-06-04 ready runtime dispatch recovery proof persistence update:
workflow-service active-run state now persists the admitted dependency
readiness proof when a runtime task reaches `Ready`, and bootstrap recovery
report/plan DTOs expose whether that persisted redispatch recovery input is
available. Ready runtime redispatch remains blocked until durable
duplicate-dispatch/idempotency guard state is implemented; diagnostics now
distinguish missing proof from proof-present-but-guard-missing. Verification
passed: `cargo fmt -p pantograph-workflow-service`; `cargo test -p
pantograph-workflow-service orchestrator_persists_started_runtime_task_result
--lib`; `cargo test -p pantograph-workflow-service
active_run_bootstrap_recovery_snapshot_classifies_runtime_task_states --lib`;
`cargo test -p pantograph-workflow-service
bootstrap_recovery_plan_reports_persisted_proof_when_guard_state_missing
--lib`; `cargo test -p pantograph-workflow-service bootstrap_recovery --lib`;
`cargo check -p pantograph-workflow-service`; `cargo fmt -p
pantograph-workflow-service -- --check`; `git diff --check`; and targeted
no-fallback/no-legacy source search. Remaining lifecycle work: implement the
durable duplicate-dispatch/idempotency guard for ready runtime redispatch,
then add diagnostics-ledger attempt/timing facts.

2026-06-04 ready runtime redispatch recovery update: bootstrap recovery now
treats proof-present ready runtime tasks as actionable
`RedispatchReadyRuntime` decisions and invokes the existing active-run resume
shell, which dispatches through persisted readiness proof and the scheduler
start-attempt guard. Proof-missing ready tasks remain blocked with typed
diagnostics. The slice also fixed a discovered progress-loop bug where ready
runtime tasks were incorrectly offered to the non-runtime node-engine start
path before runtime dispatch. Verification passed: `cargo fmt -p
pantograph-workflow-service`; `cargo test -p pantograph-workflow-service
bootstrap_recovery_plan_accepts_ready_redispatch_with_recovery_state --lib`;
`cargo test -p pantograph-workflow-service
workflow_execution_session_bootstrap_recovery_redispatches_ready_runtime_task
--lib`; and `cargo test -p pantograph-workflow-service bootstrap_recovery
--lib`; `cargo check -p pantograph-workflow-service`; `cargo fmt -p
pantograph-workflow-service -- --check`; `git diff --check`; and targeted
no-fallback/no-legacy source search. Remaining lifecycle work:
diagnostics-ledger attempt/timing facts.

2026-06-04 diagnostics-ledger attempt/timing re-plan decision: implement
Option 1 next. The next source slice must first add a diagnostics-ledger owned
scheduler task-attempt event contract and validation tests before workflow
service emits any events. The contract must represent scheduler task id,
attempt id, execution class, lifecycle transition, timing fields, and optional
runtime/reservation facts without reusing `WorkflowTimingObservation`, because
that existing type is run/node timing history and does not own scheduler
attempt lifecycle identity. 2026-06-04 follow-up re-plan adjustment: because
`DiagnosticEventPayload` is a shared public enum, this same contract slice may
also update the existing `pantograph-workflow-service` projection-refresh
classification match so direct consumers continue compiling. That adaptation
must be limited to classifying the new payload for existing diagnostics
projection refresh behavior; it must not emit scheduler attempt events, change
scheduler lifecycle policy, add projection/read-model fields, or write
frontend/Tauri/runtime/Pumas code. Workflow-service event emission and
query/read-model behavior remain follow-up slices after the ledger contract
and consumer compile adaptation are frozen. This keeps DTO/schema ownership in
`pantograph-diagnostics-ledger`, scheduler lifecycle ownership in
`pantograph-workflow-service`, and avoids combining contract design, producer
wiring, and projection behavior in one slice.

2026-06-04 diagnostics-ledger task-attempt contract source update:
`pantograph-diagnostics-ledger` now owns a typed
`scheduler.task_attempt_lifecycle_changed` event contract with scheduler task
id, scheduler attempt id, execution class, lifecycle transition,
started/ended/duration timing fields, optional runtime/reservation facts, and
typed validation for terminal timing. The slice exports the contract, persists
it through the existing append-only diagnostic event ledger, includes it in the
existing scheduler/run projection-refresh classification, and documents that
generic `WorkflowTimingObservation` remains run/node timing history rather
than scheduler attempt identity. No workflow-service scheduler producer emits
the event yet. Verification passed: `cargo fmt -p
pantograph-diagnostics-ledger -p pantograph-workflow-service`; `cargo test -p
pantograph-diagnostics-ledger scheduler_task_attempt --lib`; `cargo test -p
pantograph-workflow-service
workflow_scheduler_task_attempt_event_requests_scheduler_projection_refresh
--lib`; `cargo check -p pantograph-diagnostics-ledger`; `cargo check -p
pantograph-workflow-service`; `cargo fmt -p pantograph-diagnostics-ledger -p
pantograph-workflow-service -- --check`; `git diff --check`; and targeted
no-fallback/no-legacy search over touched source/docs. Search matches were
existing compatibility/front-end documentation, existing legacy migration
regression tests, existing inference compatibility diagnostics, and existing
documentation text about producers; no new fallback, old graph/backend/runtime,
Tauri/frontend, Pumas, runtime-host execution, or workflow-service event
emission path was added. Remaining lifecycle work: wire workflow-service
start/terminal/cancel/redispatch emission through the existing
diagnostics-ledger append helper, then add projection/read-model fields only
after emitted event ordering is proven.

2026-06-04 workflow-service terminal scheduler attempt diagnostics update:
workflow-service now emits `scheduler.task_attempt_lifecycle_changed`
`Completed`/`Failed` events after canonical terminal store mutations for
non-runtime completion/failure, runtime dispatch selection failure, runtime
dispatch/supervisor failure, and runtime task results. Terminal runtime events
carry selected runtime and reservation facts from scheduler-owned dispatch/
terminal mutation data; failed `Ok(result)` runtime task statuses now emit a
failed attempt event instead of being classified by host-call success. The
obsolete non-terminal dispatch-failure wrappers were removed and tests use the
terminal-mutation API directly. This adds no diagnostics-ledger schema change,
projection/read-model fields, graph-path or reduced-plan launch behavior,
node-engine runtime launch branch, Tauri/frontend policy, runtime-adapter
lifecycle ownership, Pumas fact change, lockfile/generated/workflow fixture
change, or compatibility shim. Verification passed with focused non-runtime
completed, runtime completed, and runtime failed-result session tests,
`cargo test -p pantograph-workflow-service task_orchestrator --lib`, `cargo
check -p pantograph-workflow-service`, fmt check, `git diff --check`, and
targeted no-fallback/no-legacy source search. Remaining lifecycle work: emit
cancel and redispatch attempt lifecycle events, then add projection/read-model
fields only after emitted event ordering is proven.

2026-06-05 scheduler cancellation diagnostics re-plan decision: use Option 1,
the canonical observed-cancellation terminalization path. The next source
slice must first add a workflow-service-owned path that turns observed runtime
cancellation into a matching scheduler cancel terminal mutation, reservation
release intent/application, and then a `scheduler.task_attempt_lifecycle_changed`
`Cancelled` event. Cancellation request remains intent-only: it must not emit
terminal attempt diagnostics, release reservations, or mark scheduler state
terminal before runtime observation. This keeps scheduler task lifecycle,
reservation release, and diagnostics ordering in `pantograph-workflow-service`
instead of Tauri/frontend or runtime adapters. Option 2, emitting `Cancelled`
at cancellation request time, is rejected because it would lie about terminal
state and violate existing cancellation-intent semantics. Option 3, skipping to
redispatch events, is deferred because it leaves cancellation ordering
undefined. Option 4, adding a separate cancellation-requested diagnostics
contract, is a possible later control-plane event but not part of scheduler
attempt terminal lifecycle. The slice must add no graph-path or reduced-plan
execution, node-engine runtime launch, Tauri/frontend policy, runtime adapter
lifecycle ownership, Pumas fact changes, diagnostics-ledger schema/DTO change,
projection/read-model fields, lockfile/generated/workflow fixture changes, or
compatibility shim.

2026-06-05 observed scheduler cancellation terminalization update:
workflow-service now treats observed runtime supervisor cancellation as a
canonical terminal cancellation path. When the lifecycle owner aborts a
blocked runtime dispatch supervisor, the session runner applies the matching
scheduler cancel terminal mutation, releases the reservation through
`WorkflowCancelled`, emits a
`scheduler.task_attempt_lifecycle_changed` event with transition `Cancelled`,
and returns `WorkflowServiceError::Cancelled` so the run terminal status is
cancelled. Cancellation request remains intent-only and still does not mark the
attempt terminal or release the reservation before runtime observation. This
adds no diagnostics-ledger schema/DTO change, projection/read-model fields,
graph-path or reduced-plan launch behavior, node-engine runtime launch branch,
Tauri/frontend policy, runtime adapter lifecycle ownership, Pumas fact change,
lockfile/generated/workflow fixture change, or compatibility shim.
Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo test
-p pantograph-workflow-service
workflow_shutdown_aborts_blocked_runtime_dispatch_supervisor --lib`; `cargo
test -p pantograph-workflow-service
workflow_cancel_active_task_records_intent_without_terminal_mutation --lib`;
`cargo test -p pantograph-workflow-service
workflow_execution_session_records_failed_runtime_host_result_as_terminal_task_failure
--lib`; `cargo test -p pantograph-workflow-service task_orchestrator --lib`;
`cargo check -p pantograph-workflow-service`; `cargo fmt -p
pantograph-workflow-service -- --check`; `git diff --check`; and targeted
no-fallback/no-legacy source search. Search matches were existing legacy
negative tests and compatibility fixture fields only. Discovered/deferred
issue: the runtime-host execution contract still has no explicit cancelled
terminal response state, so cooperative runtime-host cancellation response
mapping remains a later re-plan if needed; this slice covers workflow-service
supervisor cancellation observation. Remaining lifecycle work: redispatch
attempt lifecycle events and projection/read-model fields after event ordering
is proven.

2026-06-05 scheduler redispatch attempt diagnostics update: bootstrap recovery
now marks ready-runtime redispatch attempt starts with the existing
`scheduler.task_attempt_lifecycle_changed` `Redispatched` transition instead
of emitting an ordinary `Started` event. The public runtime dependency
readiness resume request remains unchanged and continues to emit `Started`;
only the backend-owned bootstrap recovery decision path carries the private
redispatch transition into the workflow-service session runner. This uses the
existing diagnostics-ledger contract, scheduler attempt identity, persisted
readiness proof, and duplicate-dispatch guard path without adding
diagnostics-ledger schema/DTO changes, projection/read-model fields, graph-path
or reduced-plan launch behavior, node-engine runtime launch branch,
Tauri/frontend policy, runtime adapter lifecycle ownership, Pumas fact change,
lockfile/generated/workflow fixture change, or compatibility shim.
Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo test
-p pantograph-workflow-service
workflow_execution_session_bootstrap_recovery_redispatches_ready_runtime_task
--lib`; `cargo test -p pantograph-workflow-service
bootstrap_recovery_plan_accepts_ready_redispatch_with_recovery_state --lib`;
`cargo test -p pantograph-workflow-service
workflow_execution_session_resume_consumes_fresh_dependency_readiness_snapshot_and_dispatches_active_run
--lib`; `cargo test -p pantograph-workflow-service
workflow_execution_session_records_non_runtime_scheduler_attempt_lifecycle_events
--lib`; `cargo test -p pantograph-workflow-service task_orchestrator --lib`;
`cargo check -p pantograph-workflow-service`; `cargo fmt -p
pantograph-workflow-service -- --check`; `git diff --check`; and targeted
no-fallback/no-legacy source search. Search matches were existing legacy
negative tests, compatibility fixture fields, and existing no-compatible
candidate diagnostic text only. Remaining lifecycle work: add projection/
read-model fields now that started, terminal, cancelled, and redispatched
event ordering has been proven.

2026-06-05 diagnostics-ledger scheduler attempt timeline projection update:
diagnostics-ledger now includes `scheduler.task_attempt_lifecycle_changed`
events in the scheduler timeline projection drain and exposes typed nullable
attempt fields on `SchedulerTimelineProjectionRecord`: scheduler task id,
attempt id, execution class, lifecycle transition, start/end/duration timing,
selected runtime/backend/device/network-node facts, and reservation id. The
projection version was bumped and schema repair adds the columns for existing
ledgers. Workflow-service remains a pass-through owner for the read model; its
contract snapshot was updated to include the new nullable fields. Discovered
issue fixed: the scheduler timeline record builder already understood task
attempt events, but the drain query excluded them, so attempt events could not
appear in the timeline read model. This slice adds no graph-path or
reduced-plan launch behavior, node-engine runtime launch branch, Tauri/frontend
policy, runtime adapter lifecycle ownership, Pumas fact change, lockfile,
generated file, saved workflow fixture change, or compatibility shim.
Verification passed: `cargo test -p pantograph-diagnostics-ledger
scheduler_timeline_projection_exposes_scheduler_task_attempt_fields --lib`;
`cargo test -p pantograph-diagnostics-ledger
scheduler_timeline_projection_drains_events_incrementally --lib`; `cargo test
-p pantograph-diagnostics-ledger current_schema_repairs_all_drifted_projection_tables
--lib`; `cargo test -p pantograph-workflow-service
workflow_scheduler_timeline_query_reads_refreshed_projection --lib`; `cargo
test -p pantograph-workflow-service --test contract
workflow_scheduler_timeline_query_contract_snapshot`; `cargo check -p
pantograph-diagnostics-ledger`; `cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-diagnostics-ledger -p pantograph-workflow-service
-- --check`; `git diff --check`; and targeted no-fallback/no-legacy source
search. Search matches were pre-existing compatibility/legacy terminology,
schema compatibility helpers, and legacy negative tests only. Remaining
follow-up: consume these typed scheduler attempt facts from frontend/UI or
operator diagnostics views only through the backend-owned scheduler timeline
read model when that display slice is scheduled.

2026-06-05 scheduler page attempt timeline display update: the Scheduler page
now consumes the backend-owned scheduler timeline attempt fields through the
typed diagnostics service DTO and pure scheduler presenter rows. Attempt
timeline records render task id, attempt id, transition, execution class,
start/end/duration timing, selected runtime/backend/device/network-node facts,
and reservation id when those typed projection fields are present. The
frontend does not parse raw payload JSON, infer scheduler state, or introduce
Tauri/runtime/Pumas business logic. Verification passed: `node
--experimental-strip-types --test
src/components/workbench/schedulerPagePresenters.test.ts`; `npm run
typecheck`; `npm run build`; targeted `npx eslint
src/services/diagnostics/types.ts src/components/workbench/schedulerPagePresenters.ts
src/components/workbench/schedulerPagePresenters.test.ts
src/components/workbench/SchedulerPage.svelte --max-warnings 0`; `git diff
--check`; and targeted no-fallback/no-legacy source search. Verification
deviation/discovered issue: broad `npm run lint:full` still fails on an
unrelated existing `svelte/prefer-writable-derived` issue in
`src/components/nodes/workflow/PumaLibNode.svelte`; the touched files pass
targeted ESLint. Remaining follow-up: optional Diagnostics/Network page reuse
of the same presenter rows can be scheduled separately if those operator views
also need expanded attempt facts.

## Standards Rule

The standards constraints in
`02-image-generation-family-planner.md#standards-guardrails` and
`02-image-generation-family-planner.md#standards-compliance-matrix`, plus the
device/runtime constraints in `06-device-runtime-selection.md`, are binding
for every milestone. The directory READMEs in this plan are also part of the
documentation-traceability contract for the split plan structure. If
implementation needs to violate one of these constraints, stop and re-plan
before editing production code.
