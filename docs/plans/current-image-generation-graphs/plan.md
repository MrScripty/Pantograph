# Plan: Current Image Generation Graphs And Stale Graph Diagnostics

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

## Standards Rule

The standards constraints in
`02-image-generation-family-planner.md#standards-guardrails` and
`02-image-generation-family-planner.md#standards-compliance-matrix`, plus the
device/runtime constraints in `06-device-runtime-selection.md`, are binding
for every milestone. The directory READMEs in this plan are also part of the
documentation-traceability contract for the split plan structure. If
implementation needs to violate one of these constraints, stop and re-plan
before editing production code.
