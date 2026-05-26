# crates/pantograph-workflow-service/src

Host-agnostic workflow service source boundary.

## Purpose
This directory owns Pantograph workflow application-service contracts and
orchestration entrypoints. It keeps workflow execution, graph mutation,
scheduler queues, runtime preflight, technical-fit request shaping, and trace
diagnostics reusable across Tauri, UniFFI, Rustler, and tests.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Public module exports for the workflow service crate. |
| `workflow.rs` | Public workflow facade exports, execution/session facade methods, and orchestration logic. |
| `workflow/` | Private workflow contracts, execution-plan DTOs, host traits, graph API methods, diagnostics ledger query methods, capability/preflight API methods, workflow run and session execution API methods, queue and lifecycle API methods, service configuration, request validation, I/O derivation, runtime preflight, and session-runtime helpers extracted from the main facade. |
| `scheduler/` | Backend-owned workflow-session queue/store contracts used by the workflow facade. |
| `scheduler/task_orchestrator.rs` | Workflow-service async shell for task orchestration calls into shared scheduler/runtime-host contracts without owning runtime policy or embedded-runtime implementation details. |
| `trace/` | Workflow trace contracts, request validation, in-memory trace state, and runtime/scheduler snapshot merge helpers. |
| `graph/` | Graph DTOs and session-kind contracts shared by service operations. |
| `technical_fit.rs` | Technical-fit request/decision DTOs, dependency-readiness proof DTOs, normalization helpers, session context assembly, and runtime-preflight integration. |
| `capabilities.rs` | Shared workflow capability and validation utilities. |

## Problem
Workflow behavior crosses UI, runtime, diagnostics, and binding boundaries.
Without one host-agnostic source owner, adapters can drift on execution ids,
session affinity, graph edits, runtime readiness, queue policy, and trace
semantics.

## Constraints
- No transport-framework dependencies such as Tauri, UniFFI, Rustler, or UI
  packages.
- Host and runtime dependencies enter through traits and DTOs.
- Public response shapes are consumed by frontend stores and generated/native
  bindings.
- Runtime install/remove/status mutations remain outside this crate.
- Runtime preflight and scheduler decisions must stay backend-owned.

