# Plan: Current Image Generation Graphs And Stale Graph Diagnostics

2026-06-07 runtime-branch worker dispatch re-plan decision: use the
backend-rehydration bridge as the immediate source path, then promote to the
full durable task-attempt lifecycle after the complete inference path is
working. The worker already claims durable runtime-branch events and owns the
composition-root host boundary, but dispatch still needs facts currently held
by existing backend active-run/session state: session summary, queued inputs,
output targets, timeout/dequeued timing, active scheduler task graph/state,
task-run summary, and terminal diagnostic context. The next source slices must
therefore make the claimed durable event the execution authority and rehydrate
the remaining execution context from workflow-service active-run/session
records, not from request parameters, frontend/Tauri state, graph path
inference, or compatibility DTOs.

Immediate Option 1 bridge sequence:
1. Completed 2026-06-07: replace the current production `DispatchUnavailable`
   terminal deferral with
   a typed non-terminal blocked/ready-for-dispatch handling path or otherwise
   ensure a dispatch-capable worker can claim the same durable event without
   relying on a terminal `Deferred` event. The worker now releases a claimed
   runtime-branch task event back to `Ready` when dispatch is unavailable while
   returning a typed in-memory deferred notification to the caller.
2. Completed 2026-06-07: add a workflow-service-owned rehydration boundary
   that accepts a claimed runtime-branch event plus claim and reads only
   backend active-run/session records to build the execution context needed by
   the worker.
3. Completed 2026-06-07: move the existing runtime dispatch-boundary
   execution body behind the worker using that rehydrated context and the
   owned host boundary; the request path now only enqueues and awaits
   notification.
4. Completed 2026-06-07: persist completed/deferred/failed durable event
   state before notifying the in-memory responder, and keep missing/stale/
   expired facts as typed diagnostics. Readiness-pending runtime dependency
   outcomes now persist retryable `Deferred` state instead of releasing the
   event as plain `Ready`.
5. Once this proves a complete inference path, execute the follow-on Option 3
   lifecycle plan: promote runtime-branch events into the durable scheduler
   task-attempt lifecycle with explicit non-terminal running/dispatching,
   retry/defer/replay, batching, duplicate-dispatch guard, and restart
   semantics.

2026-06-07 complete inference path validation re-plan trigger:
`cargo test -p pantograph-workflow-service session_execution --lib` failed
with 21 of 44 filtered tests failing after the bridge completion. The dominant
pattern is stale test and call-site coverage that still drives runtime
inference through direct `WorkflowService::run_workflow_execution_session` or
asserts request-scoped runtime execution behavior; the new no-legacy rule
requires runtime-containing runs to enter through `WorkflowSessionExecutionRuntime`
so the task-execution worker owns dispatch. Other failures expose stale
expectations around source-input materialization, snapshot requirements,
diagnostic phase hints, terminal diagnostics, runtime host failure
classification, and shutdown supervisor observation under the worker-owned
path. Do not promote to the follow-on full task-attempt lifecycle until this
is replanned: the next plan decision must choose how to update the
workflow-service session execution tests and any remaining direct runtime
call sites without reintroducing compatibility shims or request-scoped
runtime execution.

2026-06-07 complete inference path validation re-plan decision: use Option 2,
the boundary contract migration. `WorkflowService::run_workflow_execution_session`
remains a direct workflow-service API for non-runtime runs and fail-closed
runtime rejection; runtime-containing workflow execution must enter through
`WorkflowSessionExecutionRuntime`, which is the composition-root runtime owner
for the task-execution worker, shared backend service, and host boundary.
This keeps business logic in backend workflow-service ownership, keeps worker
and runtime lifecycle ownership in the composition root, avoids Tauri/frontend
runtime policy, and removes request-scoped runtime execution rather than
preserving it through a compatibility shim. Do not start the follow-on full
durable task-attempt lifecycle promotion until this migration validates the
complete inference path.

Option 2 validation sequence:
1. Classify the failing `session_execution` tests by owner: direct
   non-runtime `WorkflowService` coverage, direct runtime rejection coverage,
   runtime-capable `WorkflowSessionExecutionRuntime` coverage, diagnostics
   coverage, and shutdown/supervisor coverage.
2. Migrate runtime-capable workflow-service tests to construct and use
   `WorkflowSessionExecutionRuntime`; keep direct `WorkflowService` tests only
   for non-runtime behavior and explicit fail-closed runtime rejection.
3. Update any production adapter/call-site that executes runtime-containing
   workflows to enter through the composition-root runtime boundary instead of
   direct `WorkflowService` execution.
4. Update diagnostics expectations for worker-owned execution: source-input
   materialization, executable snapshot requirements, phase hints, terminal
   diagnostics, runtime host failures, and shutdown supervisor observation.
5. Run `cargo test -p pantograph-workflow-service session_execution --lib`,
   followed by the focused runtime-branch checks. Only when this passes may the
   plan resume the follow-on full durable scheduler task-attempt lifecycle.

2026-06-07 runtime-capable session_execution test migration slice:
in this runtime migration slice, moved runtime-containing `session_execution`
coverage that was still calling direct `WorkflowService` APIs into
`WorkflowSessionExecutionRuntime`:
`workflow_execution_session_dispatches_ready_runtime_task_through_scheduler_selection`,
`workflow_execution_session_resume_consumes_fresh_dependency_readiness_snapshot_and_dispatches_active_run`,
and `workflow_execution_session_records_failed_runtime_host_result_as_terminal_task_failure`.
The stale
`workflow_execution_session_records_load_completed_only_with_runtime_proof`
test was removed instead of migrated: it asserted the old
`SchedulerRunAdmitted`/`WorkflowSessionRuntimeLoadProof` diagnostic path, but
current production code no longer writes `SchedulerRunAdmitted` and no longer
uses `WorkflowHost::session_runtime_load_proof` during one-shot direct
execution. Keeping or reintroducing that behavior would preserve a legacy
fallback diagnostic path instead of the canonical runtime-owner and
task-attempt lifecycle systems.
No-fallback/no-legacy confirmation: runtime-owning sessions now enter through the
composition-root runtime owner; no request-scoped dispatch/completion was
added and no compatibility path was preserved. The obsolete test fixture helper
for manufacturing runtime load proofs was removed from workflow-service tests.

Verification:
`cargo test -p pantograph-workflow-service workflow_execution_session_dispatches_ready_runtime_task_through_scheduler_selection --lib`
passed,
`cargo test -p pantograph-workflow-service workflow_execution_session_resume_consumes_fresh_dependency_readiness_snapshot_and_dispatches_active_run --lib`
passed,
`cargo test -p pantograph-workflow-service workflow_execution_session_records_failed_runtime_host_result_as_terminal_task_failure --lib`
passed, and `cargo fmt -p pantograph-workflow-service -- --check` passed after
formatting. Broader verification
`cargo test -p pantograph-workflow-service session_execution --lib` still fails
with 10 remaining tests after the 2026-06-07 source-input materialization slice, covering
expected follow-on migration work: stale
direct runtime entrypoints, tests that now require saved executable validation
snapshots, legacy host-execution assumptions after scheduler task ownership,
and runtime-load/unload diagnostics still expecting direct request-scoped
surfaces.

Behavioral note:
runtime-branch failures from worker-owned dispatch remain surfaced as
`InternalError` with message `scheduler task ... final state was TerminalFailed`
instead of the previous direct `WorkflowService` terminal-invalid surface.

2026-06-07 runtime dispatch panic migration slice:
smallest useful vertical slice: migrate
`workflow_execution_session_records_runtime_dispatch_panic_as_terminal_task_failure`
from direct `WorkflowService::run_workflow_execution_session` to
`WorkflowSessionExecutionRuntime`. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`
and this plan. No-fallback/no-legacy confirmation: the test now enters through
the composition-root runtime owner and does not add a request-scoped dispatch
shim or restore direct runtime execution.

Verification:
`cargo test -p pantograph-workflow-service workflow_execution_session_records_runtime_dispatch_panic_as_terminal_task_failure --lib`
passed,
`cargo test -p pantograph-workflow-service workflow_execution_session_records_failed_runtime_host_result_as_terminal_task_failure --lib`
passed,
and `cargo fmt -p pantograph-workflow-service -- --check` passed.

Behavioral note:
runtime host dispatch panics now surface as `InternalError` while preserving the
canonical supervisor diagnostic text `runtime task supervisor join failed`.

2026-06-07 runtime dispatch-boundary migration slice:
smallest useful vertical slice: migrate
`workflow_execution_session_fresh_dependency_readiness_snapshot_stops_at_dispatch_boundary`
from direct `WorkflowService::run_workflow_execution_session` to
`WorkflowSessionExecutionRuntime`. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`
and this plan. No-fallback/no-legacy confirmation: the test now reaches the
canonical runtime worker path and verifies fail-closed dispatch selection
without restoring direct runtime execution.

Verification:
`cargo test -p pantograph-workflow-service workflow_execution_session_fresh_dependency_readiness_snapshot_stops_at_dispatch_boundary --lib`
passed and `cargo fmt -p pantograph-workflow-service -- --check` passed.

Behavioral note:
missing runtime dispatch selection now surfaces through the worker-owned path as
`InternalError` while preserving the canonical diagnostic text
`runtime scheduler dispatch selection failed closed`.

2026-06-07 assertion correction slice:
fixed an accidental assertion drift in
`workflow_execution_session_rejects_new_run_when_task_lifecycle_shutdown`; this
test remains direct fail-closed coverage and correctly expects
`CapabilityViolation` when task execution ownership is unavailable. Verification:
`cargo test -p pantograph-workflow-service workflow_execution_session_rejects_new_run_when_task_lifecycle_shutdown --lib`
passed and `cargo fmt -p pantograph-workflow-service -- --check` passed.

2026-06-07 reservation lifecycle missing-port migration slice:
smallest useful vertical slice: migrate
`workflow_execution_session_fails_closed_when_reservation_lifecycle_port_is_missing`
from direct `WorkflowService::run_workflow_execution_session` to
`WorkflowSessionExecutionRuntime`, while keeping the reservation lifecycle port
intentionally absent. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`
and this plan. No-fallback/no-legacy confirmation: the test now reaches the
canonical runtime worker path and fails closed before runtime-host execution
without restoring direct request-scoped runtime execution.

Verification:
`cargo test -p pantograph-workflow-service workflow_execution_session_fails_closed_when_reservation_lifecycle_port_is_missing --lib`
passed,
`cargo test -p pantograph-workflow-service workflow_execution_session_runtime_run_fails_closed_before_legacy_launch --lib`
passed,
and `cargo fmt -p pantograph-workflow-service -- --check` passed.

Behavioral note:
missing reservation lifecycle configuration now surfaces through the
worker-owned path as `InternalError` while preserving the diagnostic text
`reservation lifecycle port is not configured`. Direct runtime fail-closed
coverage remains direct and still rejects before legacy launch when executable
validation snapshots are absent.

2026-06-07 source-input materialization test slice:
smallest useful vertical slice: update non-runtime session tests that still
depended on legacy host-level execution shortcuts by binding external inputs to
the canonical source node `text-input-1:text`. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
`crates/pantograph-workflow-service/src/workflow/tests/fixtures/execution_hosts.rs`,
and this plan. No-fallback/no-legacy confirmation: the slice does not restore
request-scoped host execution; `RecordingRuntimeHost` now owns its in-memory
workflow I/O contract explicitly, and one-shot non-runtime execution now asserts
that session runtime loading is not invoked. The stale
`workflow_execution_session_run_passes_logical_session_id_in_run_options` test
was removed because it asserted legacy host `WorkflowRunOptions` propagation;
the scheduler-owned path no longer calls `WorkflowHost::run_workflow` for this
behavior, and the attribution snapshot store currently rejects non-runtime
validation snapshots with no inference nodes.

Verification:
`cargo test -p pantograph-workflow-service one_shot_non_runtime_session_run_does_not_load_session_runtime --lib`
passed,
`cargo test -p pantograph-workflow-service workflow_execution_session_repeated_runs_create_distinct_backend_run_ids --lib`
passed,
`cargo test -p pantograph-workflow-service workflow_execution_session_records_retained_node_io_artifact_bodies --lib`
passed,
`cargo fmt -p pantograph-workflow-service -- --check` passed, and
`cargo test -p pantograph-workflow-service session_execution --lib` now fails
with 10 remaining tests.

Discovered issue:
workflow run snapshots are currently coupled to inference-only executable
validation snapshots; a non-runtime graph cannot be represented by the current
`WorkflowExecutableValidationSnapshotRecord` validator because it requires at
least one inference node. Keep this as a follow-up for the snapshot/attribution
test migration instead of adding an invalid placeholder snapshot.

2026-06-07 scheduler task-state initialization test slice:
smallest useful vertical slice: migrate
`workflow_execution_session_initializes_scheduler_task_state_before_run_execution`
away from the legacy `WorkflowHost::run_workflow` synchronization point. The
test now keeps the non-runtime scheduler run active with delayed workflow I/O,
waits for the backend active-run record, and asserts the canonical scheduler
task-state read models for source and output tasks before task execution can
complete. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`
and this plan. No-fallback/no-legacy confirmation: this slice does not restore
host-level `run_workflow` execution, does not add a compatibility shim, and
keeps task-state initialization verified at the scheduler-owned boundary.

Verification:
`cargo test -p pantograph-workflow-service workflow_execution_session_initializes_scheduler_task_state_before_run_execution --lib`
passed,
`cargo fmt -p pantograph-workflow-service -- --check` passed, and
`cargo test -p pantograph-workflow-service session_execution --lib` now fails
with 9 remaining tests: runtime load/unload diagnostic classification and
phase hints, snapshot/attribution requirements for tests without saved
executable validation snapshots, sanitized terminal runtime failure
classification, dependency-readiness resume planning, and shutdown supervisor
observation under the worker-owned path.

2026-06-07 snapshot and attribution session_execution slice:
smallest useful vertical slice: migrate
`workflow_execution_session_run_records_snapshot_before_execution` and
`attributed_workflow_execution_session_carries_client_bucket_into_run_events`
to the canonical saved executable validation snapshot requirement and source
input materialization boundary. The slice also fixes scheduler task-attempt
diagnostic attribution by reading the run snapshot from the attribution store
instead of emitting task-attempt events with empty client/session/bucket
metadata. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/session_scheduler_runner.rs`,
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
and this plan. No-fallback/no-legacy confirmation: the tests do not bypass
snapshot admission, do not restore `SchedulerRunAdmitted`, do not manufacture
runtime reservation/model-load events for a non-runtime scheduler path, and do
not reintroduce request-scoped host execution.

Verification:
`cargo test -p pantograph-workflow-service workflow_execution_session_run_records_snapshot_before_execution --lib`
passed,
`cargo test -p pantograph-workflow-service attributed_workflow_execution_session_carries_client_bucket_into_run_events --lib`
passed,
`cargo fmt -p pantograph-workflow-service -- --check` passed, and
`cargo test -p pantograph-workflow-service session_execution --lib` now fails
with 7 remaining tests: runtime load/unload diagnostic classification and
phase hints, sanitized terminal runtime failure classification,
dependency-readiness resume planning, and shutdown supervisor observation under
the worker-owned path.

2026-06-07 direct runtime diagnostic cleanup slice:
smallest useful vertical slice: remove stale `session_execution` tests that
asserted direct request-scoped runtime load, run, unload, and sanitized terminal
diagnostic behavior through `WorkflowService::run_workflow_execution_session`.
Those paths are no longer canonical: direct workflow-service runtime execution
must fail closed, while runtime-host dispatch and terminal failures are covered
through `WorkflowSessionExecutionRuntime` and the task-execution worker. The
unused direct-load/direct-run poisoned diagnostic fixtures were removed; the
unload poisoned fixture remains because session capacity coverage still owns
that legacy unload-capacity behavior. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
`crates/pantograph-workflow-service/src/workflow/tests/fixtures/execution_hosts.rs`,
and this plan. No-fallback/no-legacy confirmation: this slice deletes stale
direct-runtime expectations instead of adding compatibility shims, does not
restore host-level runtime load/run/unload execution, and preserves direct
runtime fail-closed coverage.

Verification:
`cargo fmt -p pantograph-workflow-service -- --check` passed, and
`cargo test -p pantograph-workflow-service session_execution --lib` now runs 37
filtered tests with 35 passing and 2 remaining failures:
`workflow_execution_session_bootstrap_recovery_applies_dependency_readiness_resume_plan`
and `workflow_shutdown_aborts_blocked_runtime_dispatch_supervisor`.

Outstanding Option 2 migration work:
- keep explicit runtime fail-closed coverage on direct `WorkflowService`,
- update dependency-readiness resume and shutdown supervisor coverage to the
  worker-owned path, and then proceed to adapter/call-site migration.

2026-06-07 bootstrap recovery and shutdown re-plan decision: do Option 2 now,
then Option 3 after validation. Option 2 is the runtime-owned recovery/shutdown
execution boundary: `WorkflowService` remains the owner of backend recovery
reporting, recovery planning, active-run/session records, and typed diagnostics,
but runtime recovery execution must move to `WorkflowSessionExecutionRuntime`
and its `WorkflowTaskExecutionRuntimeOwner`. Direct `WorkflowService`
recovery may continue to plan and may apply non-runtime/progress-loop backend
state transitions, but it must not execute runtime dependency-readiness resume,
ready-runtime redispatch, runtime-host dispatch, or runtime dispatch supervisor
shutdown without the composition-root runtime owner. Runtime recovery or
shutdown without that owner must fail closed with typed diagnostics instead of
falling back to request-scoped host execution. Option 3 remains the follow-on
durable lifecycle promotion after the Option 2 boundary validates the complete
inference path.

Option 2 recovery/shutdown sequence:
1. Add a composition-root recovery facade on `WorkflowSessionExecutionRuntime`
   that delegates recovery planning/reporting to `WorkflowService` but applies
   runtime resume/redispatch through the owned task-execution runtime owner.
   Keep shared contracts and runtime-owner wiring serial integration-owner
   work.
2. Split bootstrap recovery application so `WorkflowService` exposes backend
   plan/report primitives plus non-runtime/progress-loop recovery, while
   runtime dependency-readiness resume and ready-runtime redispatch require an
   explicit runtime owner.
3. Migrate
   `workflow_execution_session_bootstrap_recovery_applies_dependency_readiness_resume_plan`
   to use `WorkflowSessionExecutionRuntime` and assert the runtime-host dispatch
   request, dependency-readiness work queue cleanup, and absence of direct host
   runtime load/run attempts.
4. Move shutdown/supervisor coverage to the same runtime-owned boundary:
   runtime dispatch starts through the worker, lifecycle shutdown cancels the
   owned runtime dispatch supervisor, and the runtime-host cancellation handle
   observes the shutdown request. Direct service shutdown must not synthesize
   runtime dispatch state.
5. Preserve direct `WorkflowService` fail-closed tests for runtime recovery or
   shutdown attempted without a runtime owner, with typed diagnostics that name
   the missing composition-root runtime owner.
6. Run `cargo test -p pantograph-workflow-service session_execution --lib`,
   focused runtime-branch/task-execution worker tests, and formatting checks.
   Only after this passes may adapter/call-site migration continue.

2026-06-07 bootstrap runtime recovery facade slice: completed Option 2
steps 1-3 for bootstrap runtime recovery. Smallest useful vertical slice:
route bootstrap runtime dependency-readiness resume and ready-runtime
redispatch through `WorkflowSessionExecutionRuntime` and
`WorkflowTaskExecutionRuntimeOwner`, while direct `WorkflowService` recovery
still owns planning, progress-loop recovery, and typed fail-closed diagnostics
when runtime recovery is attempted without the composition-root owner. Allowed
files touched:
`crates/pantograph-workflow-service/src/scheduler/store_task_results.rs`,
`crates/pantograph-workflow-service/src/scheduler/task_orchestrator.rs`,
`crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
`crates/pantograph-workflow-service/src/workflow/session_scheduler_runner.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_facade.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_owner.rs`,
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
and this plan. No-fallback/no-legacy confirmation: this slice does not call
the request-scoped runtime helper for bootstrap runtime recovery, does not
preserve direct service runtime redispatch, does not synthesize successful
completion, and routes runtime dispatch through the owned worker command path.
Focused behavior added/updated: the runtime facade exposes bootstrap recovery,
runtime recovery ensures claimable runtime-branch task events exist before
worker dispatch, rehydrated worker redispatch preserves the typed
`Redispatched` scheduler attempt lifecycle transition, and progress-loop
recovery is idempotent for already-materialized source-input results.
Verification: `cargo fmt` passed;
`cargo test -p pantograph-workflow-service workflow_execution_session_bootstrap_recovery --lib`
passed with 3 tests; `cargo test -p pantograph-workflow-service session_execution --lib`
now runs 37 filtered tests with 36 passing and one remaining known failure,
`workflow_shutdown_aborts_blocked_runtime_dispatch_supervisor`, which still
needs Option 2 step 4 migration to the runtime-owned shutdown/supervisor
boundary.

2026-06-07 runtime-owned shutdown facade slice: completed Option 2
step 4 for the blocked runtime dispatch supervisor. Smallest useful vertical
slice: add `WorkflowSessionExecutionRuntime::shutdown_workflow_execution_runtime`
as the composition-root shutdown entrypoint, migrate the blocked runtime
dispatch cancellation test to run and shut down through the same runtime
facade, and preserve worker-projected runtime cancellation as typed
`WorkflowServiceError::Cancelled` instead of flattening it to an internal
error. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_facade.rs`,
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
and this plan. No-fallback/no-legacy confirmation: this slice does not start
runtime work through direct `WorkflowService`, does not synthesize dispatch
state during shutdown, does not bypass the worker-owned runtime branch path,
and preserves the runtime-host cancellation handle as the shutdown signal.
Verification: `cargo fmt` passed;
`cargo test -p pantograph-workflow-service workflow_shutdown_aborts_blocked_runtime_dispatch_supervisor --lib`
passed with 1 test; `cargo test -p pantograph-workflow-service session_execution --lib`
passed with 37 tests; `cargo test -p pantograph-workflow-service task_execution_runtime --lib`
passed with 4 tests; and
`cargo test -p pantograph-workflow-service task_execution_facade --lib`
passed with 3 tests. Remaining Option 2 follow-up: keep direct
`WorkflowService` fail-closed coverage for runtime recovery/shutdown without a
runtime owner, then proceed to adapter/call-site migration if no new re-plan
trigger appears.

2026-06-07 direct recovery fail-closed coverage slice: completed the
remaining Option 2 step 5 recovery assertion by adding explicit coverage that
direct `WorkflowService::recover_workflow_execution_session_bootstrap` rejects
a ready-runtime redispatch plan unless recovery enters through
`WorkflowSessionExecutionRuntime`. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`
and this plan. No-fallback/no-legacy confirmation: the direct service recovery
path returns the typed missing composition-root owner diagnostic and does not
dispatch to the runtime host. Verification: `cargo fmt` passed;
`cargo test -p pantograph-workflow-service workflow_execution_session_bootstrap_recovery_redispatches_ready_runtime_task --lib`
passed with 1 test, and
`cargo test -p pantograph-workflow-service session_execution --lib` passed
with 37 tests. Remaining Option 2 work: update any production adapter/call-site
that still executes runtime-containing workflows directly through
`WorkflowService`; if no such call sites remain, mark the Option 2 validation
sequence complete and resume the planned Option 3 durable lifecycle work.

2026-06-07 embedded runtime session-run adapter slice: migrated the
production `EmbeddedRuntime::run_workflow_execution_session` and
`run_workflow_execution_session_with_event_sink` call sites to construct
`WorkflowSessionExecutionRuntime` from the shared workflow service and cloned
embedded host before running a session. Allowed files touched:
`crates/pantograph-embedded-runtime/src/embedded_workflow_service_api.rs` and
this plan. No-fallback/no-legacy confirmation: the embedded adapter no longer
executes session runs by calling direct `WorkflowService` with a request-scoped
host, does not branch on runtime/non-runtime in the adapter, and does not add
compatibility shims. Verification: `cargo fmt` passed;
`cargo check -p pantograph-embedded-runtime` passed;
`cargo test -p pantograph-embedded-runtime test_runtime_run_and_session_execution --lib`
passed with 1 test. Discovered during broader verification and deferred as
separate test/architecture follow-up:
`cargo test -p pantograph-embedded-runtime session_execution_state --lib`
failed 2 stale keep-alive executor tests that still expect the old backend
session executor to exist under worker-owned session runs, and
`cargo test -p pantograph-embedded-runtime workflow_run_execution --lib`
failed 7 tests that mix direct `WorkflowService` runtime calls, stale
unknown-node/diagnostic expectations, human-input classification expectations,
and live-event assumptions from the legacy execution path. These failures are
recorded for the next embedded-runtime test migration slice and must not be
fixed by reintroducing direct request-scoped runtime execution.

2026-06-07 embedded keep-alive input carry-forward re-plan decision: use
Option 3 and remove implicit input carry-forward from keep-alive session
execution. The failed `session_execution_state` migration exposed that stale
tests still treat keep-alive as backend node-memory persistence: a later
keep-alive run may omit a required input and still expect the old
`EmbeddedRuntime.session_executions` executor to replay a value from an earlier
run. That behavior is rejected. Keep-alive means runtime/model residency only:
Pantograph may keep a compatible runtime instance and loaded model resident so
later compatible tasks can reuse it without memory churn, but workflow inputs,
upstream task results, node memory, prompt values, and other run-scoped context
must not be carried from the workflow run that first loaded the runtime into
later runs. Every workflow/session run owns its own source inputs. Missing,
wrong-type, unavailable, or invalid required inputs must return typed
diagnostics before task execution, not silently reuse prior values.

Rejected alternatives:
1. Backend-owned keep-alive input memory in workflow-service session state is
   rejected because it would formalize the input carry-forward anti-feature and
   couple runtime residency to workflow data persistence.
2. Durable scheduler/session facts for carried-forward omitted inputs are
   rejected for the same reason. The durable task-attempt lifecycle remains the
   target for runtime attempt ownership, batching, replay, retry/defer, and
   residency evidence, but not for replaying omitted workflow inputs.

Next embedded-runtime test migration slice:
1. Update
   `crates/pantograph-embedded-runtime/src/lib_tests/session_execution_state_tests.rs`
   so keep-alive tests assert runtime/model residency semantics and per-run
   input ownership, not backend executor or node-memory input reuse.
2. Replace omitted-required-input expectations with typed missing-input
   diagnostics from the worker-owned path.
3. Preserve or add focused coverage that same-model/same-runtime keep-alive
   reuse is driven by canonical runtime residency facts rather than by carried
   workflow inputs.
4. Run `cargo test -p pantograph-embedded-runtime session_execution_state --lib`
   and `cargo check -p pantograph-embedded-runtime`, then update this plan with
   the verification result and any remaining stale checkpoint/README ownership
   follow-ups.

Do not restore the embedded node-engine executor as a compatibility shim, do
not add backend input memory for omitted keep-alive inputs, and do not make the
embedded adapter choose legacy execution for keep-alive runs.

2026-06-07 embedded keep-alive per-run input test migration slice:
completed the selected Option 3 test migration for
`session_execution_state`. Smallest useful vertical slice: update the two stale
keep-alive execution-state tests so they no longer assert
`EmbeddedRuntime.session_executions` executor reuse or node-memory input
carry-forward. Allowed files touched:
`crates/pantograph-embedded-runtime/src/lib_tests/session_execution_state_tests.rs`,
`crates/pantograph-embedded-runtime/src/lib_tests/README.md`, and this plan.
No-fallback/no-legacy confirmation: the tests now assert that keep-alive
preserves canonical runtime residency facts through a keep-alive reservation,
while every session run must provide its own source inputs; omitted required
inputs return typed diagnostics from the worker-owned path instead of replaying
old workflow data.

Verification:
`cargo test -p pantograph-embedded-runtime session_execution_state --lib`
passed with 2 tests,
`cargo check -p pantograph-embedded-runtime` passed, and
`cargo fmt -p pantograph-embedded-runtime -- --check` passed.

Discovered follow-up:
`cargo test` and `cargo check` still report existing unused
`WorkflowSchedulerStartedRuntimeTaskSupervisor` methods
`abort_handle`, `abort`, and `abort_and_join` in
`crates/pantograph-workflow-service/src/scheduler/task_orchestrator.rs`. This
warning is outside the embedded-runtime keep-alive test migration slice and is
not fixed here.

2026-06-07 embedded human-input scheduler rejection test slice:
smallest useful vertical slice: update the stale
`workflow_run_execution_session_returns_invalid_request_for_human_input_workflow`
expectation after session runs moved to the scheduler-owned execution path.
Allowed files touched:
`crates/pantograph-embedded-runtime/src/lib_tests/workflow_run_execution_tests.rs`
and this plan. No-fallback/no-legacy confirmation: the test now accepts the
canonical fail-closed `CapabilityViolation` for an unsupported scheduler task
state with no execution path; it does not restore the old invalid-request
surface, direct node-engine execution, or a human-input compatibility runner.

Verification:
`cargo test -p pantograph-embedded-runtime workflow_run_execution_session_rejects_human_input_workflow_without_execution_path --lib`
passed with 1 test. Broader verification
`cargo test -p pantograph-embedded-runtime workflow_run_execution --lib` now
has 3 passing tests and 6 remaining failures: three stale `onnx-inference`
fixture tests blocked by stale graph diagnostics, one production embedded image
runtime-host test still entering direct `WorkflowService` runtime execution,
one run-detail/node-IO projection expectation, and one live-event expectation
that still looks for legacy `TaskCompleted` node-engine events.

2026-06-07 embedded retired ONNX fixture diagnostics slice:
smallest useful vertical slice: migrate the three stale `onnx-inference`
workflow-run tests from successful Python-sidecar execution expectations to the
canonical stale-graph diagnostic path. Allowed files touched:
`crates/pantograph-embedded-runtime/src/lib_tests/workflow_run_execution_tests.rs`
and this plan. No-fallback/no-legacy confirmation: the slice does not restore
the retired `onnx-inference` graph node, does not route the stale graph into the
Python sidecar, and does not reconcile an `onnx-runtime` runtime-registry record
from a retired graph fixture. The tests now assert `UnknownNodeType`
diagnostics for the retired node type and no Python-sidecar calls.

Verification:
`cargo test -p pantograph-embedded-runtime retired_onnx_audio_workflow --lib`
passed with 3 tests. Broader verification
`cargo test -p pantograph-embedded-runtime workflow_run_execution --lib` now
has 6 passing tests and 3 remaining failures: one production embedded image
runtime-host test still entering direct `WorkflowService` runtime execution,
one run-detail/node-IO projection expectation, and one live-event expectation
that still looks for legacy `TaskCompleted` node-engine events.

2026-06-07 embedded production image runtime-owner test slice:
smallest useful vertical slice: migrate
`workflow_execution_session_dispatches_through_production_embedded_image_runtime_host`
from direct `WorkflowService::run_workflow_execution_session` to
`WorkflowSessionExecutionRuntime`. Allowed files touched:
`crates/pantograph-embedded-runtime/src/lib_tests/workflow_run_execution_tests.rs`
and this plan. No-fallback/no-legacy confirmation: the test now uses the
composition-root runtime owner and shared workflow service, does not restore
direct request-scoped runtime execution, and still verifies the production
embedded runtime-host port, artifact retention, dependency readiness, dispatch
source refresh, reservation lifecycle, and absence of legacy host runtime/run
attempts.

Verification:
`cargo test -p pantograph-embedded-runtime workflow_execution_session_dispatches_through_production_embedded_image_runtime_host --lib`
passed with 1 test. Broader verification
`cargo test -p pantograph-embedded-runtime workflow_run_execution --lib` now
has 7 passing tests and 2 remaining failures: one run-detail/node-IO projection
expectation and one live-event expectation that still looks for legacy
`TaskCompleted` node-engine events.