## Decision
Keep public workflow orchestration in this source directory. `workflow.rs`
remains the compatibility facade while cohesive contracts and internals move
into focused private modules. Adapters may translate transport payloads but
must delegate workflow decisions to this crate.
Session runtime preflight cache fingerprinting now lives with the
session-runtime helper that owns cache lookup and refresh behavior.
Session runtime loaded-state invalidation now lives with the same helper that
owns runtime load and unload transitions.
Graph edit-session and persistence methods now live behind the facade in the
workflow graph API helper.
Workflow capability, I/O discovery, and preflight methods now live behind the
facade in the workflow preflight API helper.
Generic workflow run execution now lives behind the facade in the workflow run
API helper.
Service construction, capacity-limit configuration, diagnostics-provider setup,
diagnostics-ledger setup, and the session-store guard now live in the workflow
service configuration helper. The root workflow facade tests now live in
`workflow/tests.rs`; shared
test fixture families now live under `workflow/tests/fixtures/` and are
re-exported by `workflow/tests/fixtures.rs`. Scheduler snapshot facade coverage
now lives in `workflow/tests/scheduler_snapshot.rs`, while scheduler admission,
runtime-registry, and rebalance diagnostics coverage lives in
`workflow/tests/scheduler_snapshot_diagnostics.rs`,
and session queue item/admission coverage now lives in
`workflow/tests/session_queue.rs`. Workflow capability discovery and default
capability derivation coverage now lives in
`workflow/tests/workflow_capabilities.rs`. Workflow I/O discovery and validation
coverage now lives in `workflow/tests/workflow_io.rs`, and workflow preflight
coverage now lives in `workflow/tests/workflow_preflight.rs`. Runtime preflight
policy coverage now lives in `workflow/tests/runtime_preflight.rs`. Private
workflow run implementation coverage now lives in `workflow/tests/workflow_run.rs`.
Workflow DTO serialization and error-envelope coverage now lives in
`workflow/tests/contracts.rs`. Workflow session execution and retention-hint
coverage now lives in `workflow/tests/session_execution.rs`. Session and
runtime capacity limit/error coverage now lives in
`workflow/tests/session_capacity_limits.rs`, while runtime capacity rebalance
coverage lives in `workflow/tests/session_capacity.rs`.
Runtime capacity/admission wait coverage now lives in
`workflow/tests/session_admission.rs`. Session runtime preflight cache and
keep-alive preflight failure coverage now lives in
`workflow/tests/session_runtime_preflight.rs`. Session runtime loaded-state
invalidation coverage now lives in `workflow/tests/session_runtime_state.rs`.
Session stale cleanup, inspection, and stale cleanup worker coverage now lives in
`workflow/tests/session_stale_cleanup.rs`.
Validated workflow identity grammar now lives in `workflow/identity.rs` and is
used by workflow service validation plus saved graph persistence boundaries, so
Stage 01 versioning work has one stable id contract instead of filesystem
sanitization.
Session creation and queued session run methods now live behind the facade in
the workflow session execution API helper.
Session status, queue inspection, scheduler snapshot, session-scoped
cancellation/reprioritization/push-front methods, and the first GUI-admin
queued-run cancel/reprioritize/push-front boundaries now live behind the facade
in the workflow session queue API helper.
Stale cleanup, stale cleanup worker, keep-alive, and close-session methods now
live behind the facade in the workflow session lifecycle API helper.
Model/license usage diagnostics query methods now live behind the facade in the
workflow diagnostics API helper and delegate to `pantograph-diagnostics-ledger`
for storage and query semantics.
Scheduler estimate query methods also live in the diagnostics API helper and
read the hot run-detail projection so clients can request estimate facts
without parsing raw event payloads. Those estimate DTOs preserve typed
scheduler model-cache posture from the run-detail projection.
Run-inspection query methods live in the diagnostics API helper and compose
the immutable run graph, run detail, node-status projection, I/O artifact
descriptors, retention summary, and per-source projection states from existing
persisted sources. The query is a presentation-neutral read model; it must not
create a second persistence subsystem or define frontend grouping, labels,
visual order, or selected-node behavior.
Retention cleanup now lives behind the same diagnostics API helper and is
re-exported by the crate facade so Tauri and frontend command adapters can
trigger backend-owned artifact expiration without touching ledger internals.
Library asset access audit recording and trusted diagnostic event append now
live behind the diagnostics API helper and are re-exported by the crate facade
so adapters can record typed Pumas/Library operations and bounded runtime facts
without owning diagnostic ledger storage.
Local Network status now carries scheduler-owned run placement plus typed
model-cache posture through workflow-service DTOs so adapters can render
selected-run Network facts without reconstructing scheduler or diagnostics
payload state.
Workflow backend task capability DTOs now carry optional
`WorkflowTaskRequestContract` payload metadata projected from backend facts.
Workflow runtime requirement extraction is intentionally node-family explicit:
canonical `llm-inference` reads only the graph-authored `runtime` input as a
hard scheduler requirement, while dedicated legacy node families such as
`onnx-inference` and `audio-generation` map from their node type. The workflow
service must not recursively scan arbitrary node JSON for `backend_key`, Pumas
package hints, dependency bindings, or GGUF paths when building scheduler
runtime requirements.
Workflow technical-fit and run execution-plan DTOs now carry reduced
dependency-readiness proof as scheduler/admission evidence. The proof is
transported as workflow-owned DTOs only; this crate does not infer package
readiness from graph inputs, diagnostics messages, runtime display names, or
worker behavior.
The contract describes task input/result families and executable versus
contract-only status without encoding runtime selection, scheduler admission, or
backend residency policy.
Workflow backend capability DTOs now also carry runtime variant capability
facts and typed device diagnostics projected from inference. Workflow-service
stores and transports these backend-owned facts; it does not derive runtime
readiness, choose devices, or translate raw backend device strings.
Workflow technical-fit requests may carry typed device policy intent. That
intent is normalized and forwarded to backend/runtime selectors, but this crate
must not turn explicit device requests into executable defaults or infer
backend-local device syntax.
Workflow technical-fit override intent may name runtime id, runtime variant id,
model id, or backend key. Workflow-service normalizes and transports that
intent, while runtime-registry owns candidate matching and rejection
diagnostics.
Workflow capability extraction treats canonical inference-node `runtime` input
as graph-authored scheduler intent. Omitted `runtime` values leave scheduler
policy free to choose among valid executable candidates. Explicit values become
hard scheduler requirements only after selector validation, and package facts
such as `recommended_backend` remain dependency/capability evidence rather than
required runtime constraints.
Workflow model discovery uses explicit model identity fields such as
`model_id` and `pumas_model_ref.model_id`. It does not derive model ids from
`model_path`, `entry_path`, or `selected_artifact_path`; Pumas owns path to
model interpretation and Pantograph consumes Pumas-owned model references or
artifact load targets.
Workflow technical-fit decisions now also mirror selected runtime variant,
device class/id, resource estimates, observed-throughput hints, and bounded
device diagnostics from backend/runtime selectors. These fields are transport
facts only; workflow-service does not rank devices or infer missing values from
backend ids, runtime ids, or raw device strings.
Workflow execution-plan DTOs now define the run-scoped contract that scheduler
admission will use to hand selected per-node backend/runtime/device decisions
to embedded-runtime projection. The DTO carries schema version, workflow/run
ids, node ids, selected task/runtime/device facts, optional model ref, bounded
diagnostics, and trace ids only; it must not carry graph inputs, full Pumas
facts, worker envelopes, image bytes, local paths, or mutable scheduler
internals.
Workflow scheduler task graph projection now lives behind the workflow facade
as a path-free bridge from validated workflow topology to scheduler-owned task
contracts. It preserves graph dependency bindings and canonical scheduler
identifier parsing, emits typed projection diagnostics for missing canonical
inference facts, and never treats legacy `model_path` or `model_ref` fields as
runtime identity.
Executable validation snapshots now live in `workflow/executable_validation_snapshot.rs`.
They are the path-free, version-keyed service contract that will let executable
publish persist validated inference authority and queue admission fail closed
before scheduler graph materialization. The snapshot stores only bounded typed
execution facts such as workflow version, graph revision, descriptor
fingerprints, task kind, Pumas model ref, explicit runtime/device constraints,
trait settings, estimate hints, availability status, and diagnostics; it does
not store local model paths, full Pumas package facts, frontend presentation
state, media payloads, or scheduler placement decisions.
Workflow-service serializes those typed snapshots into attribution-owned opaque
storage keyed by `WorkflowVersionId` and validates all returned attribution
metadata before using the compact snapshot for scheduler projection. Attribution
owns durability and version identity; workflow-service owns snapshot schema,
serde, validation, and fail-closed projection.
Workflow scheduler task orchestration is a service-owned composition boundary.
`WorkflowService` owns a `WorkflowSchedulerTaskOrchestrator`, exposes an
explicit runtime-host execution port configuration hook, and installs a
typed-unavailable runtime-host port by default so production wiring fails
closed until the embedded runtime supplies the shared port. Session execution
now initializes active-run scheduler task state from the immutable task graph
after queue admission and before the current whole-run execution path
continues. Full task progression, runtime-host dispatch lifecycle, and old
node-engine output-demand launch removal remain staged Milestone 5c work.
The first scheduler-task execution entrypoint is intentionally narrow: it
executes only active-run tasks already in ready non-runtime node-engine state,
transitions them through running under the active-run store lock, returns a
started task payload that can be executed after the lock is dropped, and then
commits success or failure through a fresh store mutation. Runtime inference
tasks are rejected before node-engine execution, and adapter failures move the
task to terminal failed without storing a successful result.
Dependent non-runtime readiness advancement is also scheduler-owned. The
orchestrator validates active-run task bindings against materialized scheduler
task results and moves `AwaitingInputs` tasks only to ready, input-unavailable,
or invalid scheduler states; missing upstream results stay blocked without
calling graph output demand or passing raw graph data to node-engine.
Artifact format metadata retains typed conversion status, conversion command
identity, conversion id, and per-conversion dependency lease attribution
fields. Scheduler-task output materialization is the replacement boundary for
new artifactization behavior; the old whole-run host-injected media conversion
path is removed.