2026-06-07 embedded scheduler projection refresh test slice:
smallest useful vertical slice: update the scheduler projection test to refresh
canonical `RunDetail`, `NodeStatus`, and `IoArtifact` projections explicitly
before querying, and align retained artifact expectations with the worker-owned
path. Allowed files touched:
`crates/pantograph-embedded-runtime/src/lib_tests/workflow_run_execution_tests.rs`
and this plan. No-fallback/no-legacy confirmation: the test does not rely on
implicit projection side effects from legacy node-engine execution, does not
manufacture old `node_input` artifacts for the output node, and records that
this worker-owned path currently has no node-status rows while preserving run
detail plus retained workflow input, node output, and workflow output artifact
coverage.

Verification:
`cargo test -p pantograph-embedded-runtime scheduler_run_retains_detail_and_terminal_output_projection --lib`
passed with 1 test. Broader verification
`cargo test -p pantograph-embedded-runtime workflow_run_execution --lib` now
has 8 passing tests and 1 remaining failure: the live-event expectation still
looks for legacy `TaskCompleted` node-engine events.

2026-06-07 embedded live-event legacy removal test slice:
smallest useful vertical slice: update the remaining live-event test so the
worker-owned session path asserts the absence of legacy node-engine
`TaskCompleted` events instead of requiring them. Allowed files touched:
`crates/pantograph-embedded-runtime/src/lib_tests/workflow_run_execution_tests.rs`
and this plan. No-fallback/no-legacy confirmation: the slice does not restore
node-engine task-completed event emission as a compatibility shim; it preserves
the backend workflow-run id/session-id distinction and verifies that neither id
is emitted through the old node-engine event sink for task completion.

Verification:
`cargo test -p pantograph-embedded-runtime scheduler_session_event_sink_omits_legacy_task_completed_events --lib`
passed with 1 test,
`cargo test -p pantograph-embedded-runtime workflow_run_execution --lib` passed
with 9 tests,
`cargo check -p pantograph-embedded-runtime` passed, and
`cargo fmt -p pantograph-embedded-runtime -- --check` passed. `cargo test` and
`cargo check` still report the existing unused
`WorkflowSchedulerStartedRuntimeTaskSupervisor` method warning recorded above.

2026-06-07 broader embedded-runtime verification re-plan trigger:
after the focused `session_execution_state` and `workflow_run_execution`
migrations passed, broader verification
`cargo test -p pantograph-embedded-runtime --lib` failed with 389 passing tests
and 17 failing tests. This is a re-plan trigger before additional source work
because the failures span multiple ownership boundaries rather than one safe
vertical slice:
- Six `node_execution_ledger::tests::node_execution_workflow_sink_*` tests now
  observe zero retained node artifacts where legacy node-engine event sink
  behavior expected one artifact.
- Two `data_graph_execution_tests` still expect successful retired
  Python-sidecar ONNX/audio graph execution and runtime-registry reconciliation.
- One `technical_fit` test expects owner selector package facts to materialize
  one required-model fact but now observes zero.
- Six keep-alive checkpoint capacity/recovery tests still expect
  `EmbeddedRuntime.session_executions` executor/checkpoint state under
  worker-owned session runs.
- Two runtime-preflight restart tests now return `InvalidRequest` instead of
  the stale expected `RuntimeNotReady` classification.

Do not continue by fixing these as one batch. The next re-plan must choose
which ownership area to migrate first, with non-overlapping write sets and
explicit no-fallback rules. Candidate next slices are:
1. Keep-alive checkpoint tests: decide whether checkpoint preservation remains
   a supported runtime/model residency behavior without input carry-forward, or
   whether the old executor checkpoint API is retired and tests should move to
   runtime-registry/session-state facts.
2. Node-execution ledger sink tests: decide whether retained node artifacts
   should be produced by canonical worker/task result facts, or whether the
   old node-engine event sink artifact expectations should be retired.
3. Data-graph Python sidecar tests: align retired ONNX/audio graph fixtures
   with stale-graph diagnostics or migrate them to a canonical supported
   data-graph task path.
4. Technical-fit Pumas facts: inspect whether this is a fixture/setup drift
   after the pushed Pumas owner-selector changes or a real package-fact
   projection regression.
5. Runtime-preflight restart classification: decide whether `InvalidRequest`
   is the canonical typed diagnostic for stale selected-runtime restart facts
   or whether the backend should still classify that condition as
   `RuntimeNotReady`.

2026-06-07 broader embedded-runtime re-plan decision: execute Option 4 first,
the Technical-Fit/Pumas fact classification slice, because a required-model
package-fact projection failure can block later runtime scheduling and
resource-admission decisions. This is an investigation and contract-alignment
slice, not a license to add fallback behavior. The slice must reproduce the
focused
`required_model_package_facts_resolve_from_owner_selector_access` failure,
classify it as either Pantograph fixture/setup drift after the pushed Pumas
owner-selector changes or a real Pantograph owner-API projection regression,
and then make the smallest correction in Pantograph's technical-fit boundary.
It must not synthesize package facts from graph paths, selector summaries,
read-only/local-client access, UI state, or legacy workflow metadata; owner
selector access remains the only source for full package facts. If the failing
test proves Pumas no longer exposes the required owner-API fact contract,
stop and re-plan with a Pumas handoff instead of inventing a Pantograph shim.
After this slice validates, re-plan the keep-alive checkpoint failures next so
runtime/model residency can be settled without implicit input carry-forward.

2026-06-07 Technical-Fit/Pumas owner fact slice: completed. Smallest useful
vertical slice: classify and fix the embedded-runtime technical-fit failure
where Pumas owner selector access resolved full package facts but Pantograph
projected zero required-model facts. Classification: this was a Pantograph
projection/test fixture drift after the current Pumas owner API added
`model_ref.model_ref_contract_version` and expects fixtures to use the
owner model-library metadata/index path. The fix keeps full package facts
owner-sourced, strips the additive Pumas model-ref contract marker before
decoding into the current inference contract, and updates the focused test to
write metadata through `PumasApi::model_library().save_metadata` before index
rebuild. No-fallback/no-legacy confirmation: no selector summary promotion,
path scan, graph-visible package fact, UI/Tauri state, local-client/read-only
full-fact access, or compatibility workflow metadata path was added.
Verification passed:
`cargo test -p pantograph-embedded-runtime required_model_package_facts_resolve_from_owner_selector_access --lib`,
`cargo test -p pantograph-embedded-runtime technical_fit --lib`,
`cargo check -p pantograph-embedded-runtime`, and
`cargo fmt -p pantograph-embedded-runtime -- --check`. Broader verification
`cargo test -p pantograph-embedded-runtime --lib` now fails with 390 passing
tests and 16 failing tests; the technical-fit failure is removed. Remaining
re-plan area: keep-alive checkpoint capacity/recovery semantics without
workflow input carry-forward, followed by node-execution ledger artifacts,
retired data-graph fixtures, and runtime-preflight restart classification.

2026-06-07 keep-alive checkpoint capacity/recovery re-plan decision: use
Option 2 and redefine the stale checkpoint behavior as runtime/model residency
fact coverage. The six failing checkpoint capacity/recovery tests still assert
the retired `EmbeddedRuntime.session_executions` executor/checkpoint path.
Those assertions are not canonical checkpoint requirements. The supported
behavior is that a compatible runtime/model instance may remain resident or
become reusable after capacity pressure, recovery, or scheduler reclaim; it is
not that node-engine memory, workflow inputs, upstream task results, prompt
values, run envelopes, or executor snapshots carry into later workflow runs.

Canonical checkpoint/residency evidence may include runtime id, runtime
instance or reservation id when one exists, backend/model/artifact identity,
retention intent, residency state, reclaim/reuse eligibility, reservation
release facts, and bounded typed diagnostics for failed restore, unavailable
runtime, or incompatible reuse. It must not include workflow input values,
upstream task outputs, node memory snapshots, prompt/context values, frontend
state, graph-authored paths, or the old executor checkpoint snapshot state.
The durable task-attempt lifecycle remains the later target for replay,
retry/defer, process-restart recovery, and batching; this slice only replaces
the embedded-runtime keep-alive checkpoint semantics with canonical residency
facts.

Next embedded-runtime checkpoint/residency slice:
1. Allowed write set:
   `crates/pantograph-embedded-runtime/src/lib_tests/session_checkpoint_capacity_tests.rs`,
   `crates/pantograph-embedded-runtime/src/lib_tests/session_checkpoint_recovery_tests.rs`,
   `crates/pantograph-embedded-runtime/src/lib_tests/README.md` if test
   ownership notes need updating, and this plan. Source files are allowed only
   if focused tests prove a missing narrow residency fact boundary:
   `crates/pantograph-embedded-runtime/src/workflow_execution_session_execution.rs`,
   `crates/pantograph-embedded-runtime/src/embedded_workflow_host.rs`,
   `crates/pantograph-embedded-runtime/src/embedded_runtime_lifecycle.rs`, and
   `crates/pantograph-embedded-runtime/src/lib.rs`.
2. Migrate capacity/recovery tests away from
   `EmbeddedRuntime.session_executions`, preserved node-memory counts, carried
   KV handles, and executor checkpoint summaries.
3. Assert only canonical residency facts: resident/warm/checkpointed-unloaded
   state where still exposed, reservation/load-proof identity, release/reclaim
   behavior, reuse eligibility, and typed diagnostics when restore or runtime
   readiness fails.
4. If the current embedded-runtime API cannot expose enough canonical
   residency facts without `session_executions`, add the smallest backend-owned
   fact projection and document it; do not restore executor snapshots as a
   compatibility shim.
5. Run
   `cargo test -p pantograph-embedded-runtime session_checkpoint_capacity --lib`,
   `cargo test -p pantograph-embedded-runtime session_checkpoint_recovery --lib`,
   `cargo check -p pantograph-embedded-runtime`, and
   `cargo fmt -p pantograph-embedded-runtime -- --check`. Rerun
   `cargo test -p pantograph-embedded-runtime --lib` only after focused
   verification passes to update the remaining-failure inventory.

Re-plan triggers: stop if implementation would require preserving
`EmbeddedRuntime.session_executions` as the source of truth, carrying workflow
inputs or node memory between runs, using graph/frontend paths to infer
residency, moving durable process-restart replay into this narrow slice, or
editing outside the allowed checkpoint/residency write set.

2026-06-07 keep-alive checkpoint capacity/recovery test migration slice:
completed. Smallest useful vertical slice: migrate the six stale
embedded-runtime checkpoint capacity/recovery tests from executor checkpoint
snapshots and node-memory carry-forward to runtime/model residency facts and
per-run input ownership. Allowed files touched:
`crates/pantograph-embedded-runtime/src/lib_tests/session_checkpoint_capacity_tests.rs`,
`crates/pantograph-embedded-runtime/src/lib_tests/session_checkpoint_recovery_tests.rs`,
`crates/pantograph-embedded-runtime/src/lib_tests/README.md`, and this plan.
No source projection was required.

No-fallback/no-legacy confirmation: the migrated tests no longer read
`EmbeddedRuntime.session_executions` for checkpoint state, no longer record or
assert synthetic KV node-memory snapshots, no longer expect empty-input resume
to replay prior workflow inputs, and no longer expect executor pointer
identity as the reuse proof. The tests now assert fresh input on every
successful run, typed missing-input or scheduler rejection diagnostics on
invalid resumes, and runtime-registry reservation facts where the current
capacity path exposes stable residency. Where capacity-managed paths may
legitimately release or retain a single shared keep-alive reservation, the
tests assert the bounded canonical shape instead of forcing owner churn.

Verification passed:
`cargo test -p pantograph-embedded-runtime session_checkpoint_capacity --lib`,
`cargo test -p pantograph-embedded-runtime session_checkpoint_recovery --lib`,
`cargo check -p pantograph-embedded-runtime`, and
`cargo fmt -p pantograph-embedded-runtime -- --check`. Broader verification
`cargo test -p pantograph-embedded-runtime --lib` now reports 396 passing and
10 failing tests; the six keep-alive checkpoint failures are removed.

Discovered issues and follow-ups:
- The focused test runs now warn that the old shared
  `synthetic_kv_node_memory_snapshot` fixture/import is unused after this
  migration. Cleanup is deferred because it touches shared test harness files
  outside the approved checkpoint slice.
- Runtime-residency reservation lifecycle is not yet fully normalized across
  capacity-managed paths: some paths release immediately and some retain a
  shared keep-alive reservation. The tests intentionally assert only the
  canonical bounded shape. Full lifecycle normalization remains part of the
  later durable task-attempt/runtime residency work.
- Remaining embedded-runtime broad failures are now the six
  `node_execution_ledger::tests::node_execution_workflow_sink_*` retained
  artifact expectations, two retired data-graph Python sidecar reconciliation
  expectations, and two runtime-preflight restart classification expectations.

2026-06-07 runtime-preflight restart diagnostic test migration slice:
completed. Smallest useful vertical slice: migrate the two stale
embedded-runtime runtime-preflight restart tests so the preflight checks still
assert selected-runtime readiness failures, while the subsequent session-run
assertions accept the canonical executable graph validation boundary. Allowed
files touched:
`crates/pantograph-embedded-runtime/src/lib_tests.rs`,
`crates/pantograph-embedded-runtime/src/lib_tests/runtime_preflight_tests.rs`,
and this plan.

No-fallback/no-legacy confirmation: the slice does not revive the old
`llm-inference` fixture ports, does not bypass stale-graph executable
validation, does not reorder session execution so runtime readiness can mask
invalid graph structure, and does not restore request-scoped runtime
execution. The tests now assert typed `MissingTargetInput` and
`MissingSourceOutput` diagnostics for stale `llm-1.prompt` and `llm-1.text`
ports when the runtime-required fixture is executed.

Verification passed:
`cargo test -p pantograph-embedded-runtime runtime_preflight --lib`,
`cargo fmt -p pantograph-embedded-runtime -- --check`,
`cargo check -p pantograph-embedded-runtime`,
`cargo test -p pantograph-embedded-runtime --lib` with 406 passing tests, and
`git diff --check` for the slice files.

Discovered follow-ups:
- The full embedded-runtime lib run still reports the existing unused
  `synthetic_kv_node_memory_snapshot` fixture/import warning after the
  keep-alive checkpoint migration. Clean this up in a dedicated shared test
  harness slice.
- `cargo check` and `cargo test` still report the existing unused
  `WorkflowSchedulerStartedRuntimeTaskSupervisor` abort helper warning in
  workflow-service. Keep that cleanup with the worker cancellation/shutdown
  lifecycle work.

Option 3 durable lifecycle sequence after Option 2 validation:
1. Promote runtime-branch events into the durable scheduler task-attempt
   lifecycle with explicit non-terminal running/dispatching/deferred states,
   instead of treating the runtime-branch event repository as a bridge.
2. Add duplicate-dispatch guard, replay/restart semantics, retry/defer
   decisions, and durable supervisor lifecycle facts before enabling runtime
   recovery from process restart.
3. Add batching/coalescing support for compatible runtime tasks across
   simultaneous workflow runs while preserving backend-owned active-run facts
   and typed diagnostics. This must proceed in three thin phases: first define
   the backend-owned batch eligibility contract; then add durable grouped
   claim/lease behavior without coalesced runtime execution; then add the
   worker-owned coalesced runtime-host execution path with per-run output,
   cancellation, timeout, and diagnostic fan-out.
4. Replace remaining bridge-specific worker rehydration with canonical durable
   task-attempt facts, then remove bridge-only state and tests in a dedicated
   cleanup slice.
5. Verify with boundary invariant tests that direct request paths cannot
   execute runtime work, worker-owned runtime attempts are replayable and
   cancelable, and no frontend/Tauri adapter owns runtime scheduling policy.

Rejected immediate paths: passing a full in-memory execution envelope in the
worker command would preserve request-scoped execution ownership; duplicating
all run facts into the runtime-branch event now would create a second mutable
truth before the full task-attempt lifecycle is ready; moving dispatch by
calling the direct helper from the request path would violate the no-fallback/
no-legacy rule. This re-plan follows the standards by keeping business logic in
backend workflow-service ownership, preserving composition-root lifecycle
ownership, using typed diagnostics instead of fallback behavior, and keeping
the next source work in validated thin vertical slices.

2026-06-07 runtime-branch batching/coalescing re-plan decision: use Option 1,
then Option 2, then Option 3. True coalesced runtime execution is the target,
but implementing it before a backend-owned compatibility contract would push
scheduling policy into ad hoc worker code and would make batch membership hard
to test at the boundary. The current `batching_key` is provisional workflow
shape metadata and must not be treated as sufficient proof that tasks can share
one runtime execution. Batch eligibility must be derived from canonical backend
facts: model/artifact identity, backend/runtime family, device/load target,
runtime instance residency requirements, memory estimate and reservation,
context/input shape required by the runtime operation, task operation type,
and cancellation/timeout compatibility. Eligibility failures must return typed
diagnostics, not fall back to independent request-scoped runtime execution.

Batching Option 1, contract first: define a pure workflow-service-owned batch
eligibility contract and focused tests. The contract must explain why two
runtime-branch tasks are compatible or incompatible using typed diagnostics
and canonical facts only. It must not execute runtime work, claim groups,
create frontend/Tauri policy, use graph path inference, or preserve legacy
runtime execution through a compatibility shim.

Batching Option 2, durable grouped claim without coalesced execution: add a
runtime-branch repository boundary that can claim a compatible group under one
backend-owned lease while still dispatching each member independently. This
validates group ownership, duplicate-dispatch protection, retry/defer behavior,
replay after expired leases, and cancellation/timeout fan-out without adding a
new runtime-host batch API in the same slice.

Batching Option 3, worker-owned coalesced runtime execution: once Options 1
and 2 validate, add a task-execution-worker path that claims a compatible
group, calls a composition-root-owned runtime-host batch operation once, and
fans out per-run outputs, terminal facts, cancellation outcomes, timeouts, and
typed diagnostics. The worker may reuse a kept-alive model/runtime instance
for compatible tasks, but must never carry forward workflow inputs or mutable
per-run context from the workflow that originally loaded the model.

Rejected batching paths: do not implement true coalescing before the
eligibility contract; do not use `batching_key` alone as the compatibility
source of truth; do not batch by frontend/Tauri request shape, graph node
labels, or in-memory responder state; do not create request-scoped runtime
instances as fallback for incompatible tasks; and do not add a runtime-host
batch API in the same slice as durable grouped claiming. Verification for this
sequence must start with eligibility boundary tests, then repository grouped
claim/replay tests, then worker fan-out tests and the existing
runtime-branch/workflow-service checks.

2026-06-07 runtime-branch batch eligibility contract slice: completed
Batching Option 1. Smallest useful vertical slice: add a
workflow-service-owned runtime-branch batch eligibility profile to durable
runtime-branch task-event requests/records, plus a pure compatibility boundary
that compares canonical facts and returns typed incompatibility diagnostics.
Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`,
`crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
`crates/pantograph-workflow-service/src/workflow/runtime_branch_rehydration.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`,
and this plan.

No-fallback/no-legacy confirmation: this slice does not execute runtime work,
does not claim grouped events, does not add a runtime-host batch API, does not
use frontend/Tauri state, does not infer compatibility from graph paths, and
does not treat the provisional `batching_key` as sufficient compatibility
authority. Missing eligibility profiles fail closed through typed diagnostics.
Production admission/recovery records currently persist `batch_eligibility:
None` because the canonical model/artifact/backend/device/runtime residency
and memory facts are not yet wired into this boundary; the next batching slice
must populate those facts from backend-owned planning/resource facts before
durable grouped claiming can proceed.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`,
`cargo test -p pantograph-workflow-service runtime_branch --lib`,
`cargo check -p pantograph-workflow-service`, and
`cargo fmt -p pantograph-workflow-service -- --check`.

2026-06-07 runtime-branch batch eligibility fact-population re-plan trigger:
stop before populating `batch_eligibility` during runtime-branch event
admission/recovery. The current admission/recovery boundaries receive the
scheduler task graph, output targets, and timeout. The task graph carries
model reference, task type, runtime/device constraints, trait settings, and
estimate hints, but it does not carry all canonical facts required by the
batch eligibility contract: selected backend, resolved device/load target when
policy is automatic, runtime residency key, reservation/resource-fit facts,
and selected runtime instance/load state. Filling these fields from
`batching_key`, graph node shape, request parameters, or partial scheduler
intent would violate the no-fallback/no-legacy rule by inventing scheduling
authority outside the backend planning/resource facts.

Re-plan options:
1. Thread selected technical-fit/resource facts into runtime-branch event
   admission from the composition-root runtime owner. This preserves the
   current event shape but requires a clear per-task mapping from selected
   runtime/backend/device/resource facts to each runtime-branch task event.
2. Promote the needed selected backend, resolved device/load target,
   residency, and resource-fit facts into the scheduler task graph or a
   sibling backend-owned task-planning snapshot before runtime-branch events
   are persisted. This keeps admission deterministic and replayable, but
   changes the shared planning contract and requires boundary tests.
3. Defer batching fact population until the full durable scheduler
   task-attempt lifecycle owns selected runtime/resource facts, then derive
   runtime-branch batch eligibility from those canonical attempt facts. This
   best matches the target architecture, but blocks Batching Option 2 grouped
   claims until task-attempt fact promotion is done.

Do not proceed to durable grouped claims until one option is selected and the
plan identifies the owner of selected runtime/backend/device/residency/resource
facts.

2026-06-07 runtime-branch batch eligibility fact-population decision: use
Option 3. Do not populate `batch_eligibility` from runtime-branch
admission/recovery, scheduler graph shape, provisional `batching_key`, request
parameters, or partial scheduler intent. The selected runtime/backend/device,
load-target, residency, reservation, resource-fit, loaded-runtime memory, and
runtime-instance facts must become canonical durable scheduler task-attempt
facts first; runtime-branch batch eligibility can then be derived from those
attempt facts. Until that promotion is complete, `batch_eligibility: None` is
the correct fail-closed state and durable grouped claims remain blocked.

Option 3 implementation sequence:
1. Define the durable runtime task-attempt fact contract in workflow-service
   without changing execution. The contract must name the selected model and
   artifact, backend/runtime family, resolved device/load target, residency
   key, loaded-runtime memory estimate, resource-fit/reservation facts,
   operation/context shape, cancellation mode, timeout policy, workflow run,
   scheduler task, and task-attempt identity. Add pure validation and boundary
   tests proving missing selected facts fail closed with typed diagnostics.
2. Persist task-attempt selected runtime/resource facts only at the backend
   boundary that already owns the selected dispatch/resource decision. Do not
   synthesize facts from graph nodes, request DTOs, frontend/Tauri state, or
   worker command envelopes. If the current code has no such boundary for all
   required facts, stop and re-plan that ownership boundary before adding
   persistence.
3. Rehydrate worker dispatch from durable task-attempt facts, not from
   bridge-only runtime-branch records, while preserving the existing
   worker-owned dispatch path and typed stale/missing-fact diagnostics.
4. Derive runtime-branch `batch_eligibility` from task-attempt facts or remove
   the bridge field once task-attempt facts become the only compatibility
   source. The derivation must never carry forward workflow inputs from the
   run that originally loaded a kept-alive model/runtime instance.
5. Add durable grouped claiming over compatible task-attempt facts before
   adding runtime-host coalescing. This validates group lease ownership,
   duplicate-dispatch protection, retry/defer behavior, replay after expired
   leases, and per-run cancellation/timeout diagnostics without a runtime-host
   batch API in the same slice.
6. Add worker-owned coalesced runtime execution only after grouped claims pass:
   claim a compatible task-attempt group, reuse a kept-alive compatible
   runtime/model instance when available, call the composition-root-owned
   runtime-host batch operation once, and fan out per-run outputs, terminal
   facts, cancellations, timeouts, and typed diagnostics.
7. Remove bridge-specific runtime-branch state/tests that duplicate
   task-attempt lifecycle facts after the task-attempt path owns claim,
   dispatching/running, defer/retry, replay, cancellation, terminal facts, and
   batching eligibility.

Next thin slice: implement only step 1, the durable runtime task-attempt fact
contract and pure tests, in workflow-service. Allowed source scope should stay
inside workflow-service task-attempt/runtime-branch lifecycle modules plus
this plan. No execution, repository grouped claim, runtime-host batch API,
frontend/Tauri policy, generated DTO, lockfile, saved workflow fixture, or
bridge cleanup may be included in that slice.

Re-plan triggers for Option 3:
- The selected dispatch/resource decision is not represented at a stable
  backend boundary that can own all required task-attempt facts.
- Persisting task-attempt facts would require changing shared scheduler,
  runtime-registry, Pumas, generated DTO, lockfile, or saved workflow fixture
  contracts in the same slice.
- A runtime-host batch API is needed before grouped task-attempt claims are
  durable and tested.
- Any implementation path would preserve request-scoped runtime execution,
  use compatibility shims, or treat old graph/backend/runtime/device/frontend
  execution methods as fallback.

2026-06-07 durable runtime task-attempt fact contract slice: completed
Option 3 implementation sequence step 1. Smallest useful vertical slice:
define an internal workflow-service runtime task-attempt fact contract and
pure validation boundary without changing execution, persistence, dispatch,
group claiming, runtime-host APIs, frontend/Tauri policy, generated DTOs,
lockfiles, saved workflow fixtures, or bridge cleanup. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_task_attempt_fact.rs`,
`crates/pantograph-workflow-service/src/workflow.rs`, and this plan.

The contract records the selected model/artifact, runtime id/variant, backend,
runtime family, resolved device/load target, residency key,
loaded-runtime memory estimate, resource-fit facts, reservation facts,
operation/context shape, cancellation mode, timeout policy, workflow run,
scheduler task, task-attempt identity, generation, and recorded timestamp.
Validation fails closed with typed diagnostics for missing selected facts,
invalid attempt identity, zero memory estimate, non-fit resource state without
diagnostics, invalid reservation facts, and invalid timeout policy.

No-fallback/no-legacy confirmation: this slice does not populate
`batch_eligibility`, does not synthesize selected facts from graph shape,
`batching_key`, request parameters, frontend/Tauri state, worker command
envelopes, or partial scheduler intent, and does not preserve or reintroduce
request-scoped runtime execution. The new module is internal and marked
dead-code allowed because later slices must attach it only at the backend
boundary that owns selected dispatch/resource facts.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_task_attempt_fact --lib`,
`cargo test -p pantograph-workflow-service runtime_branch --lib`,
`cargo check -p pantograph-workflow-service`, and
`cargo fmt -p pantograph-workflow-service -- --check`.

Remaining follow-up: implement Option 3 sequence step 2 by locating or
creating the stable backend boundary that owns the selected dispatch/resource
decision and persists these task-attempt facts. Stop and re-plan if that
requires shared scheduler/runtime-registry/Pumas/generated/fixture contract
changes in the same slice.

2026-06-07 durable runtime task-attempt fact persistence re-plan trigger:
stop before persisting task-attempt facts. Inspection found that
workflow-service runtime dispatch selection is the closest backend boundary:
`WorkflowRuntimeDispatchCandidateFact` and scheduler `SchedulerDispatchDecision`
own selected runtime id, runtime variant, selected device ids, selected model
ref, reservations, and resource-fit assessment. They do not own every fact
required by the new task-attempt contract: selected backend id, runtime family,
resolved load target, runtime residency key, loaded-runtime memory estimate,
and selected runtime-instance/load-state facts. Persisting now would require
synthesizing those values from runtime id strings, graph shape, provisional
`batching_key`, request DTOs, trait settings, or partial scheduler estimates,
which would violate the backend-owned source-of-truth and no-fallback rules.

Re-plan options for Option 3 step 2:
1. Extend the scheduler dispatch candidate/decision contract to carry the
   missing selected backend, runtime family, load target, residency key,
   loaded-runtime memory estimate, and runtime-instance/load-state facts. This
   makes the selected dispatch boundary the durable source for task-attempt
   persistence, but it changes a shared scheduler contract and needs serial
   contract tests before workflow-service persistence.
2. Add a workflow-service-owned selected runtime/resource fact sidecar beside
   dispatch candidate collection, produced by the same backend provider that
   owns canonical runtime/resource facts and keyed by candidate id. Scheduler
   selection would still rank candidates, and workflow-service would persist
   the sidecar for the selected candidate. This avoids changing scheduler DTOs
   immediately, but requires a strict invariant that the sidecar and scheduler
   candidate share identity and cannot diverge.
3. Move selected backend/load-target/residency/memory fact production earlier
   into the runtime dispatch candidate provider contract, then build both the
   scheduler candidate and the runtime task-attempt fact from one validated
   workflow-service candidate fact. This keeps policy/mechanism separated and
   may be the simplest long-term workflow-service boundary, but it expands the
   existing candidate fact contract and tests before any persistence slice.

Do not continue by persisting partial task-attempt facts. The next plan update
must choose one option and define the exact owner of missing backend,
load-target, residency, memory, and runtime-instance/load-state facts before
source implementation resumes.

2026-06-07 durable runtime task-attempt fact persistence decision: use
Option 3. `WorkflowRuntimeDispatchCandidateFact` is the workflow-service-owned
source of selected runtime/resource evidence. Expand that contract so it owns
the missing selected backend id, runtime family, resolved load target,
runtime residency key, loaded-runtime memory estimate, and selected
runtime-instance/load-state facts before projection into scheduler
`SchedulerDispatchCandidate`. Scheduler remains responsible for ranking and
selection mechanics, but it must not be the owner of rich runtime/resource
evidence. Do not add a sidecar keyed by candidate id, because that would
create drift risk between the sidecar and scheduler candidate projection.

Option 3 step 2 implementation sequence:
1. Extend `WorkflowRuntimeDispatchCandidateFact` and its validation tests with
   the missing selected runtime/resource evidence fields. This is a
   workflow-service contract slice only: no persistence, no scheduler DTO
   changes, no runtime-host batch API, no grouped claims, no generated DTOs,
   no lockfiles, and no saved workflow fixture edits.
2. Add a pure projection from validated workflow-service candidate fact into
   the existing scheduler `SchedulerDispatchCandidate`, preserving scheduler
   selection behavior. Tests must prove the richer fields are validated before
   projection and cannot be derived from runtime id strings, graph node shape,
   `batching_key`, request DTOs, or trait settings.
3. Add a pure projection from the selected validated workflow-service
   candidate fact plus scheduler attempt identity into
   `WorkflowRuntimeTaskAttemptFactRecord`. Tests must prove missing backend,
   load-target, residency, memory, and runtime-instance facts return typed
   diagnostics before persistence is possible.
4. Persist task-attempt facts at the existing backend dispatch-selection
   boundary only after the selected candidate can be traced back to one
   validated workflow-service candidate fact. If scheduler selection returns a
   selected candidate that cannot be matched exactly to a validated
   workflow-service candidate fact, fail closed with a typed diagnostic.
5. Continue to worker rehydration from durable task-attempt facts only after
   persistence is validated; grouped claims and runtime-host coalescing remain
   blocked until then.

No-fallback/no-legacy confirmation for this decision: selected backend,
load-target, residency, memory, and runtime-instance facts must come from the
backend candidate provider contract, not from frontend/Tauri state, graph
shape, request parameters, `batching_key`, runtime id parsing, trait settings,
or partial estimate hints. Existing scheduler dispatch DTOs remain unchanged
until a later explicit shared-contract re-plan requires otherwise.

Next thin slice: implement sequence step 1 only in
`crates/pantograph-workflow-service/src/workflow/runtime_dispatch_selection.rs`
and this plan. Stop before persistence or projection into task-attempt facts
if the provider cannot supply the required fields without changing
scheduler/runtime-registry/Pumas/generated/fixture contracts in the same
slice.

2026-06-07 runtime dispatch candidate evidence re-plan trigger: stop before
extending `WorkflowRuntimeDispatchCandidateFact` in source. Inspection found
that the embedded resource-backed candidate provider can supply selected
backend key, selected model ref, selected runtime id, selected device id,
reservations, resource fit, runtime registry status, and optional runtime
instance id from existing backend-owned facts. It does not yet have
first-class canonical fields for runtime family, resolved load target,
runtime residency key, loaded-runtime memory estimate, or runtime load-state
semantics that are independent of runtime id parsing, graph/request shape,
`batching_key`, trait settings, or partial scheduler estimate hints. Adding
mandatory workflow-service contract fields now would either break the
embedded provider or force it to synthesize facts, violating the
backend-owned source-of-truth and no-fallback rules.

Re-plan options:
1. Extend runtime-registry registration/snapshot/capability projection with
   canonical runtime family and load-state facts, and extend the runtime
   resource fact source with selected load target, residency key, and
   loaded-runtime memory estimate facts. This keeps the workflow-service
   candidate contract strict and makes the embedded provider able to satisfy
   it from backend-owned facts. It requires serial contract work in
   runtime-registry and embedded-runtime before the workflow-service contract
   slice resumes.
2. Add an embedded-runtime-owned runtime dispatch evidence projection layer
   that consumes the existing runtime-registry snapshot, Pumas logical size
   facts, resource reservations, and runtime observation ledger facts, then
   emits one validated evidence object per candidate. This avoids changing
   scheduler DTOs and keeps derivation policy out of workflow-service, but it
   still needs explicit typed diagnostics for every missing canonical field
   and must not infer runtime family/load target/residency from arbitrary
   strings.
3. Defer task-attempt fact persistence until the worker/runtime lifecycle
   model lands, then make durable worker/runtime instance records the source
   of runtime family, load target, residency key, memory estimate, and
   load-state facts. This aligns with the final worker architecture, but it
   blocks the current task-attempt persistence path and leaves grouped claims
   and runtime-host coalescing blocked longer.

Do not implement `WorkflowRuntimeDispatchCandidateFact` rich evidence fields
until one option is selected and the chosen source owns every required fact
with fail-closed diagnostics. The next plan update must choose the
intermediate owner for runtime family, load target, residency key,
loaded-runtime memory estimate, and load-state facts.

2026-06-07 runtime dispatch candidate evidence decision: use Option 2 as the
intermediate path and keep Option 3 as the final architecture target.
Embedded-runtime will own a runtime dispatch evidence projection layer that
combines existing backend-owned facts into one validated evidence object per
candidate before workflow-service rich candidate facts are extended. The
projection may consume runtime-registry capability snapshots, Pumas logical
size facts, runtime resource reservation facts, and runtime observation ledger
facts. It must not infer runtime family, load target, residency key,
loaded-runtime memory estimate, load state, or runtime instance identity from
runtime id strings, graph/request shape, `batching_key`, trait settings, or
partial scheduler estimate hints.

Option 2 implementation sequence:
1. Add an embedded-runtime internal runtime dispatch evidence contract with
   typed diagnostics and validation tests. The contract must represent the
   selected backend id/key, runtime family, resolved load target,
   runtime-residency key, loaded-runtime memory estimate, load state,
   optional runtime instance id, selected model/artifact identity, selected
   device id, reservation facts, and resource-fit facts. Missing required
   evidence must return typed diagnostics rather than producing a candidate.
   Allowed files for this slice: a new embedded-runtime evidence module,
   `crates/pantograph-embedded-runtime/src/lib.rs` module wiring if needed,
   focused embedded-runtime tests, and this plan. No workflow-service
   contract, scheduler DTO, generated DTO, Pumas, lockfile, or saved workflow
   fixture edits.
2. Wire the embedded runtime dispatch candidate provider to build evidence
   from already-canonical source facts before constructing
   `WorkflowRuntimeDispatchCandidateFact`. If the evidence layer cannot
   provide a required fact, the provider must emit typed scheduler diagnostics
   and fail closed with no fallback candidate. Tests must cover missing
   Pumas logical size, missing runtime instance/load-state facts, missing
   load target, missing residency key, and missing memory estimate evidence.
3. Extend `WorkflowRuntimeDispatchCandidateFact` only after the embedded
   evidence layer is validated, then populate the richer workflow-service
   candidate fields from the evidence object. Scheduler
   `SchedulerDispatchCandidate` remains unchanged in this sequence; projection
   into scheduler candidates must ignore the richer evidence only after it has
   been validated at the workflow-service boundary.
4. Add the selected validated workflow-service candidate to
   `WorkflowRuntimeTaskAttemptFactRecord` projection and persistence at the
   backend dispatch-selection boundary. If scheduler selection cannot be
   matched exactly to one validated workflow-service candidate fact, fail
   closed with a typed diagnostic.
5. When durable worker/runtime lifecycle records land, migrate the evidence
   source for runtime family, load target, residency key, memory estimate,
   runtime instance id, and load state from the embedded projection layer to
   those durable records. The evidence contract remains the boundary so the
   migration replaces the source owner without changing scheduler selection
   semantics.

No-fallback/no-legacy confirmation for this decision: embedded-runtime owns
the intermediate projection because it is the composition boundary that can
see runtime-registry facts, Pumas logical size facts, resource reservations,
and observation ledger facts together. Workflow-service remains strict and
does not synthesize missing evidence. Scheduler remains ranking/selection
mechanism and does not become the owner of rich runtime evidence. The final
worker/runtime lifecycle architecture must replace the intermediate evidence
source rather than preserve parallel legacy paths.

Next thin slice: implement Option 2 step 1 only. Stop and re-plan again if the
evidence contract cannot be defined without changing shared scheduler,
workflow-service, runtime-registry, Pumas, generated, lockfile, or saved
workflow fixture contracts in the same slice.

2026-06-07 runtime dispatch evidence contract slice: completed Option 2 step
1. Smallest useful vertical slice: add an internal embedded-runtime
`runtime_dispatch_evidence` boundary contract that validates one complete
runtime dispatch evidence record before any workflow-service rich candidate
fact or scheduler candidate is produced. Allowed files touched:
`crates/pantograph-embedded-runtime/src/runtime_dispatch_evidence.rs`,
`crates/pantograph-embedded-runtime/src/lib.rs`, and this plan. No
workflow-service contract, scheduler DTO, runtime-registry, Pumas, generated,
lockfile, saved workflow fixture, provider wiring, persistence, grouped-claim,
or runtime-host coalescing files were changed.

No-fallback/no-legacy confirmation: the evidence contract requires canonical
selected backend key, runtime family, resolved load target, residency key,
loaded-runtime memory estimate, load state, selected model/artifact identity,
selected device, reservation facts, and resource-fit facts. Missing or invalid
facts return typed diagnostics. The contract rejects path-carrying model refs,
zero memory estimates, missing load state, missing loaded-runtime instance id,
reservations for unselected devices, and non-fit resource facts without
diagnostics. It does not derive evidence from runtime id strings,
graph/request shape, `batching_key`, trait settings, or partial scheduler
estimate hints.

Verification passed:
`cargo fmt -p pantograph-embedded-runtime -- --check`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_evidence --lib`,
and `cargo check -p pantograph-embedded-runtime`.

Remaining follow-up: implement Option 2 step 2 by wiring the embedded runtime
dispatch candidate provider to build this evidence from already-canonical
source facts before constructing workflow-service candidate facts. Stop and
re-plan if provider wiring needs shared workflow-service, scheduler,
runtime-registry, Pumas, generated, lockfile, or saved workflow fixture
contract changes in the same slice.

2026-06-07 runtime dispatch evidence provider gate slice: completed the
fail-closed portion of Option 2 step 2. Smallest useful vertical slice: route
the embedded runtime dispatch candidate provider through
`runtime_dispatch_evidence` before it may reserve resources or construct a
workflow-service runtime dispatch candidate fact. Allowed files touched:
`crates/pantograph-embedded-runtime/src/runtime_dispatch_candidate_provider.rs`
and this plan. No workflow-service contract, scheduler DTO,
runtime-registry, Pumas, generated, lockfile, saved workflow fixture,
task-attempt persistence, grouped-claim, or runtime-host coalescing files were
changed.

No-fallback/no-legacy confirmation: the provider now emits typed scheduler
diagnostics and produces no candidate when canonical runtime dispatch evidence
is incomplete. It does not keep the old resource-backed candidate path alive
without evidence validation, and it does not acquire runtime-registry
reservations before evidence validation succeeds. Runtime-registry status is
projected only into the evidence load-state enum; missing runtime family,
resolved load target, residency key, and loaded-runtime memory estimate remain
fail-closed diagnostics rather than being derived from runtime ids,
graph/request shape, `batching_key`, trait settings, or scheduler estimate
hints.

Verification passed:
`cargo fmt -p pantograph-embedded-runtime -- --check`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_candidate_provider --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_evidence --lib`,
and `cargo check -p pantograph-embedded-runtime`.

Remaining follow-up: complete Option 2 step 2 by adding canonical embedded
runtime evidence sources for runtime family, resolved load target, residency
key, and loaded-runtime memory estimate. Do not proceed to
`WorkflowRuntimeDispatchCandidateFact` rich fields or task-attempt persistence
until the provider can produce one complete validated evidence record from
backend-owned facts. Stop and re-plan if that requires changing shared
workflow-service, scheduler, runtime-registry, Pumas, generated, lockfile, or
saved workflow fixture contracts in the same slice.

2026-06-07 runtime dispatch loaded-memory evidence slice: completed the
loaded-runtime memory estimate portion of Option 2 step 2. Smallest useful
vertical slice: expose a narrow embedded-runtime estimator helper for the
loaded model-residency memory component derived from Pumas logical size facts,
then carry that estimate into runtime dispatch candidate drafts and the
evidence request. Allowed files touched:
`crates/pantograph-embedded-runtime/src/inference_resource_estimator.rs`,
`crates/pantograph-embedded-runtime/src/runtime_dispatch_candidate_provider.rs`,
and this plan. No workflow-service contract, scheduler DTO,
runtime-registry, Pumas, generated, lockfile, saved workflow fixture,
task-attempt persistence, grouped-claim, or runtime-host coalescing files were
changed.

No-fallback/no-legacy confirmation: loaded-runtime memory evidence now comes
from Pumas logical size through the existing conservative estimator. Weak or
insufficient logical-size facts produce no memory estimate, so the evidence
contract continues to fail closed instead of synthesizing memory from
scheduler estimate hints, runtime ids, request shape, `batching_key`, trait
settings, or sentinel values. Candidate production remains blocked until
runtime family, resolved load target, and residency key have canonical
embedded-runtime evidence sources.

Verification passed:
`cargo fmt -p pantograph-embedded-runtime -- --check`,
`cargo test -p pantograph-embedded-runtime inference_resource_estimator --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_candidate_provider --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_evidence --lib`,
and `cargo check -p pantograph-embedded-runtime`.

Remaining follow-up: complete Option 2 step 2 by adding canonical sources for
runtime family, resolved load target, and runtime residency key. Do not
proceed to workflow-service rich candidate facts or task-attempt persistence
until the provider can produce one complete validated evidence record from
backend-owned facts.

2026-06-07 runtime dispatch canonical source re-plan trigger: stop before
filling runtime family, resolved load target, and runtime residency key in the
embedded provider. Inspection found that the current source snapshot owns Pumas
package facts and runtime-registry capability facts, but not every remaining
required dispatch evidence fact. Pumas package facts expose backend hints,
logical size, task evidence, and diffusers family evidence; they do not expose
the resolved artifact load target. Runtime-registry capability facts expose
runtime id, backend keys, status, instance id, loaded model ids, reservations,
and admission-budget presence; they do not expose a canonical runtime family or
runtime residency key. The existing runtime-host load-target resolver can ask
the Pumas owner API for a load target, but it runs at the later runtime-host
execution boundary from a scheduler dispatch decision and currently derives
the Pumas consumer runtime family from runtime id/variant. Reusing that as
dispatch evidence would violate the no-fallback rule because it would infer
canonical facts from runtime naming and a later execution request rather than
from a first-class dispatch evidence source.

Re-plan options:
1. Extend the embedded runtime dispatch source snapshot/refresher to own
   runtime dispatch load-target evidence before candidate construction. The
   async refresher would call the Pumas owner load-target API from the
   composition-root-owned Pumas access, validate the response, and store a
   dispatch-safe load-target summary in the source snapshot. This keeps the
   provider synchronous and fail-closed, but it still needs a separate
   canonical owner for runtime family and residency key instead of deriving
   them from runtime ids.
2. Extend runtime-registry registration/snapshot capability facts with
   first-class runtime family and residency identity. The embedded provider
   would then combine runtime-registry runtime family/residency facts with the
   Pumas load-target snapshot from Option 1. This keeps lifecycle/runtime
   identity owned by the runtime registry and keeps Pumas responsible only for
   artifact/load-target facts, but it requires serial runtime-registry
   contract work before provider candidates can be emitted.
3. Move this remaining evidence source directly to the durable worker/runtime
   lifecycle records now. Worker/runtime lifecycle records would become the
   canonical owner for runtime family, load target, residency key, memory
   estimate, load state, and runtime instance id before workflow-service rich
   task-attempt facts are populated. This best matches the final architecture,
   but it is a larger sequence and would block the intermediate embedded
   provider candidate path until lifecycle records exist.

Do not continue by filling blank evidence fields, using selected backend key as
runtime family, reusing device id as load target, constructing residency keys
from model/runtime/device strings, or deriving any fact from `batching_key`,
runtime id, scheduler estimate hints, graph/request shape, trait settings, or
runtime-host execution requests. The next plan decision must choose the owner
for the remaining canonical dispatch evidence fields before source code
implementation resumes.

2026-06-07 runtime dispatch canonical source re-plan decision: use Options 1
and 2 as the intermediate implementation path, and keep Option 3 as the final
architecture target. Runtime-registry will become the canonical source for
runtime family and runtime residency identity in registration/snapshot/
capability facts. Embedded-runtime will become the canonical dispatch source
for Pumas owner API load-target evidence during the async source-refresh phase,
before synchronous candidate construction. The provider will only combine these
first-class facts; it must not synthesize runtime family, load target, or
residency key from runtime ids, selected backend keys, device ids, task
batching keys, scheduler hints, graph/request shape, trait settings, or later
runtime-host execution requests. When durable worker/runtime lifecycle records
land, migrate the evidence source for runtime family, load target, residency
key, memory estimate, runtime instance id, and load state to those records
without changing scheduler ranking semantics.

Intermediate implementation sequence:
1. Extend runtime-registry runtime registration, snapshot, and embedded-runtime
   capability projection with explicit `runtime_family` and
   `runtime_residency_key` facts. The registration boundary must reject blank
   values when these fields are required for dispatch-capable runtimes, and the
   snapshot must expose them as data fields, not derive them from `runtime_id`.
   Allowed files for this serial contract slice: runtime-registry source/tests,
   embedded-runtime capability projection/tests, and this plan. No
   workflow-service contract, scheduler DTO, Pumas, generated, lockfile, or
   saved workflow fixture edits.
2. Add an embedded-runtime Pumas load-target evidence source for dispatch
   source refresh. The async refresher must call the Pumas owner load-target
   API with path-free model/artifact facts and the registry-owned runtime
   family, validate a ready response, strip host-only paths from candidate
   evidence, and store a dispatch-safe load-target identifier/summary in the
   source snapshot. Missing, unavailable, stale, or path-carrying load-target
   facts must produce typed source diagnostics and no candidate. Allowed files:
   embedded-runtime load-target evidence/source snapshot/provider tests and
   this plan unless a later inspection proves a shared contract change is
   required.
3. Wire `runtime_dispatch_candidate_provider` to build a complete
   `RuntimeDispatchEvidenceRecord` from the runtime-registry family/residency
   facts, Pumas load-target source facts, Pumas logical-size memory estimate,
   runtime load state/instance facts, selected model/device facts, and resource
   facts. Candidate production remains blocked until the evidence record
   validates. Tests must cover missing runtime family, missing residency key,
   missing load target, weak logical-size memory facts, loaded/busy runtime
   without instance id, and successful complete evidence.
4. Only after step 3 validates, extend `WorkflowRuntimeDispatchCandidateFact`
   with richer evidence fields and project them from the validated evidence
   record. Scheduler candidate DTOs remain ranking-only; workflow-service
   remains strict and does not synthesize evidence.
5. Continue task-attempt fact persistence and grouped-claim/coalescing work
   only after workflow-service candidate facts carry complete validated
   evidence.
6. Final migration: replace the intermediate embedded projection sources with
   durable worker/runtime lifecycle records as the owner for runtime family,
   load target, residency key, memory estimate, runtime instance id, and load
   state. The migration must delete or make unreachable the intermediate source
   ownership path rather than preserve parallel legacy behavior.

Next thin slice: implement sequence step 1 only. Stop and re-plan if adding
first-class runtime family and residency identity cannot be done without
changing workflow-service contracts, scheduler DTOs, Pumas, generated files,
lockfiles, or saved workflow fixtures in the same slice.

2026-06-07 runtime-registry dispatch identity slice: completed intermediate
sequence step 1. Smallest useful vertical slice: add runtime-registry-owned
`RuntimeDispatchIdentity` with explicit runtime family and runtime residency
key, carry those facts through runtime registration records and runtime
snapshots, and require embedded-runtime dispatch capability projection to
receive both fields before it emits dispatch-capable runtime facts. Allowed
files touched:
`crates/pantograph-runtime-registry/src/state.rs`,
`crates/pantograph-runtime-registry/src/lib.rs`,
`crates/pantograph-runtime-registry/src/snapshot.rs`,
`crates/pantograph-runtime-registry/src/registry_queries.rs`,
`crates/pantograph-runtime-registry/src/lib_tests/lifecycle.rs`,
`crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
`crates/pantograph-embedded-runtime/src/runtime_dispatch_capability_facts.rs`,
`crates/pantograph-embedded-runtime/src/runtime_dispatch_source_snapshot.rs`,
`crates/pantograph-embedded-runtime/src/runtime_dispatch_candidate_provider.rs`,
`crates/pantograph-embedded-runtime/src/inference_interface_facts_provider.rs`,
and this plan. No workflow-service contract, scheduler DTO, Pumas, generated,
lockfile, or saved workflow fixture files were changed.

No-fallback/no-legacy confirmation: runtime family and residency key are now
first-class registry facts, not values derived from runtime id, selected
backend key, device id, `batching_key`, graph/request shape, scheduler hints,
trait settings, or runtime-host execution requests. Dispatch capability
projection fails closed with a typed diagnostic when a runtime has backend keys
but no dispatch identity. Existing runtime registrations without dispatch
identity remain valid registry records for non-dispatch/lifecycle use; they do
not project as dispatch candidates until identity is supplied.

Verification passed:
`cargo fmt -p pantograph-runtime-registry -p pantograph-embedded-runtime -- --check`,
`cargo test -p pantograph-runtime-registry lifecycle --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_capability_facts --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_candidate_provider --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_source_snapshot --lib`,
`cargo test -p pantograph-embedded-runtime inference_interface_facts_provider --lib`,
`cargo check -p pantograph-runtime-registry`, and
`cargo check -p pantograph-embedded-runtime`.

2026-06-07 embedded-runtime dispatch load-target facts slice: completed
intermediate sequence step 2. Smallest useful vertical slice: add a
Pumas-owner-backed load-target fact source to embedded-runtime dispatch source
refresh, project only dispatch-safe load-target identifiers into the snapshot,
and keep synchronous candidate construction blocked until the later complete
evidence step. Allowed files touched:
`crates/pantograph-embedded-runtime/src/runtime_dispatch_load_target_facts.rs`,
`crates/pantograph-embedded-runtime/src/lib.rs`,
`crates/pantograph-embedded-runtime/src/runtime_dispatch_source_snapshot.rs`,
`crates/pantograph-embedded-runtime/src/runtime_dispatch_candidate_provider.rs`,
`crates/pantograph-embedded-runtime/src/workflow_service_composition.rs`, and
this plan. The composition-root wiring was a necessary embedded-runtime
production source hookup for the new source owner; no workflow-service
contract, scheduler DTO, Pumas API/contract, generated, lockfile, or saved
workflow fixture files were changed.

No-fallback/no-legacy confirmation: Pumas owner API load-target resolution is
the only load-target source for this dispatch evidence path. The request to
Pumas uses a path-free model ref plus the registry-owned runtime family; the
projection validates a ready response, strips host-only selected artifact path
data, never stores the local filesystem load path, and emits typed diagnostics
for missing owner access, unsupported access role, missing runtime family,
lookup failure, unavailable load targets, ready responses without targets,
empty load paths, and stripped path facts. The provider still fails closed
until intermediate sequence step 3 builds a complete validated
`RuntimeDispatchEvidenceRecord`.

Verification passed:
`cargo fmt -p pantograph-embedded-runtime -- --check`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_load_target_facts --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_source_snapshot --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_candidate_provider --lib`,
`cargo test -p pantograph-embedded-runtime workflow_service_composition --lib`,
`cargo check -p pantograph-embedded-runtime`, and `git diff --check`.

2026-06-07 embedded-runtime complete dispatch evidence slice: completed
intermediate sequence step 3 inside the embedded-runtime provider boundary.
Smallest useful vertical slice: wire `runtime_dispatch_candidate_provider` to
compose candidate drafts from runtime-registry family/residency facts, Pumas
load-target facts, Pumas logical-size memory estimates, runtime load-state and
instance facts, selected model/device facts, and runtime-registry resource
reservation facts, then validate a complete `RuntimeDispatchEvidenceRecord`
before emitting a scheduler candidate. Allowed files touched:
`crates/pantograph-embedded-runtime/src/runtime_dispatch_candidate_provider.rs`,
`crates/pantograph-embedded-runtime/src/runtime_dispatch_resource_facts.rs`,
and this plan. No workflow-service contract, scheduler DTO, Pumas
API/contract, generated, lockfile, or saved workflow fixture files were
changed.

No-fallback/no-legacy confirmation: the provider now fails closed when any
canonical dispatch evidence is missing or invalid. It does not infer runtime
family, resolved load target, residency key, memory estimate, runtime
instance, selected device, or resource fit from runtime ids, backend keys,
device ids, scheduler hints, graph/request shape, trait settings, or
runtime-host requests. Pumas load-target diagnostics are projected as typed
provider diagnostics; missing load-target facts for a backend-compatible
runtime family block that candidate. A pre-reservation evidence guard prevents
resource acquisition for malformed selected/load-state evidence, and a
post-reservation full evidence record validates actual reservation and
resource-fit facts before candidate emission. If post-reservation validation
fails, the registry reservation is released instead of preserving a leaked
partial reservation.

Verification passed:
`cargo fmt -p pantograph-embedded-runtime -- --check`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_candidate_provider --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_evidence --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_resource_facts --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_load_target_facts --lib`,
`cargo check -p pantograph-embedded-runtime`, and `git diff --check`.

2026-06-07 workflow-service dispatch evidence fact contract slice: completed
intermediate sequence step 4. Smallest useful vertical slice: extend
`WorkflowRuntimeDispatchCandidateFact` with explicit validated evidence fields
for selected backend key, runtime family, resolved load target, runtime
residency key, loaded-runtime memory estimate, runtime load state, and runtime
instance id; validate those fields in workflow-service before mapping to
scheduler candidates; and project them from the embedded-runtime provider's
validated `RuntimeDispatchEvidenceRecord`. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_dispatch_selection.rs`,
`crates/pantograph-workflow-service/src/workflow.rs`,
`crates/pantograph-embedded-runtime/src/runtime_dispatch_candidate_provider.rs`,
and this plan. No scheduler DTO, Pumas API/contract, generated, lockfile, or
saved workflow fixture files were changed.

No-fallback/no-legacy confirmation: workflow-service now rejects candidate
fact bundles that omit canonical dispatch evidence or try to represent loaded
or busy runtime state without an instance id. Scheduler candidate DTOs remain
ranking-only: the richer evidence is validated at the workflow-service
provider contract and is not synthesized by workflow-service or scheduler
selection from runtime ids, backend keys, device ids, graph/request shape,
trait settings, or runtime-host requests. The embedded provider projects the
workflow-service fact fields from the already-validated evidence record.

Verification passed:
`cargo fmt -p pantograph-workflow-service -p pantograph-embedded-runtime -- --check`,
`cargo test -p pantograph-workflow-service runtime_dispatch_selection --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_candidate_provider --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_evidence --lib`,
`cargo check -p pantograph-workflow-service`,
`cargo check -p pantograph-embedded-runtime`, and `git diff --check`.

Remaining follow-up: implement intermediate sequence step 5 by continuing
task-attempt fact persistence and grouped-claim/coalescing work now that
workflow-service candidate facts carry complete validated evidence. Stop and
re-plan if durable persistence requires changing scheduler DTOs, Pumas
contracts, generated files, lockfiles, saved workflow fixtures, or if the
durable owner for these facts conflicts with the planned final worker/runtime
lifecycle migration.

2026-06-07 task-attempt fact persistence re-plan trigger: step 5 cannot
continue safely without choosing the ownership path for selected candidate
evidence retention. The step-4 contract now validates rich
`WorkflowRuntimeDispatchCandidateFact` evidence before mapping to scheduler
candidates, but `WorkflowRuntimeDispatchCandidateSet` currently discards the
rich fact records and exposes only scheduler-ranking candidates plus
diagnostics. After scheduler selection, `SelectedRuntimeTaskDispatch` carries
only the scheduler handoff, reservation lease id, and candidate id. The
existing durable `runtime_task_attempt_fact` contract already has the fields
needed for selected backend, runtime family, load target, residency key,
memory, resource fit, reservations, operation type, context shape, and
cancellation mode, but there is no canonical path from selected scheduler
candidate id back to the validated workflow-service candidate fact when
runtime-host dispatch starts. Do not proceed by adding evidence to scheduler
DTOs, re-deriving it from scheduler candidates, runtime ids, graph/request
shape, runtime-host requests, or embedded provider internals.

2026-06-07 task-attempt fact persistence re-plan decision: use Option 1 now,
the workflow-service candidate evidence context. Workflow-service will retain
the validated `WorkflowRuntimeDispatchCandidateFactBundle` as an
application-layer dispatch context while building the scheduler ranking
request. After scheduler selection returns a candidate id, workflow-service
will resolve that id against the retained validated fact context and use that
selected fact to build the durable `WorkflowRuntimeTaskAttemptFactRecord`.
Scheduler DTOs remain ranking-only, embedded-runtime remains the source owner
for producing validated provider facts, and workflow-service remains the owner
for retaining selected candidate evidence long enough to persist the
task-attempt fact. This preserves the standards requirements for single state
ownership, backend-owned durable truth, explicit contracts, and boundary
invariant testing.

Rejected immediate paths:
1. Persist all candidate facts before scheduler selection. This remains a
   valid later durability/replay upgrade, but it is too broad for the next
   thin slice because it needs store shape, cleanup, stale-attempt handling,
   and replay semantics before implementation.
2. Move selected evidence ownership directly into runtime-branch task events.
   This remains the likely long-term worker/runtime lifecycle direction, but
   it is too broad until the immediate workflow-service context proves
   selected candidate evidence can be resolved without scheduler DTO changes
   or fallback derivation.

Option 1 implementation sequence:
1. Change the workflow-service dispatch selection boundary so
   `WorkflowRuntimeDispatchCandidateSet` can retain the validated fact bundle
   or an equivalent workflow-service-owned context alongside the scheduler
   candidates. The context must be keyed by candidate id and must not expose
   policy to scheduler DTOs.
2. Update runtime dispatch selection to return selected scheduler dispatch
   plus the matching validated workflow-service candidate fact. Missing,
   duplicate, or stale selected fact lookups must return typed diagnostics and
   fail closed before runtime-host dispatch.
3. Build `WorkflowRuntimeTaskAttemptFactRecord` from the selected validated
   candidate fact, started task attempt identity, operation/context/cancel
   facts, and current timestamp. Persist or record it at the workflow-service
   task-attempt boundary before runtime-host dispatch starts.
4. Add boundary invariant tests that prove scheduler candidates do not carry
   rich evidence, workflow-service does not synthesize selected evidence after
   selection, and missing selected fact context blocks dispatch.
5. After this validates, plan the later durable candidate evidence store or
   runtime-branch event ownership upgrade for restart/replay/coalescing.

Next thin slice: implement Option 1 sequence step 1 only in workflow-service
runtime dispatch selection contracts/tests and the embedded-runtime provider
projection if required by the changed constructor shape. Stop and re-plan if
the slice requires scheduler DTO changes, Pumas contracts, generated files,
lockfiles, saved workflow fixtures, or any derivation from graph/request/
runtime-host state.

2026-06-07 workflow-service selected-candidate evidence context slice:
completed Option 1 sequence step 1. Smallest useful vertical slice: extend
`WorkflowRuntimeDispatchCandidateSet` with an opaque
`WorkflowRuntimeDispatchCandidateEvidenceContext` keyed by scheduler candidate
id, retain validated `WorkflowRuntimeDispatchCandidateFact` records when
building scheduler-ranking candidates, and add a diagnostics-only constructor
for fail-closed provider paths that intentionally carry no candidate evidence.
Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_dispatch_selection.rs`,
`crates/pantograph-workflow-service/src/workflow.rs`,
`crates/pantograph-embedded-runtime/src/runtime_dispatch_candidate_provider.rs`,
`crates/pantograph-embedded-runtime/src/lib_tests/workflow_run_execution_tests.rs`,
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
and this plan.

No-fallback/no-legacy confirmation: scheduler DTOs remain ranking-only and do
not receive backend key, runtime-family, load-target, runtime-instance,
memory-estimate, or resource-fit evidence beyond their existing scheduler
ranking fields. Workflow-service now retains the validated provider facts
beside the scheduler projection instead of re-deriving selected evidence from
runtime ids, backend keys, graph/request shape, runtime-host requests, Pumas
contracts, or scheduler hints. Empty/fail-closed providers carry an explicit
empty evidence context rather than a synthesized fact.

Verification passed:
`cargo fmt -p pantograph-workflow-service -p pantograph-embedded-runtime -- --check`,
`cargo test -p pantograph-workflow-service runtime_dispatch_selection --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_candidate_provider --lib`,
`cargo check -p pantograph-workflow-service`,
`cargo check -p pantograph-embedded-runtime`, and `git diff --check`.

Next thin slice: implement Option 1 sequence step 2 by threading the retained
candidate evidence context through runtime dispatch selection so a selected
scheduler candidate id resolves to exactly one validated
`WorkflowRuntimeDispatchCandidateFact` before runtime-host dispatch. Missing,
duplicate, or stale selected fact lookups must return typed diagnostics and
fail closed. Stop and re-plan if this requires scheduler DTO changes, Pumas
contracts, generated files, lockfiles, saved workflow fixtures, or evidence
synthesis from graph/request/runtime-host state.