## Alternatives Rejected
- Keep workflow behavior in Tauri commands: rejected because native bindings
  and tests need the same backend behavior without desktop transport.
- Let frontend stores reconstruct graph mutation or diagnostics truth:
  rejected because backend-owned responses are the source of truth.
- Move runtime readiness into runtime adapters: rejected because preflight and
  runtime-not-ready semantics are workflow service contracts.

## Invariants
- Workflow execution/session identity is owned here and exposed through public
  DTOs.
- Workflow identity validation is centralized through `WorkflowIdentity`; saved
  workflow names, execution requests, capabilities, I/O, preflight, and future
  version records must not use independent identity grammars.
- Edit-session graph mutations, including collapsed node group create,
  ungroup, and port-mapping changes, return backend-owned snapshots that
  adapters render directly.
- `workflow_get_io` exposes only nodes marked as input/output with
  `io_binding_origin == "client_session"`.
- Workflow execution never triggers runtime installation implicitly.
- Session runtime preflight cache keys include graph fingerprint, runtime
  capability fingerprint, and normalized technical-fit override selection.
- Technical-fit decisions with fallback selection or missing candidate/runtime
  state are blocking runtime diagnostics; workflow execution must not proceed
  from those selected-runtime facts.
- Technical-fit decisions with error-severity device diagnostics are blocking
  runtime diagnostics, including explicit device requests that canonical
  planning cannot satisfy.