2026-06-07 workflow-service selected-candidate fact resolution slice:
completed Option 1 sequence step 2. Smallest useful vertical slice: change the
workflow-service runtime dispatch selection request builder to return the
validated scheduler selection request plus the retained candidate evidence
context, expose the selected scheduler candidate id from
`SelectedRuntimeTaskDispatch`, and resolve that id against the retained
`WorkflowRuntimeDispatchCandidateEvidenceContext` before runtime-host dispatch.
Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_dispatch_selection.rs`,
`crates/pantograph-workflow-service/src/workflow.rs`,
`crates/pantograph-workflow-service/src/scheduler/task_orchestrator.rs`,
`crates/pantograph-workflow-service/src/workflow/session_scheduler_runner.rs`,
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
`crates/pantograph-embedded-runtime/src/lib_tests/workflow_run_execution_tests.rs`,
and this plan.

No-fallback/no-legacy confirmation: scheduler selection still receives only
the validated `SchedulerDispatchSelectionRequest`; the retained evidence
context is not added to scheduler DTOs and is not reconstructed from runtime
ids, backend keys, graph/request shape, runtime-host requests, Pumas
contracts, or scheduler hints. Missing selected candidate ids and stale
candidate ids now return typed workflow-service diagnostics and fail closed
before runtime-host dispatch. Duplicate candidate ids remain blocked at the
validated candidate fact bundle boundary before the context is built. Test
providers that previously emitted scheduler-only candidates were moved to the
validated candidate fact bundle path instead of preserving a scheduler-only
test fallback.

Verification passed:
`cargo fmt -p pantograph-workflow-service -p pantograph-embedded-runtime -- --check`,
`cargo test -p pantograph-workflow-service runtime_dispatch_selection --lib`,
`cargo test -p pantograph-embedded-runtime runtime_dispatch_candidate_provider --lib`,
`cargo check -p pantograph-workflow-service`, and
`cargo check -p pantograph-embedded-runtime`.

Additional verification/deviation:
`cargo test -p pantograph-workflow-service session_execution --lib` now fixes
the selected-evidence failures from scheduler-only test providers and reports
36 passing tests plus one remaining failure:
`workflow_execution_session_bootstrap_recovery_applies_dependency_readiness_resume_plan`
fails with `no due runtime branch task event is available for workflow run`.
The same test fails when run alone, so this is recorded as a discovered
runtime-branch bootstrap recovery issue outside the selected-evidence slice.

Next thin slice: implement Option 1 sequence step 3 by constructing the
`WorkflowRuntimeTaskAttemptFactRecord` from the selected validated candidate
fact, the started task attempt identity, operation/context/cancel facts, and
current timestamp before runtime-host dispatch starts. Stop and re-plan if
this requires scheduler DTO changes, Pumas contracts, generated files,
lockfiles, saved workflow fixtures, or any synthesis of selected runtime
evidence from graph/request/runtime-host state.

2026-06-07 task-attempt fact construction re-plan trigger: stop before
implementing Option 1 sequence step 3. Inspection found that the selected
`WorkflowRuntimeDispatchCandidateFact` now provides canonical runtime/model/
resource evidence, and the started scheduler task provides attempt id and
start timestamp, but `WorkflowRuntimeTaskAttemptFactRecord` also requires
attempt generation, operation type, context shape key, cancellation mode, and
timeout. Those fields are not owned by the scheduler dispatch selection path:
attempt generation, timeout, operation type, context shape key, and
cancellation mode are currently represented in runtime-branch task events and
active-run/session state. Building a task-attempt fact in
`session_scheduler_runner` now would require synthesizing operation/context/
cancellation fields from graph/request/runtime-host state or reaching across
runtime-branch ownership boundaries. Do not proceed until ownership is
re-planned so the task-attempt fact builder consumes canonical facts from the
same backend-owned source that owns those fields.

2026-06-07 task-attempt fact construction re-plan decision: use Option 1,
the runtime-branch/worker-owned construction path. Runtime-branch task
events/claims remain the canonical owner for attempt generation, operation
type, context shape key, cancellation mode, and timeout. Runtime dispatch
selection remains the canonical owner for selected candidate runtime/model/
resource evidence. Scheduler attempt lifecycle remains the canonical owner for
attempt id and start timestamp. The worker/runtime-branch dispatch boundary
will compose those already-validated backend facts into
`WorkflowRuntimeTaskAttemptFactRecord` before runtime-host dispatch starts.

No-fallback/no-legacy confirmation: do not move operation/context/cancellation
facts into scheduler DTOs, do not derive them from graph/request/runtime-host
state in `session_scheduler_runner`, do not copy Pumas contracts into this
boundary, and do not preserve request-scoped dispatch as a hidden legacy owner.
If any required field is missing from the runtime-branch/worker rehydrated
context, dispatch must fail closed with typed diagnostics.

Option 1 implementation sequence:
1. Add a runtime-branch/worker task-attempt source context contract that
   groups the existing runtime-branch profile fields (`attempt_generation`,
   `operation_type`, `context_shape_key`, `cancellation_mode`, `timeout_ms`)
   with the selected candidate evidence resolved from
   `WorkflowRuntimeDispatchCandidateEvidenceContext`. The contract must be
   workflow-service-owned and validated at construction.
2. Persist or carry the selected candidate evidence into the runtime-branch
   dispatch/claim boundary as backend-owned evidence, not as scheduler DTO
   expansion and not as a runtime-host request field. Missing or stale
   selected candidate evidence must fail closed before a worker dispatches.
3. Extend runtime-branch worker rehydration to return the validated
   task-attempt source context plus scheduler attempt id/start timestamp.
   Rehydration must reject mismatched workflow run, task id, attempt
   generation, timeout, or selected candidate evidence.
4. Build `WorkflowRuntimeTaskAttemptFactRecord` at the worker dispatch
   boundary from the rehydrated source context, selected candidate fact,
   scheduler attempt identity, and current recorded timestamp before
   runtime-host dispatch starts.
5. Add boundary invariant tests proving the worker does not synthesize
   operation/context/cancellation facts, scheduler DTOs remain ranking-only,
   and missing selected candidate evidence or runtime-branch profile fields
   block dispatch.
6. After this validates, remove or narrow any temporary session-runner
   selected-evidence checks that are superseded by durable worker ownership,
   without restoring request-scoped runtime execution.

Next thin slice: implement Option 1 sequence step 1 by introducing the
workflow-service runtime-branch/worker task-attempt source context contract
and focused validation tests only. Allowed files should be limited to
workflow-service runtime-branch/task-attempt contract modules, their focused
tests, and this plan. Stop and re-plan if the slice requires scheduler DTO
changes, Pumas contracts, generated files, lockfiles, saved workflow fixtures,
or any synthesis from graph/request/runtime-host state.

2026-06-08 runtime-branch worker task-attempt source context contract slice:
completed Option 1 sequence step 1. Smallest useful vertical slice: add
`WorkflowRuntimeTaskAttemptSourceContextRequest` and
`WorkflowRuntimeTaskAttemptSourceContext` in the workflow-service
task-attempt fact contract module, grouping runtime-branch profile facts
(`attempt_generation`, `operation_type`, `context_shape_key`,
`cancellation_mode`, `timeout_ms`) with the selected candidate fact. The
contract validates identity fields, attempt generation, timeout, runtime-branch
profile required fields, and cross-fact consistency between the runtime-branch
profile and selected candidate evidence for model artifact, runtime family,
backend id, load target, residency key, and loaded-runtime memory estimate.
Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_task_attempt_fact.rs`
and this plan.

No-fallback/no-legacy confirmation: the new contract does not change scheduler
DTOs, Pumas contracts, runtime-host request fields, generated files, lockfiles,
or saved workflow fixtures. It does not synthesize operation/context/
cancellation facts from graph/request/runtime-host state. Missing or mismatched
runtime-branch profile and selected candidate facts return typed diagnostics
and fail closed.

Verification passed:
`cargo fmt -p pantograph-workflow-service -- --check`,
`cargo test -p pantograph-workflow-service runtime_task_attempt_fact --lib`,
`cargo check -p pantograph-workflow-service`, and `git diff --check`.

Next thin slice: implement Option 1 sequence step 2 by carrying selected
candidate evidence into the runtime-branch dispatch/claim boundary as
backend-owned evidence. Stop and re-plan if this requires scheduler DTO
changes, Pumas contracts, generated files, lockfiles, saved workflow fixtures,
runtime-host request fields, or synthesis from graph/request/runtime-host
state.

2026-06-08 runtime-branch selected candidate evidence boundary slice:
completed Option 1 sequence step 2. Smallest useful vertical slice: extend the
workflow-service runtime-branch task-event contract to carry a selected
`WorkflowRuntimeDispatchCandidateFact` under the current task-event claim,
validate that fact with the canonical dispatch-candidate fact bundle validator,
and reject selected evidence that does not match the runtime-branch batch
eligibility profile. The in-memory runtime-branch repository now persists the
selected candidate fact through a claim-bound method, and reclaims clear any
previous selected evidence so stale runtime/model/resource facts cannot carry
forward into a replayed attempt. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`
and this plan.

No-fallback/no-legacy confirmation: the slice keeps selected evidence as
backend-owned runtime dispatch evidence at the runtime-branch claim boundary.
It does not expand scheduler DTOs, change Pumas contracts, add runtime-host
request fields, touch generated/lock/workflow fixture files, or synthesize
selected runtime/model/resource facts from graph/request/runtime-host state.
Missing, stale, invalid, or batch-profile-mismatched selected evidence returns
typed runtime-branch diagnostics and fails closed before worker dispatch.

Verification passed:
`cargo fmt -p pantograph-workflow-service`,
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`,
`cargo check -p pantograph-workflow-service`, and `git diff --check`.

Next thin slice: implement Option 1 sequence step 3 by extending
runtime-branch worker rehydration to return the validated task-attempt source
context plus scheduler attempt id/start timestamp. Allowed files should be
limited to workflow-service runtime-branch rehydration, task-attempt source
context plumbing, focused tests, and this plan. Stop and re-plan if this
requires scheduler DTO changes, Pumas contracts, generated files, lockfiles,
saved workflow fixtures, runtime-host request fields, request-scoped fallback
execution, or synthesis from graph/request/runtime-host state.

2026-06-08 runtime-branch task-attempt source rehydration slice: completed
Option 1 sequence step 3. Smallest useful vertical slice: extend
runtime-branch rehydration so a claimed/dispatching runtime-branch task event
returns a validated `WorkflowRuntimeTaskAttemptSourceContext`, the active
scheduler task-attempt id, and the active scheduler task-attempt start
timestamp. Rehydration now reads scheduler attempt facts from the canonical
session store, rejects missing or mismatched active attempt facts, rejects
missing selected candidate evidence, rejects selected candidate resource-fit or
reservation scope mismatches, and maps the new fail-closed diagnostics through
the task-execution worker. The task-execution owner now asserts the returned
source context aligns with the runtime-branch command before continuing to the
existing dispatch-boundary runner. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_rehydration.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_owner.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`,
and this plan.

No-fallback/no-legacy confirmation: this slice does not change scheduler DTOs,
Pumas contracts, runtime-host request fields, generated files, lockfiles, or
saved workflow fixtures. It does not synthesize attempt identity, selected
runtime/model/resource evidence, operation type, context shape, cancellation
mode, or timeout from graph/request/runtime-host state. Missing or mismatched
canonical runtime-branch, selected-candidate, or scheduler-attempt facts return
typed rehydration diagnostics and fail closed before runtime-host dispatch.

Verification passed:
`cargo fmt -p pantograph-workflow-service`,
`cargo test -p pantograph-workflow-service runtime_branch_rehydration --lib`,
`cargo check -p pantograph-workflow-service`, and `git diff --check`.

Next thin slice: implement Option 1 sequence step 4 by building
`WorkflowRuntimeTaskAttemptFactRecord` at the worker dispatch boundary from
the rehydrated source context, selected candidate fact, scheduler attempt id,
scheduler attempt start timestamp, and current record timestamp before
runtime-host dispatch starts. Allowed files should be limited to
workflow-service task-attempt fact construction/plumbing, focused tests,
worker/owner boundary integration, and this plan. Stop and re-plan if this
requires scheduler DTO changes, Pumas contracts, generated files, lockfiles,
saved workflow fixtures, runtime-host request fields, request-scoped fallback
execution, or synthesis from graph/request/runtime-host state.

2026-06-08 task-attempt fact construction re-plan trigger: stop before
implementing Option 1 sequence step 4. Worker rehydration now correctly
requires selected candidate evidence, but the current worker path has no
canonical source for that evidence at the point where it rehydrates the
runtime-branch task event. The selected candidate fact is produced later in
`session_scheduler_runner` during runtime dispatch selection. Building the
task-attempt fact in `WorkflowTaskExecutionOwner` before the existing dispatch
runner would therefore require either synthesizing selected runtime/model/
resource facts, preserving request-scoped fallback behavior, or changing the
boundary so the selected dispatch fact is produced/carried before fact
construction.

Discovered issue: the previous selected-evidence boundary slices established
validated storage and rehydration contracts, but did not yet define the
production handoff that records selected candidate evidence onto the
runtime-branch task event before worker rehydration. Existing worker tests that
exercise runtime-branch dispatch fail closed at rehydration without that
handoff. Do not proceed by weakening rehydration, making selected evidence
optional, copying scheduler DTO fields, or deriving selected facts from graph,
request, active-run, or runtime-host state.

Re-plan options must decide where canonical runtime dispatch selection occurs
relative to runtime-branch worker claim/rehydration and task-attempt fact
construction.

2026-06-09 task-attempt fact construction re-plan decision: use
worker-owned pre-dispatch selection now, and record durable dispatch
assignment as the later target architecture. This supersedes the stale next
thin slice above that attempted to build `WorkflowRuntimeTaskAttemptFactRecord`
before selected candidate evidence exists at the worker boundary.

The immediate path keeps runtime-branch events/claims as the canonical owner
for attempt generation, operation type, context shape key, cancellation mode,
and timeout; keeps scheduler attempt facts as the canonical owner for
scheduler task-attempt id and start timestamp; and moves canonical runtime
dispatch selection to a worker-callable backend boundary that runs after the
worker has claimed and marked the event `Dispatching`, but before worker
rehydration and task-attempt fact construction. The selected
`WorkflowRuntimeDispatchCandidateFact` must be recorded on the claimed
runtime-branch task event under the current claim generation before rehydration
continues.

Worker-owned pre-dispatch selection sequence:
1. Extract the existing runtime dispatch selection/selected-candidate fact
   production from `session_scheduler_runner` into a workflow-service backend
   dispatch-selection boundary that the task-execution worker can call. The
   boundary must return a validated `WorkflowRuntimeDispatchCandidateFact`,
   reservation binding evidence, and typed diagnostics. It must not change
   scheduler DTOs, Pumas contracts, runtime-host request fields, generated
   DTOs, lockfiles, or saved workflow fixtures, and it must not synthesize
   evidence from graph/request/runtime-host state.
2. After the worker claims a due runtime-branch task event and marks it
   `Dispatching`, invoke the backend dispatch-selection boundary and record
   the selected candidate fact on that runtime-branch task event through the
   repository under the current claim generation. Missing, stale, mismatched,
   or non-recordable selected evidence must fail closed before rehydration.
3. Rehydrate the runtime-branch task event from durable backend facts after
   selected evidence is recorded. Rehydration must continue to validate
   runtime-branch profile fields, selected candidate evidence, scheduler
   attempt identity/start timestamp, reservation scope, workflow run scope,
   task id, and attempt generation.
4. Build `WorkflowRuntimeTaskAttemptFactRecord` at the worker dispatch
   boundary from the rehydrated source context, selected candidate fact,
   scheduler task-attempt id, scheduler attempt start timestamp, current
   recorded timestamp, operation/context/cancellation facts, timeout, and
   attempt generation before runtime-host dispatch starts. If the existing
   task-attempt fact contract needs the scheduler attempt start timestamp as an
   explicit field rather than a validation-only input, stop and re-plan that
   contract change as a serial shared-contract slice.
5. Narrow `session_scheduler_runner` for the runtime-branch worker path so it
   consumes the preselected dispatch fact and cannot re-run runtime dispatch
   selection. Any remaining non-worker path must be explicitly canonical and
   must not preserve request-scoped runtime execution as a hidden fallback.
6. Add vertical-slice tests covering worker claim -> `Dispatching` ->
   pre-dispatch selection -> selected-evidence recording -> rehydration ->
   task-attempt fact build/persist -> runner consumption of preselected
   evidence. Boundary tests must prove missing selected evidence blocks worker
   dispatch, stale claims cannot record selected evidence, selected evidence
   cannot be synthesized, and the runner cannot reselect for the worker-owned
   runtime-branch path.

Later durable dispatch assignment target: after the worker-owned
pre-dispatch path is validated end to end, re-plan the bridge fields into a
first-class durable dispatch assignment record. That record should own the
runtime-branch event id, workflow run id, scheduler task-attempt identity and
start timestamp, selected candidate fact, reservation bindings, source context,
claim generation, and dispatch lifecycle state. It is also the right future
home for grouping/coalescing metadata used to batch compatible runtime tasks
from simultaneous workflow runs without creating a new inference runtime per
workflow. This later slice must replace the bridge-style selected evidence on
runtime-branch events rather than preserve parallel legacy dispatch owners.

No-fallback/no-legacy confirmation for this decision: selected runtime/model/
resource evidence remains mandatory before worker dispatch. The worker path
must not weaken rehydration, make selected evidence optional, copy scheduler
DTO fields into task-attempt facts, derive facts from graph/request/runtime-host
state, or call request-scoped runtime execution when canonical selection fails.
Canonical planning failures return typed diagnostics and stop dispatch.

Next thin slice: implement only worker-owned pre-dispatch selection sequence
step 1. Allowed files should be limited to extracting/introducing the
workflow-service backend dispatch-selection boundary, focused tests for that
boundary, any minimal module wiring required to compile, and this plan. Do not
wire the worker, change scheduler DTOs, change runtime-host request fields,
change Pumas contracts, edit generated/lock/workflow fixture files, or build
`WorkflowRuntimeTaskAttemptFactRecord` in the same slice.

2026-06-09 worker-owned pre-dispatch selection boundary slice: completed
sequence step 1. Smallest useful vertical slice: introduced
`WorkflowRuntimeDispatchSelectionBoundary` in workflow-service as the
backend-owned dispatch-selection handoff. The boundary prepares dispatch
selection from the ready runtime task, readiness proof, runtime dispatch source
refresher, and canonical candidate provider; then resolves a started scheduler
runtime task into both `SelectedRuntimeTaskDispatch` reservation-binding
evidence and the retained `WorkflowRuntimeDispatchCandidateFact`. The existing
session scheduler runner now delegates its inline refresh/candidate/request/
selected-fact plumbing to the boundary while preserving its current scheduler
start order and terminal mutation handling. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_dispatch_selection.rs`,
`crates/pantograph-workflow-service/src/workflow/session_scheduler_runner.rs`,
`crates/pantograph-workflow-service/src/workflow.rs`, and this plan.

No-fallback/no-legacy confirmation: this slice does not wire the worker, does
not change scheduler DTOs, Pumas contracts, runtime-host request fields,
generated files, lockfiles, or saved workflow fixtures, and does not build
`WorkflowRuntimeTaskAttemptFactRecord`. The boundary preserves fail-closed
typed diagnostics for source refresh, candidate collection, scheduler request
construction, scheduler selection, and selected-candidate evidence resolution;
it does not synthesize selected runtime/model/resource facts from graph,
request, runtime-host, or active-run state.

Focused tests added:
`preselection_boundary_prepares_request_with_retained_candidate_evidence`,
`preselection_boundary_returns_typed_refresh_diagnostic`, and
`preselection_boundary_returns_typed_candidate_collection_diagnostic`.

Verification passed:
`cargo fmt -p pantograph-workflow-service`,
`cargo test -p pantograph-workflow-service runtime_dispatch_selection --lib`,
and `cargo check -p pantograph-workflow-service`.

Discovered issue retained for the next slice:
`cargo test -p pantograph-workflow-service
workflow_execution_session_dispatches_ready_runtime_task_through_scheduler_selection --lib`
still fails with `runtime branch rehydration diagnostic (TaskAttemptUnavailable):
runtime branch scheduler task has no active attempt fact`. This is the
previously known worker-ordering gap: the runtime-branch worker marks
`Dispatching` and rehydrates before starting the scheduler runtime task and
recording selected evidence. Do not fix this by weakening rehydration or
making selected evidence/attempt facts optional.

Next thin slice: implement worker-owned pre-dispatch selection sequence step
2. After the worker claims a due runtime-branch task event and marks it
`Dispatching`, it must start or otherwise bind the active scheduler runtime
task attempt through the canonical scheduler lifecycle, invoke
`WorkflowRuntimeDispatchSelectionBoundary`, and record the selected candidate
fact on the runtime-branch task event under the current claim generation before
rehydration. Allowed files should be limited to worker dispatch ordering,
runtime-branch selected-evidence repository use, focused tests, and this plan.
Stop and re-plan if this requires scheduler DTO changes, Pumas contract
changes, generated/lock/workflow fixture edits, runtime-host request-field
changes, request-scoped fallback execution, or synthetic selected evidence.

2026-06-09 worker-owned pre-dispatch selection step 2 re-plan trigger: stop
before wiring the worker. A thin attempt to invoke
`WorkflowRuntimeDispatchSelectionBoundary` immediately after worker claim and
`Dispatching` proved that the worker can claim a runtime-branch event while
the active scheduler task is still `AwaitingInputs` and before a persisted
runtime dispatch readiness proof exists. The old `session_scheduler_runner`
materialized external inputs, advanced scheduler progress, admitted runtime
dependency readiness, and persisted the readiness proof after rehydration. The
new selected-evidence invariant requires selected candidate evidence before
rehydration, so step 2 cannot be implemented as a simple worker insertion
point without moving ownership of scheduler progress/readiness preparation.

Re-plan options must decide which backend boundary owns pre-dispatch scheduler
progress before selected candidate evidence is recorded:
1. Extract a worker-callable pre-dispatch readiness/progress boundary from
   `session_scheduler_runner`. The worker would claim the runtime-branch
   event, mark `Dispatching`, invoke this boundary to materialize inputs,
   advance non-runtime scheduler progress, admit runtime dependency readiness,
   and persist the readiness proof, then invoke
   `WorkflowRuntimeDispatchSelectionBoundary`, start the scheduler runtime
   task attempt, bind the reservation, and record selected evidence before
   rehydration. This aligns best with the selected worker-owned path, but it
   is a larger slice than step 2 and must preserve typed diagnostics for
   dependency-readiness pending/deferred outcomes.
2. Move the runtime-branch task-event claim until after the existing runner has
   progressed the active run to `Ready`. This is smaller, but it keeps the
   runner as the owner of request-scoped pre-dispatch progress and delays the
   worker-owned architecture. It risks preserving legacy runner ownership
   unless the plan explicitly narrows it immediately afterward.
3. Introduce the durable dispatch assignment record now and make scheduler
   progress/readiness preparation part of assignment creation. This matches the
   later target architecture, but it expands the current bridge slice into a
   durable contract and persistence change that must be planned as serial
   shared-contract work.

Do not proceed by making rehydration tolerate missing selected evidence,
making scheduler readiness proof optional, synthesizing readiness from graph or
request state, or allowing runtime-host/request-scoped dispatch to run when
canonical pre-dispatch preparation has not produced a ready scheduler task and
persisted proof.

2026-06-09 pre-dispatch readiness/progress ownership decision: use Option 1.
Extract a worker-callable backend boundary from `session_scheduler_runner` for
pre-dispatch scheduler preparation, then resume worker-owned dispatch
selection once that boundary can produce a ready runtime scheduler task and
persisted runtime dispatch readiness proof before rehydration.

Selected implementation sequence:
1. Extract the existing scheduler pre-dispatch preparation from
   `session_scheduler_runner` into a workflow-service-owned boundary that can
   be called by the task-execution worker. The boundary owns materializing
   external inputs, running scheduler progress for already-admitted
   non-runtime/source tasks, retrying deferred runtime dependency readiness,
   admitting runtime dependency readiness, persisting the runtime dispatch
   readiness proof, and returning typed pending/deferred diagnostics when
   readiness is incomplete. This boundary must not invoke runtime-host
   dispatch, perform runtime dispatch selection, or record selected candidate
   evidence.
2. Have the worker call that boundary after claiming the runtime-branch event
   and marking it `Dispatching`, before invoking
   `WorkflowRuntimeDispatchSelectionBoundary`. If the boundary returns
   dependency-readiness pending/deferred diagnostics, the worker must persist a
   durable deferred runtime-branch event with explicit retry timing rather than
   falling back to request-scoped runner execution.
3. Once the boundary returns a ready runtime task and persisted readiness
   proof, invoke `WorkflowRuntimeDispatchSelectionBoundary`, start the
   scheduler runtime task attempt through the canonical scheduler lifecycle,
   bind the selected reservation, and record the selected
   `WorkflowRuntimeDispatchCandidateFact` on the runtime-branch task event
   under the current claim generation before rehydration.
4. Rehydrate only after scheduler attempt identity/start timestamp, selected
   candidate fact, runtime-branch profile, and readiness proof are all present
   as backend-owned facts. Missing or mismatched facts must keep returning
   typed diagnostics.
5. Narrow `session_scheduler_runner` so it consumes the worker-prepared state
   for the worker-owned runtime-branch path and no longer owns runtime
   dispatch readiness/progress as a request-scoped legacy path. Any remaining
   non-worker path must be explicitly canonical and fail closed if it cannot
   produce the same facts.
6. Keep the durable dispatch assignment record as the later target
   architecture. After the worker-owned bridge path validates end to end,
   re-plan the pre-dispatch preparation output, selected candidate fact,
   scheduler attempt identity, reservation binding, claim generation, and
   lifecycle state into that durable assignment record rather than preserving
   bridge fields as a parallel owner.

No-fallback/no-legacy confirmation for this decision: scheduler progress and
readiness proof preparation move to a backend-owned worker-callable boundary,
not to frontend/Tauri state, graph/request inference, runtime-host inputs, or
request-scoped runner fallback. The worker must fail closed or persist typed
deferred state when pre-dispatch preparation cannot produce canonical ready
facts. Scheduler DTOs, Pumas contracts, runtime-host request fields, generated
files, lockfiles, and saved workflow fixtures remain out of scope unless a
future re-plan explicitly makes them serial shared-contract work.

Next thin slice: implement only sequence step 1. Allowed files should be
limited to extracting the workflow-service pre-dispatch preparation boundary,
focused tests for materialization/progress/readiness-proof outcomes, minimal
module wiring, and this plan. Do not wire the worker, change scheduler DTOs,
change Pumas contracts, edit generated/lock/workflow fixture files, change
runtime-host request fields, record selected candidate evidence, start runtime
task attempts, or build `WorkflowRuntimeTaskAttemptFactRecord` in this slice.

2026-06-09 pre-dispatch preparation boundary extraction slice: completed
sequence step 1 as a behavior-preserving extraction. Smallest useful vertical
slice: add `WorkflowPreDispatchPreparationBoundary` and
`WorkflowPreDispatchPreparationOutcome` in the workflow-service scheduler
runner module, route non-runtime input materialization/progress, runtime-run
external input materialization, resume progress, and runtime dependency
readiness preparation through that boundary, and keep runtime dispatch
selection/host execution outside the boundary. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/session_scheduler_runner.rs`
and this plan.

No-fallback/no-legacy confirmation: the boundary only wraps canonical
workflow-service scheduler preparation steps. It does not synthesize readiness
from graph/request/frontend/Tauri state, does not invoke runtime-host
dispatch, does not perform runtime dispatch selection, does not record selected
candidate evidence, does not start scheduler runtime task attempts, and does
not add compatibility DTOs, Pumas contract changes, generated files,
lockfiles, or workflow fixtures.

Verification passed:
`cargo fmt -p pantograph-workflow-service`,
`cargo test -p pantograph-workflow-service runtime_dispatch_selection --lib`,
`cargo check -p pantograph-workflow-service`, and `git diff --check -- crates/pantograph-workflow-service/src/workflow/session_scheduler_runner.rs docs/plans/current-image-generation-graphs/plan.md`.

Verification failures recorded for the next planned worker-preparation slice:
`cargo test -p pantograph-workflow-service workflow_execution_session_runtime_run_defers_pending_dependency_readiness_before_dispatch --lib`
still returns an `InternalError` from the worker-owned runtime branch path
before typed dependency-readiness deferral because the worker has not yet
called the new pre-dispatch preparation boundary. `cargo test -p
pantograph-workflow-service task_execution_worker --lib` still fails
`task_execution_worker_executes_runtime_branch_and_fails_invalid_dispatch_state`
with `runtime branch execution context rehydration failed` for the same
missing worker-side preparation/rehydration facts. These failures are not fixed
in this slice because wiring the worker, selected candidate evidence, scheduler
attempt start, and rehydration facts are explicitly sequence steps 2-4.

Deviation: no test source or fixture was changed in this extraction slice. The
existing focused readiness and dispatch-selection tests were rerun instead
because changing test fixtures would have crossed into the worker wiring and
rehydration behavior that the slice explicitly forbids.

Next thin slice: implement sequence step 2. Allowed files should be limited to
the task-execution worker runtime-branch path, the pre-dispatch preparation
boundary surface needed by that worker, focused worker tests for typed
dependency-readiness deferral, and this plan. The worker must call
`WorkflowPreDispatchPreparationBoundary` after claiming the runtime-branch
event and marking it `Dispatching`, before dispatch selection. If preparation
returns dependency-readiness pending/deferred diagnostics, the worker must
persist a durable deferred runtime-branch event with explicit retry timing and
return typed deferral. Do not record selected candidate evidence, start runtime
task attempts, change scheduler DTOs, change Pumas contracts, edit generated
or lock files, or preserve request-scoped runner fallback in this slice.

2026-06-09 worker pre-dispatch preparation slice: completed sequence step 2.
Smallest useful vertical slice: have the task-execution worker, after claiming
a runtime-branch event and persisting `Dispatching`, read backend-owned
active-run input bindings, materialize external inputs through
`WorkflowPreDispatchPreparationBoundary`, run canonical pre-dispatch
preparation, and persist a durable deferred runtime-branch event with explicit
retry timing when dependency readiness remains pending. Invalid scheduler
preparation still fails closed before `Running`. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`,
`crates/pantograph-workflow-service/src/workflow/session_scheduler_runner.rs`,
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
and this plan.

No-fallback/no-legacy confirmation: the worker now uses backend active-run
inputs and the workflow-service preparation boundary. It does not read inputs
from frontend/Tauri/runtime-host state, does not add runtime-host request
fields, does not synthesize readiness from graph/request state, does not
invoke request-scoped runner fallback, does not perform dispatch selection,
does not record selected candidate evidence, does not start scheduler runtime
task attempts, and does not change Pumas contracts, scheduler DTOs, generated
files, lockfiles, or workflow fixtures.

Verification passed:
`cargo fmt -p pantograph-workflow-service`,
`cargo test -p pantograph-workflow-service workflow_execution_session_runtime_run_defers_pending_dependency_readiness_before_dispatch --lib`,
`cargo test -p pantograph-workflow-service task_execution_worker_executes_runtime_branch_and_fails_invalid_dispatch_state_before_running --lib`,
`cargo test -p pantograph-workflow-service runtime_branch_dependency_defer_sets_retry_ready_time --lib`,
`cargo test -p pantograph-workflow-service task_execution_worker --lib`,
`cargo test -p pantograph-workflow-service runtime_dispatch_selection --lib`,
and `cargo check -p pantograph-workflow-service`.

Discovered issue resolved in slice: worker-owned runtime runs were previously
attempting rehydration before canonical scheduler preparation had materialized
external inputs or admitted dependency readiness, producing internal
rehydration failures instead of typed readiness deferral. The worker now
defers before rehydration when preparation cannot produce readiness proof.

Next thin slice: implement sequence step 3. Allowed files should be limited to
the task-execution worker runtime-branch path, runtime dispatch selection
boundary integration from the prepared runtime task, scheduler attempt
lifecycle start/binding code needed to produce canonical attempt identity and
start timestamp, focused worker/session tests for selected candidate evidence
and started attempt facts, and this plan. The slice must record the selected
`WorkflowRuntimeDispatchCandidateFact` under the current runtime-branch event
claim generation before rehydration and must fail closed with typed
diagnostics when candidate selection, reservation binding, or scheduler attempt
startup cannot produce canonical facts. Do not introduce the durable dispatch
assignment record yet, change Pumas contracts, change runtime-host request
fields, edit generated/lock/workflow fixture files, or preserve any
request-scoped runtime dispatch fallback.

2026-06-09 sequence step 3 re-plan trigger: do not begin the selected
candidate evidence / scheduler attempt start slice as currently written. The
existing worker-owned path rehydrates and then calls
`WorkflowTaskExecutionOwner::run_rehydrated_runtime_branch_until_dispatch_boundary`,
which re-enters `WorkflowSchedulerSessionRunner::run_until_runtime_dispatch_boundary_with_attempt_transition`.
That runner path repeats materialization, dependency-readiness preparation,
dispatch selection, and scheduler runtime task startup and still expects the
runtime scheduler task to be `Ready`. If the worker starts and binds the
runtime task before rehydration as sequence step 3 requires, the old owner path
will see the task as `Running` and fail closed. Preserving the old runner path
as a fallback would violate the no-fallback/no-legacy rule.

Additional blocker: the runtime-branch task event already stores the selected
`WorkflowRuntimeDispatchCandidateFact`, but it does not store the scheduler
runtime handoff / `SelectedRuntimeTaskDispatch` needed to spawn the runtime
supervisor after rehydration. That dispatch object cannot be reconstructed
from the selected candidate fact alone because the selected handoff and
reservation lease are scheduler-owned execution facts, not just evidence
metadata. Step 3 therefore needs an explicit owner/execution-path decision
before code changes continue.

Standards-aligned options:
1. Expand the bridge plan so sequence step 3 also extracts a worker-owned
   runtime dispatch execution boundary from the runner. The worker would
   prepare selection, start the scheduler attempt, bind the reservation, record
   selected candidate evidence under the current claim, rehydrate to validate
   backend-owned facts, then execute the already-selected in-memory
   `SelectedRuntimeTaskDispatch` through the new boundary. This keeps the
   durable assignment record deferred, avoids request-scoped fallback, and is
   the smallest end-to-end vertical bridge slice, but it is larger than the
   current step 3 wording.
2. Move the durable dispatch assignment record forward now. Persist the
   scheduler attempt identity, selected candidate fact, selected runtime
   handoff/reservation binding, claim generation, readiness proof, and
   lifecycle state as one canonical assignment before rehydration. This is the
   cleanest final architecture and best recovery model, but it is a serial
   shared-contract/persistence slice and larger than the current bridge step.
3. Split step 3 into a facts-only slice and a follow-up execution-boundary
   slice. The first slice would start/bind/record selected evidence and verify
   facts without completing runtime execution. This reduces immediate code
   volume, but it is not a useful end-to-end vertical slice by itself and
   risks leaving the system in a partially promoted bridge state unless the
   follow-up is done immediately.

Do not proceed by letting the worker start a scheduler attempt and then
falling back to the old request-scoped runner, by rehydrating without selected
dispatch execution ownership, by reconstructing handoff from evidence-only
candidate facts, or by making `Running` acceptable to the old readiness
preparation path.

2026-06-09 durable dispatch assignment decision: use Option 2. Move the
durable dispatch assignment record forward now instead of extending the bridge
with another in-memory handoff. The selected architecture is a backend-owned
assignment record that persists scheduler attempt identity, selected candidate
evidence, selected runtime dispatch handoff/reservation binding, runtime
branch claim generation, readiness proof identity/payload, and lifecycle state
before rehydration. Rehydration and worker execution must consume that
assignment record as the canonical dispatch source of truth. The old
request-scoped runtime dispatch runner path must be removed from the
worker-owned runtime branch path rather than preserved as a fallback.

Standards alignment for this decision:
- Plan standards: this is serial shared-contract/persistence work. Do not
  delegate it to sub-agents and do not mix it with generated files, lockfiles,
  saved workflow fixtures, Pumas contracts, or unrelated proposal docs. Execute
  it in thin, committed slices with focused verification after each slice.
- Coding standards: the assignment record separates evidence, scheduler
  lifecycle, dispatch handoff, and worker execution ownership so maintainers do
  not have to reason about request handling, persistence, runtime policy, and
  execution timing in one legacy runner path. Backend-owned facts remain the
  single source of truth.
- Concurrency/ownership: the worker owns runtime-branch claim, assignment
  creation, assignment lifecycle transitions, rehydration validation, and
  execution. Claim generation and assignment state must prevent duplicate
  dispatch, stale-claim writes, and replaying a selected handoff under a
  different claim.

Selected durable assignment implementation sequence:
1. Add the workflow-service durable dispatch assignment contract and in-memory
   repository. The record must include schema version, assignment id,
   runtime-branch event id, session id, workflow id, workflow run id,
   scheduler task id, scheduler task attempt id/start timestamp, attempt
   generation, runtime-branch claim owner/lease/generation, persisted
   readiness proof, selected candidate fact, selected runtime handoff /
   reservation binding data, lifecycle state, and created/updated timestamps.
   Repository operations must validate claim correlation, prevent duplicate
   active assignments for the same runtime-branch event/task attempt, and
   return typed diagnostics. This slice must not wire worker execution yet.
2. Teach the runtime-branch task event contract/repository to link to the
   assignment id and selected scheduler attempt id as assignment-owned facts,
   without duplicating assignment lifecycle ownership. Existing selected
   candidate evidence on the event may remain only as a rehydration correlation
   projection until a later cleanup removes bridge fields.
3. Update worker pre-dispatch flow to create the assignment after canonical
   readiness preparation and dispatch selection, start the scheduler runtime
   task attempt, bind the reservation, persist selected evidence/attempt facts,
   and then rehydrate from the assignment. Selection/start/bind failures must
   fail closed with typed diagnostics and must not call request-scoped runtime
   dispatch.
4. Replace `WorkflowTaskExecutionOwner::run_rehydrated_runtime_branch_until_dispatch_boundary`
   for worker-owned runtime branches with assignment-owned runtime execution.
   Execution must consume the selected dispatch stored in the assignment,
   spawn the runtime supervisor, complete/cancel/fail scheduler task lifecycle,
   apply reservation lifecycle updates, and update assignment/runtime-branch
   terminal state.
5. Tighten rehydration to require assignment id, selected candidate fact,
   selected dispatch handoff/reservation binding, scheduler attempt identity,
   readiness proof, runtime-branch profile, and matching claim generation.
   Missing or mismatched facts must return typed diagnostics.
6. Remove or narrow the legacy request-scoped runtime dispatch runner path so
   worker-owned runtime branches cannot re-enter readiness/selection/startup
   after assignment creation. Non-worker paths must either use the same
   assignment machinery or fail closed with typed diagnostics.
7. After the assignment path validates end to end, re-plan cleanup of bridge
   projections such as duplicated selected candidate fields on runtime-branch
   events and any compatibility-only helper APIs.

No-fallback/no-legacy confirmation: durable assignment is now the canonical
owner for selected runtime dispatch execution. Do not reconstruct dispatch
from evidence-only facts, do not pass selected handoff through request-scoped
runner inputs, do not make scheduler `Running` acceptable to the old
preparation path, do not preserve request-scoped runtime dispatch as fallback,
and do not split assignment lifecycle across frontend/Tauri/runtime-host
state.

2026-06-09 durable dispatch assignment repository slice: completed sequence
step 1. Smallest useful vertical slice: add a workflow-service durable runtime
dispatch assignment contract/module and in-memory repository, then wire the
repository into `WorkflowService` construction without worker execution
changes. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_dispatch_assignment.rs`,
`crates/pantograph-workflow-service/src/workflow.rs`,
`crates/pantograph-workflow-service/src/workflow/service_config.rs`, and this
plan.

No-fallback/no-legacy confirmation: the slice adds the canonical assignment
record and validates claim generation, scheduler task attempt identity,
selected candidate evidence, selected runtime handoff, readiness proof, and
reservation binding before storing a prepared assignment. It does not wire the
worker, alter runtime-branch task-event schema, change Pumas contracts, change
scheduler DTOs, edit generated/lock/workflow fixture files, start scheduler
attempts, invoke runtime-host dispatch, or preserve request-scoped runtime
dispatch fallback.

Verification passed:
`cargo fmt -p pantograph-workflow-service`,
`cargo test -p pantograph-workflow-service runtime_dispatch_assignment --lib`,
and `cargo check -p pantograph-workflow-service`. The focused test suite
covers prepared assignment creation, duplicate active assignment rejection,
terminal retry allowance, lifecycle ordering, selected candidate correlation,
and selected handoff correlation. Worktree deviation: unrelated Pumas proposal
documents were dirty before the slice and were left untouched.

2026-06-09 runtime-branch event assignment-link slice: completed sequence
step 2. Smallest useful vertical slice: bump the runtime-branch task-event
record schema, add a typed dispatch-assignment link carrying assignment id,
selected scheduler task attempt id, claim generation, and link timestamp, and
add repository support for linking that data under the current active claim.
Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`
and this plan.

No-fallback/no-legacy confirmation: the event link is only a correlation
projection for assignment-owned execution. It does not own assignment
lifecycle, wire worker runtime execution, change Pumas contracts, change
scheduler DTOs, edit generated/lock/workflow fixture files, invoke
runtime-host dispatch, remove bridge selected-candidate projections, or
restore request-scoped runtime dispatch. Reclaim, defer, and release now clear
per-attempt selected candidate, scheduler attempt, and assignment-link facts so
retry/replay cannot carry facts from a previous claim.

Verification passed:
`cargo fmt -p pantograph-workflow-service`,
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`,
`cargo test -p pantograph-workflow-service runtime_branch_rehydration --lib`,
`cargo test -p pantograph-workflow-service task_execution_worker --lib`,
and `cargo check -p pantograph-workflow-service`. The focused tests cover
current-claim assignment linking, idempotent same-link writes, stale-claim
rejection, different-link rejection, replay clearing, repository persistence,
and adjacent rehydration compatibility. Worktree deviation: unrelated Pumas
proposal documents remained dirty and were left untouched.

2026-06-09 worker assignment execution re-plan trigger: sequence step 3 cannot
stop at assignment creation. Creating a durable assignment requires starting
and binding the scheduler runtime task attempt, but the remaining worker path
still re-enters the old runner that expects the runtime task to be `Ready` and
would attempt readiness/selection/startup again. Proceeding with assignment
creation only would either strand a `Running` attempt, preserve request-scoped
runner fallback, or split lifecycle ownership across the worker and old runner.
That violates the no-fallback/no-legacy rule, the single-owner stateful-flow
standard, and the Rust async cancellation-safety rule against splitting a
multi-step durable lifecycle across cancellation points without an explicit
state machine.

Decision: use Option 2 now. The next implementation path is a direct
worker-owned runtime dispatch slice that creates the durable assignment and
replaces the old worker call to
`WorkflowTaskExecutionOwner::run_rehydrated_runtime_branch_until_dispatch_boundary`
in the same vertical path. The worker must own the sequence from canonical
pre-dispatch readiness through scheduler start/select/bind, assignment
creation/linking, runtime supervisor execution, scheduler terminal mutation,
reservation lifecycle release, assignment terminal state, and runtime-branch
terminal/deferred state. Do not introduce a temporary compatibility shim or
call the old request-scoped runner after a scheduler runtime attempt has been
started.

Standards alignment for Option 2:
- Plan standards: this is one larger but coherent vertical slice because the
  smaller assignment-only slice would be invalid. It remains serial
  integration-owner work; do not delegate it to sub-agents and do not mix it
  with generated files, lockfiles, saved workflow fixtures, Pumas contracts,
  scheduler DTO changes, or unrelated proposal docs.
- Coding standards: the worker becomes the single owner of the runtime branch
  stateful flow. Scheduler policy remains inside backend workflow-service /
  scheduler boundaries; frontend/Tauri/runtime-host code must not infer or
  persist business state.
- Rust async standards: durable transitions must be ordered so cancellation
  cannot leave an unobservable half-started task. Every spawned runtime
  supervisor remains tracked by the scheduler lifecycle owner and terminal or
  cancellation outcomes must be observed before assignment/runtime-branch
  terminalization.

Option 3 is recorded as a later promotion, not the current path. After Option
2 validates end to end, re-plan a dedicated runtime-dispatch lifecycle API only
if a second real caller needs the same lifecycle, recovery/replay/batching
starts duplicating phase logic, or worker code becomes a broad phase coordinator
that would be simpler as an explicit lifecycle subsystem. Until one of those
triggers exists, extracting that subsystem now is treated as overengineering.

Next thin slice: implement the smallest useful Option 2 vertical slice.
Allowed files should be limited to task-execution worker runtime-branch flow,
assignment repository usage, the minimal workflow-service/scheduler helper
accessors needed to reuse existing canonical start/select/bind and supervisor
logic, focused worker/session tests for assignment creation, event linking,
runtime execution, terminal/failure/defer behavior, and this plan. The slice
must not change Pumas contracts, change scheduler DTOs, edit generated/
lock/workflow fixture files, preserve request-scoped runtime dispatch fallback,
or remove bridge selected-candidate projections until assignment-owned
execution is validated.

2026-06-10 assignment-owned execution re-plan trigger: stop before wiring the
worker-owned assignment execution slice. An exploratory implementation proved
the worker can prepare readiness, start/select/bind a scheduler runtime task,
create a dispatch assignment, record selected candidate evidence, and link the
assignment before rehydration. The next fail-closed boundary is now the
runtime-branch task-attempt source context: production runtime-branch task
events are still admitted with `batch_eligibility: None`, while rehydration
requires the runtime-branch profile fields (`operation_type`,
`context_shape_key`, `cancellation_mode`, timeout correlation, and the selected
runtime/model/resource compatibility profile) before assignment-owned runtime
execution may continue. Filling those fields inside the worker from the
selected candidate, graph shape, provisional `batching_key`, request data, or
runtime-host inputs would synthesize canonical source-context facts and
violate the no-fallback/no-legacy rule.

Re-plan options:
1. Populate the runtime-branch task-attempt source profile at runtime-branch
   event admission/recovery from a backend-owned planning snapshot that
   explicitly owns operation type, context shape, cancellation mode, timeout,
   and selected runtime/model/resource compatibility facts. This is the
   strictest assignment path, but it requires identifying or adding that
   backend-owned planning snapshot before worker execution can proceed.
2. Move the profile into the durable dispatch assignment record and make
   assignment creation the source-context owner. This keeps profile ownership
   with the new assignment lifecycle and avoids duplicating it on
   runtime-branch events, but it requires changing rehydration to consume the
   assignment profile instead of `batch_eligibility` on the event.
3. Narrow the current task-attempt source context contract so runtime-branch
   events own only operation/context/cancellation/timeout, while selected
   candidate evidence owns runtime/model/resource compatibility. This reduces
   duplication, but it must be done as a contract slice with tests proving the
   worker does not synthesize operation/context/cancellation facts and does not
   treat candidate evidence as a replacement for run-scoped source context.

Do not continue by making `batch_eligibility` optional in rehydration, deriving
context shape from `batching_key`, deriving operation/cancellation policy from
graph/request/runtime-host state, or letting assignment-owned execution fall
back to the old request-scoped runner. Exploratory source edits for the failed
assignment-owned execution attempt were reverted; no unverified code slice was
kept.

Decision: use Option 3. Runtime-branch events will own only the run-scoped
source facts needed to explain what this branch is attempting:
`operation_type`, `context_shape_key`, `cancellation_mode`, and timeout
correlation. Durable selected-candidate evidence remains the owner of
runtime/model/resource compatibility, scheduler selection, reservation, and
dispatch-decision facts. Rehydration must combine those two durable fact
sources and return typed diagnostics when either side is missing or
inconsistent. The worker remains forbidden from synthesizing run-scoped facts
from request payloads, graph shape, provisional batching keys, runtime-host
inputs, or selected candidate facts.

Standards alignment for Option 3:
- Plan standards: this is a prerequisite contract slice before the larger
  assignment-owned execution slice. It keeps the next implementation thin by
  validating the source-context contract first instead of mixing contract
  reshaping with worker lifecycle replacement.
- Coding standards: runtime-branch admission/recovery is the single owner of
  run-scoped execution semantics, selected-candidate persistence is the single
  owner of scheduler/runtime compatibility evidence, and rehydration is a
  reader that validates the joined facts rather than inventing defaults.
- Testing standards: add focused contract tests for successful rehydration from
  the two durable fact sources plus fail-closed diagnostics for missing
  run-scoped source facts, missing selected-candidate evidence, and
  inconsistent selected-candidate/runtime-branch data.

Next thin slice: implement the Option 3 source-context contract split before
resuming worker-owned assignment execution. Allowed files should be limited to
runtime-branch task-event source profile records, runtime-branch rehydration,
task-attempt source-context DTO/validation, runtime-branch event
admission/recovery where the run-scoped facts are persisted, focused
workflow-service tests for the new contract and diagnostics, and this plan.
The slice must not change Pumas contracts, change scheduler DTOs, edit
generated/lock/workflow fixture files, make source-context fields optional,
derive source facts inside the worker, or preserve request-scoped runtime
dispatch fallback. Option 1 remains deferred unless replay-from-event-alone
becomes a hard requirement. Option 2 is not the current source-context design;
dispatch assignments own handoff and execution lifecycle facts, not the
semantic run-scoped source context.

2026-06-10 Option 3 source-context projection re-plan trigger: stop before
implementing the source-context contract split. Inspection found that
runtime-branch task events and task-attempt source-context validation can hold
the intended split, but admission/recovery currently receives
`WorkflowSchedulerTask` records that do not carry explicit
`operation_type`, `context_shape_key`, or `cancellation_mode` fields. The
validated executable snapshot and scheduler inference task projection also do
not currently expose those fields. Implementing the split only inside
runtime-branch task events would require deriving source facts from task kind,
node type, batching key, graph shape, request data, or worker/runtime-host
state, which is forbidden by the selected Option 3 decision and the
no-fallback/no-legacy rule. Implementing it correctly requires expanding the
next slice to include the canonical projection owner that carries run-scoped
source-context fields into `WorkflowSchedulerTask` before event admission.

Re-plan options:
1. Promote explicit source-context fields onto the executable validation
   snapshot node and ready inference task projection, then project them into
   `WorkflowSchedulerTask` and persist them on runtime-branch task events at
   admission/recovery. This keeps the validated backend snapshot as the
   canonical owner before scheduling and preserves the Option 3 split, but it
   broadens the next contract slice to include snapshot/projection/task-graph
   contracts and focused tests.
2. Add a narrower workflow-service runtime source-context projection adjacent
   to the task graph, keyed by workflow run and scheduler task id, and have
   runtime-branch admission consume it alongside `WorkflowSchedulerTask`. This
   avoids changing `WorkflowSchedulerTask` immediately, but it introduces a
   second projection that must be kept correlated with the task graph and may
   create avoidable ownership friction.
3. Reconsider Option 1 from the previous decision and add a full backend-owned
   planning snapshot that owns both run-scoped source facts and selected
   compatibility facts before dispatch. This is strict and replayable from one
   snapshot, but it duplicates selected-candidate evidence that is already
   owned by dispatch selection and is broader than needed for the current
   assignment-owned execution unblocker.

Recommended path: use Option 1. The next slice should explicitly add the
run-scoped source-context fields to the validated executable snapshot ->
ready inference projection -> scheduler task graph -> runtime-branch event
admission path, then update rehydration/task-attempt source-context validation
to combine event-owned source facts with selected-candidate evidence. This is
the smallest standards-compliant path that avoids derivation, keeps one
backend-owned source of truth for run-scoped semantics, and preserves the
selected-candidate evidence owner for runtime/model/resource compatibility.
Do not implement Option 3 by adding defaults, deriving source-context fields
from task type or node type, making source fields optional, or letting the
worker fill missing fields.

2026-06-10 upstream source-context ownership re-plan trigger: stop before
adding the source-context fields to the executable validation snapshot.
Inspection found that the snapshot is built from
`InferenceInterfaceNodeProjectionRecord`, which is built from inference
interface descriptors and authored inference snapshots. None of those upstream
contracts currently own `operation_type`, `context_shape_key`, or
`cancellation_mode`. Adding the fields only at the executable snapshot layer
would still require deriving them from task kind, node type, graph shape,
request data, or defaults before snapshot construction. That would violate the
selected Option 3 source-context split and the no-fallback/no-legacy rule.

Re-plan options:
1. Add an explicit workflow runtime source-context contract to the workflow
   graph/projection path before executable validation snapshot construction.
   The contract would carry `operation_type`, `context_shape_key`, and
   `cancellation_mode` as validated authored/planned workflow facts, then
   project those facts through the executable validation snapshot, ready
   inference projection, scheduler task graph, and runtime-branch event. This
   keeps run-scoped semantics owned by workflow-service planning instead of the
   worker and avoids treating model/interface metadata as workflow execution
   semantics. It requires a broader but coherent contract slice.
2. Promote the fields into `pantograph-inference-interface-contracts`
   descriptors and authored inference snapshots, then flow them through
   `InferenceInterfaceNodeProjectionRecord`. This is appropriate only if
   operation/context/cancellation are properties of the model interface itself.
   It is likely too broad for the current worker unblocker and risks putting
   workflow-run semantics into a reusable interface descriptor.
3. Put the fields into dependency-readiness proof/projection records. This
   keeps them near an existing validated upstream record, but dependency
   readiness owns environment/materialization proof, not execution semantics;
   using it as the owner would blur responsibility and make future planning
   harder to reason about.

Recommended path: use Option 1. Add the explicit workflow runtime
source-context contract at the workflow-service planning/projection boundary,
then carry it forward as a required fact through the already selected
snapshot/projection/task-graph/event path. The next implementation slice must
start by identifying the narrowest existing workflow graph/projection record
that can own authored or planned runtime source context without changing Pumas
contracts, scheduler DTOs outside the workflow-service task graph, generated
files, lockfiles, or workflow fixtures. If no existing workflow-service owner
can carry those explicit facts without derivation, stop again and split out a
dedicated source-context planning contract before rehydration work resumes.

2026-06-07 runtime-branch durable active-state slice: completed the first
Option 3 promotion step. Smallest useful vertical slice: extend the durable
runtime-branch task-event contract from bridge-style `Claimed` directly to
terminal/deferred states into explicit non-terminal `Dispatching` and
`Running` states with timestamps, add repository transitions for those states,
and have the task-execution worker persist `Dispatching` before backend
rehydration and `Running` before dispatch-boundary execution. The rehydration
boundary now accepts claimed-or-dispatching active events so it can read
backend active-run/session facts after the worker has durably recorded
dispatch intent. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`,
`crates/pantograph-workflow-service/src/workflow/runtime_branch_rehydration.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`,
`crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
and this plan.

No-fallback/no-legacy confirmation: this slice does not move execution facts
into the worker command, does not synthesize facts from graph/frontend/Tauri
state, does not add compatibility DTOs, does not call request-scoped runtime
execution, and does not change batching/replay policy. Runtime branch events
remain backend-owned durable facts; recovery treats `Dispatching` and
`Running` as active non-terminal states instead of terminal events.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`,
`cargo test -p pantograph-workflow-service runtime_branch_rehydration --lib`,
`cargo test -p pantograph-workflow-service task_execution_worker --lib`,
`cargo test -p pantograph-workflow-service runtime_branch --lib`,
`cargo check -p pantograph-workflow-service`,
`cargo fmt -p pantograph-workflow-service -- --check`, and `git diff --check`
for the slice files.

2026-06-07 runtime-branch duplicate-dispatch guard slice: completed the next
Option 3 duplicate-dispatch step. Smallest useful vertical slice: make
claim-by-workflow-run return a typed `AlreadyClaimed` diagnostic when a
runtime-branch event for that run is already active under an unexpired
`Claimed`, `Dispatching`, or `Running` lease, instead of reporting that no due
event exists. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`
and this plan.

No-fallback/no-legacy confirmation: duplicate detection is owned by the
durable runtime-branch event repository, not by request parameters, frontend
state, in-memory responder state, graph path inference, or a worker command
compatibility envelope. Deferred future retries and future ready events still
return no due event until their durable `ready_at_ms`; active in-flight work
returns a typed guard diagnostic.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`,
`cargo test -p pantograph-workflow-service runtime_branch --lib`,
`cargo check -p pantograph-workflow-service`,
`cargo check -p pantograph-embedded-runtime`,
`cargo fmt -p pantograph-workflow-service -- --check`, and `git diff --check`
for the slice files.

2026-06-07 runtime-branch retry-ready defer slice: completed the next Option 3
retry/defer step. Smallest useful vertical slice: separate durable
`deferred_at_ms` from retry `ready_at_ms` for runtime-branch task events and
have the task-execution worker use a bounded dependency-readiness retry delay
when persisting readiness-pending `Deferred` state. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`, and
this plan.

No-fallback/no-legacy confirmation: readiness-pending runtime work remains a
durable `Deferred` event, not a released plain `Ready` event, request-scoped
retry loop, frontend/Tauri timer, graph-derived retry policy, or synthetic
completion. Existing immediate `defer` remains a test/state-machine shorthand;
worker-owned runtime dependency readiness uses `defer_until` with an explicit
backend-owned retry-ready timestamp.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`,
`cargo test -p pantograph-workflow-service task_execution_worker --lib`,
`cargo test -p pantograph-workflow-service runtime_branch --lib`,
`cargo check -p pantograph-workflow-service`,
`cargo check -p pantograph-embedded-runtime`,
`cargo fmt -p pantograph-workflow-service -- --check`, and `git diff --check`
for the slice files.

2026-06-07 runtime-branch expired-running replay invariant slice: completed
the next Option 3 replay/restart guard. Smallest useful vertical slice: add a
repository boundary invariant proving an expired `Running` runtime-branch event
can be reclaimed by a later worker as a new attempt generation, with stale
dispatching/running timestamps cleared when the event returns to `Claimed`.
Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`
and this plan.

No-fallback/no-legacy confirmation: replay authority remains the durable event
claim lease and generation, not request-scoped runtime execution, in-memory
responder state, graph/frontend state, or synthesized worker facts. Unexpired
active dispatch remains blocked by the duplicate-dispatch guard; only expired
active work is reclaimable.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`,
`cargo check -p pantograph-workflow-service`,
`cargo check -p pantograph-embedded-runtime`,
`cargo fmt -p pantograph-workflow-service -- --check`, and `git diff --check`
for the slice files.

2026-06-07 runtime-branch dispatch-unavailable claim-release update:
completed the first immediate bridge slice. Smallest useful vertical slice:
add `release_claim` to the runtime-branch task-event repository/state machine
and have the task-execution worker release a claimed event back to `Ready`
when dispatch is unavailable. The worker still returns a typed
`RuntimeBranchDeferred` notification with `RuntimeBranchDispatchUnavailable`
for the waiting caller, but durable state is no longer terminal `Deferred`, so
a later dispatch-capable worker can reclaim the same event. Allowed files
touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`,
and plan docs. No-fallback/no-legacy confirmation: this slice does not execute
runtime branches, does not call the direct helper, does not add request-scoped
dispatch/completion, does not synthesize graph/frontend facts, does not add
frontend/Tauri policy, does not add compatibility DTOs, and does not fake
successful completion. Verification:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`
passed with 20 tests, and
`cargo test -p pantograph-workflow-service task_execution_worker --lib` passed
with 15 tests. Remaining follow-up: add the workflow-service-owned
rehydration boundary that accepts a claimed event plus claim and reads only
backend active-run/session records, then move dispatch-boundary execution
behind the worker.

2026-06-07 runtime-branch backend rehydration boundary update: completed the
second immediate bridge slice. Smallest useful vertical slice: add the
workflow-service-owned `runtime_branch_rehydration` boundary that accepts a
claimed runtime-branch task event plus current claim, validates the claim and
runtime task correlation, and reads session summary, active-run context,
scheduler task graph/state, and task-run summary only from backend
active-run/session records. The task-execution worker now calls this boundary
after claiming an event and before returning dispatch-unavailable; rehydration
failure returns typed worker diagnostics and releases the claim instead of
falling back. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_rehydration.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`,
`crates/pantograph-workflow-service/src/workflow.rs`,
`crates/pantograph-workflow-service/src/scheduler/mod.rs`, and plan docs.
No-fallback/no-legacy confirmation: this slice does not execute runtime
branches, does not move dispatch, does not call the direct helper, does not
add request-scoped execution envelopes, does not synthesize graph/frontend
facts, does not add frontend/Tauri policy, does not add compatibility DTOs,
and does not fake successful completion. Verification:
`cargo test -p pantograph-workflow-service runtime_branch_rehydration --lib`
passed with 3 tests,
`cargo test -p pantograph-workflow-service task_execution_worker --lib` passed
with 15 tests, and `cargo check -p pantograph-workflow-service` passed with
one existing warning for unused `WorkflowSchedulerStartedRuntimeTaskSupervisor`
abort helpers in `scheduler/task_orchestrator.rs`. Discovered issue recorded:
those unused supervisor abort helpers should be either wired into the planned
worker cancellation/shutdown lifecycle or removed in a dedicated lifecycle
cleanup slice. Remaining follow-up: move dispatch-boundary execution behind
the worker using the rehydrated backend context and owned host boundary, then
persist durable completed/deferred/failed event state before responder
notification.

2026-06-07 runtime-branch worker dispatch ownership update: completed the
third immediate bridge slice and the terminal portion of the fourth slice.
Smallest useful vertical slice: expose enqueue timestamp and scheduler
decision reason through the backend active-run context, add a
workflow-service-owned active-run `RunStarted` diagnostic recorder, move
runtime dispatch-boundary execution into the task-execution worker after
claim/rehydration, and switch the session runtime branch request path to
persist the durable event, enqueue a worker command, and await the worker
notification. The obsolete request-scoped runtime-branch context and direct
dispatch helper were removed rather than retained as a compatibility shim.
Terminal worker results now complete/fail the claimed durable runtime-branch
event before notifying the in-memory responder; readiness-pending runtime
dependency outcomes still release the claim back to `Ready` as bridge
behavior until the durable deferred/retry lifecycle slice lands. Allowed files
touched:
`crates/pantograph-workflow-service/src/scheduler/store.rs`,
`crates/pantograph-workflow-service/src/scheduler/store_queue.rs`,
`crates/pantograph-workflow-service/src/workflow/runtime_branch_rehydration.rs`,
`crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_owner.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_runtime.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_facade.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`,
and plan docs. No-fallback/no-legacy confirmation: this slice does not pass
request-scoped execution envelopes into the worker, does not call the removed
direct helper from the request path, does not synthesize graph/frontend/Tauri
facts, does not add compatibility DTOs, does not fake successful completion,
and returns typed worker/service diagnostics for missing events, stale
rehydration facts, readiness-pending dispatch, and terminal dispatch failures.
Verification:
`cargo test -p pantograph-workflow-service task_execution_worker --lib` passed
with 15 tests,
`cargo test -p pantograph-workflow-service session_execution_runtime --lib`
passed with 3 tests,
`cargo test -p pantograph-workflow-service runtime_branch_rehydration --lib`
passed with 3 tests,
`cargo test -p pantograph-workflow-service runtime_branch --lib` passed with
34 tests, and `cargo check -p pantograph-workflow-service` passed with the
known warning for unused `WorkflowSchedulerStartedRuntimeTaskSupervisor` abort
helpers in `scheduler/task_orchestrator.rs`. Discovered issue recorded: the
bridge still releases readiness-pending events back to `Ready`; the next slice
must promote readiness-pending runtime dependency outcomes into durable
deferred event state with explicit retry/replay semantics before responder
notification.

2026-06-07 runtime-branch retryable deferred state update: completed the
remaining deferred-state bridge slice. Smallest useful vertical slice: make
`WorkflowRuntimeBranchTaskEventState::Deferred` a durable retryable state
instead of a terminal state, clear the active claim when deferring, preserve
`deferred_at_ms`, set `ready_at_ms` for replay, allow due deferred events to
be reclaimed with a new attempt generation, and have the task-execution worker
persist readiness-pending runtime dependency outcomes through `defer` before
notifying the responder. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`,
and plan docs. No-fallback/no-legacy confirmation: this slice does not release
readiness-pending events as plain `Ready`, does not preserve terminal
`Deferred` behavior, does not synthesize retry facts from request/frontend/
Tauri state, does not fake successful completion, and keeps stale/expired
claim handling typed through the runtime-branch task-event diagnostics.
Verification:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`
passed with 20 tests,
`cargo test -p pantograph-workflow-service task_execution_worker --lib` passed
with 15 tests,
`cargo test -p pantograph-workflow-service runtime_branch --lib` passed with
34 tests, and `cargo check -p pantograph-workflow-service` passed with the
known warning for unused `WorkflowSchedulerStartedRuntimeTaskSupervisor` abort
helpers in `scheduler/task_orchestrator.rs`. Remaining follow-up: once a
complete inference path is validated through the bridge, promote the runtime
branch events into the full durable scheduler task-attempt lifecycle with
explicit running/dispatching state, batching, duplicate-dispatch guard, and
restart semantics.

2026-06-06 runtime-branch worker host-boundary update: completed the next
Option 3 preparation slice by threading the composition-root owned
`WorkflowHost` into `WorkflowTaskExecutionRuntimeOwner`,
`WorkflowTaskExecutionRuntimeBranchContext`, and the task-execution worker
runtime-branch environment. Smallest useful vertical slice: preserve the
existing durable-event claim/defer behavior while proving the worker will own
the same backend host/runtime boundary needed by the later claimed-event
dispatch slice. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/task_execution_facade.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_runtime.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`,
and plan docs. No-fallback/no-legacy confirmation: this slice does not
execute runtime branches, does not call the direct helper, does not add
request-scoped worker/runtime instances, does not synthesize graph/frontend
facts, does not add frontend/Tauri policy, does not add compatibility DTOs,
and does not fake successful completion. Verification:
`cargo test -p pantograph-workflow-service task_execution_worker --lib` passed
with 15 tests,
`cargo test -p pantograph-workflow-service runtime_owner_ --lib` passed with
5 tests, and
`cargo test -p pantograph-workflow-service session_execution_runtime --lib`
passed with 4 tests. Remaining follow-up: have the worker reconstruct
execution facts from the claimed durable event and backend run records, then
execute dispatch through this owned host boundary before durable terminal
state and responder notification.