- Technical-fit diagnostic DTOs carry scheduler-facing attribution fields from
  runtime-registry decisions. Workflow-service normalizes and transports these
  fields; it must not derive evidence meaning from diagnostic messages.
- Execution-plan DTOs are correct-by-construction. Public builders and serde
  deserialization must reject missing selected node-decision facts, unknown
  task/device selections, oversized diagnostics, unsupported schema versions,
  and node-decision key mismatches before embedded-runtime receives a plan.
- Workflow capability extraction does not treat legacy `runtime_hint` values
  as backend requirements. Current workflow backend requirements must come from
  explicit backend/package facts until typed backend preference intent replaces
  raw workflow strings.
- Graph edit helpers and KV-cache memory-impact summaries do not treat legacy
  `runtime_hint`, `resolved_model_source`, or `resolved_model_package_facts`
  fields as current runtime/backend/model identity signals.
- Host calls that load/unload runtimes occur outside session-store locks.
- Trace stores own canonical event timestamps, idempotent terminal replay, and
  retry/reset behavior for repeated execution ids.
- Scheduler snapshots omit execution attribution when identity is ambiguous.
- Service configuration API methods preserve constructor defaults, capacity
  bounds, and shared lock error mapping behind the facade.
- Workflow facade test coverage stays in the workflow helper directory so the
  production facade file remains small enough to review directly.
- Workflow run API methods preserve timeout cancellation, output validation,
  and runtime-readiness checks as backend-owned behavior.
- Session execution API methods preserve queue admission, runtime preflight,
  runtime load, and run finalization as one backend-owned workflow.
- Session queue API methods preserve the public facade while keeping direct
  scheduler-store access in the workflow session queue helper.
- Session lifecycle API methods preserve the public facade while keeping
  cleanup, keep-alive, close, and runtime unload side effects together.
- Diagnostics usage query API methods preserve the public facade while keeping
  durable ledger storage and retention semantics in
  `pantograph-diagnostics-ledger`.
- Diagnostics projection re-exports include typed I/O artifact retention state
  and retention summary records so adapter callers can preserve serialized
  service contracts without depending on private ledger modules.
- Run-inspection DTOs expose backend-owned graph/status/artifact facts and
  projection cursors only. Artifact bodies stay behind ArtifactStore read or
  stream APIs, and display composition stays in frontend presenters.
- Diagnostics projection re-exports include run-list facet records so adapter
  callers can consume backend-owned comparison counts without depending on
  private ledger modules or sampled frontend pages.
- Run-list query request DTOs preserve client, client-session, bucket, and
  accepted-at range filters so adapter callers can narrow projection reads
  without raw ledger access.
- Retention cleanup request/response DTOs are public workflow-service
  contracts and must preserve backend cleanup counts rather than client-side
  artifact deletion state.
- Library asset access audit request/response DTOs are public workflow-service
  contracts and must preserve typed operation/cache labels rather than
  adapter-authored free-form audit rows.
- Artifact descriptor format metadata is a public contract. Real conversion
  fields must be added and mapped together across workflow-service,
  diagnostics-ledger, Tauri, embedded runtime, frontend contracts, and contract
  tests so conversion attribution does not drift across boundaries.
- Workflow-service must remain host-agnostic for managed media conversion:
  hosts inject a neutral executor, while lease acquisition, executable path
  resolution, command execution, and inference/Tauri-specific dependency state
  stay outside this crate.

## Revisit Triggers
- Public workflow DTOs need versioning rather than additive migration.
- Scheduler or diagnostics persistence becomes durable across app restarts.
- The `workflow.rs` facade decomposition requires public module changes.
- Runtime lifecycle supervision moves into a dedicated backend manager.

## Dependencies
**Internal:** `node-engine`, `workflow-nodes`, `pantograph-runtime-identity`,
`pantograph-diagnostics-ledger`, and sibling source modules in this crate.

**External:** `async-trait`, `serde`, `serde_json`, `thiserror`, `tokio`,
`uuid`, `chrono`, and `parking_lot`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- `docs/adr/ADR-011-scheduler-only-workflow-execution.md`

## Usage Examples
```rust
use pantograph_workflow_service::{
    WorkflowExecutionSessionCreateRequest, WorkflowExecutionSessionRunRequest, WorkflowService,
};
```

## API Consumer Contract
- Inputs: public request DTOs, workflow ids, graph edit/session ids,
  host-trait implementations, runtime capabilities, and technical-fit override
  selections.
- Outputs: public response DTOs for session runs, capabilities, IO discovery,
  preflight, sessions, queues, graph mutations, traces, and diagnostics.
- Lifecycle: hosts create a service, call workflow/session operations, and
  explicitly close sessions; the service owns scheduler and graph-session state.
- Errors: invalid requests, missing workflows/sessions, runtime-not-ready
  conditions, cancellations, capacity exhaustion, and host failures surface as
  `WorkflowServiceError`.
- Versioning: DTO changes should be additive unless Tauri, frontend, UniFFI,
  Rustler, examples, and contract tests migrate together.

## Structured Producer Contract
- Stable fields: workflow responses, graph mutation responses, queue records,
  runtime issues, technical-fit decisions, trace snapshots, and scheduler
  diagnostics are machine-consumed.
- Defaults: omitted optional fields use service-defined defaults and must be
  covered by contract tests when observable.
- Enums and labels: workflow/session states, runtime readiness states, queue
  statuses, trace statuses, and issue categories carry behavior.
- Ordering: queue, trace, runtime issue, and diagnostics ordering are part of
  observable behavior where clients display or compare sequences.
- Compatibility: saved workflows, frontend stores, and binding consumers may
  depend on serialized field names and semantics across releases.
- Canonical inference migration inventory tracks `task_kind`, `pumas_model_ref`,
  `resolved_model_source`, `resolved_model_package_facts`, runtime hints, typed
  options, and migration diagnostics as the stable graph data fields for
  migrated `llm-inference` nodes.
- Required backend extraction treats GGUF artifact evidence as a llama.cpp
  requirement and suppresses conflicting backend hints inside that model-fact
  context; workflow-service must not route GGUF through Candle/PyTorch hints.
- Regeneration/migration: response-shape changes must update Tauri wire
  contracts, frontend stores, binding wrappers, examples, and contract tests in
  the same slice.

## Testing
```bash
cargo test -p pantograph-workflow-service
```

## Notes
- `workflow.rs` remains over the decomposition threshold and is tracked in the
  standards compliance plan.