2026-06-06 runtime-branch worker claim verification fix: corrected stale
runtime-owner/facade tests that still expected the pre-claim fail-closed
message after the worker began returning typed durable-event claim diagnostics.
Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/task_execution_runtime.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_facade.rs`,
and plan docs. No-fallback/no-legacy confirmation: the tests still assert
failure when no durable runtime-branch event is claimable; no dispatch
execution, direct helper call, compatibility DTO, or fake completion was
added. Verification:
`cargo test -p pantograph-workflow-service runtime_owner_ --lib` passed with
5 tests, and
`cargo test -p pantograph-workflow-service session_execution_runtime --lib`
passed with 4 tests.

2026-06-06 runtime-branch worker event claim/defer update: completed the
fourth Option 3 source slice by making the task-execution worker claim a due
runtime-branch task event from the workflow-service repository by workflow run
id, then persist a typed deferred event state while dispatch execution remains
unmoved. Smallest useful vertical slice: add repository claim-by-workflow-run
selection, wire `ExecuteRuntimeBranch` to claim through the worker-owned
backend service environment, return typed missing-event diagnostics when no
due durable event exists, and immediately defer a claimed event with a typed
`RuntimeBranchDispatchUnavailable` diagnostic so the lease is not left
hanging. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`,
and plan docs. No-fallback/no-legacy confirmation: this slice does not execute
runtime branches, does not call the direct helper, does not synthesize
execution facts from graph/frontend state, does not add frontend/Tauri
business policy, does not add compatibility DTOs, does not fake successful
completion, and does not make the in-memory responder the durable source of
truth. Verification:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`
passed with 19 tests, and
`cargo test -p pantograph-workflow-service task_execution_worker --lib` passed
with 15 tests. Remaining follow-up: reconstruct execution facts from claimed
durable records and move dispatch-boundary execution into the worker loop,
then persist completed/deferred/failed outcomes before responder notification.

2026-06-06 runtime-branch admission event persistence update: completed the
third Option 3 source slice by wiring runtime-containing run admission to
persist claimable runtime-branch task events through the workflow-service
repository boundary. Smallest useful vertical slice: add the repository owner
to `WorkflowService`, derive one deterministic event id per runtime inference
scheduler task from backend-owned task graph/run facts, persist queued input
keys, output targets, timeout, batching key, and no scheduler attempt id until
the later worker-claim/start slice creates the attempt. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow.rs`,
`crates/pantograph-workflow-service/src/workflow/service_config.rs`,
`crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`,
and plan docs. Deviation recorded: scheduler task ids are available at
admission, but scheduler task attempt ids are not; the event contract now
allows `scheduler_task_attempt_id` to be absent until worker claim/start binds
the scheduler attempt. No-fallback/no-legacy confirmation: this slice does not
execute runtime branches, does not claim events in the worker, does not call
the direct helper as completion, does not synthesize graph/frontend facts, does
not add frontend/Tauri policy, does not add compatibility DTOs, and does not
fake completion. Verification:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`
passed with 17 tests, and
`cargo test -p pantograph-workflow-service runtime_branch_admission --lib`
passed with 2 tests. Remaining follow-up: make the task-execution worker claim
due runtime-branch task events from the repository and return typed diagnostics
for missing/stale facts.

2026-06-06 durable runtime-branch task-event repository update: completed
the second Option 3 source slice by adding the workflow-service repository
boundary for durable runtime-branch task-event records. Smallest useful
vertical slice: add enqueue, direct claim, next-due claim, complete, defer,
fail, lookup, duplicate-event, no-due-event, stale-claim, active-claim,
lease-expiry, and reclaim behavior around the existing pure claim state
machine. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`
and plan docs. No-fallback/no-legacy confirmation: this repository boundary
does not wire runtime dispatch, does not execute branches, does not preserve
request-scoped completion, does not call the direct helper, does not synthesize
facts from graph/frontend state, does not add frontend/Tauri policy, and does
not add compatibility DTOs. Verification:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`
passed with 17 tests. Remaining follow-up: wire runtime branch admission to
persist claimable events through this repository boundary, then make the
task-execution worker claim due events from the repository.

2026-06-06 durable runtime-branch task-event contract update: completed the
first Option 3 source slice by adding the workflow-service-owned
`runtime_branch_task_event.rs` contract and pure state machine for durable
runtime-branch task-event claiming. Smallest useful vertical slice: define the
claim event identity, claim owner/lease identity, ready/claimed/completed/
deferred/failed states, attempt generation, lease expiry, queued input keys,
output targets, timeout, batching key, and typed diagnostics before adding any
storage or execution wiring. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`,
`crates/pantograph-workflow-service/src/workflow.rs`,
`crates/pantograph-workflow-service/src/workflow/README.md`, and this plan.
No-fallback/no-legacy confirmation: this slice does not execute runtime
branches, does not preserve request-scoped dispatch, does not synthesize facts
from graph/frontend state, does not add Tauri/frontend policy, does not add
compatibility DTOs, and does not fake successful completion. Verification:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`
passed with 10 tests. Remaining follow-up: add the durable
workflow-service repository boundary with enqueue, claim, complete, fail,
defer, lease-expiry, duplicate-claim, stale-claim, and reclaim tests, then wire
runtime branch admission to persist claimable events.

2026-06-06 runtime branch durable task-event claiming decision: use Option 3
as the active next implementation path. This supersedes the earlier immediate
Option 2 worker-rehydration continuation for runtime-branch dispatch: the next
source lane must introduce backend-owned durable task-event claiming before
moving runtime dispatch into the task-execution worker loop. The existing
runtime-branch completion responder may remain only as an in-memory
notification for callers waiting on the durable event outcome; it is not the
source of truth for task state, replay, or restart. Runtime branch admission
must persist or emit a durable claimable execution event with stable event id,
workflow run id, scheduler task/attempt identity, claim owner, lease expiry,
attempt generation, queued inputs/output targets, timeout policy, and typed
state transitions. Workers must claim due events, load or derive execution
facts from durable backend records only, execute through the
workflow-service/composition-root-owned host/runtime environment, persist
completed/deferred/failed terminal state, and only then notify the waiting
responder when one exists. Missing, stale, already-claimed, or lease-expired
facts must return typed diagnostics and transition or leave durable state
according to the claim contract; do not synthesize facts from graph/frontend
state.

Option 1 in-memory execution envelopes and Option 2 standalone rehydration are
rejected as next source paths because they would keep the current worker move
request/completion-shaped instead of durable/replay-shaped. They may appear
only as private implementation details inside the durable claim contract. The
no-fallback/no-legacy rule remains active: do not keep direct request-scoped
runtime dispatch, do not call the existing direct helper as the production
completion path, do not add Tauri/frontend business policy, do not add
compatibility DTOs, and do not fake successful completion.

Option 3 thin sequence:
1. Define the durable runtime-branch task-event claim contract and state
   machine: event ids, task/attempt ids, claim owner, lease expiry, attempt
   generation, ready/claimed/completed/deferred/failed states, restart/replay
   semantics, batching eligibility, and typed diagnostics.
2. Add the workflow-service durable storage/repository boundary with focused
   tests for enqueue, claim, complete, fail, defer, lease expiry, stale claim,
   duplicate claim, and reclaim.
3. Change runtime branch admission to persist or emit a durable runtime-branch
   execution event instead of depending on an in-memory execution envelope or
   direct helper call.
4. Make the task-execution worker claim due durable runtime-branch events and
   reconstruct execution facts only from backend-owned durable records,
   returning typed diagnostics for missing or stale facts. Partial
   2026-06-06: the worker now claims due events by workflow run and defers the
   claimed event with typed dispatch-unavailable diagnostics; execution-fact
   reconstruction and dispatch remain next.
5. Move dispatch-boundary execution into the worker loop using the claimed
   durable event facts and owned host/runtime environment, then persist the
   branch outcome before responder notification.
6. Preserve the current blocking inference response shape by awaiting the
   responder notification over durable completion while keeping durable task
   state as the source of truth.
7. Add restart/replay/batching tests, worker-unavailable/shutdown diagnostics,
   and duplicate-dispatch prevention coverage.
8. Remove or make unreachable the production direct runtime helper path once
   the durable worker path is validated; direct `WorkflowService` runtime
   execution must remain fail-closed.

2026-06-06 runtime branch worker execution re-plan trigger: stop before
moving dispatch-boundary execution into the task-execution worker loop. The
worker command now has a real responder and backend service environment, but
the current direct execution helper also needs the owned host boundary,
session summary, optional run snapshot, admitted/dequeued run facts, queued
inputs/output targets, timeout, and scheduler task-run summary. The next
implementation must choose how those facts enter the worker without keeping
request-scoped runtime execution. Standards-aligned options: (1) immediate
in-memory execution envelope: include the already-admitted execution facts in
the worker command and put the owned host in the worker environment; (2)
worker rehydration: add explicit workflow-service/store read APIs so the
worker rehydrates the execution facts from run ids; (3) defer to durable
task-event claiming, which is the later target but would block the current
inference path. Do not move dispatch by calling the existing direct helper
from the request path, do not synthesize missing execution facts from graph or
frontend state, and do not add durable claiming inside the immediate Option 2
path unless this re-plan selects it.

2026-06-06 runtime branch completion responder update:
`ExecuteRuntimeBranch` now carries a real typed completion responder, and
`WorkflowTaskExecutionRuntimeOwner` plus `WorkflowSessionExecutionRuntime`
can enqueue a runtime branch command and await the worker-owned outcome. The
worker currently returns a typed fail-closed `RuntimeBranchFailed` outcome
because dispatch-boundary execution has not moved into the loop yet. This is
not a fallback path: it does not execute runtime branches directly, synthesize
successful completion, add request-scoped runtime dispatch, add frontend/Tauri
policy, introduce compatibility DTOs, or add durable claiming. Next Option 2
slice: move runtime branch dispatch-boundary execution into the worker loop
and map completed/deferred/failed paths to typed worker outcomes.

2026-06-06 task-execution worker runtime environment update:
`WorkflowTaskExecutionRuntimeOwner` now builds a typed
`WorkflowTaskExecutionWorkerRuntimeBranchEnvironment` from its shared
`Arc<WorkflowService>` and passes that environment into
`WorkflowTaskExecutionWorker::spawn`. The worker loop owns that backend
runtime-branch environment and observes runtime-branch commands without
executing them yet. This preserves the no-fallback boundary: no branch
execution, fake completion responder, request-scoped runtime dispatch, Tauri
policy, compatibility DTO, or durable claiming was added. Next Option 2
slice: add a real runtime-branch completion responder to
`ExecuteRuntimeBranch` and have the facade enqueue/await it.

2026-06-06 composition-root host boundary ownership update:
`WorkflowSessionExecutionRuntime` now owns the shared `Arc<WorkflowService>`,
an `Arc<dyn WorkflowHost>` backend host/runtime boundary, and
`WorkflowTaskExecutionRuntimeOwner`. Facade session runs use the owned host
boundary instead of taking request-scoped host references, and focused tests
prove the facade-owned host/service/runtime-owner construction plus the
canonical runtime readiness-deferral path. Direct `WorkflowService` runtime
execution remains fail-closed; no worker-owned completion, fake oneshot,
request-scoped runtime dispatch/completion, frontend/Tauri policy,
compatibility DTO, or durable claiming was added. Next Option 2 slice: pass
the backend runtime branch execution environment from
`WorkflowTaskExecutionRuntimeOwner` into the task-execution worker spawn path.

2026-06-06 worker-owned runtime branch completion decision: use Option 2 for
the immediate worker-owned completion path. `WorkflowSessionExecutionRuntime`
must become the production composition-root owner for the shared
`Arc<WorkflowService>`, an owned host/runtime boundary, and
`WorkflowTaskExecutionRuntimeOwner`. The task-execution worker must receive
backend execution context from that composition-root owner, execute
`ExecuteRuntimeBranch` inside the worker loop, and return completed,
failed, deferred, unavailable, or shutdown outcomes through a real completion
responder owned by the command path. This preserves backend-owned business
logic, runtime reuse across workflow runs, and the standards' composition-root
and task-lifecycle rules. Direct `WorkflowService` runtime execution must
remain fail-closed. Do not add fake direct-execution oneshots, request-scoped
runtime dispatch/completion, Tauri/frontend policy, compatibility DTOs, or
durable claiming in the next source slice. Later target after the blocking
inference path is complete and validated: decouple runtime branch execution
from `WorkflowHost` where feasible, then evolve to durable task-event-loop
claiming for replay, restart, and batching.

2026-06-06 worker-owned runtime branch completion re-plan trigger: stop
before moving runtime branch completion behind the task-execution worker
command path. The current worker owns bounded queue/lifecycle mechanics only:
`WorkflowTaskExecutionWorker::spawn` receives scheduler lifecycle state but
does not own `Arc<WorkflowService>` or a runtime-branch execution context, and
`ExecuteRuntimeBranch` carries only serializable run identifiers with no
completion responder. Implementing worker-owned completion now requires
choosing the worker execution/completion contract. Do not add a fake oneshot
around the existing direct helper call, do not move business logic back into
request handlers, and do not make Tauri/frontend own runtime branch policy.

2026-06-06 runtime branch context entry update: runtime-containing session
runs now require the `WorkflowSessionExecutionRuntime` composition-root
entrypoint to supply a `WorkflowTaskExecutionRuntimeOwner`; direct
`WorkflowService` runtime runs fail closed with typed `CapabilityViolation`
diagnostics before admission instead of preserving request-scoped runtime
execution. The runtime branch now constructs a
`WorkflowTaskExecutionRuntimeBranchContext` through the facade-owned runtime
owner and routes dispatch-boundary execution through that context. This does
not yet enqueue/await worker-owned branch completion; that remains the next
Option 2 slice. One runtime readiness-deferral test now enters through the
facade to prove the canonical path still reaches dependency-readiness deferral.

2026-06-06 composition-root session execution delegation update:
`WorkflowSessionExecutionRuntime` now exposes
`run_workflow_execution_session` and delegates to the existing
`WorkflowService` blocking session-run behavior. This changes the production
entrypoint shape only; runtime dispatch, readiness continuation, reservation
lifecycle, terminal mutation, completion signaling, lifecycle snapshots,
diagnostics-ledger events, frontend/Tauri policy, durable claiming, and
fallback execution remain unmoved. Next Option 2 slice: migrate the runtime
branch entry to construct its worker-owned runtime-branch context through the
facade-owned runtime owner.

2026-06-06 composition-root session execution facade update:
workflow-service now exposes `WorkflowSessionExecutionRuntime`, a production
composition-root entrypoint that owns both a shared `Arc<WorkflowService>` and
the `WorkflowTaskExecutionRuntimeOwner`. This slice only added construction,
shared-service access, README ownership documentation, and focused tests that
prove the facade-owned runtime owner uses the same backend service; it did not
move runtime dispatch, request execution, completion signaling, lifecycle
snapshots, diagnostics-ledger events, Tauri/frontend policy, durable claiming,
or fallback execution. Next Option 2 slice: add a session-execution method on
the facade that delegates the existing blocking behavior without changing
dispatch, then migrate the runtime branch entry through the facade-owned
runtime owner.

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

2026-06-06 branch migration re-plan trigger: stop before migrating a request
execution branch. Reading the current runtime branch showed that the request
path still owns the full task-attempt sequence: readiness refresh, candidate
collection, dispatch selection, ready-to-start transition, reservation
binding, runtime supervisor spawn/join, terminal task mutation, attempt
lifecycle events, reservation release/reconcile, and final output projection.
A standards-compliant worker migration cannot be made by queueing a marker
while leaving that sequence request-scoped, and it cannot add a fake completion
channel around the current direct request execution. The next plan decision
must choose the worker-owned execution command shape that moves this
task-attempt sequence behind the composition-root owner while keeping
`WorkflowService` as request facade and keeping runtime-host/runtime-registry
instances long-lived and reusable across workflow runs.

2026-06-06 branch migration decision: use Option 2 now, with Option 3 recorded
as the later target architecture. The immediate path is a worker-owned runtime
run branch: after admission, the request facade enqueues a runtime-branch
command through the composition-root owner and awaits a typed result, while the
worker-owned branch owns readiness continuation, dispatch selection,
reservation binding/release, runtime supervisor execution, terminal task
mutation, attempt lifecycle events, and branch output projection. This
preserves the no-fallback/no-legacy rule because execution ownership moves
behind the worker boundary rather than queueing a marker around request-scoped
execution or adding a direct-execution oneshot. Immediate thin-slice sequence:
define the worker-owned runtime branch command/result contract; extract the
runtime branch execution context behind the composition-root owner; migrate one
runtime request branch to enqueue and await worker-owned completion; add
shutdown/cancellation behavior; then cover successful dispatch, dispatch
failure, timeout/cancel, and reservation release with focused tests. Later
Option 3: after the worker-owned inference path is complete and validated,
evolve to the durable task event-loop architecture where workers claim ready
scheduler task attempts from durable task state, execute independently across
workflow runs, persist terminal/deferred outcomes, support replay/recovery, and
enable batching across simultaneous runs. Do not start durable task claiming,
public worker lifecycle snapshots, diagnostics-ledger worker events, frontend
policy, or compatibility DTOs before the Option 2 worker-owned inference path
is complete.

2026-06-06 worker-owned runtime branch contract slice: completed the first
Option 2 implementation slice by adding internal `ExecuteRuntimeBranch`
command and `RuntimeBranchCompleted`, `RuntimeBranchFailed`, and
`RuntimeBranchDeferred` outcomes to the task-execution worker contract. The
contract is run-scoped and carries workflow/session/run identity, output
targets, timeout, and a typed branch start reason; outcomes preserve typed
diagnostics and do not introduce durable task claiming, public worker lifecycle
snapshots, diagnostics-ledger worker events, request-scoped workers, frontend
policy, compatibility DTOs, or a direct-execution oneshot. Runtime dispatch and
request execution behavior remain unchanged. Verification:
`cargo fmt -p pantograph-workflow-service` and
`cargo test -p pantograph-workflow-service task_execution_worker --lib`. Next
Option 2 slice: extract the runtime branch execution context behind the
composition-root owner while preserving long-lived runtime-host/runtime-registry
reuse across workflow runs.

2026-06-06 runtime branch execution context slice: completed the next Option 2
implementation slice by adding an internal runtime-branch execution context to
`task_execution_runtime`. The composition-root owner now builds a context from
the worker-owned runtime-branch command and carries the shared
`Arc<WorkflowService>` into that context, preserving access to long-lived
backend workflow, scheduler, runtime-host, and runtime-registry services
without creating request-scoped workers or per-run runtime instances. This slice
does not move runtime dispatch, task mutation, reservation lifecycle, output
projection, durable task claiming, public worker lifecycle snapshots,
diagnostics-ledger worker events, frontend policy, compatibility DTOs, or
direct-execution oneshots. Verification:
`cargo fmt -p pantograph-workflow-service` and
`cargo test -p pantograph-workflow-service runtime_owner_ --lib`. Next Option
2 slice: migrate one runtime request branch to construct this context through
the composition-root owner and enqueue/await worker-owned completion.

2026-06-06 composition-root entrypoint re-plan trigger: stop before migrating a
runtime request branch. The composition-root owner now has the worker,
lifecycle state, typed diagnostics, runtime-branch command contract, and
runtime-branch context, but the session execution request facade still admits
runs through `&WorkflowService` directly. There is no production entrypoint that
owns both the `WorkflowService` and `WorkflowTaskExecutionRuntimeOwner`, so a
branch migration from the current request code would either instantiate a
request-scoped owner/worker, bypass the selected composition-root owner, or keep
runtime dispatch and completion request-scoped behind a worker-shaped command.
All three violate the selected Option 2 ownership and no-fallback/no-legacy
rules. The next plan decision must choose how production session execution is
entered through the composition-root owner while keeping `WorkflowService` as
request facade/API translation and preserving long-lived runtime-host and
runtime-registry reuse across workflow runs.

2026-06-06 composition-root entrypoint decision: use the production
composition-root facade path. Add a production entrypoint type that owns both
`Arc<WorkflowService>` and `WorkflowTaskExecutionRuntimeOwner`; production
session execution must enter through that type before runtime branch migration.
`WorkflowService` remains the request facade/API translation and backend
service holder, but it must not own the background worker or create
request-scoped workers/runtimes. Immediate thin-slice sequence: add the
composition-root facade with construction and service accessors; expose a
session-execution method on that facade that delegates to existing
`WorkflowService` behavior without changing runtime dispatch; migrate the
runtime branch entry to construct the runtime-branch context through the
facade-owned runtime owner; then move worker-owned branch completion behind
that context. This preserves long-lived runtime-host/runtime-registry reuse
across workflow runs and keeps Option 3 durable task claiming deferred until
the Option 2 worker-owned inference path is complete and validated.

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

2026-06-07 node-execution I/O artifact projection test refresh slice:
completed. Smallest useful vertical slice: update the six
`node_execution_workflow_sink_*` retained-artifact tests to refresh the
canonical diagnostics I/O artifact projection before querying projection rows.
Allowed files touched:
`crates/pantograph-embedded-runtime/src/node_execution_ledger_tests.rs` and
this plan. No source projection change was needed.

No-fallback/no-legacy confirmation: the tests still exercise
`NodeExecutionWorkflowLedgerSink` recording `IoArtifactObserved` diagnostics
and reading through the workflow-service I/O artifact projection. The slice
does not restore request-scoped artifact rows, bypass diagnostics projection
refresh, read artifact store manifests directly, add compatibility shims, or
use graph/frontend/runtime fallback paths. The status assertion in the
task-completion test now also refreshes the canonical node-status projection
before querying it.

Verification passed:
`cargo test -p pantograph-embedded-runtime node_execution_workflow_sink --lib`,
`cargo fmt -p pantograph-embedded-runtime -- --check`, and
`cargo check -p pantograph-embedded-runtime`. Broader verification
`cargo test -p pantograph-embedded-runtime --lib` now reports 402 passing and
4 failing tests; the six node-execution ledger retained-artifact failures are
removed.

Discovered issues and follow-ups:
- The broad embedded-runtime lib suite still warns that
  `synthetic_kv_node_memory_snapshot` is unused after the checkpoint
  migration. Cleanup remains deferred because it is a shared fixture cleanup,
  not part of this node-ledger slice.
- Remaining embedded-runtime broad failures are now the two retired
  data-graph Python sidecar reconciliation expectations and the two
  runtime-preflight restart classification expectations.

2026-06-07 retired data-graph Python sidecar test migration slice:
completed. Smallest useful vertical slice: migrate the two stale
`data_graph_execution_tests` that expected successful direct Python-sidecar
media execution and runtime-registry reconciliation from retired
`onnx-inference`/`audio-generation` graph fixtures. Allowed files touched:
`crates/pantograph-embedded-runtime/src/lib_tests/data_graph_execution_tests.rs`
and this plan.

No-fallback/no-legacy confirmation: the tests no longer ask
`EmbeddedRuntime::execute_data_graph` to execute retired media/runtime nodes
through the Python adapter, no longer expect synthetic audio output from
retired fixtures, and no longer reconcile `onnx-runtime` or `stable_audio`
runtime-registry records from direct data-graph execution. The migrated tests
assert the canonical fail-closed shape: no Python adapter calls, no sidecar
runtime reconciliation, no audio output for the retired intermediate ONNX
path, and typed `dependency_preflight_retired` errors for retired terminal
media nodes.

Verification passed:
`cargo test -p pantograph-embedded-runtime execute_data_graph_retired --lib`,
`cargo fmt -p pantograph-embedded-runtime -- --check`, and
`cargo check -p pantograph-embedded-runtime`. Broader verification
`cargo test -p pantograph-embedded-runtime --lib` now reports 404 passing and
2 failing tests; the retired data-graph Python sidecar reconciliation failures
are removed.

Discovered issues and follow-ups:
- The two runtime-preflight restart classification failures were resolved by
  the 2026-06-07 runtime-preflight restart diagnostic test migration slice:
  preflight still asserts selected-runtime readiness, while session execution
  now rejects the stale runtime-required fixture at the canonical stale-graph
  validation boundary.
- Broader embedded-runtime verification now passes:
  `cargo test -p pantograph-embedded-runtime --lib` reports 406 passing tests.
- The unused `synthetic_kv_node_memory_snapshot` warning was cleaned up by
  the 2026-06-07 shared embedded-runtime fixture cleanup slice.

2026-06-07 shared embedded-runtime fixture cleanup slice: completed. Smallest
useful vertical slice: remove the unused
`synthetic_kv_node_memory_snapshot` import and fixture helper left behind after
the keep-alive checkpoint migration replaced executor node-memory assertions
with runtime/model residency facts. Allowed files touched:
`crates/pantograph-embedded-runtime/src/lib_tests.rs`,
`crates/pantograph-embedded-runtime/src/lib_tests/graph_fixtures.rs`, and this
plan.

No-fallback/no-legacy confirmation: this slice deletes dead test-harness code
that represented the retired node-memory checkpoint shape. It does not restore
executor snapshots, KV handle carry-forward, workflow input replay, or any
legacy runtime execution path.

Verification passed:
`cargo test -p pantograph-embedded-runtime --lib`,
`cargo check -p pantograph-embedded-runtime`,
`cargo fmt -p pantograph-embedded-runtime -- --check`, and `git diff --check`
for the slice files. The embedded-runtime lib suite reports 406 passing tests;
the previous `synthetic_kv_node_memory_snapshot` warnings are gone. The
workflow-service unused `WorkflowSchedulerStartedRuntimeTaskSupervisor` abort
helper warning remains owned by the worker cancellation/shutdown lifecycle
cleanup.

2026-06-07 workflow-service supervisor test-helper cleanup slice: completed.
Smallest useful vertical slice: mark the unused
`WorkflowSchedulerStartedRuntimeTaskSupervisor` abort helpers as test-only,
because they are exercised by scheduler orchestrator tests but are not part of
the production library surface. Allowed files touched:
`crates/pantograph-workflow-service/src/scheduler/task_orchestrator.rs` and
this plan.

No-fallback/no-legacy confirmation: this slice does not alter scheduler task
execution, worker cancellation, supervisor tracking, runtime dispatch,
request-scoped execution, or any lifecycle state transition. It only removes a
production dead-code warning for helpers whose current owner is focused test
coverage.

Verification passed:
`cargo test -p pantograph-workflow-service task_orchestrator --lib`,
`cargo check -p pantograph-workflow-service`,
`cargo check -p pantograph-embedded-runtime`,
`cargo fmt -p pantograph-workflow-service -- --check`, and `git diff --check`
for the slice files.

2026-06-10 workflow-service runtime source-context and worker dispatch fact
slice: completed. Smallest useful vertical slice: introduce explicit
workflow-service-owned runtime source context and carry it from graph inference
nodes through inference publication, executable validation snapshots,
scheduler-ready inference projections, scheduler task graph records,
runtime-branch task events, runtime-branch rehydration, and task-attempt source
context; then resolve the re-plan trigger by moving canonical runtime dispatch
selection and scheduler task-attempt start ahead of runtime-branch rehydration
inside the worker. Allowed files touched:
`crates/pantograph-workflow-service/src/graph/*`,
`crates/pantograph-workflow-service/src/scheduler/*` fixture/test literals,
`crates/pantograph-workflow-service/src/workflow/*`, focused workflow-service
tests, and this plan.

No-fallback/no-legacy confirmation: the implementation does not derive
`operation_type`, `context_shape_key`, or `cancellation_mode` from task kind,
node type, runtime/backend keys, batching keys, request inputs, or
runtime-host state. Valid inference nodes must provide the explicit
`runtime_source_context` graph data contract; missing or malformed values
produce typed graph-resolution diagnostics. Runtime branch event admission
rejects runtime scheduler tasks without propagated source context instead of
fabricating defaults. The worker now claims the durable branch event, marks it
dispatching, prepares dependency-readiness and runtime dispatch selection,
starts the canonical scheduler runtime task attempt, records selected-candidate
evidence, links the dispatch assignment to the scheduler attempt id, and only
then rehydrates from durable facts. Task-attempt source context groups the
run-scoped runtime source context with durable selected-candidate evidence
rather than using batch-eligibility profile facts or request-scoped runtime
state as substitutes.

Verification passed:
`cargo check -p pantograph-workflow-service`;
`cargo test -p pantograph-workflow-service graph::inference_interface_request::tests --lib`;
`cargo test -p pantograph-workflow-service graph::inference_interface_publication::tests --lib`;
`cargo test -p pantograph-workflow-service workflow::tests::task_graph --lib`;
`cargo test -p pantograph-workflow-service workflow::tests::task_binding_resolution --lib`;
`cargo test -p pantograph-workflow-service workflow::runtime_branch_rehydration::tests --lib`;
`cargo test -p pantograph-workflow-service workflow::runtime_task_attempt_fact::tests --lib`;
`cargo test -p pantograph-workflow-service workflow::runtime_branch_task_event::tests --lib`;
`cargo test -p pantograph-workflow-service workflow::tests::session_execution::workflow_execution_session_dispatches_ready_runtime_task_through_scheduler_selection --lib -- --exact`;
and `cargo fmt -p pantograph-workflow-service -- --check`.

Discovered issue resolved by the 2026-06-11 session-admission test migration
slice:
`cargo test -p pantograph-workflow-service workflow::tests::session_admission::workflow_execution_session_run_waits_for_runtime_admission_before_dequeue --lib -- --exact`
failed at `session_admission.rs:185` with `left: 0` and `right: 1` because the
test still expected the retired host-level runtime-load admission gate to keep
a non-runtime run queued. The follow-up slice removed the obsolete
host-`run_workflow` blocking fixture, added the explicit in-memory workflow I/O
contract to the admission-gated fixture, and migrated the remaining admission
coverage to assert canonical cold-start behavior: a non-runtime run is admitted
and completed without opening the legacy runtime gate, emits a `RunStarted`
diagnostic with `cold_start_required`, and does not fabricate
`SchedulerRunDelayed` diagnostics.

No-fallback/no-legacy confirmation: this migration does not reintroduce
request-scoped runtime execution, host-level blocking hooks, synthetic
runtime-admission waits, or fallback delayed-run events. Capacity and scheduler
snapshot diagnostics remain covered by their focused session-capacity and
scheduler-snapshot test modules.

Verification passed:
`cargo test -p pantograph-workflow-service workflow::tests::session_admission --lib`;
`cargo test -p pantograph-workflow-service workflow::tests::session_execution::workflow_execution_session_dispatches_ready_runtime_task_through_scheduler_selection --lib -- --exact`;
and `cargo check -p pantograph-workflow-service`. Formatting was applied with
`cargo fmt -p pantograph-workflow-service`; the final format check is part of
the slice commit gate.

2026-06-11 runtime-branch admission source-context fixture slice: completed.
Smallest useful vertical slice: update the workflow-service
`runtime_task_graph()` unit-test fixture used by runtime-branch admission tests
so its runtime inference scheduler task carries the explicit
`WorkflowRuntimeSourceContext` now required by admission. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/session_execution_api.rs` and
this plan.

No-fallback/no-legacy confirmation: the fixture now supplies the same typed
source-context contract production admission requires. The test does not
relax admission validation, synthesize source context inside admission, or
restore batch-profile/request-scoped fallback behavior.

Verification passed:
`cargo test -p pantograph-workflow-service workflow::session_execution_api::tests::runtime_branch_admission_persists_claimable_task_event --lib -- --exact`;
and
`cargo test -p pantograph-workflow-service workflow::session_execution_api::tests::runtime_branch_admission_rejects_duplicate_claimable_task_event --lib -- --exact`.

2026-06-11 bootstrap recovery deferred runtime-branch event slice:
completed. Smallest useful vertical slice: keep the worker-owned bootstrap
dependency-readiness recovery path canonical by making an already-deferred
runtime-branch task event due when bootstrap recovery rechecks dependency
readiness after a fresh snapshot is available. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`,
`crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
and this plan.

Discovered issue resolved in-slice: an initial readiness-pending runtime run
creates and defers the durable runtime-branch task event. Bootstrap recovery
correctly ensured the event existed, but treated a deferred event as already
claimable, so the worker had no due event to claim even after dependency
readiness became fresh. The recovery path now uses a first-class
runtime-branch event repository transition to mark the deferred event due, then
the task-execution worker reclaims it and re-evaluates dependency readiness
through the existing typed proof path.

No-fallback/no-legacy confirmation: this slice does not bypass dependency
readiness, dispatch selection, runtime-branch claiming, selected-candidate
evidence, task-attempt start, runtime-host execution, or worker ownership. It
does not restore request-scoped runtime execution or synthesize successful
recovery. Recovery only makes the durable event claimable so the canonical
worker path can evaluate the fresh typed readiness proof and fail closed if
canonical planning still cannot dispatch.

Verification passed:
`cargo test -p pantograph-workflow-service workflow::runtime_branch_task_event::tests::runtime_branch_task_event_marks_deferred_event_ready_for_recovery --lib -- --exact`;
`cargo test -p pantograph-workflow-service workflow::runtime_branch_task_event::tests::runtime_branch_task_event_rejects_mark_ready_for_non_deferred_event --lib -- --exact`;
`cargo test -p pantograph-workflow-service workflow::tests::session_execution::workflow_execution_session_ready_runtime_task_fails_closed_without_dispatch_candidate --lib -- --exact`;
`cargo test -p pantograph-workflow-service workflow::tests::session_execution::workflow_execution_session_bootstrap_recovery_applies_dependency_readiness_resume_plan --lib -- --exact`;
`cargo test -p pantograph-workflow-service session_execution --lib`; and
`cargo test -p pantograph-workflow-service workflow::runtime_branch_task_event::tests --lib`.

2026-06-11 runtime task-attempt reservation identity re-plan decision: use
Option 1, making scheduler reservation lease identity the canonical
reservation fact for the current worker-owned bridge. The planned
task-attempt fact construction slice cannot safely require a separate
`reservation_id`, because the selected dispatch, selected candidate fact, and
scheduler reservation evidence expose `SchedulerReservationLeaseId` but no
separate backend-owned reservation id. Synthesizing one from request, graph,
runtime-host, or lease text would create a fake fact and violate the
no-fallback/no-legacy rule.

Immediate sequence:
1. Narrow `WorkflowRuntimeTaskAttemptReservationFact` to
   `reservation_lease_id`, `resource_kind`, and `reserved_bytes`, with focused
   validation/tests. This keeps the fact contract aligned with the canonical
   scheduler reservation evidence available at the worker dispatch boundary.
2. Build `WorkflowRuntimeTaskAttemptFactRecord` from the rehydrated source
   context and selected candidate fact using the scheduler reservation lease
   evidence. Do not introduce a separate reservation id until the later durable
   dispatch assignment/reservation lifecycle architecture owns one explicitly.

No-fallback/no-legacy confirmation: the bridge may record only facts already
owned by runtime-branch events, scheduler task attempts, selected dispatch
selection, and scheduler reservation leases. It must not fabricate reservation
ids or preserve parallel reservation owners.

2026-06-11 runtime task-attempt reservation fact contract slice: completed.
Smallest useful vertical slice: remove the non-canonical separate
`reservation_id`/`lease_id` pair from workflow-service runtime task-attempt
reservation facts and replace it with a single `reservation_lease_id` field.
Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_task_attempt_fact.rs`
and this plan.

No-fallback/no-legacy confirmation: this slice does not build task-attempt
facts yet, does not change scheduler DTOs, runtime-host request fields, Pumas
contracts, generated files, lockfiles, saved workflow fixtures, or worker
dispatch behavior, and does not synthesize a separate reservation id.

Verification passed:
`cargo test -p pantograph-workflow-service workflow::runtime_task_attempt_fact::tests --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and
`git diff --check`.

2026-06-11 runtime task-attempt device placement re-plan decision: use the
reservation-level device evidence option. The next task-attempt fact
construction slice must not keep or populate a singular `resolved_device_id`.
The scheduler may select CPU, GPU, NPU, system RAM, device VRAM, shared
memory, or a split placement across multiple physical devices/resources. A
single resolved device would either discard scheduler evidence or force the
worker to synthesize a primary device. Instead, runtime task-attempt facts must
record per-reservation evidence from the selected candidate's canonical
scheduler reservations.

Updated immediate sequence:
1. Extend `WorkflowRuntimeTaskAttemptReservationFact` to carry
   `reservation_lease_id`, `device_id`, `resource_kind`, and `reserved_bytes`;
   remove the top-level `resolved_device_id` from
   `WorkflowRuntimeTaskAttemptFactRecord` and its request contract. Validation
   must require nonblank reservation lease id and device id plus
   `reserved_bytes > 0`.
2. Build `WorkflowRuntimeTaskAttemptFactRecord` from the rehydrated source
   context and selected candidate fact by projecting
   `selected_candidate_fact.reservations` into reservation-level facts. Do not
   infer devices from workflow input, graph path, runtime-host state, request
   DTOs, batching keys, or runtime/model identifiers.
3. Keep runtime/model residency separate from workflow ownership. A workflow
   may request persistence/keep-alive, but the scheduler owns whether a
   runtime/model remains loaded, where it is loaded, when it is unloaded, and
   which later workflow runs may reuse it. The workflow run records usage
   evidence for its attempt; it does not own the resident runtime.
4. Defer any separate physical-device inventory, runtime-residency table, or
   durable dispatch-assignment ownership expansion to a later serial
   architecture slice unless the immediate task-attempt fact construction
   cannot validate reservation-level evidence without it.

No-fallback/no-legacy confirmation: the worker-owned bridge may record only
reservation facts already selected by scheduler-owned dispatch evidence. It
must not hard-link models to runtimes, make workflows owners of persistent
runtimes, collapse multi-device placement into a single device, or fabricate
device facts from runtime/model names.

2026-06-11 runtime task-attempt reservation-level device fact contract slice:
completed. Smallest useful vertical slice: remove the singular
`resolved_device_id` from the workflow-service runtime task-attempt fact
request/record and require each reservation fact to carry its scheduler
reservation lease id, device id, resource kind, and reserved bytes. Allowed
files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_task_attempt_fact.rs`
and this plan.

No-fallback/no-legacy confirmation: this slice does not build task-attempt
facts yet, does not infer a primary device, does not change scheduler DTOs,
runtime-host request fields, Pumas contracts, generated files, lockfiles,
saved workflow fixtures, or worker dispatch behavior, and does not link runtime
residency ownership to the workflow that requested persistence.

Verification passed:
`cargo test -p pantograph-workflow-service workflow::runtime_task_attempt_fact::tests --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and `git diff --check`.

2026-06-11 runtime task-attempt source-context projection slice: completed.
Smallest useful vertical slice: add a pure workflow-service constructor that
builds `WorkflowRuntimeTaskAttemptFactRecord` from validated
`WorkflowRuntimeTaskAttemptSourceContext`, scheduler task-attempt id, scheduler
task-attempt start timestamp, and fact record timestamp by projecting selected
candidate model/runtime/resource-fit facts and reservation-level scheduler
reservation evidence. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_task_attempt_fact.rs`
and this plan.

No-fallback/no-legacy confirmation: this slice does not persist facts, does
not wire the worker, does not infer devices from load target/runtime/model
names, does not synthesize a primary device for split placements, and does not
change scheduler DTOs, Pumas contracts, runtime-host request fields, generated
files, lockfiles, or saved workflow fixtures. Future scheduler resource-fit
states or resource kinds fail closed through typed diagnostics until the fact
contract explicitly supports them.

Focused tests added:
`runtime_task_attempt_fact_builds_from_source_context_selected_candidate_reservations`
and
`runtime_task_attempt_fact_build_rejects_missing_scheduler_attempt_start`.

Verification passed:
`cargo test -p pantograph-workflow-service workflow::runtime_task_attempt_fact::tests --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and `git diff --check`.

2026-06-11 runtime task-attempt fact persistence re-plan trigger: stop before
worker-side task-attempt fact persistence. The workflow-service contract can
now build `WorkflowRuntimeTaskAttemptFactRecord` from canonical rehydrated
source context, selected candidate reservations, scheduler attempt identity,
and timestamps, but there is no canonical persistence owner for the built
fact. Inspection found that `WorkflowRuntimeDispatchAssignmentRepository`
exists, but the task-execution worker currently only links a generated
dispatch-assignment id onto the runtime-branch event; it does not create a
durable dispatch-assignment record. Building the fact in the worker and
discarding it would not advance durable task-attempt facts, while silently
adding a store would decide ownership outside the plan.

Standards-aligned re-plan options:
1. Add a dedicated workflow-service runtime task-attempt fact repository now.
   The worker would build and persist a fact after rehydration and before
   marking the runtime-branch event `Running`. This is the smallest direct
   persistence slice, but it creates a second durable owner beside the
   dispatch-assignment target architecture and would need a later serial
   migration to avoid parallel lifecycle ownership.
2. Promote the durable dispatch-assignment record now and make it the owner of
   task-attempt selected runtime/resource facts. The worker would create a real
   `WorkflowRuntimeDispatchAssignmentRecord` after selected evidence is
   recorded and before rehydration/linking, then persist or derive the
   `WorkflowRuntimeTaskAttemptFactRecord` from that assignment-owned source.
   This is larger, but it aligns with the planned worker/scheduler
   architecture, keeps claim generation, selected candidate evidence,
   scheduler attempt identity, reservation binding, and lifecycle state under
   one backend-owned dispatch record, and avoids a temporary parallel
   task-attempt fact owner.
3. Record task-attempt facts as another field on runtime-branch events. This
   is the smallest mechanical worker edit, but it is not standards-aligned for
   the target architecture because runtime-branch events are already bridge
   state scheduled for replacement by durable dispatch assignments; adding
   more fact ownership there would preserve legacy bridge behavior.

Recommended direction: Option 2. The implementation should be split into
serial slices: first make the worker create and persist
`WorkflowRuntimeDispatchAssignmentRecord` using existing selected candidate,
runtime handoff, reservation lease, readiness proof, claim, and scheduler
attempt facts; then build/persist or derive the task-attempt fact from that
assignment-owned evidence; then update rehydration/worker tests to consume the
assignment-backed fact path. Do not add a standalone fact store or bridge-field
fact persistence unless the plan explicitly chooses that ownership model.

2026-06-11 durable dispatch-assignment worker persistence slice: completed.
Smallest useful vertical slice: make the task-execution worker create a real
`WorkflowRuntimeDispatchAssignmentRecord` after selected candidate evidence is
recorded and before linking the runtime-branch event to the assignment id.
Allowed files touched:
`crates/pantograph-workflow-service/src/scheduler/task_orchestrator.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`,
`crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
and this plan.

No-fallback/no-legacy confirmation: this slice does not add a standalone
runtime task-attempt fact store, does not persist task-attempt facts on
runtime-branch bridge events, does not change scheduler DTOs, Pumas contracts,
runtime-host request fields, generated files, lockfiles, saved workflow
fixtures, or runtime execution behavior, and does not infer device placement
from runtime/model/load-target names. The assignment records the dispatch-level
reservation lease while retaining the selected candidate fact's full
reservation list for reservation-level device/resource evidence.

Focused test update: the session execution scheduler-selection test now proves
the runtime-branch dispatch-assignment link resolves to a persisted durable
assignment record with matching event id, scheduler attempt id, selected
candidate id, and reservation lease.

Verification passed:
`cargo test -p pantograph-workflow-service workflow::tests::session_execution::workflow_execution_session_dispatches_ready_runtime_task_through_scheduler_selection --lib -- --exact`;
`cargo test -p pantograph-workflow-service task_execution_worker --lib`;
`cargo test -p pantograph-workflow-service runtime_dispatch_assignment --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and `git diff --check`.

2026-06-11 durable dispatch-assignment source-context slice: completed.
Smallest useful vertical slice: extend
`WorkflowRuntimeDispatchAssignmentRecord` and its create request to own the
run-scoped runtime source context and timeout needed for later task-attempt
fact derivation from assignment-owned evidence. The worker now copies those
facts from the already-claimed runtime-branch task event into the assignment
at creation time. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_dispatch_assignment.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`,
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
and this plan.

No-fallback/no-legacy confirmation: this slice does not build or persist
runtime task-attempt facts, does not derive source context from graph shape,
request payloads, batching keys, selected candidate evidence, runtime-host
inputs, or runtime/model/load-target names, and does not change scheduler DTOs,
Pumas contracts, generated files, lockfiles, or saved workflow fixtures.
Runtime source context remains required and invalid assignment requests fail
closed through typed diagnostics.

Focused test update: dispatch-assignment repository tests now reject missing
source context, and the session execution scheduler-selection test proves the
worker-created assignment carries the admitted source context and timeout.

Verification passed:
`cargo test -p pantograph-workflow-service workflow::tests::session_execution::workflow_execution_session_dispatches_ready_runtime_task_through_scheduler_selection --lib -- --exact`;
`cargo test -p pantograph-workflow-service task_execution_worker --lib`;
`cargo test -p pantograph-workflow-service runtime_dispatch_assignment --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and `git diff --check`.

2026-06-11 dispatch-assignment task-attempt fact derivation slice:
completed. Smallest useful vertical slice: add a pure
`WorkflowRuntimeDispatchAssignmentRecord::task_attempt_fact_record` derivation
that builds `WorkflowRuntimeTaskAttemptFactRecord` from assignment-owned
source context, selected candidate evidence, scheduler attempt identity,
scheduler attempt start timestamp, timeout, and reservation facts. Allowed
files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_dispatch_assignment.rs`
and this plan.

No-fallback/no-legacy confirmation: this slice does not add a standalone
task-attempt fact repository, does not persist task-attempt facts on
runtime-branch bridge events, does not wire worker-side fact persistence, does
not derive source context or devices from graph/request/batching/runtime-host
state, and does not change scheduler DTOs, Pumas contracts, generated files,
lockfiles, or saved workflow fixtures. The derivation reuses the existing
typed task-attempt fact validator so missing or inconsistent assignment-owned
facts fail closed.

Focused test added:
`runtime_dispatch_assignment_derives_task_attempt_fact_from_assignment_owned_evidence`.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_dispatch_assignment --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and `git diff --check`.

2026-06-11 assignment-backed runtime-branch rehydration slice: completed.
Smallest useful vertical slice: make runtime-branch worker rehydration require
the persisted dispatch-assignment link and build task-attempt source context
from the assignment-owned selected candidate/source facts instead of the
runtime-branch event's bridge-only selected candidate projection. Allowed files
touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_rehydration.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`, and
this plan.

No-fallback/no-legacy confirmation: this slice does not fall back to
runtime-branch selected-candidate bridge fields, does not recreate
request-scoped runtime execution, does not add a standalone task-attempt fact
repository, does not persist task-attempt facts on runtime-branch events, and
does not infer devices from graph/request/batching/runtime-host state,
runtime/model ids, or load-target strings. Missing or stale dispatch
assignments now fail closed through typed runtime-branch rehydration
diagnostics.

Focused test updates: runtime-branch rehydration tests now create
assignment-backed scheduler handoff evidence, reject missing dispatch
assignments with `DispatchAssignmentUnavailable`, and prove that stale
bridge-only selected candidate projections are ignored in favor of the
persisted assignment evidence.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_branch_rehydration --lib`;
`cargo test -p pantograph-workflow-service task_execution_worker --lib`;
`cargo test -p pantograph-workflow-service workflow::tests::session_execution::workflow_execution_session_dispatches_ready_runtime_task_through_scheduler_selection --lib -- --exact`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and `git diff --check`.

2026-06-11 dispatch-assignment running task-attempt fact persistence slice:
completed. Smallest useful vertical slice: make the persisted
`WorkflowRuntimeDispatchAssignmentRecord` own the durable
`WorkflowRuntimeTaskAttemptFactRecord` when the assignment transitions from
`Prepared` to `Running`, and make the task-execution worker perform that
assignment transition before marking the runtime-branch task event running.
Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_dispatch_assignment.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`,
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
and this plan.

No-fallback/no-legacy confirmation: this slice does not add a standalone
task-attempt fact repository, does not persist task-attempt facts on
runtime-branch events, does not infer device/runtime/model facts outside the
assignment-owned selected candidate evidence, and does not restore
request-scoped runtime execution. Prepared assignments have no task-attempt
fact; the fact is recorded only at the canonical running transition from
assignment-owned evidence and typed validation failures fail closed.

Focused test updates: dispatch-assignment repository tests now prove
`mark_running` records and stores the derived task-attempt fact, and the
runtime scheduler-selection session test proves the worker-created assignment
contains the persisted task-attempt fact with scheduler attempt, residency, and
reservation-level device evidence.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_dispatch_assignment --lib`;
`cargo test -p pantograph-workflow-service workflow::tests::session_execution::workflow_execution_session_dispatches_ready_runtime_task_through_scheduler_selection --lib -- --exact`;
`cargo test -p pantograph-workflow-service task_execution_worker --lib`;
`cargo test -p pantograph-workflow-service runtime_branch_rehydration --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and `git diff --check`.

2026-06-11 batch-eligibility task-attempt derivation re-plan trigger:
stop before deriving the existing runtime-branch
`WorkflowRuntimeBranchBatchEligibilityProfile` from persisted task-attempt
facts. The newly canonical task-attempt fact records reservation-level device
evidence and can represent split placement across CPU/GPU/NPU/system-memory
resources, but the legacy batch profile still carries a singular
`device_load_target`. Populating that field from task-attempt facts would
collapse multi-device placement into one compatibility value or treat the
resolved load target string as a substitute for reservation evidence. That
would violate the reservation-level device decision, backend-owned data
ownership, and the no-fallback/no-legacy rule. Additionally, scheduler
reservation lease ids are per-attempt ownership facts; they must remain
attached to the task attempt and must not become the equality key that decides
whether two different workflow runs can batch together.

Standards-aligned re-plan options:
1. Replace the legacy batch profile with a reservation-level compatibility
   profile derived from `WorkflowRuntimeTaskAttemptFactRecord`. The profile
   should compare model artifact, runtime family, backend, runtime residency,
   loaded-runtime memory estimate, operation type, context shape, cancellation
   mode, timeout policy, and normalized reservation requirements such as
   device ids/resource kinds/reserved bytes. It must not compare
   per-attempt reservation lease ids as batching identity. This is the best
   match for multi-device support and keeps task-attempt facts as the
   canonical compatibility source, but it requires a focused contract update
   and migration of existing batch-eligibility tests.
2. Remove the event-owned `batch_eligibility` bridge field now and defer
   grouped claims until the grouped-claim repository compares
   assignment-owned task-attempt facts directly. This is the cleanest bridge
   removal, but it blocks intermediate batch-compatibility assertions and
   makes the next grouped-claim slice larger because it must introduce the
   compatibility comparator at the same time.
3. Keep the existing singular-load-target profile only for explicitly
   single-device runtimes and reject multi-device task-attempt facts as
   ineligible for batching. This can be standards-aligned only if the product
   intentionally limits batching to single-placement tasks for the immediate
   milestone; it is not suitable as the target architecture because it would
   exclude valid split-placement and multi-device workflows.
4. Introduce the later physical-device/runtime-residency inventory and derive
   compatibility from residency plus task-attempt facts. This is the most
   complete architecture for future scheduling, but it is broader than the
   immediate grouped-claim unblocker and should be selected only if
   reservation-level task-attempt facts cannot express enough compatibility
   without a first-class residency/device store.

Recommended direction: Option 1. It preserves the current plan's intent to
derive batch compatibility from durable task-attempt facts, avoids carrying
forward bridge event ownership, supports multi-device placement, and keeps the
next slice contract-focused before durable grouped claiming. Do not implement
Option 1 by reusing `device_load_target` as a primary device, by comparing
reservation lease ids across runs, by deriving compatibility from runtime ids
or batching keys, or by adding grouped claims before the compatibility
contract is updated and tested.

2026-06-11 batch-eligibility task-attempt derivation re-plan decision: use
Option 1. Replace the legacy singular-device runtime-branch batch profile with
a reservation-level compatibility profile derived from
`WorkflowRuntimeTaskAttemptFactRecord`. Batch compatibility must be based on
durable task-attempt facts, not event-admission bridge data, provisional
batching keys, runtime id strings, selected-candidate side fields, or
workflow inputs from any previous keep-alive run. Reservation lease ids remain
per-attempt ownership evidence and must not be used as equality keys for
cross-run batch grouping.

Selected implementation sequence:
1. Add a workflow-service-owned reservation-level batch compatibility contract
   that can be built from `WorkflowRuntimeTaskAttemptFactRecord`. It should
   carry model artifact id, runtime family, backend id, runtime residency key,
   loaded-runtime memory estimate, operation type, context shape,
   cancellation mode, timeout policy, and normalized reservation compatibility
   entries with device id, resource kind, and reserved bytes. It must not
   carry `reservation_lease_id` or a singular `device_load_target`.
2. Replace or narrow `WorkflowRuntimeBranchBatchEligibilityProfile` usages so
   compatibility checks consume the new task-attempt-derived profile. Existing
   tests that compare canonical batch facts must migrate to multi-reservation
   compatibility, and tests must prove matching provisional `batching_key`
   values never authorize grouping.
3. Add focused fail-closed tests for missing task-attempt facts, mismatched
   runtime residency, mismatched context shape/operation/cancellation, timeout
   mismatch, and reservation incompatibility across device id/resource kind/
   reserved bytes. Add a positive split-placement case to prove multi-device
   facts are preserved.
4. Only after the compatibility contract is validated, continue to durable
   grouped claiming over assignment-owned task-attempt facts. Grouped claims
   must not compare per-attempt reservation lease ids and must not rehydrate
   bridge-only event batch profiles.
5. Defer physical-device inventory/runtime-residency table work until a later
   serial architecture slice unless the reservation-level compatibility
   contract cannot express the required batching decision without it.

Next thin slice: implement sequence step 1 only. Allowed files should be
limited to workflow-service runtime-branch/task-attempt lifecycle modules,
focused tests, and this plan. Do not change Pumas contracts, scheduler DTOs,
generated files, lockfiles, saved workflow fixtures, runtime-host APIs, or
grouped-claim execution. Stop and re-plan if compatibility requires facts not
owned by `WorkflowRuntimeTaskAttemptFactRecord`.

2026-06-11 reservation-level batch compatibility contract slice: completed
sequence step 1. Smallest useful vertical slice: add the
workflow-service-owned
`WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile` contract that is
derived directly from `WorkflowRuntimeTaskAttemptFactRecord` and carries model
artifact id, runtime family, backend id, runtime residency key, loaded-runtime
memory estimate, operation type, context shape, cancellation mode, timeout
policy, and normalized reservation compatibility entries with device id,
resource kind, and reserved bytes. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`
and this plan.

No-fallback/no-legacy confirmation: this slice does not wire grouped claims,
does not migrate production batch checks yet, does not derive compatibility
from `batching_key`, selected-candidate side fields, runtime id strings, graph
shape, request state, previous workflow inputs, or runtime-host state, and
does not carry `reservation_lease_id` or the legacy singular
`device_load_target` in the new task-attempt-derived profile. Missing
reservation evidence fails closed through a typed
`ReservationProfileMissing` diagnostic; reservation incompatibility fails
closed through `ReservationProfileMismatch`.

Focused test updates: runtime-branch task-event tests now prove the new
profile preserves split-placement reservation facts, ignores per-attempt
reservation lease ids, rejects missing reservation evidence, and rejects
reservation mismatches across device id/resource kind/reserved bytes.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and `git diff --check`.

Next thin slice: implement sequence step 2 by replacing or narrowing legacy
`WorkflowRuntimeBranchBatchEligibilityProfile` usage so compatibility checks
consume the new task-attempt-derived profile. Allowed files should remain
limited to workflow-service runtime-branch/task-attempt lifecycle modules,
focused tests, and this plan unless the diff proves another workflow-service
module owns the call site. Do not change Pumas contracts, scheduler DTOs,
generated files, lockfiles, saved workflow fixtures, runtime-host APIs, or
durable grouped-claim execution in this slice.

2026-06-11 task-attempt batch compatibility check migration slice: completed
sequence step 2. Smallest useful vertical slice: remove the legacy
runtime-branch event-owned `ensure_batch_compatible_with` comparison over
`WorkflowRuntimeBranchBatchEligibilityProfile` and replace the compatibility
entry point with
`WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile::ensure_task_attempt_facts_compatible`,
which derives both sides from `WorkflowRuntimeTaskAttemptFactRecord`.
Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`
and this plan.

No-fallback/no-legacy confirmation: compatibility checks no longer compare
the legacy event-owned `device_load_target` batch profile, no longer use
runtime-branch event admission data as the compatibility authority, and still
ignore provisional `batching_key` values. Missing task-attempt facts fail
closed through `MissingTaskAttemptFact`; missing reservation evidence and
reservation mismatch continue to fail closed through the reservation-level
diagnostics added in the previous slice.

Focused test updates: runtime-branch task-event tests now compare canonical
batch facts through persisted task-attempt facts, reject missing task-attempt
facts, reject timeout mismatches, and prove that matching provisional
`batching_key` values do not authorize grouping when task-attempt facts differ.
The legacy profile-field validation test remains only as admission-field
validation while the field still exists and is not used as a compatibility
authority.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and `git diff --check`.

Next thin slice: implement sequence step 3 by adding any remaining focused
fail-closed compatibility cases that are not already covered, especially
runtime residency, context shape, operation type, cancellation mode, timeout,
reservation device/resource/bytes, missing task-attempt facts, and the positive
split-placement case. Keep the slice to workflow-service runtime-branch/
task-attempt lifecycle tests and this plan unless a missing diagnostic requires
a small runtime-branch contract change.

2026-06-11 task-attempt batch compatibility fail-closed coverage slice:
completed sequence step 3. Smallest useful vertical slice: add the remaining
focused tests for canonical task-attempt compatibility mismatches without
changing production grouping behavior. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs`
and this plan.

No-fallback/no-legacy confirmation: the tests continue to drive compatibility
through `WorkflowRuntimeTaskAttemptFactRecord` only. They do not reintroduce
event-owned batch-profile comparison, do not use `batching_key` as authority,
and do not compare reservation lease ids across workflow runs.

Focused test updates: coverage now includes missing task-attempt facts,
runtime residency mismatch, context shape mismatch, operation type mismatch,
cancellation mode mismatch, timeout mismatch, missing reservation evidence,
reservation device id mismatch, reservation resource kind mismatch, reservation
reserved-byte mismatch, and a positive split-placement case.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_branch_task_event --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and `git diff --check`.

Next thin slice: implement sequence step 4 by introducing durable grouped
claiming over assignment-owned task-attempt facts. The first grouped-claim
slice should remain repository/contract-level if possible: use existing
assignment-owned `task_attempt_fact` as compatibility authority, reject missing
facts through typed diagnostics, and do not dispatch grouped execution yet.
Stop and re-plan if grouped claiming requires a new physical-device inventory,
runtime-residency table, scheduler DTO change, generated contract, workflow
fixture update, or runtime-host API change.

2026-06-11 assignment-owned grouped-claim repository slice: completed the
first repository/contract-level part of sequence step 4. Smallest useful
vertical slice: add durable batch-claim identity, owner, lease, outcome, and
in-memory repository claiming for compatible running
`WorkflowRuntimeDispatchAssignmentRecord` values using their persisted
`task_attempt_fact` records as the compatibility authority. Allowed files
touched:
`crates/pantograph-workflow-service/src/workflow/runtime_dispatch_assignment.rs`,
`crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`, and
this plan.

No-fallback/no-legacy confirmation: grouped claims compare
assignment-owned task-attempt facts through
`WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile`, do not read
runtime-branch event batch profiles, do not compare `batching_key`, do not
compare reservation lease ids as batch identity, do not add a runtime-host
batch API, and do not execute coalesced runtime work. Incompatible running
assignments remain unclaimed; missing task-attempt facts fail closed through a
typed `MissingTaskAttemptFact` diagnostic.

Focused test updates: dispatch-assignment repository tests now prove compatible
running assignments can be claimed under a shared batch lease, incompatible
task-attempt facts are skipped, missing task-attempt facts fail closed, and an
active batch claim prevents reentry. The worker diagnostic mapping was extended
only to keep the new assignment repository diagnostics exhaustively typed.

Discovered follow-up: the current scheduler readiness-proof fixture used by
dispatch-assignment tests is workflow-run-specific. The grouped-claim
repository test therefore varies event id, scheduler attempt id, and
reservation lease under the fixture's existing workflow run instead of
fabricating readiness-proof internals for a second workflow run. Add a proper
multi-run readiness-proof fixture or fixture builder before using this coverage
as proof of cross-run grouped claiming.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_dispatch_assignment --lib`;
`cargo test -p pantograph-workflow-service task_execution_worker --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and `git diff --check`.

Re-plan checkpoint: the remaining path toward actual coalesced execution now
needs a standards-aligned decision before implementation. The repository can
durably claim compatible assignment groups, but the worker still executes the
anchor assignment through the existing single-runtime dispatch path. Continuing
to worker-level grouped claiming or runtime-host batch execution may require a
new composition-root runtime-host batch operation and fan-out/error semantics.
Do not proceed by implicitly executing only the anchor while marking peers
claimed, by adding a runtime-host batch API in the same slice as worker grouped
claiming, or by preserving single-run behavior behind a grouped-claim shim.

2026-06-11 coalesced runtime execution re-plan decision: use the fixture-first
then contract-first path. First add a proper multi-run readiness-proof fixture
or fixture builder and cross-run acceptance coverage for grouped claims. Then
define the composition-root-owned runtime-host batch execution contract before
worker-level grouped execution calls it. This sequence keeps the grouped-claim
repository boundary validated with real cross-run evidence before adding a
runtime-host batch API, and it keeps the batch API frozen before the worker
uses it.

Selected implementation sequence:
1. Add a standards-compliant multi-run scheduler readiness-proof fixture or
   fixture builder for dispatch-assignment tests. The fixture must produce
   internally consistent workflow/run/task/readiness/reservation facts for at
   least two simultaneous runs without mutating fixture internals inside tests.
   It must remain a serial integration-owner change because workflow fixtures
   are shared contract artifacts.
2. Add a cross-run grouped-claim acceptance test that uses the fixture/builder
   to prove compatible assignment-owned task-attempt facts from distinct
   workflow runs can be claimed under one batch lease, while incompatible facts
   and missing task-attempt facts fail closed. This test must still avoid
   coalesced runtime execution.
3. Define the runtime-host batch execution contract at the composition-root
   boundary before implementation. The contract must include the grouped
   runtime-host request, per-member inputs, per-member output fan-out,
   per-member diagnostics, cancellation, timeout, partial failure, retry/defer,
   and lease/release semantics. It must not carry workflow inputs from the run
   that first loaded a runtime, must not derive identity from `batching_key`,
   and must not use reservation lease ids as cross-run compatibility identity.
4. Add focused contract and boundary tests for the runtime-host batch request
   and response shape, including fail-closed diagnostics for missing member
   inputs, stale readiness proof, mismatched handoff facts, cancellation, and
   partial failure fan-out. Do this before wiring worker execution.
5. Wire the task-execution worker to claim a compatible assignment group and
   call the runtime-host batch operation once. The worker must persist and
   notify each member assignment/result independently, release or retain
   reservations according to the batch contract, and avoid marking peers
   claimed unless they are actually included in the runtime-host batch request.
6. Only after worker-level batch execution passes acceptance checks, continue
   runtime residency and keep-alive refinements needed for loaded-runtime reuse
   across future compatible workflow runs.

Rejected paths:
- Do not continue with anchor-only execution while marking compatible peers as
  batch claimed.
- Do not add a runtime-host batch API and worker grouped-claim execution in
  the same implementation slice.
- Do not treat repository grouped claims as proof of cross-run batching until
  a real multi-run fixture or builder exists.
- Do not introduce scheduler DTO, generated contract, lockfile, saved workflow
  fixture, or runtime-host API changes as incidental edits inside worker
  grouped execution.

Next thin slice: implement sequence step 1 only. Allowed files should be
limited to the scheduler/workflow-service test fixture builder or fixture
artifact that owns readiness-proof construction, focused tests proving the
fixture produces internally consistent multi-run facts, and this plan. Stop
and re-plan if fixture construction requires scheduler DTO changes, generated
contract changes, lockfile updates, saved workflow fixture updates, runtime
host API changes, or physical-device/runtime-residency tables.

2026-06-11 multi-run dispatch-assignment fixture builder slice: completed
sequence step 1. Smallest useful vertical slice: add a workflow-service-owned
dispatch-assignment test fixture builder that derives assignment, runtime
branch event, workflow run, task-attempt, readiness-proof, handoff, dispatch
decision, candidate, and reservation facts from one member definition. Allowed
files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_dispatch_assignment.rs`
and this plan.

No-fallback/no-legacy confirmation: this slice does not change production
grouping behavior, does not add coalesced runtime execution, does not mutate
saved scheduler fixture JSON, does not introduce runtime-host APIs, and does
not derive cross-run identity from `batching_key`, reservation lease id, or a
run that first loaded a runtime. The builder validates member-specific
readiness proofs and scheduler handoffs before repository insertion.

Focused test update:
`runtime_dispatch_assignment_multi_run_fixture_builder_produces_consistent_readiness_facts`
now proves two distinct workflow-run members carry internally consistent
readiness proof execution context, selected handoff, task intent, dispatch
decision, candidate resource-fit, and reservation facts.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_dispatch_assignment --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and `git diff --check`.

Next thin slice: implement sequence step 2 only by adding cross-run grouped
claim acceptance coverage using the new dispatch-assignment fixture builder.
Allowed files should remain limited to
`crates/pantograph-workflow-service/src/workflow/runtime_dispatch_assignment.rs`
and this plan unless the existing repository diagnostics cannot express the
fail-closed case. The slice must prove compatible assignment-owned
task-attempt facts from distinct workflow runs can be claimed under one batch
lease, while incompatible facts and missing task-attempt facts fail closed. Do
not add runtime-host batch execution, scheduler DTO changes, generated
contracts, lockfile updates, saved workflow fixture updates, runtime
residency/device inventory tables, or worker grouped execution in this slice.

2026-06-11 cross-run grouped-claim acceptance slice: completed sequence step
2. Smallest useful vertical slice: update dispatch-assignment repository tests
to use the multi-run fixture builder for grouped-claim acceptance, proving the
repository can claim compatible assignment-owned task-attempt facts from two
distinct workflow runs under one batch lease. Allowed files touched:
`crates/pantograph-workflow-service/src/workflow/runtime_dispatch_assignment.rs`
and this plan.

No-fallback/no-legacy confirmation: this slice remains repository/test-level
only. It does not add coalesced runtime execution, does not mark peer work as
claimed without a repository batch lease, does not add runtime-host batch APIs,
does not change scheduler DTOs or saved workflow fixtures, and still derives
compatibility from assignment-owned task-attempt facts rather than
`batching_key`, reservation lease id, or runtime-host state.

Focused test updates:
`runtime_dispatch_assignment_repository_claims_compatible_cross_run_batch`
now proves two distinct workflow runs can be included in one compatible batch
claim; `runtime_dispatch_assignment_repository_skips_incompatible_batch_candidates`
now proves a cross-run incompatible context-shape candidate remains unclaimed;
and
`runtime_dispatch_assignment_repository_rejects_cross_run_batch_candidate_without_task_attempt_facts`
proves missing candidate task-attempt facts fail closed without claiming the
anchor or candidate.

Verification passed:
`cargo test -p pantograph-workflow-service runtime_dispatch_assignment --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and `git diff --check`.

Next thin slice: implement sequence step 3 by defining the runtime-host batch
execution contract at the composition-root boundary before worker execution
uses it. Allowed files should be limited to the workflow-service or embedded
runtime boundary module that owns runtime-host dispatch contracts, focused
contract tests, and this plan. The contract must include grouped request
identity, anchor/member assignments, per-member inputs and output fan-out,
per-member diagnostics, cancellation, timeout, partial failure, retry/defer,
and lease/release semantics. Stop and re-plan if this requires generated
contract updates, frontend/Tauri DTO changes, lockfile updates, saved workflow
fixture changes, physical-device/runtime-residency tables, or wiring the
task-execution worker to execute grouped batches in the same slice.

2026-06-11 runtime-host batch execution contract slice: completed sequence
step 3. Smallest useful vertical slice: define contract-only runtime-host
batch execution request/response DTOs, validation wrappers, member policy and
disposition enums, public exports, and README traceability in
`pantograph-runtime-host-contracts`. Allowed files touched:
`crates/pantograph-runtime-host-contracts/src/runtime_host_execution.rs`,
`crates/pantograph-runtime-host-contracts/src/runtime_host_execution_tests.rs`,
`crates/pantograph-runtime-host-contracts/src/lib.rs`,
`crates/pantograph-runtime-host-contracts/src/README.md`, and this plan.

No-fallback/no-legacy confirmation: this slice defines the composition-root
runtime-host batch boundary before worker use, but does not add a runtime-host
port method, does not wire task-execution worker grouped execution, does not
change scheduler DTOs, generated contracts, lockfiles, frontend/Tauri DTOs, or
saved workflow/runtime-host fixture JSON, and does not introduce runtime
residency/device inventory tables. The contract keeps each member's handoff,
materialized inputs, timeout, failure policy, reservation policy, outputs,
diagnostics, retry/defer disposition, and reservation disposition member-owned
instead of carrying workflow inputs forward from the run that first loaded a
runtime.

Focused test updates:
runtime-host contract tests now validate a grouped request with an anchor and
two members, reject empty member sets, reject anchors that do not identify a
member, validate a partial member-failure fan-out response, and reject outputs
on failed batch members.

Verification passed:
`cargo test -p pantograph-runtime-host-contracts runtime_host_execution --lib`;
`cargo check -p pantograph-runtime-host-contracts`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-runtime-host-contracts -- --check`; and
`git diff --check`.

Next thin slice: implement sequence step 4 as boundary-test-only coverage for
the runtime-host batch contract and dispatcher boundary. Allowed files should
remain limited to `pantograph-runtime-host-contracts` contract/dispatcher
tests, any minimal contract validation refinements those tests expose, and this
plan. Cover missing member inputs at serde boundary, stale or readiness-only
handoff rejection, response/request member correlation, cancellation context
propagation shape for future batch dispatch, partial failure fan-out, and
duplicate member identity rejection. Do not add a runtime-host batch port
method, do not wire task-execution worker execution, and stop/re-plan if these
tests require generated contract changes, frontend/Tauri DTOs, lockfile
updates, saved workflow fixture updates, physical-device/runtime-residency
tables, or embedded-runtime implementation changes.

2026-06-11 runtime-host batch boundary tests slice: completed sequence step
4. Smallest useful vertical slice: add focused contract and dispatcher-boundary
tests for runtime-host batch execution without adding a runtime-host batch port
method or worker execution. Allowed files touched:
`crates/pantograph-runtime-host-contracts/src/runtime_host_execution_tests.rs`,
`crates/pantograph-runtime-host-contracts/src/runtime_host_dispatch.rs`,
`crates/pantograph-runtime-host-contracts/src/runtime_host_dispatch_tests.rs`,
and this plan.

No-fallback/no-legacy confirmation: this slice remains contract/boundary
coverage only. It does not add runtime-host batch execution, does not call the
embedded runtime, does not wire the task-execution worker, does not change
generated contracts, scheduler DTOs, frontend/Tauri DTOs, lockfiles, or saved
fixtures, and does not carry workflow inputs forward from a runtime-loading
workflow run.

Focused test updates: batch request tests now reject missing member
`materialized_inputs`, readiness-only member handoffs, duplicate member
identity, and validate batch cancellation-context shape. Dispatcher-boundary
tests now validate response/member correlation and reject unknown members and
workflow-run mismatches. Existing partial-failure fan-out coverage remains in
the contract tests.

Verification passed:
`cargo test -p pantograph-runtime-host-contracts runtime_host_execution --lib`;
`cargo test -p pantograph-runtime-host-contracts runtime_host_dispatch --lib`;
`cargo check -p pantograph-runtime-host-contracts`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-runtime-host-contracts -- --check`; and
`git diff --check`.

Discovered sequencing gap: sequence step 5 says the worker should call a
runtime-host batch operation, but the plan also rejects adding the batch API
and worker grouped execution in the same slice. Insert a runtime-host batch
port/dispatcher operation slice before worker wiring. This is a decomposition
change, not an objective change: it preserves the selected contract-first path
and keeps worker execution from introducing the API it consumes.

Next thin slice: add the runtime-host batch port and scheduler dispatcher
operation only. Allowed files should remain limited to
`crates/pantograph-runtime-host-contracts/src/runtime_host_dispatch.rs`,
`crates/pantograph-runtime-host-contracts/src/runtime_host_dispatch_tests.rs`,
possibly `crates/pantograph-runtime-host-contracts/src/lib.rs` if public
exports are needed, and this plan. The operation must validate the batch
request before the port call, pass the batch cancellation handle, validate
response/request member correlation, and validate the batch response contract.
Do not wire task-execution worker grouped execution, do not implement embedded
runtime batch execution, and stop/re-plan if this requires generated contract
updates, frontend/Tauri DTOs, lockfile updates, saved fixture changes, or
physical-device/runtime-residency tables.

2026-06-11 runtime-host batch dispatcher API slice: completed the inserted
API-only prerequisite before worker grouped execution. Smallest useful
vertical slice: add an explicit `RuntimeHostBatchExecutionPort` trait and
`SchedulerRuntimeHostBatchDispatcher` operation that validates batch requests
before the port call, passes a batch cancellation handle, validates
response/request member correlation, and validates the batch response
contract. Allowed files touched:
`crates/pantograph-runtime-host-contracts/src/runtime_host_dispatch.rs`,
`crates/pantograph-runtime-host-contracts/src/runtime_host_dispatch_tests.rs`,
`crates/pantograph-runtime-host-contracts/src/lib.rs`,
`crates/pantograph-runtime-host-contracts/src/README.md`, and this plan.

No-fallback/no-legacy confirmation: this slice adds a separate batch port
trait instead of changing the existing single-execution port trait or adding a
default fallback implementation. It does not wire task-execution worker
grouped execution, does not implement embedded-runtime batch execution, does
not call single-task execution as a batch substitute, and does not change
generated contracts, scheduler DTOs, frontend/Tauri DTOs, lockfiles, saved
fixtures, or runtime residency/device inventory tables.

Focused test updates: dispatcher tests now prove a valid batch request reaches
the batch port with the correct cancellation context, invalid readiness-only
member handoffs fail before the port call, and mismatched batch response
member correlation is rejected.

Verification passed:
`cargo test -p pantograph-runtime-host-contracts runtime_host_dispatch --lib`;
`cargo check -p pantograph-runtime-host-contracts`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-runtime-host-contracts -- --check`; and
`git diff --check`.

Next thin slice: inspect task-execution worker and composition ownership before
worker grouped execution. If the existing embedded runtime cannot provide a
real `RuntimeHostBatchExecutionPort` without looping through the single-task
runtime-host path or otherwise preserving single-member behavior, stop and
re-plan. If implementation is possible without fallback behavior, the next
worker slice may wire the task-execution worker to claim a compatible
assignment group and call `SchedulerRuntimeHostBatchDispatcher` once, while
persisting and notifying each member independently. Allowed files for that
worker slice must be identified after inspection and must not include
generated contracts, scheduler DTOs, frontend/Tauri DTOs, lockfiles, saved
fixtures, or physical-device/runtime-residency tables unless explicitly
replanned.

2026-06-11 worker grouped-execution inspection re-plan trigger: inspection
found that worker grouped execution cannot be implemented as the next code
slice without violating the no-fallback/no-legacy rule. The task-execution
worker currently creates one dispatch assignment, rehydrates one runtime
branch, and executes it through `WorkflowTaskExecutionOwner` and
`WorkflowSchedulerSessionRunner::run_started_runtime_dispatch_task_to_completion`,
which spawn a single runtime task supervisor and call the single
`SchedulerRuntimeHostDispatcher`. The embedded composition configures only a
single `RuntimeHostExecutionPort`, and `EmbeddedRuntimeHostExecutionPort`
implements only `execute_runtime_host_request`. There is no real embedded
`RuntimeHostBatchExecutionPort`, no workflow-service composition slot for a
batch dispatcher, and no batch execution owner that can persist and notify
each member independently after one grouped runtime-host call.

Do not implement a batch port by looping over
`execute_runtime_host_request`, do not execute only the anchor member and mark
peers claimed, and do not fall back to request-scoped or `WorkflowHost`
runtime execution. Those paths would preserve the legacy single-task execution
method behind a batch-shaped API and would break the selected reservation-level
batching architecture.

Re-plan options aligned with the coding standards:

1. Preferred: implement a real embedded runtime-host batch execution port
   before worker wiring. Thin slices should add fail-closed batch composition
   plumbing, then a real `EmbeddedRuntimeHostBatchExecutionPort` that validates
   a grouped request, resolves shared package/load-target facts once per
   compatible batch, executes each member through a batch-owned gateway
   operation without calling the single runtime-host port as fallback, writes
   member artifacts independently, and returns member-level retry and
   reservation dispositions. Follow with workflow-service batch owner/wiring
   slices that claim compatible assignments and persist each member result.
2. Contract-first workflow-service plumbing: add the workflow-service
   batch-dispatcher dependency and fail-closed worker guard first, but keep
   grouped assignment claiming disabled until a real embedded batch port
   exists. This is standards-compliant because it fails closed, but it does
   not deliver usable grouped execution until the embedded batch port lands.
3. Defer worker wiring and deepen mapping tests first: add tests/contracts for
   converting assignment batches into runtime-host batch member requests and
   converting member responses into task result, reservation lifecycle, and
   notification mutations. This reduces integration risk but still leaves
   execution blocked on the real embedded batch port.

2026-06-11 worker grouped-execution re-plan decision: use option 1. Build the
real embedded runtime-host batch execution port before worker grouped execution
is wired. The batch port is the canonical executor for grouped runtime-host
work; it must not call `RuntimeHostExecutionPort::execute_runtime_host_request`
in a member loop, must not execute only the anchor member while claiming peers,
must not fall back to request-scoped or `WorkflowHost` runtime execution, and
must not carry workflow inputs or ownership forward from the workflow run that
first caused a runtime/model to be loaded. Runtime/model residency remains
scheduler and reservation controlled; physical device inventory and runtime
residency tables remain out of scope unless a later explicit re-plan adds
them.

Option 1 execution sequence:

1. Add fail-closed batch composition plumbing without enabling grouped worker
   execution. The workflow-service scheduler task orchestrator should be able
   to own a `SchedulerRuntimeHostBatchDispatcher` backed by an unavailable
   `RuntimeHostBatchExecutionPort`, and service config/composition should be
   able to inject a real batch port later. Allowed files for this slice should
   remain limited to workflow-service scheduler/service configuration tests and
   code, embedded composition injection code if needed, README traceability,
   and this plan. Do not claim runtime dispatch assignment batches in the
   worker yet.
2. Implement the real embedded runtime-host batch execution port. This port
   must validate the grouped request, handle the shared cancellation context,
   resolve package/load-target facts through batch-owned logic, preserve
   member-specific materialized inputs and outputs, write member artifacts
   independently, and return member-level retry and reservation dispositions.
   It may share pure projection/artifact helper functions with the single-task
   port after refactoring, but it must not delegate execution to the
   single-task runtime-host port. If the inference gateway cannot execute
   multiple batch members against the selected runtime/model without preserving
   the single-task port path, stop and re-plan for a gateway-level batch
   operation.
3. Add workflow-service batch request/response mapping ownership before worker
   dispatch. Convert claimed compatible assignment groups into
   `RuntimeHostBatchExecutionRequest` members, and convert batch member
   responses into scheduler task result mutations, reservation lifecycle
   events, retry/defer decisions, and per-run notifications. Keep this owner in
   backend workflow-service code and cover boundary invariants before worker
   activation.
4. Wire the task-execution worker to claim compatible running assignment
   groups and call `SchedulerRuntimeHostBatchDispatcher` once per group. Each
   member must persist, notify, retry/defer, and release/retain reservations
   independently. The worker must fail closed when a real batch dispatcher is
   unavailable or when group correlation is invalid.
5. Run focused batch-port, workflow-service mapping, worker grouped execution,
   and existing runtime-branch dispatch checks. Then run the relevant crate
   checks and format checks before resuming later milestones.

Next thin slice: implement sequence step 1 only. Identify the smallest
fail-closed composition slice and allowed write set before editing. Do not
change generated contracts, scheduler DTOs, frontend/Tauri DTOs, lockfiles,
saved workflow fixtures, physical-device/runtime-residency tables, or worker
grouped execution in this slice. Stop and re-plan if adding the fail-closed
batch dispatcher requires changing the public runtime-host contract API that
was just stabilized.

2026-06-11 fail-closed runtime-host batch composition slice: completed option
1 sequence step 1. Smallest useful vertical slice: give
`WorkflowSchedulerTaskOrchestrator` an owned `SchedulerRuntimeHostBatchDispatcher`
slot backed by an unavailable `RuntimeHostBatchExecutionPort` by default, add
`WorkflowService::with_runtime_host_batch_execution_port` for later real port
injection, and cover both default fail-closed behavior and explicit injection
with focused tests. Allowed files touched:
`crates/pantograph-workflow-service/src/scheduler/task_orchestrator.rs`,
`crates/pantograph-workflow-service/src/scheduler/task_orchestrator_tests.rs`,
`crates/pantograph-workflow-service/src/workflow/service_config.rs`,
`crates/pantograph-workflow-service/src/scheduler/README.md`, and this plan.

No-fallback/no-legacy confirmation: no worker grouped assignment claiming was
enabled, no embedded runtime batch executor was faked, no single-task
`RuntimeHostExecutionPort` loop was added behind the batch API, no anchor-only
batch execution path was added, and no generated contracts, scheduler DTOs,
frontend/Tauri DTOs, lockfiles, saved fixtures, physical-device tables, or
runtime-residency tables were changed. Embedded runtime composition injection
is intentionally deferred until the real embedded batch port exists, so this
slice does not require a fake embedded batch dependency.

Focused test updates:
`orchestrator_fails_closed_when_runtime_host_batch_port_is_unconfigured`
proves default construction rejects batch dispatch through the unavailable
batch port; `orchestrator_dispatches_runtime_batch_through_injected_batch_port`
proves explicit orchestrator injection passes a validated batch request and
running cancellation snapshot to the batch port; and
`workflow_service_injects_runtime_host_batch_execution_port` proves the
workflow-service configuration hook wires the batch port into the orchestrator.

Verification passed:
`cargo test -p pantograph-workflow-service orchestrator_fails_closed_when_runtime_host_batch_port_is_unconfigured --lib`;
`cargo test -p pantograph-workflow-service orchestrator_dispatches_runtime_batch_through_injected_batch_port --lib`;
`cargo test -p pantograph-workflow-service workflow_service_injects_runtime_host_batch_execution_port --lib`;
`cargo check -p pantograph-workflow-service`;
`cargo fmt -p pantograph-workflow-service -- --check`; and
`git diff --check`.

Next thin slice: begin option 1 sequence step 2 by inspecting the embedded
runtime-host single execution port, image projection, media artifact sink,
package/load-target resolvers, and inference gateway API to identify the
smallest real batch-port slice. Do not implement batch execution by delegating
to `execute_runtime_host_request`, do not preserve request-scoped or
`WorkflowHost` runtime execution, and stop/re-plan if the inference gateway
cannot accept a batch-owned operation without a gateway-level contract change.

2026-06-11 embedded batch-port inspection re-plan trigger: inspection found
that the current inference gateway cannot yet support a real multi-member
runtime-host batch operation without adding a gateway-level batch contract.
`EmbeddedRuntimeHostExecutionPort` projects exactly one validated
`RuntimeHostExecutionRequest` into one `ImageGenerationPlanningInput` and then
calls `InferenceGateway::generate_image_from_planning_input_with_cancellation`.
The inference gateway and `InferenceBackend` trait expose single planned image
generation calls (`generate_image_from_plan_with_cancellation` and
`generate_image_from_plan`) for one `ImageGenerationExecutionPlan`; there is no
gateway-owned operation that accepts multiple planned image-generation members
as one scheduler-owned batch. A real embedded `RuntimeHostBatchExecutionPort`
therefore cannot be implemented next without either adding a gateway/backend
batch contract or violating the plan by looping single-member execution.

Rejected paths: do not implement embedded batch execution by looping
`RuntimeHostExecutionPort::execute_runtime_host_request`, do not loop
`InferenceGateway::generate_image_from_planning_input_with_cancellation` once
per batch member behind the batch port, do not execute only the anchor member,
and do not use `ImageGenerationRequest` image count as a substitute for
multiple workflow/task members. Image count is one member's output cardinality,
not a scheduler batch of independently persisted workflow runs.

Re-plan options aligned with the coding standards:

1. Preferred: add a gateway-level image-generation batch contract first. Define
   explicit request/member/response DTOs in the inference crate, add a gateway
   operation that validates/plans all members before backend execution,
   preserves member-specific inputs/results/diagnostics, and fails closed when
   the active backend lacks batch support. Then implement backend support in a
   separate slice and wire the embedded runtime-host batch port to that
   gateway operation.
2. Add a fail-closed gateway batch facade now, with no backend execution. This
   would let the embedded batch port compile against the intended boundary and
   return typed unsupported diagnostics until real backend batch execution
   lands. It is standards-compliant but does not produce usable grouped
   execution.
3. Re-scope the immediate runtime-host batch port to planning/projection only.
   Build and test conversion from runtime-host batch members into planned
   image-generation members, then stop before execution. This reduces mapping
   risk but keeps execution blocked on option 1.

Recommended path: use option 1. It creates the missing canonical gateway
boundary required for a real batch executor and avoids adding another
fail-closed layer around an absent backend capability.

2026-06-13 embedded batch-port re-plan decision: use option 1. Add the
canonical gateway-level image-generation batch contract before implementing
the embedded runtime-host batch execution port. The gateway/backend boundary is
the missing owner for executing multiple planned image-generation members as
one scheduler-owned batch; embedded runtime must consume that boundary instead
of manufacturing batch semantics by looping single-member gateway or
runtime-host calls.

Option 1 gateway-batch execution sequence:

1. Completed 2026-06-13: add inference-crate image-generation batch DTOs and
   validation tests. The new `image_generation_batch` module defines owned
   `ImageGenerationBatchExecutionRequest` / response/member DTOs with stable
   batch and member ids, original request plus planned execution facts per
   member, bounded diagnostics, and explicit batch/member states. Validation
   rejects duplicate member ids, unknown anchors, request/plan correlation
   drift, invalid terminal member results, missing terminal diagnostics, and
   inconsistent batch terminal states. This slice did not use borrowed
   `ImageGenerationPlanningInput` as a shared DTO, did not conflate one
   member's image count with multiple workflow/task members, did not change
   generated runtime-host contracts, and did not add any loop-based fallback
   execution path.
2. Completed 2026-06-13: add a gateway operation that validates the planned
   batch contract before backend execution, preserves member correlation
   through the owned batch/member DTOs, and returns typed diagnostics when the
   active backend does not support batch execution. The new
   `InferenceGateway::generate_image_batch_from_execution_request` and
   cancellation-aware variant reject invalid contracts before touching the
   backend and reject cancelled batches before validation/dispatch. Because
   backend batch execution does not exist yet, the gateway currently fails
   closed with `unsupported_batch_execution` instead of looping through the
   single planned-image method.
3. Completed 2026-06-13 for backend trait support only: add
   `BackendCapabilities::image_generation_batch`, add
   `InferenceBackend::generate_image_batch_from_execution_request` with a
   default unsupported implementation, and update the gateway to dispatch to
   that trait method only when the active backend explicitly advertises batch
   support. Gateway response validation now rejects invalid response contracts
   and mismatched response/request member ids. Real PyTorch/diffusers or
   Python worker batch execution remains a separate future slice and must only
   be implemented where the backend can execute a true multi-member batch
   without looping through the single planned-image method as hidden fallback.
4. Implement `EmbeddedRuntimeHostBatchExecutionPort` against the gateway batch
   operation. The embedded port may share pure projection and artifact-writing
   helpers with the single-task port, but it must not delegate execution to
   `RuntimeHostExecutionPort` or call the single gateway operation per member.
5. Resume workflow-service batch mapping and worker grouped dispatch only
   after the embedded batch port is backed by the canonical gateway batch
   operation.

Step 1 verification:
`cargo test -p inference image_generation_batch --lib` passed,
`cargo check -p inference` passed, and
`cargo fmt -p inference -- --check` passed.

Step 1 files touched:
`crates/inference/src/image_generation_batch.rs`,
`crates/inference/src/image_generation_batch_tests.rs`,
`crates/inference/src/lib.rs`, `crates/inference/src/README.md`, and this
plan.

Step 2 verification:
`cargo test -p inference generate_image_batch --lib` passed,
`cargo check -p inference` passed, and
`cargo fmt -p inference -- --check` passed.

Step 2 files touched:
`crates/inference/src/gateway.rs`, `crates/inference/src/gateway_tests.rs`,
`crates/inference/src/README.md`, and this plan.

Step 2 deviation:
the gateway entrypoint accepts planned batch members and validates
request/plan correlation; it does not add a second gateway-local borrowed
planning DTO. This keeps the contract owned and avoids carrying borrowed
workflow inputs through the batch boundary. Per-member planning rejection will
be represented by the caller that builds planned members until a future slice
adds an explicit owned batch planning DTO.

Step 3 verification:
`cargo test -p inference generate_image_batch --lib` passed,
`cargo check -p inference` passed, and
`cargo fmt -p inference -- --check` passed.

Step 3 files touched:
`crates/inference/src/backend/mod.rs`, `crates/inference/src/backend/llamacpp.rs`,
`crates/inference/src/gateway.rs`, `crates/inference/src/gateway_tests.rs`,
`crates/inference/src/README.md`, and this plan.

Step 3 deviation:
`crates/inference/src/backend/llamacpp.rs` needed a mechanical
`image_generation_batch: false` static-capability field because its
`BackendCapabilities` literal is exhaustive. This does not enable llama.cpp
batch execution and does not change backend behavior.

Next thin slice: decide whether to implement a real backend batch executor or
pause gateway/backend work and return to the embedded runtime-host batch port.
Do not wire `EmbeddedRuntimeHostBatchExecutionPort` to the gateway until at
least one backend can execute the batch method without looping through
single-image execution, unless the product decision is to keep embedded batch
execution fail-closed until backend support lands. If implementing real backend
execution next, inspect the PyTorch image-generation worker envelope and stop
to re-plan if true multi-member diffusion batching requires Python-side
contract or scheduler/runtime-residency changes.

2026-06-13 backend batch execution re-plan decision: implement real backend
batch execution next, with a PyTorch/Diffusers worker-envelope inspection gate
before any source changes. This chooses the standards-aligned contract-first
path that can make grouped workflow execution actually usable. Do not return
to embedded runtime-host batch wiring as a usable execution path until a real
backend batch executor exists. Do not add a Rust-side loop over
`generate_image_from_plan`, do not execute only the anchor member, do not use
workflow-service or embedded runtime to manufacture backend batch semantics,
and do not enable `BackendCapabilities::image_generation_batch` for PyTorch
until the backend method executes a true multi-member batch contract.

Backend batch execution sequence:

1. Inspection gate only: inspect
   `crates/inference/src/backend/pytorch_image_generation.rs`,
   `crates/inference/src/backend/pytorch_worker_image_contract.rs`,
   `crates/inference/torch/worker_image_contract.py`, the focused
   PyTorch worker contract/python tests, and the worker JSON fixtures to
   determine whether the existing worker envelope can represent a
   multi-member image-generation batch without preserving member state from
   unrelated workflow runs. Record the finding in this plan before source
   edits. If true batch execution requires a new Python/Rust envelope, stop
   and run step 2 instead of modifying backend execution.
2. If the inspection gate finds the current worker envelope cannot express
   true batch execution, add a dedicated PyTorch image batch worker contract
   first. Allowed files should be limited to
   `crates/inference/src/backend/pytorch_worker_image_contract.rs`,
   `crates/inference/torch/worker_image_contract.py`, focused Rust/Python
   contract tests, worker JSON fixtures, `crates/inference/src/README.md`, and
   this plan. The contract must preserve stable batch/member ids, member-local
   inputs and planned execution facts, per-member success/failure diagnostics,
   cancellation behavior, and resource-observation semantics. Do not call the
   worker from the backend in this slice.
3. Implement PyTorch backend batch execution against the frozen batch worker
   contract. Allowed files should be limited to PyTorch image backend/worker
   bridge files, focused PyTorch backend tests, capability facts, README, and
   this plan. The backend may set `image_generation_batch: true` only in the
   same slice that proves the batch method executes the new backend-owned
   batch operation. It must not loop through the gateway or single planned
   image backend method.
4. After a real backend batch method passes verification, implement
   `EmbeddedRuntimeHostBatchExecutionPort` against the gateway batch operation.
   The embedded port may share pure projection and artifact-writing helpers
   with the single-task port, but it must not delegate execution to
   `RuntimeHostExecutionPort` or call the single gateway operation per member.
5. Resume workflow-service batch mapping and worker grouped dispatch only
   after the embedded batch port is backed by the real gateway/backend batch
   operation and has focused fan-out, partial-failure, cancellation, and
   diagnostics tests.

Rejected paths:
- Do not implement embedded runtime-host batch execution first as a usable path
  while every backend remains unsupported; that would only move the fail-closed
  boundary outward and would not deliver grouped execution.
- Do not use a compatibility shim that loops over single-image gateway or
  backend calls behind a batch-shaped API.
- Do not change generated runtime-host contracts, workflow-service worker
  behavior, frontend/Tauri DTOs, lockfiles, saved workflow fixtures,
  physical-device tables, or runtime-residency tables during the PyTorch
  worker inspection or worker-envelope contract slices.
- Do not use sub-agents for the worker-envelope contract, generated DTOs,
  shared gateway/backend contracts, lockfiles, saved workflow fixtures, or this
  plan; those remain serial integration-owner work.

## Standards Rule

The standards constraints in
`02-image-generation-family-planner.md#standards-guardrails` and
`02-image-generation-family-planner.md#standards-compliance-matrix`, plus the
device/runtime constraints in `06-device-runtime-selection.md`, are binding
for every milestone. The directory READMEs in this plan are also part of the
documentation-traceability contract for the split plan structure. If
implementation needs to violate one of these constraints, stop and re-plan
before editing production code.
