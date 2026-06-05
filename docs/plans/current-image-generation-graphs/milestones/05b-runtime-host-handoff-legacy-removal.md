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

Selected retired model-path cleanup re-plan as of 2026-05-31: use the phased
fail-closed retirement path. Before another source deletion slice, inventory
and classify every remaining `model_path`, `modelPath`,
`ModelDependencyRequest`, `ModelDependencyResolver`, `ModelRefV2`, and
`build_model_ref_v2` reference as runtime graph execution, canonical
scheduler/runtime-host state, app configuration/embedding/RAG,
diagnostic/stale fixture, tooling/probe, or frontend/Tauri transport. Runtime
execution references are replacement targets; non-execution references may be
retained only with explicit ownership and only if they cannot feed successful
runtime launch. If any production caller requires facts missing from the
canonical scheduler task state/results, runtime-host request/response,
readiness proof, or inference-interface descriptor contracts, stop and re-plan
a shared contract extension instead of creating a `ModelRefV2` or path-shaped
adapter.

Selected legacy dependency/model-ref cleanup sequencing re-plan as of
2026-05-31: use the two-step hybrid transition with canonical
scheduler/runtime-host task-result coverage as the target architecture. The
next source slice must first make every remaining production
`ModelDependencyResolver`, `ModelDependencyRequest`, `ModelRefV2`,
`build_model_ref_v2`, dependency-preflight, and path-repair surface
diagnostic-only if it can still be reached before deletion. That slice must
prove those surfaces cannot launch inference, cannot produce executable launch
inputs, cannot feed runtime-host dispatch, and cannot adapt canonical
readiness/handoff facts back into legacy request or model-ref shapes. After
that guardrail, implement workflow/session task-result coverage that dispatches
runtime work from scheduler task state through the runtime-host port and
records typed responses. Only after canonical task-result/runtime-host response
coverage is verified may production resolver/model-ref composition, old
dependency-preflight output, path-shaped fixtures, and legacy helper contracts
be deleted. This sequencing follows the standards' simplicity/complection rule:
diagnostics, lifecycle/state transition, runtime side effects, transport, and
persistence remain independently owned and do not become a temporary fallback
adapter.

Selected production resolver-composition re-plan as of 2026-05-31: use option
2, the backend diagnostic/activity split. Production runtime execution must
stop installing or snapshotting `ModelDependencyResolver` into workflow
executor extensions, because runtime execution no longer owns legacy
dependency/model-ref resolution. Dependency activity emission and any retained
diagnostic/tooling resolver behavior must move behind a backend-owned
diagnostic/activity boundary that Tauri can subscribe to or forward through
transport only. Tauri must not manage `TauriModelDependencyResolver` as
business state or attach activity policy directly to that resolver. Retained
resolver/probe surfaces must be explicitly diagnostic/tooling-only, must not
produce executable launch inputs, and must remain isolated from
scheduler/runtime-host execution while canonical task-result/runtime-host
response coverage is completed. This follows the standards' layering and
simplicity/complection rules by separating runtime execution, backend
diagnostics/activity lifecycle, app-shell transport, and legacy tooling
cleanup.

Selected production inference-path sequencing re-plan as of 2026-05-31: use
option 1 now, then option 2, then option 3. First complete the minimal
production image inference path through the existing scheduler session runner,
runtime-host port, embedded image execution, artifact sink, and scheduler task
result persistence. This slice may assume canonical readiness/load proof facts
already exist and must fail closed with typed diagnostics when they are missing,
stale, or invalid. After that path is verified, implement the backend-owned
dependency-readiness and runtime-load proof producer lifecycles. Durable
scheduler leases, retry/defer, cancellation, reservation release, replay, and
recovery remain the next hardening phase before broad production workloads.
This sequencing follows the standards' simplicity/complection rule by keeping
runtime execution, producer lifecycle, scheduler lifecycle, persistence,
diagnostics, and Tauri transport as separate ownership boundaries. Do not use
this ordering to add graph-path fallback, node-engine planned-inference launch,
`ModelRefV2`, Tauri-owned business policy, or a temporary compatibility branch.

Selected dependency-readiness active-run lifecycle re-plan as of 2026-06-03:
before continuing successful production runtime-host dispatch, preserve pending
dependency readiness as a non-terminal active workflow-run state and add an
explicit backend-owned resume command for that active run. The existing
first-run deferral path may return typed runtime-not-ready diagnostics, but the
public runtime session API must not finish the run or append terminal events for
that pending state. Scheduler task state remains the backend source of truth for
inspection and later retry/admission. The resume command must validate the
active `session_id` plus `workflow_run_id`, reject non-active or
non-readiness-pending runs with typed diagnostics, retry readiness admission
from canonical backend facts, and continue toward dispatch or return typed
pending/fail-closed diagnostics. This active-run lifecycle option supersedes
the earlier synchronous-provider first-run option; it must not block
workflow-service on async probes, infer readiness from static declarations, or
move retry/readiness/resource policy into Tauri or frontend code. After the
first complete inference path is proven through this explicit backend resume
path, implement the later event-driven backend worker/listener lifecycle from
the composition root with tracked tasks, cancellation/shutdown, freshness,
timeout, retry, reservation release, overlap prevention, and observability.
2026-06-03 auto-resume lifecycle re-plan resolution: implement that later
lifecycle as an embedded-runtime-owned handle returned by composition, not as a
Tauri-owned timer and not as a workflow-service internal host loop. The handle
must reuse the existing snapshot producer, workflow-service resume-candidate
query, embedded backend host, and explicit backend resume command. It may poll
on a bounded interval initially, but must prevent overlapping resume attempts
for the same active run, treat pending readiness as non-terminal, expose
idempotent shutdown, and log cancellation/panic/failure paths at the lifecycle
owner. A typed snapshot notification stream remains the future event-first
upgrade after this path is working.

Selected Tauri runtime/scheduler snapshot event cleanup re-plan as of
2026-06-03: use option 2, the retired Tauri event contract deletion path. The
remaining `record_workflow_event`, `WorkflowRuntimeSnapshotEventInput`,
`WorkflowSchedulerSnapshotEventInput`, `WorkflowEvent::RuntimeSnapshot`,
`WorkflowEvent::SchedulerSnapshot`, and their constructors/serialization
branches must be handled as one diagnostics contract cleanup, not as unrelated
warning fixes. Before deletion, inventory active consumers in Tauri events,
event serialization, diagnostics overlay/trace/store, headless diagnostics,
runtime debug surfaces, and tests. If the snapshot event variants have no
active production transport owner, delete them and migrate tests to the active
backend-owned diagnostics snapshot/update APIs. Runtime and scheduler
diagnostics state must remain owned by workflow-service/headless diagnostics
read models and diagnostics-store update/record helpers; Tauri must not add a
new graph snapshot fallback, scheduler adapter, runtime launch path, or
business-policy branch. Option 1 is rejected because test-only quarantine would
leave the retired event contract in production. Option 3, promoting a new
backend-owned push snapshot event stream, is deferred unless inspection proves
live consumers require push snapshot delivery rather than read-model updates.

2026-06-03 implementation result: active-consumer inventory found no remaining
production transport owner for the Tauri runtime/scheduler snapshot event
variants. The slice deleted the event input DTOs, enum variants, constructors,
serializer branches, event-adapter ownership branches, diagnostics overlay
event branches, and workflow-event trace projection branches. Diagnostics
snapshot recording remains backend-owned through diagnostics-store/headless
record and update helpers; test-only snapshot record helpers now write
`WorkflowTraceEvent::RuntimeSnapshotCaptured` and
`WorkflowTraceEvent::SchedulerSnapshotCaptured` plus diagnostics state directly
instead of constructing Tauri events. Verification passed:
`cargo fmt --manifest-path src-tauri/Cargo.toml`, `cargo fmt --manifest-path
src-tauri/Cargo.toml -- --check`, `cargo check --manifest-path
src-tauri/Cargo.toml`, `cargo test --manifest-path src-tauri/Cargo.toml
diagnostics`, `cargo test --manifest-path src-tauri/Cargo.toml event_adapter`,
strict deleted-symbol search for the removed event input/variant symbols, and
`git diff --check`.

Selected active lane as of 2026-06-05: resume Milestone 5b deletion/
replacement of remaining successful legacy runtime paths before Milestone 6.
The production image inference proof through scheduler task state,
runtime-host execution, embedded image gateway execution, artifact persistence,
task-result mapping, and output projection is complete enough to remove the
old launch surfaces. Remaining Milestone 5c retry/defer, replay/bootstrap,
worker lifecycle diagnostics, cooperative cancellation response mapping, and
extra task-state attempt counters stay tracked as hardening follow-ups; they
must not be used to justify retaining node-engine runtime launch,
planned-inference launch, `ModelRefV2`, `ModelDependencyRequest`, or graph
`model_path` success behavior. The next source slice must inventory one
remaining legacy surface, choose delete/replace/fail-closed based on active
callers, keep Tauri/frontend as transport/display only, and verify no
canonical scheduler/runtime-host facts are adapted back into legacy shapes.

Selected `model_ref` graph identity cleanup re-plan as of 2026-06-05: use the
contract-lane cleanup path. `pumas_model_ref` is the only canonical graph model
identity value/port for inference graph authoring and validation. The remaining
`model_ref` surface is not a single stale validator field: it spans
workflow-node descriptors, graph contract validation, semantic fingerprint
fixtures, Pumas option metadata fallbacks, mock backend ports, and frontend
selection helpers. Therefore it must be removed as a shared contract cleanup
slice, not as a validator-only patch. The slice must inventory and update
`crates/workflow-nodes`, workflow-service graph validation/types/fixtures,
Tauri/frontend Pumas option metadata and selection helpers, and Svelte graph
mocks so no graph authoring, validation, or presentation path accepts
`model_ref` as a model identity substitute. Retained uses of `model_ref` may
exist only in backend-internal Pumas/dependency-planning DTOs where the field
name is part of the Pumas contract, not as graph identity or executable runtime
launch input. Do not add compatibility aliases, migrations, frontend
inference, or Tauri policy; stale saved graphs should receive typed diagnostics
and canonical workflows/fixtures should use `pumas_model_ref`.

2026-06-05 runtime-host mapping cleanup result: workflow-service runtime-host
task input materialization no longer accepts `model_ref` as a model identity
target port. `PumasModelRef` task results are skipped only for the canonical
`pumas_model_ref` target, because graph model identity lives in scheduler
handoff rather than materialized runtime inputs. A retired `model_ref` target
port now fails closed with a typed unsupported-input diagnostic.

2026-06-05 node-engine inference request cleanup result: node-engine
inference request builders and lifecycle model-id extraction no longer read a
direct graph input named `model_ref` as an alias for `pumas_model_ref`.
Backend-internal Pumas and resolved-source DTOs may still carry their
contractual nested `model_ref` fields, but direct graph-authored `model_ref`
no longer provides runtime model identity, lifecycle model id, or launch
inputs. A follow-up node-engine diagnostic cleanup removed lifecycle model-id
extraction from retired `resolved_model_source.model_ref` as well; that
retired resolved-source shape remains ignored or fail-closed by existing
request and dependency-preflight guardrails.

2026-06-05 embedded-runtime Python metadata cleanup result: the retired
Python task-executor metadata path no longer reads `model_ref` for dependency
environment ids or runtime backend selection. Environment ids come only from
the explicit `environment_ref` runtime input, and Python runtime backend ids
come from node-type defaults while the Python adapter remains fail-closed.

**Tasks:**

- [x] Define the runtime-host execution request/response contract first. It must
  consume `SchedulerRuntimeHandoff`, scheduler dispatch decision, dependency
  environment ref, and Pumas model/artifact identity without exposing
  `ModelRefV2`, `model_path`, executable load targets, reservations, batching
  groups, or worker launch internals to graph/node-engine contracts.
- [x] Consume the canonical `DependencyReadinessProofEnvelope` through the
  remaining production runtime-host dispatch and legacy-retirement path. The
  proof must come from backend validation summaries, dependency-planning facts,
  selected runtime/device facts, descriptor fingerprints, explicit user
  constraints, and typed availability evidence. It must carry
  freshness/correlation ids, selected environment identity when one exists,
  and bounded diagnostics without graph-visible paths, executable load targets,
  scheduler policy in Tauri/frontend code, or a second adapter-local readiness
  proof type. 2026-06-03 reconciliation: workflow-service resolves the
  canonical envelope through `WorkflowDependencyReadinessLifecycle`, threads it
  through session scheduler dispatch source refresh, runtime dispatch candidate
  collection, scheduler selection, and `RuntimeHostExecutionRequest`, and
  embedded-runtime consumes that request behind `EmbeddedRuntimeHostExecutionPort`.
  No adapter-local proof type or Tauri/frontend readiness policy remains in the
  dispatch path.
- [x] Add the production dependency-readiness snapshot composition and producer
  lifecycle before runtime-host dispatch wiring. The application composition
  root must create a single
  `DependencyEnvironmentReadinessSnapshotProvider`, inject it into
  `WorkflowService` before the service is shared behind `Arc`, and pass the
  same backend-owned snapshot writer to an embedded-runtime or infrastructure
  lifecycle owner. The producer must own async host package/runtime probes,
  publish only validated path-free snapshots, track all spawned tasks, support
  cancellation/shutdown/retry/tracing, and fail closed through existing
  dependency-environment diagnostics when no fresh matching snapshot exists.
  The production producer source must be a typed backend-owned readiness
  work-item queue emitted by workflow-service or scheduler when a runtime task
  enters `WaitingDependencyReadiness`. Work items must carry task/run/session
  provenance, the validated `DependencyEnvironmentRequest` or stable validated
  request identity plus required request payload, freshness/retry/cancellation
  policy, and bounded diagnostic context. Concrete requirement and binding
  payloads needed for host package/runtime probes must come from a
  backend-owned dependency requirements registry keyed by
  `DependencyRequirementsId`; workflow-service records the validated payload
  once, and embedded-runtime/infrastructure producers resolve ids through a
  narrow registry trait before probing. Registry seeding uses the selected
  2026-05-30 two-phase dependency-environment source: workflow-service may
  insert registry payloads from valid canonical resolve results, ready results,
  or equivalent validated `DependencyRequirementsPayload` values produced by
  the canonical dependency-environment boundary when they include matching
  requirements id, identity key, selected binding ids, requirement rows, and
  binding rows. Resolve-and-seed must happen before readiness work is queued;
  check/install host probes happen later in embedded-runtime or infrastructure
  producers using the registry payload. It must not seed the registry from
  dependency proof identity alone, graph/editor/frontend state, technical-fit
  previews, reduced execution plans, runtime-host load targets,
  `ModelDependencyRequest`, `ModelRefV2`, or path/model-path fields. The
  earlier ready-only workflow-service seed boundary was a validated guardrail
  slice; the two-phase resolve/check implementation widens it without
  accepting invalid, unavailable, stale, missing, or row-mismatched results. The
  synchronous snapshot provider must remain read-only from the caller's
  perspective and must not become the
  producer work source by recording provider misses. Active scheduler-state
  scanning may be planned later only as a reconciliation/audit loop; it must
  not be the primary producer input path.
  Workflow-service, scheduler, node-engine, Tauri, frontend, and runtime-host
  adapters must consume the resulting typed readiness state; they must not own
  package probing, install policy, filesystem/process checks, or readiness
  fallback synthesis. 2026-06-03 reconciliation: embedded-runtime composition
  now creates the shared snapshot provider, work queue, and requirements
  registry before workflow-service is wrapped in `Arc`; standalone and hosted
  construction own tracked producer handles that resolve queued work through
  the backend registry and dependency inventory boundary, publish typed
  snapshots, and shut down explicitly. The companion auto-resume lifecycle
  consumes workflow-service resume candidates and the typed backend resume API;
  Tauri only stores and shuts down returned handles.
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
- [x] Wire scheduler dispatch to call runtime-host execution directly from the
  actual dispatch-selected `SchedulerRuntimeHandoff`. The reduced
  `WorkflowExecutionPlan` may remain an inspection/diagnostics projection but
  must not be used to launch inference or build handoff. Runtime requests must
  include the canonical dependency readiness proof and workflow-service-owned
  materialized runtime inputs derived from validated upstream task results.
  2026-06-03 status reconciliation: workflow-service scheduler orchestration
  now dispatches runtime tasks through the runtime-host execution port from a
  dispatch-selected `SchedulerRuntimeHandoff`, maps runtime-host responses
  into scheduler task results, and projects requested workflow outputs without
  reduced-plan launch.
- [x] Delete the dead node-engine dependency-preflight enforcement surface.
  2026-06-03 slice: node-engine no longer exports or tests
  `enforce_dependency_preflight`, the compatibility diagnostics constructor,
  or the preflight lifecycle context. `dependency_preflight.rs` now only
  rejects retired model-reference input shapes and re-exports path-free
  input/planning projection helpers; it cannot perform resolver lookup,
  `ModelDependencyRequest` construction, compatibility acceptance,
  runtime-host dispatch, lifecycle preflight emission, or `ModelRefV2` output.
  The still-active embedded-runtime Python-backed dependency preflight path is
  a separate replacement/deletion target.
- [x] Add the first complete image-generation inference path behind
  `EmbeddedRuntimeHostExecutionPort`. Selected 2026-05-31 direction: implement
  an image-generation-first embedded-runtime executor as the smallest useful
  vertical slice. Keep the port responsible only for request validation,
  correlation, Pumas load-target resolution, and response shaping; put the
  image-specific mapping from validated runtime-host request, materialized task
  inputs, and resolved Pumas load target into a focused backend-owned module
  that calls the canonical `inference::InferenceGateway` image API. If that
  mapping is unclear, first add a tested projection-only micro-slice and keep
  execution fail-closed. Unsupported task kinds must return typed diagnostics.
  Do not put this business logic in Tauri, frontend, graph editor,
  node-engine, scheduler policy, or the legacy planned-inference contract.
  2026-05-31 progress: the projection-only guardrail is complete in
  `crates/pantograph-embedded-runtime/src/runtime_host_image_execution.rs`.
  It maps validated runtime-host image requests, scheduler dispatch decisions,
  full Pumas package facts, and Pumas load targets into canonical
  image-generation planning input, and fails closed for unsupported task kinds,
  unsupported materialized ports, unsupported runtime ids, invalid devices,
  missing prompt, and invalid launch handoff facts. The runtime-host port
  remains fail-closed after load-target resolution; successful gateway
  execution is not wired yet. Remaining before completing this task: select or
  add the runtime-host full Pumas package-facts source, carry explicit selected
  backend facts through dispatch instead of long-term runtime-id recognition,
  extend runtime-host inputs for typed float image options when exposed, and
  implement path-free media artifact output projection. 2026-05-31 update: the
  runtime-host full Pumas package-facts source guardrail is now complete in
  `crates/pantograph-embedded-runtime/src/runtime_host_package_facts.rs`; it
  resolves only from the scheduler-selected model ref and fails closed for
  missing dispatch decisions, Pumas lookup errors, decode failures, stale
  package-facts contracts, and selected-artifact mismatches. 2026-05-31 media
  output re-plan update: before gateway execution is enabled, add the selected
  option 2 `RuntimeHostMediaArtifactSink` boundary. The sink must persist
  generated images through the backend-owned artifact store/workflow-service
  artifact boundary and return path-free
  `RuntimeHostExecutionMediaArtifactRef` values. The runtime-host port must not
  own workflow-service persistence internals, invent artifact ids, return
  inline base64 media as scheduler task results, or call Tauri/frontend code.
  Missing sink or artifact write failures must return typed runtime-host
  diagnostics. 2026-05-31 production composition re-plan update: the hosted
  production path must not inject the current full-workflow-service-backed
  sink through `Arc<WorkflowService>` because that creates a self-reference
  with the runtime-host execution port. The selected next slice is a shared
  backend artifact writer handle created before `WorkflowService` is wrapped,
  injected into both workflow-service artifact operations and the runtime-host
  media sink. Tauri must remain an app-shell/composition caller and must not
  own artifact persistence policy. 2026-05-31 progress: hosted
  resource-backed embedded-runtime composition now requires a configured
  backend artifact writer before sharing `WorkflowService`, injects that
  writer through `WorkflowServiceRuntimeHostMediaArtifactSink`, and constructs
  `EmbeddedRuntimeHostExecutionPort` with Pumas load-target resolution, full
  Pumas package-facts resolution, the inference gateway, and the writer-backed
  media sink. Partial hosted composition without artifact persistence fails
  closed with typed initialization diagnostics. 2026-05-31 progress: removed
  stale staging-only `dead_code` suppressions from the Pumas load-target and
  full package-facts resolver structs/constructors now used by hosted
  runtime-host composition. Remaining before this task is fully complete: add
  session-level scheduler task-result coverage for the production runtime-host
  response path and then remove the remaining node-engine/planned-inference
  launch paths. 2026-05-31 sequencing update: the next implementation slice
  must complete the minimal production inference path by proving a scheduler
  session runner can dispatch through the production-composed runtime-host port,
  execute embedded image generation, persist generated media through the
  backend artifact writer, map the completed runtime-host response into
  `WorkflowSchedulerTaskResult`, and return requested workflow outputs. Missing
  or stale dependency-readiness/runtime-load proof facts must remain typed
  fail-closed diagnostics in this slice. The producer lifecycles and durable
  scheduler lease/retry/cancellation hardening follow after this complete-path
  proof; they must not be folded into the same implementation slice.
  2026-05-31 progress: embedded-runtime now has focused coverage proving
  `EmbeddedRuntimeHostExecutionPort` can complete image execution with the
  production Pumas load-target resolver, production Pumas package-facts
  resolver, inference gateway, and backend artifact writer sink. The slice
  also normalizes runtime-host package facts into the path-free inference
  planner contract so Pumas owner-local executable entry paths remain confined
  to the Pumas load-target response. Gateway planning diagnostics now include
  bounded planner diagnostic details in the runtime-host failure message.
  Remaining before the complete minimal inference path is fully proven: add
  session-level coverage that dispatches through the production-composed port
  from scheduler task state and verifies task-result/output projection.
  2026-06-03 progress: embedded-runtime now has session-level coverage proving
  the scheduler session runner can dispatch through the production-composed
  `EmbeddedRuntimeHostExecutionPort`, resolve Pumas load targets and package
  facts, execute image generation through the inference gateway, persist media
  through the backend artifact writer, map the runtime-host response into
  scheduler task results, and return the requested workflow output as a
  path-free artifact reference. Remaining after the complete minimal inference
  path proof: remove or replace the remaining node-engine/planned-inference
  launch paths and continue producer lifecycle/durable scheduler hardening in
  separate slices.
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
- [x] Retire node-engine planned-inference launch ownership for runtime
  inference nodes. Affected nodes must submit or reference scheduler task
  intent and consume scheduler task state/results; missing scheduler task state
  must fail closed with typed diagnostics. 2026-05-31 progress:
  embedded-runtime no longer installs or ships
  `EmbeddedPlannedInferenceExecutionHost`, so hosted production/session
  execution cannot use the old planned-inference host as a successful launch
  branch. 2026-05-31 completion: node-engine no longer exports
  `PlannedInferenceExecutionHost`, no longer accepts a
  `PLANNED_INFERENCE_EXECUTION_HOST` extension, and image-generation execution
  now validates canonical request shape then fails closed with a typed
  scheduler-owned task-state/result diagnostic. Successful runtime inference
  state/result consumption remains owned by workflow-service scheduler session
  orchestration and runtime-host task-result projection, not node-engine.
- [x] Inventory and classify every remaining retired model-path/model-dependency
  reference before additional source deletion. The classification must cover
  runtime graph execution, scheduler/runtime-host canonical facts,
  app configuration/embedding/RAG, stale diagnostic fixtures, tooling/probes,
  and frontend/Tauri transport. Record the result in this milestone before
  choosing each deletion or fail-closed conversion slice. 2026-05-31
  classification result:
  - Runtime graph execution replacement targets:
    `crates/node-engine/src/core_executor/{pytorch_nodes.rs,llamacpp_nodes.rs,audio_nodes.rs,model_nodes.rs,dependency_preflight.rs,dependency_preflight/input_projection.rs,tests.rs,inference_tests.rs}`,
    `crates/node-engine/src/{core_executor.rs,engine/dependency_inputs.rs,model_dependencies.rs,lib.rs,extensions.rs}`,
    `crates/pantograph-embedded-runtime/src/{task_executor.rs,task_executor/dependency_environment.rs,task_executor/dependency_environment/helpers.rs,task_executor/python_execution.rs,python_runtime_bridge.py,embedded_workflow_host_helpers.rs}`,
    and related node-engine/embedded-runtime tests. These are not allowed to
    remain successful launch paths. Next slices must convert them to scheduler
    task state/results plus runtime-host responses or typed fail-closed
    diagnostics.
  - Production resolver/removal targets:
    2026-05-31 update: embedded-runtime resolver composition, resolver
    modules, descriptor/requirements/Python helpers, and resolver tests were
    deleted after production composition and Tauri probe/facade consumers were
    removed. Remaining production replacement targets are the task-executor
    dependency-environment helpers/tests and node-engine contracts that still
    mention `ModelDependencyRequest`, `ModelDependencyResolver`, or
    `ModelRefV2`; these must stay fail-closed until scheduler/runtime-host
    task-result coverage replaces them.
  - Host/backend runtime-internal load target consumers:
    `crates/inference/src/{gateway.rs,server.rs,types.rs,embedding_runtime.rs,runtime_load.rs,backend/mod.rs,backend/pytorch.rs,backend/llamacpp.rs,backend/llamacpp_support.rs,backend/pytorch_worker_contract.rs}`,
    `crates/inference/{torch,audio,onnx,depth}/`, and inference backend
    tests/fixtures. These may continue to use model paths only as
    runtime/backend-local executable load targets supplied by runtime-host or
    source-owned config. They must not be fed graph `model_path`,
    `ModelRefV2`, or node-engine path repair.
  - App configuration, embedding/RAG, and direct local command surfaces:
    `src-tauri/src/{config.rs,llm/gateway.rs,llm/startup.rs,llm/recovery.rs,llm/commands/rag.rs,llm/commands/server.rs,llm/runtime_registry.rs}` and
    `crates/pantograph-embedded-runtime/src/embedding_workflow.rs`. These are
    not the graph inference path, but Tauri must remain app-shell/transport;
    any future cleanup must move business policy behind backend-owned service
    boundaries rather than connecting these paths to graph execution.
  - Tooling/probe surfaces:
    2026-05-31 update: `src-tauri/src/bin/pumas_dependency_runtime_probe.rs`
    was retired instead of preserving a Tauri-owned tool that built
    `ModelDependencyRequest` directly and used the embedded-runtime resolver.
    Future dependency/runtime probes must live behind backend-owned diagnostic
    contracts or dedicated backend test fixtures.
  - Canonical guardrails and negative tests:
    `crates/pantograph-scheduler/**`, `crates/pantograph-runtime-host-contracts/**`,
    `crates/pantograph-dependency-environment-service/**`,
    `crates/workflow-nodes/**`, and workflow-service graph/inference-interface
    tests include expected rejection, stripping, or "must not expose" cases.
    Keep these until replacement tests prove the old contracts are deleted.
  - Persisted stale workflow/artifact evidence:
    `.pantograph/workflows/**` and `.pantograph/artifacts/**` still include
    path-shaped historical workflow data. Treat these as persisted
    stale-diagnostic evidence or generated run artifacts, not current
    successful execution fixtures. Do not edit them outside an explicit saved
    workflow/artifact cleanup slice.
  - Historical/completed docs:
    `docs/completed-plans/**`, `docs/historical-plans/**`,
    `docs/standards-compliance-analysis/**`, and unrelated plan docs are
    traceability records only and are not runtime code owners.
- [x] Replace PyTorch execution so successful model loading consumes
  scheduler-dispatched runtime-host requests plus host-owned executable facts
  and no longer reads graph `model_path`, reduced execution-plan projections,
  or emits `ModelRefV2`. 2026-05-31 progress: canonical `llm-inference` with
  `backend_key=pytorch` now fails closed inside node-engine before dependency
  preflight, `ModelRefV2` construction, graph `model_path` loading, PyTorch
  backend loading, generation, KV capture, or node-engine model-ref output.
  The diagnostic states that PyTorch runtime execution is scheduler-owned and
  requires scheduler task state/results. The inference crate still contains
  backend-local PyTorch load-target contracts for runtime-host/inference-owned
  execution; those are not graph execution identity and must be fed only by
  host-owned facts in later runtime-host slices. The node-engine PyTorch
  launch module and PyTorch KV-cache source were deleted in the same slice so
  the retired branch does not leave unused successful-launch helpers behind.
- [x] Replace llama.cpp execution so successful model loading consumes
  scheduler-dispatched runtime-host requests plus host-owned executable facts
  and no longer reads graph `model_path`, reduced execution-plan projections,
  or emits `ModelRefV2`. 2026-05-31 progress: canonical `llm-inference` with
  `backend_key=llama_cpp` now fails closed inside node-engine before
  dependency preflight, gateway startup, graph `model_path` loading,
  completion requests, KV restore/capture, or `ModelRefV2` output. The
  diagnostic states that llama.cpp runtime execution is scheduler-owned and
  requires scheduler task state/results. The old llama.cpp node-engine launch
  module and the old live llama.cpp KV restore/capture helper source were
  deleted in the same slice; explicit KV-cache save/load/truncate node
  handlers remain because they do not launch runtime inference.
- [x] Replace audio execution so successful model loading consumes host-owned
  executable facts and no longer reads graph `model_path`, reduced
  execution-plan projections, or emits `ModelRefV2`. 2026-05-31 progress:
  `audio-generation` now fails closed inside node-engine before dependency
  preflight, Stable Audio Python-worker initialization, graph `model_path`
  loading, audio generation, or `ModelRefV2` output. The diagnostic states
  that Stable Audio runtime execution is scheduler-owned and requires
  scheduler task state/results. The old node-engine Stable Audio launch module
  was deleted in the same slice.
- [ ] Convert remaining runtime graph execution references to scheduler task
  state/results, runtime-host responses, or typed diagnostic-only fail-closed
  behavior. This includes PyTorch, llama.cpp, audio, and any node-engine
  inference/dependency path that can still launch from `model_path`,
  `ModelRefV2`, reduced execution-plan projections, or path repair helpers.
  Next required guardrail slice: make any still-reachable
  dependency-preflight/model-ref/path-repair surface diagnostic-only before it
  can build `ModelDependencyRequest`, emit `ModelRefV2`, repair `model_path`,
  or hand executable inputs to runtime-host dispatch. This guardrail must be
  verified before broader deletion or scheduler/runtime-host task-result
  integration continues. 2026-05-31 progress: the node-engine
  dependency-preflight guardrail is complete. Direct calls to node-engine
  runtime dependency preflight for retired PyTorch/audio paths now fail closed
  before resolver lookup, `ModelDependencyRequest` construction, path repair,
  runtime-host dispatch, or `ModelRefV2` output, while non-runtime/retired
  direct node shapes still return diagnostic skip/no-op behavior. Remaining
  guardrail work: apply the same diagnostic-only rule to embedded-runtime
  task-executor dependency preflight and production resolver composition before
  scheduler/runtime-host task-result integration or broad deletion.
  2026-05-31 progress: embedded-runtime task-executor dependency preflight is
  now diagnostic-only for Python-backed runtime nodes. It still skips node
  types that are not handled by the host Python runtime and still blocks
  non-ready `environment_ref` values with the existing gate diagnostic, but
  every still-reachable Python runtime preflight path now fails closed before
  `ModelDependencyResolver` lookup, `ModelDependencyRequest` construction,
  `ModelRefV2` emission, path repair, or Python adapter dispatch. Remaining
  guardrail work: production resolver composition and any retained commands/
  probes must be made diagnostic/tooling-only or removed after canonical
  scheduler/runtime-host task-result coverage is wired. 2026-05-31 progress:
  node-engine and embedded-runtime dependency-preflight helper APIs no longer
  return `Option<ModelRefV2>`, and embedded-runtime Python execution no longer
  has a branch that can inject a resolved legacy `model_ref` payload into
  runtime inputs. The remaining resolver/model-ref references in this area are
  diagnostic-only tests and transitional helper contracts pending the
  model-dependency/model-ref deletion slice. 2026-06-03 progress: deleted the
  orphaned app-local direct runtime node renderers
  `PyTorchInferenceNode.svelte`, `LlamaCppInferenceNode.svelte`, and
  `RerankerNode.svelte`, plus the unused exported package
  `LlamaCppInferenceNode.svelte`, so active/frontend package node surfaces no
  longer keep direct runtime `model_path` connection checks alive for those
  retired components. 2026-06-03 progress: rewrote the exported package
  `PumaLibNode.svelte` to query `puma-lib.pumas_model_ref`, filter out
  path-shaped options, and persist only `modelName`, `model_id`, and
  `pumas_model_ref`, so the reusable package surface no longer keeps
  `modelPath` persistence or `puma-lib.model_path` option lookup alive.
  2026-06-05 progress: retired the embedded-runtime process Python adapter as
  a successful launch path. `ProcessPythonRuntimeAdapter` now fails closed
  before resolving Python, writing the bridge script, spawning a process,
  loading audio/ONNX workers, reading graph `model_path`, or returning
  `modelRef`/`modelPath` outputs. The bundled `python_runtime_bridge.py` is
  also fail-closed if invoked directly and no longer contains the fallback
  model-ref builder, worker module loading, or audio/ONNX generation calls.
  Python package readiness probes and env resolution remain backend-owned and
  available for readiness diagnostics; runtime execution still belongs to
  scheduler task state/results plus runtime-host execution. 2026-06-05
  follow-up: removed the Python task-executor metadata helper that projected
  `model_ref.modelPath`, `model_path`, or `modelPath` into runtime trace model
  targets. Python runtime metadata may still report backend/runtime identity
  from typed engine facts, but it no longer carries graph path targets from
  retired Python runtime inputs. 2026-06-05 progress: workflow-service
  runtime-host task input materialization no longer treats `model_ref` as a
  model identity target port. Canonical `pumas_model_ref` bindings are skipped
  because scheduler handoff owns model identity; retired `model_ref` target
  ports now fail closed as unsupported materialized runtime inputs. 2026-06-05
  progress: node-engine inference request builders and lifecycle model-id
  extraction no longer read a direct graph input named `model_ref` as a
  `pumas_model_ref` alias. Nested Pumas/resolved-source DTO fields named
  `model_ref` remain backend-internal contract fields only. 2026-06-05
  follow-up: node-engine lifecycle model-id extraction no longer reads retired
  `resolved_model_source.model_ref`; resolved-source inputs remain ignored by
  request construction or fail closed before dependency preflight. 2026-06-05
  progress: embedded-runtime Python task-executor metadata no longer reads
  legacy `model_ref.dependencyBindings` for environment ids or
  `model_ref.engine` for runtime backend selection. Explicit
  `environment_ref` remains the only env-id input for that fail-closed Python
  path. 2026-06-05 progress: embedded-runtime edit-session embedding runtime
  preparation now fails closed before Pumas model lookup, embedded model-path
  resolution, gateway dedicated embedding runtime start, runtime-registry
  refresh, or workflow start. The remaining graph detection helpers only
  decide whether the retired path must fail closed. The slice deleted the
  embedded embedding model-path resolver helpers and removed a stale
  task-executor helper test left behind by the earlier Python metadata cleanup.
  No scheduler/runtime-host facts are adapted back into graph `model_path`,
  Pumas path, or legacy embedding runtime launch shapes.
- [x] Remove production embedded-runtime composition of
  `ModelDependencyResolver` and `ModelRefV2`-producing paths using the
  selected option 2 backend diagnostic/activity split. The next composition
  slice must stop installing `MODEL_DEPENDENCY_RESOLVER` into production
  workflow executor extensions and stop including the resolver in runtime
  extension snapshots/application. It must introduce or reuse a backend-owned
  dependency diagnostic/activity handle so app shells can subscribe to bounded
  activity events without managing resolver business state. Tauri may forward
  activity events but must not own resolver lifecycle, dependency policy,
  install/check decisions, or event attachment policy. Retained resolver
  commands/probes/tests must be explicitly classified as diagnostic/tooling-only
  or pending deletion, and must not produce executable launch inputs,
  `ModelRefV2`, runtime-host dispatch facts, or path-shaped success payloads.
  Full deletion of resolver modules and old tests remains gated on canonical
  scheduler/runtime-host task-result response coverage unless a focused
  deletion slice proves the surface has no production consumers. 2026-05-31
  progress: production runtime extension snapshots and runtime extension
  application no longer carry `ModelDependencyResolver`, hosted resource-backed
  startup no longer installs `MODEL_DEPENDENCY_RESOLVER` into shared runtime
  execution extensions, and standalone embedded runtime construction no longer
  seeds the resolver into execution extensions. Remaining follow-up: replace
  Tauri-managed resolver activity wiring with a backend-owned diagnostic/
  activity subscription handle, then classify or retire retained resolver
  commands/probes/tests. 2026-05-31 progress: hosted startup now returns a
  backend-owned dependency activity hub instead of exposing
  `TauriModelDependencyResolver` to Tauri as managed state. Tauri subscribes to
  that hub and forwards bounded `dependency-activity` transport events only;
  it no longer attaches activity emitters to the resolver or owns resolver
  lifecycle/policy. Remaining follow-up: classify or retire retained resolver
  commands/tests and keep any retained surface diagnostic/tooling-only until
  deletion is possible. `pumas_dependency_runtime_probe` was retired as the
  first probe cleanup slice. 2026-05-31 progress: the stale Tauri workflow
  `model_dependencies.rs` facade was deleted after the probe removal, so Tauri
  no longer re-exports embedded-runtime resolver internals as a workflow-owned
  module. 2026-05-31 progress: embedded-runtime deleted the retained
  `TauriModelDependencyResolver` stack and no longer ships resolver,
  descriptor, requirements, Python install/check, model-ref, or resolver test
  modules. `DependencyActivityHub` remains the only public dependency activity
  boundary. 2026-05-31 progress: the stale
  `MODEL_DEPENDENCY_RESOLVER` extension key was deleted from node-engine, and
  runtime-extension tests no longer install a legacy resolver just to prove it
  is filtered. This removes the last standard extension slot that could carry
  legacy resolver business logic into runtime execution. Remaining resolver
  references are the explicit public legacy contract, diagnostic strings, and
  diagnostic/tooling test stubs pending the model-dependency/model-ref
  deletion slice. 2026-05-31 progress: embedded-runtime task-executor tests
  no longer construct legacy resolver stubs, import model-dependency/model-ref
  DTOs, or assert resolver call counts. Those tests now prove fail-closed
  behavior through diagnostics and absent Python adapter/recorder effects
  without installing any resolver fixture.
- [x] Replace node-engine dependency preflight output with typed readiness or
  scheduler task-state facts after scheduler-to-runtime-host dispatch exists.
  Missing scheduler task state must fail closed with typed diagnostics, not
  repair old inputs. Any old dependency/preflight command reached before its
  replacement must be diagnostic-only and must not successfully produce
  `ModelDependencyRequest`, `ModelRefV2`, path-shaped dependency payloads, or
  executable launch inputs. The immediate transition step is not a new
  readiness adapter; it is a fail-closed diagnostic guardrail that prevents the
  old output shape from being used while canonical task-result coverage is
  completed. 2026-05-31 progress: the preflight helper signatures were narrowed
  to success/typed diagnostic failure, so callers can no longer receive
  `ModelRefV2` output from dependency preflight. This does not add a readiness
  adapter or scheduler/runtime-host fallback; the remaining work is to replace
  or delete the legacy request/helper contracts after canonical task-result
  coverage is complete. 2026-06-03 reconciliation: after the minimal
  scheduler/runtime-host image path and the dead node-engine enforcement
  deletion, node-engine has no dependency-preflight output-producing helper or
  lifecycle preflight authority. The remaining dependency-preflight module is
  limited to retired input rejection and path-free planning projection helpers.
  Embedded-runtime Python-backed dependency preflight remains a separate
  fail-closed replacement/deletion target.
- [x] Remove embedded-runtime `ModelDependencyResolver`/`ModelRefV2` resolution
  paths after runtime host load-target resolution, diagnostic-only legacy
  guardrails, and scheduler task-result/runtime-host response coverage are
  wired.
- [ ] Remove retired node-engine contracts and helpers:
  `ModelDependencyRequest`, `ModelDependencyResolver`, `ModelRefV2`,
  `build_model_ref_v2`, `PlannedInferenceExecutionHost`, path repair helpers,
  and successful `model_path` test fixtures only after scheduler-to-runtime-host
  dispatch and load-target resolution are wired. 2026-05-31 progress:
  `EmbeddedPlannedInferenceExecutionHost` and its embedded-runtime tests were
  removed after hosted runtime-host composition was wired; node-engine
  `PlannedInferenceExecutionHost` and its extension key were removed in the
  follow-up fail-closed node-engine slice. 2026-05-31 progress: the
  workflow-service active execution-plan storage/API/tests and embedded-runtime
  reduced execution-plan projection helper/tests were deleted after
  scheduler-selected runtime-host dispatch coverage existed for session
  runtime tasks. 2026-05-31 progress: the node-engine `build_model_ref_v2`
  constructor and its path-shaped unit test were deleted after dependency
  preflight stopped returning `Option<ModelRefV2>`. 2026-05-31 progress: the
  node-engine `build_model_dependency_request` constructor and its
  builder-specific tests were deleted after canonical dependency planning was
  already owned by `planning_projection.rs` and shared
  `pantograph-dependency-planning` contracts. 2026-05-31 progress: the
  embedded-runtime private `build_model_dependency_request` helper and its
  builder-specific tests were deleted after Python runtime dependency
  preflight became diagnostic-only. 2026-05-31 progress: the node-engine
  `unload-model` handler now fails closed before validating legacy
  `ModelRefV2` `model_ref` input, so runtime lifecycle can only be handled by
  scheduler/runtime-host-owned state. Remaining cleanup in this task is the
  broader public `ModelDependencyRequest`/`ModelDependencyResolver`/
  `ModelRefV2` contract, retained diagnostic strings proving retired contract
  rejection, path-repair helper, and fixture removal. 2026-05-31 progress:
  the public node-engine `model_dependencies` module and re-exports were
  deleted, the unused dependency-binding reader was removed, and retained
  dependency override parsing now uses the shared
  `pantograph-dependency-planning` DTO instead of a node-engine legacy
  re-export. Remaining source hits for retired contract names are diagnostic
  strings/tests only. 2026-05-31 progress: the node-engine
  `inputs_with_model_path_from_ref` path-repair helper was removed. Canonical
  `llm-inference` still rejects retired resolved/unresolved model-reference
  inputs, but it no longer copies `model_path` or `mmproj_path` aliases into a
  repaired launch input map before dispatch. 2026-05-31 progress: the
  node-engine core `puma-lib` executor helper was deleted and `puma-lib` now
  fails closed in `CoreTaskExecutor` with a host-specific Pumas selector
  diagnostic instead of emitting graph-visible `model_path`,
  `inference_settings`, dependency payloads, backend hints, or load facts.
  2026-05-31 progress: node-engine dependency input projection now rejects
  direct graph `model_path` targets and only merges model intent through typed
  `pumas_model_ref` targets. 2026-05-31 progress: typed rerank execution no
  longer emits successful graph outputs named `model_path` or `model_ref`
  carrying `modelPath`; it now returns only canonical rerank result and
  diagnostic outputs. 2026-05-31 re-plan boundary: the next remaining
  successful path behavior is embedded-runtime workflow-host lifecycle
  resolution, which reads graph node `model_path`/`modelPath` and active
  gateway descriptors by path. Replacing it requires a backend-owned runtime
  session state/readiness contract rather than another node-engine-local
  deletion. Decision: implement the option 3 target in two thin stages: first
  prove embedded-runtime-owned runtime session/load-proof state that lifecycle
  checks consume instead of graph paths, then promote the stable DTOs into the
  shared executable contract area. Tauri/frontend may query and display the
  resulting diagnostics, but must not own lifecycle decisions, infer model
  identity from `modelPath`, resolve Pumas artifacts, or repair missing backend
  proofs. 2026-06-03 progress: active node-engine and embedded-runtime Rust
  source and tests no longer mention the retired concrete contract names
  `ModelDependencyRequest`, `ModelDependencyResolver`, `ModelRefV2`,
  `build_model_ref_v2`, or `PlannedInferenceExecutionHost`. Remaining
  fail-closed diagnostics describe retired dependency/model-reference
  contracts generically and still block before legacy dependency request,
  resolver, model-reference, or adapter dispatch behavior can run.
  2026-06-03 progress: deleted the unregistered Tauri
  `run_workflow_execution_session` edit-session command wrapper and the
  desktop-local `workflow_execution_runtime.rs` launcher. The app-facing
  TypeScript `runSession` boundary remains fail-closed and now has a focused
  regression test proving it throws before invoking a legacy Tauri command.
  Successful GUI submission remains scheduler-owned through execution-session
  commands; Tauri no longer retains an internal graph-snapshot runtime launch
  branch.
  2026-06-03 progress: removed the now-unused
  `TauriEventAdapter::with_execution_graph` builder and execution-graph field
  that only served the deleted edit-session launcher. Active event translation
  still records backend execution metadata, artifact stream references, and
  diagnostics projection updates; graph snapshots remain owned by backend
  diagnostics/projection APIs rather than a Tauri adapter-local runtime hook.
- [x] Add backend-owned runtime session/load-proof state in
  `pantograph-embedded-runtime` keyed by workflow/task identity and populated
  from canonical inference planning, Pumas artifact/load-target decisions, and
  runtime-host readiness results. 2026-05-31 progress: added the
  embedded-runtime proof store and public producer hook; production producer
  wiring remains a follow-up before shared-contract promotion.
- [x] Rewire embedded-runtime lifecycle checks to consume typed load proofs and
  fail closed with diagnostics when proofs are stale, missing, backend
  mismatched, or runtime-not-ready; do not read graph node `model_path`,
  `modelPath`, selected artifact path, or Pumas entry path as executable
  authority. 2026-05-31 progress: lifecycle checks now require typed proofs
  for llama.cpp workflows and fail closed with `RuntimeModelLoad` diagnostics
  for missing, backend-mismatched, or inactive-model proofs.
- [x] Promote proven lifecycle/load-proof DTOs into the shared executable
  contract area with validation/normalization for workflow id, task id,
  backend/runtime identity, model id, artifact/load-target identity, readiness
  state, diagnostic phase, and stale/missing proof errors. 2026-05-31
  progress: `WorkflowSessionRuntimeLoadProof` now lives in
  `pantograph-runtime-host-contracts` with a versioned, path-free wire shape,
  validation wrapper, readiness state, diagnostic phase, workflow/task
  correlation, backend/runtime/model/artifact/load-target identity, and
  ready-state active-model invariant. Workflow-service re-exports the shared
  type for host traits, and embedded-runtime validates producer records before
  storing or consuming them. Remaining follow-up: production proof producers
  must populate the shared contract from canonical planning, Pumas load-target,
  and runtime-host readiness facts rather than test hooks.
- [x] Delete embedded-runtime workflow-host graph path resolver helpers and
  update session/edit workflow fixtures to store typed model intent rather than
  executable paths after the typed lifecycle proof vertical slice passes.
  2026-05-31 progress: deleted the embedded workflow-host graph path resolver
  helpers and active gateway path matching; fixture migration remains as part
  of the producer/shared-contract follow-up.
- [x] Remove frontend/Tauri dependency actions keyed by `modelPath` or
  `model_path` after backend capability and task diagnostics cover the
  replacement user-visible state. 2026-06-03 progress: the active
  dependency-environment action intent was already graph-session/revision/node
  keyed. This slice replaced the remaining path-shaped dependency activity
  transport identity with a backend-issued `target_node_id`, updated the
  frontend matcher to use the current dependency-environment node id, and kept
  Tauri as an event forwarder only. Display activity still cannot produce
  runtime launch facts, dependency readiness, scheduler policy, or model-load
  authority.
- [x] Update README/crate documentation for every new host-facing contract,
  Pumas load-target boundary, runtime migration, deleted legacy path, and
  fixture replacement. 2026-06-03 reconciliation: embedded-runtime,
  runtime-host-contracts, and workflow-service READMEs now document the
  runtime-host execution port, runtime session load-proof contract,
  dependency-readiness producer and auto-resume lifecycle, Pumas load-target
  and package-facts boundaries, media artifact sink, inference facts/resource
  estimate provider, deleted resolver/preflight authority, and path-free
  fixture/contract constraints.

**Verification:**

- Contract tests and JSON fixtures for runtime-host execution input/output and
  Pumas load-target diagnostics.
- Contract tests and JSON fixtures for dependency readiness proof success,
  stale descriptor fingerprint, missing dependency availability, invalid
  explicit runtime/device constraint, unavailable environment, and selected
  environment identity.
- Composition/lifecycle tests proving every production `WorkflowService`
  construction path receives the shared snapshot provider before use, the async
  producer writes validated path-free snapshots through a tracked lifecycle
  owner, cancellation and shutdown drain or abort spawned tasks deliberately,
  probe failures/stale snapshots remain non-ready, and no execution path reads
  technical-fit preview facts as runtime dispatch authority.
- Backend artifact-writer composition tests proving hosted workflow-service
  artifact operations and runtime-host media output share one backend-owned
  writer handle, do not form a `WorkflowService` self-reference, do not move
  artifact persistence policy into Tauri, and fail closed with typed
  diagnostics when writer wiring is missing or partial.
- Resolver-composition guardrail tests proving production workflow executor
  extensions no longer install, snapshot, or expose a
  `MODEL_DEPENDENCY_RESOLVER` extension key, Tauri receives dependency
  activity only through a backend-owned diagnostic/activity subscription
  boundary, retained resolver/probe surfaces are
  diagnostic/tooling-only, and no runtime execution path can call
  `ModelDependencyResolver`, emit `ModelRefV2`, or construct executable
  dependency launch inputs.
- Boundary tests proving graph, node-engine, saved-workflow, scheduler hint,
  and scheduler handoff payloads reject executable path fields.
- Runtime-host tests proving Pumas load targets are resolved only at the host
  boundary and unavailable states produce typed diagnostics.
- Image runtime-host executor tests proving validated image-generation runtime
  requests project into the canonical inference gateway call, gateway failures
  project into typed runtime-host diagnostics, successful execution emits
  path-free media artifact refs, and unsupported task kinds fail closed without
  invoking node-engine or planned-inference launch paths.
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
  `model_dependencies`, `build_model_ref_v2`, `PlannedInferenceExecutionHost`,
  path-repair helpers, frontend `modelPath` dependency actions, direct old
  runtime task success fixtures, or path-shaped success fixtures.
- Classification/search checks proving every retained `model_path`,
  `modelPath`, `ModelDependencyRequest`, `ModelDependencyResolver`,
  `ModelRefV2`, and `build_model_ref_v2` hit is recorded as non-execution,
  stale-diagnostic-only, tooling-only, or pending replacement before the next
  code slice starts. Successful runtime graph execution must have zero retained
  hits for those contracts.
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
- Do not install `ModelDependencyResolver` into production workflow executor
  extensions or runtime extension snapshots after the resolver-composition
  guardrail slice.
- Do not let Tauri manage `TauriModelDependencyResolver` as business state or
  attach dependency activity policy directly to the resolver; Tauri may only
  subscribe to or forward backend-owned diagnostic activity.
- Do not leave `PlannedInferenceExecutionHost` as an alternate successful
  inference launch branch after direct scheduler dispatch is wired.
- Do not let node-engine, runtime adapters, frontend actions, or Tauri commands
  choose scheduler runtime/device/dependency policy.
- Do not expose executable Pumas load targets outside the runtime host
  boundary.
- Do not install or replace the dependency-readiness provider after
  `WorkflowService` has started accepting runs unless a separate re-plan proves
  an initialization-only API with explicit lifecycle state, race-free tests, and
  no active-run provider swapping.
- Do not move host package/runtime probing, async task ownership, or snapshot
  production into `pantograph-workflow-service`; keep it in a composition-owned
  infrastructure lifecycle.
- Do not inject `Arc<WorkflowService>` back into the runtime-host execution
  port for production media output; use a shared backend artifact writer
  handle instead so artifact persistence remains backend-owned without a
  service self-reference.
- Do not derive execution readiness snapshots from technical-fit preview facts,
  graph node data, Tauri/frontend payloads, reduced execution plans, runtime
  handoff load targets, `ModelDependencyRequest`, `ModelRefV2`, or
  `model_path`/`modelPath`.

**Status:**

- [ ] In progress.
- 2026-06-03 Tauri edit-session launcher deletion slice completed. Smallest
  useful vertical slice: remove the unregistered desktop edit-session run
  command and its Tauri-local runtime orchestration module after the minimal
  scheduler/runtime-host image inference path was proven. Allowed write set:
  `src-tauri/src/workflow/workflow_execution_tauri_commands.rs`,
  `src-tauri/src/workflow/workflow_execution_commands.rs`,
  `src-tauri/src/workflow/mod.rs`,
  `src-tauri/src/workflow/workflow_execution_runtime.rs`,
  `src-tauri/src/README.md`, `src-tauri/src/workflow/README.md`,
  `src/services/workflow/WorkflowService.commands.test.ts`, this milestone,
  and the top-level plan. No-fallback confirmation: this deleted an alternate
  graph-snapshot runtime launcher instead of adapting it to scheduler facts;
  the frontend service still fails closed before invoking Tauri, and scheduler
  execution-session commands remain the only GUI run path. Verification
  passed: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`,
  `cargo check --manifest-path src-tauri/Cargo.toml`,
  `npm run test:frontend -- WorkflowService.commands.test.ts`,
  `npm run typecheck`, and a deleted-symbol search for the removed Tauri
  launcher names. Discovered issues: `cargo check` still reports unrelated
  dead-code warnings for diagnostics runtime/scheduler snapshot event helpers
  and Pumas helper functions; keep those as separate cleanup candidates because
  they are not part of this edit-session launcher deletion slice.
- 2026-06-03 Tauri event-adapter execution-graph hook deletion slice
  completed. Smallest useful vertical slice: remove the unused
  `TauriEventAdapter::with_execution_graph` builder and adapter-local optional
  graph state left behind by the deleted edit-session launcher. Allowed write
  set: `src-tauri/src/workflow/event_adapter.rs`, this milestone, the
  execution log, and the top-level plan. No-fallback confirmation: this does
  not add any graph snapshot fallback or scheduler adapter; it deletes
  Tauri-local graph attachment state so runtime graph snapshots remain
  backend-owned. Verification passed: `cargo fmt --manifest-path
  src-tauri/Cargo.toml -- --check`, `cargo check --manifest-path
  src-tauri/Cargo.toml`, `cargo test --manifest-path src-tauri/Cargo.toml
  event_adapter`, and deleted-symbol search for the removed adapter hook.
  Verification correction: `cargo test --manifest-path src-tauri/Cargo.toml
  event_adapter --lib` was invalid because the Tauri package has no library
  target; the non-`--lib` command passed. Discovered issue: deleting the
  adapter graph hook exposes `WorkflowDiagnosticsStore::set_execution_graph`
  as another dead diagnostics helper in the Tauri binary; keep it as a
  separate cleanup candidate with its diagnostics tests and projection
  ownership reviewed before deletion.
- 2026-06-03 Tauri diagnostics graph setter production-surface cleanup slice
  completed. Smallest useful vertical slice: scope
  `WorkflowDiagnosticsStore::set_execution_graph` to tests after production
  callers were removed, preserving diagnostics graph-context regression
  coverage without exposing a dead production graph snapshot attachment API.
  Allowed write set: `src-tauri/src/workflow/diagnostics/store.rs`, this
  milestone, the execution log, and the top-level plan. No-fallback
  confirmation: this does not add a graph snapshot fallback or runtime launch
  branch; it removes production access to a Tauri-local graph attachment
  helper while leaving backend-owned diagnostics projection behavior covered by
  tests. Verification passed: `cargo fmt --manifest-path src-tauri/Cargo.toml
  -- --check`, `cargo check --manifest-path src-tauri/Cargo.toml`, `cargo
  test --manifest-path src-tauri/Cargo.toml diagnostics`, `cargo test
  --manifest-path src-tauri/Cargo.toml event_adapter`, and diff hygiene.
  Verification correction: `cargo test --manifest-path src-tauri/Cargo.toml
  diagnostics event_adapter` was invalid because Cargo accepts one test-name
  filter; both corrected commands passed. Remaining warnings are
  `record_workflow_event`, runtime/scheduler snapshot event DTOs/constructors,
  and Pumas helper functions.
- 2026-06-03 Tauri Pumas selector extension helper cleanup slice completed.
  Smallest useful vertical slice: scope the unused extension-based Pumas
  selector helper wrappers in `puma_lib_commands.rs` to tests while keeping
  the production command path on the access-based helpers. Allowed write set:
  `src-tauri/src/workflow/puma_lib_commands.rs`, this milestone, the
  execution log, and the top-level plan. No-fallback confirmation: this does
  not add a Pumas fallback, owner-API bridge, path lookup, or runtime launch
  branch; production commands continue to resolve backend-owned selector
  access once and call access-based Pumas APIs. Verification passed: `cargo
  fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo check
  --manifest-path src-tauri/Cargo.toml`, `cargo test --manifest-path
  src-tauri/Cargo.toml puma_lib_commands`, and diff hygiene. Remaining Tauri
  warnings are now limited to `record_workflow_event` and the retired
  runtime/scheduler snapshot event DTO/constructor surface, which should be
  reviewed together as one diagnostics contract cleanup rather than piecemeal.
- 2026-06-03 Tauri runtime/scheduler snapshot event contract re-plan selected.
  Decision: use option 2, deleting the retired Tauri snapshot event contract
  after a focused consumer inventory proves no active production transport
  owner remains. The next source slice may touch
  `src-tauri/src/workflow/events.rs`,
  `src-tauri/src/workflow/event_serialization.rs`,
  `src-tauri/src/workflow/event_adapter/translation.rs`,
  `src-tauri/src/workflow/diagnostics/overlay.rs`,
  `src-tauri/src/workflow/diagnostics/trace.rs`,
  `src-tauri/src/workflow/diagnostics/store.rs`,
  focused diagnostics/event-adapter/headless tests, READMEs, and these plan
  docs. It must not touch runtime-host execution, scheduler admission, Pumas
  selector behavior, frontend UI state, or workflow-service contracts unless
  the inventory proves a shared contract extension is required. No-fallback
  confirmation: delete or migrate the event surface to active backend-owned
  diagnostics snapshot APIs; do not add a Tauri snapshot fallback, scheduler
  adapter, graph snapshot repair path, or compatibility event. Expected
  verification: Tauri fmt/check, `cargo test --manifest-path
  src-tauri/Cargo.toml diagnostics`, `cargo test --manifest-path
  src-tauri/Cargo.toml event_adapter`, targeted serialization/trace tests as
  needed, deleted-symbol search for the retired snapshot event names, and
  `git diff --check`.
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
- 2026-05-29 snapshot-backed dependency-readiness provider slice completed.
  Added `DependencyEnvironmentReadinessSnapshotProvider`,
  `DependencyEnvironmentReadinessSnapshot`, typed freshness state, and typed
  snapshot insertion errors in `pantograph-dependency-environment-service`.
  The provider is synchronous, path-free, and returns canonical validated
  `DependencyEnvironmentResult` values through the existing
  `DependencyEnvironmentService` facade. Fresh matching snapshots return their
  validated result. Missing snapshots, stale snapshots, and identity/detail
  mismatches fail closed with non-ready typed diagnostics instead of probing
  Pumas, package managers, filesystems, runtime hosts, technical-fit previews,
  graph node data, runtime handoff load targets, `ModelDependencyRequest`,
  `ModelRefV2`, graph `model_path`/`modelPath`, or legacy execution-plan
  payloads.
  - Keying decision: readiness snapshots match by action, path-free
    `DependencyPlanningIdentityKey`, dependency requirements id, and request
    environment ref. The inserted snapshot also validates its producer
    `DependencyPlanningRequest` against the identity key, but caller context
    such as workflow run id is intentionally not part of the readiness key
    because it is provenance, not dependency-environment identity. This avoids
    rejecting a fresh backend-owned snapshot for a later run with identical
    dependency requirements while still preventing mismatched requirement ids
    from admitting dispatch.
  - Workflow acceptance: replaced the local injected-ready test provider with
    the snapshot provider and proved a fresh matching readiness snapshot admits
    the runtime inference task only as far as the current dispatch-not-wired
    boundary. The no-snapshot path still fails before dispatch admission and
    neither path calls runtime load or legacy whole-run execution.
  - Documentation: updated dependency-environment service READMEs to record the
    snapshot provider, keying decision, fail-closed behavior, and lifecycle
    split that keeps future async producers outside this synchronous contract
    crate.
  - Verification: `cargo test -p pantograph-dependency-environment-service`;
    `cargo test -p pantograph-workflow-service
    workflow_execution_session_runtime_run_requires_dependency_readiness_before_dispatch
    -- --nocapture`; `cargo test -p pantograph-workflow-service
    workflow_execution_session_fresh_dependency_readiness_snapshot_stops_at_dispatch_boundary
    -- --nocapture`; `cargo check -p
    pantograph-dependency-environment-service`; `cargo check -p
    pantograph-workflow-service`; `cargo fmt -p
    pantograph-dependency-environment-service -p pantograph-workflow-service
    -- --check`; `git diff --check`; targeted retired-path search over touched
    service/workflow files. Verification caveat: one attempted `cargo test`
    command used two positional test filters and failed with Cargo usage before
    running tests; both filters were rerun as separate passing commands.
    Workflow-service still emits the known unused `set_active_run_execution_plan`
    warning.
  - Remaining follow-up: implement the async backend snapshot producer with
    tracked handles, cancellation, shutdown, retry policy, tracing, and
    production source wiring before real host package/runtime probes can feed
    this provider. Scheduler dispatch selection/runtime-host request
    construction from the dispatch-selected `SchedulerRuntimeHandoff` remains a
    later slice.
- 2026-05-29 production dependency-readiness snapshot producer re-plan
  decision: use the standards-aligned composition-owned provider and producer
  lifecycle path. The next implementation must add a backend composition bundle
  that creates the shared `DependencyEnvironmentReadinessSnapshotProvider`
  before `WorkflowService` is wrapped in `Arc`, wires that provider through all
  production service construction paths, and hands the same snapshot writer to
  an embedded-runtime or infrastructure lifecycle owner for async probing.
  This follows the architecture standards by keeping concrete infrastructure
  selection in the composition root, keeping workflow-service on the sync
  consumer side of the contract, and keeping background probes under one
  startup/shutdown owner with tracked tasks. Rejected alternatives:
  initialization-time provider replacement may only be reconsidered if a later
  re-plan proves explicit lifecycle state and no active-run races; moving
  probing into workflow-service violates sync-core/async-shell and crate-role
  boundaries; and reusing technical-fit preview facts as execution readiness
  authority violates the no-fallback/no-legacy rule. Smallest next slice:
  define the composition bundle/factory and wire construction paths with a
  no-probe producer stub that still fails closed unless a validated snapshot is
  present. The following slice must add the tracked async producer lifecycle
  with cancellation, shutdown, retry, tracing, and focused tests before real
  host package/runtime probes are allowed to publish production snapshots.
- 2026-05-29 dependency-readiness composition slice completed. Added
  `WorkflowDependencyReadinessComponents` in workflow-service to create the
  shared `DependencyEnvironmentReadinessSnapshotProvider` before service
  sharing, and updated `WorkflowService::with_dependency_environment_provider`
  so runtime readiness admission and graph-session dependency action
  resolution consume the same configured provider. Standalone embedded-runtime,
  UniFFI frontend HTTP/runtime construction, and Rustler frontend HTTP service
  construction now use the shared composition helper instead of constructing an
  unconfigured `WorkflowService`. No async probing, Pumas lookup, filesystem
  checks, runtime creation, technical-fit preview consumption, `ModelRefV2`,
  `ModelDependencyRequest`, `model_path`/`modelPath`, or legacy execution-plan
  adaptation was added; the configured snapshot provider still fails closed
  unless a validated fresh snapshot exists. Discovered issue fixed in slice:
  Rustler frontend HTTP still had a stale `WorkflowErrorEnvelope` initializer
  without `diagnostics`; it was updated to the current typed contract so the
  touched frontend HTTP binding path can compile. Verification passed:
  `cargo test -p pantograph-workflow-service
  components_create_empty_snapshot_provider_before_service_sharing --
  --nocapture`; `cargo check -p pantograph-workflow-service`; `cargo check -p
  pantograph-workflow-service --all-features`; `cargo check -p
  pantograph-workflow-service --no-default-features`; `cargo check -p
  pantograph-embedded-runtime --features standalone`; `cargo check -p
  pantograph-uniffi`; `cargo check -p pantograph-uniffi --no-default-features
  --features frontend-http`; `cargo check -p pantograph_rustler --features
  frontend-http`; `cargo fmt -p
  pantograph-workflow-service -p pantograph-embedded-runtime -p
  pantograph-uniffi -p pantograph_rustler -- --check`. Verification caveat:
  workflow-service still emits the known unused `set_active_run_execution_plan`
  warning. Remaining follow-up: add the tracked async producer lifecycle with
  cancellation, shutdown, retry, tracing, and real host package/runtime probe
  publication before production runtime-host dispatch relies on live
  dependency readiness.
- 2026-05-29 dependency-readiness producer lifecycle shell slice completed.
  Added `EmbeddedDependencyReadinessSnapshotProducer` and a tracked
  `EmbeddedDependencyReadinessSnapshotProducerHandle` in embedded-runtime. The
  lifecycle validates non-zero polling intervals, spawns one owned Tokio task,
  records heartbeat tracing, supports idempotent shutdown, and is wired into
  standalone embedded runtime plus the UniFFI embedded runtime constructor so
  those composition paths own startup and shutdown for the provider they
  create. The lifecycle intentionally publishes no snapshots and performs no
  package, filesystem, Pumas, runtime-host, technical-fit, or legacy preflight
  probing; dependency readiness remains fail-closed until a later slice adds
  real host probe publication. Verification passed: `cargo test -p
  pantograph-embedded-runtime dependency_readiness_lifecycle -- --nocapture`;
  `cargo check -p pantograph-embedded-runtime`; `cargo check -p
  pantograph-embedded-runtime --features standalone`; `cargo check -p
  pantograph-uniffi`; `cargo check -p pantograph-uniffi
  --no-default-features --features frontend-http`; `cargo fmt -p
  pantograph-embedded-runtime -p pantograph-uniffi -- --check`. Verification
  caveat: workflow-service still emits the known unused
  `set_active_run_execution_plan` warning. Remaining follow-up: add the real
  host package/runtime probe producer behind this lifecycle with retry/backoff,
  typed probe failures, and validated snapshot publication.
- 2026-06-03 auto-resume lifecycle re-plan resolution: the next lifecycle
  owner should be a second embedded-runtime handle beside the snapshot
  producer, not Tauri and not workflow-service host ownership. It will poll the
  workflow-service resume-candidate query, call the existing backend resume API
  through the embedded backend host, track in-flight run identities to prevent
  overlap, and participate in composition-owned shutdown. This keeps concrete
  infrastructure and long-lived workers in the composition boundary while
  preserving workflow-service as scheduler state owner and Tauri as a thin
  handle manager. A snapshot notification/event channel is deferred until after
  the polling lifecycle proves the complete inference resume path.
- 2026-06-03 auto-resume lifecycle handle slice completed: embedded-runtime
  now exports the tracked auto-resume lifecycle, config, handle, and resume
  port with focused tests for idempotent shutdown, no-op empty polls,
  duplicate candidate suppression, successful resume attempts, pending
  readiness as non-terminal, and invalid poll-interval rejection. Remaining
  follow-up: wire the real port and returned handle into hosted and standalone
  composition beside the snapshot producer.
- 2026-06-03 auto-resume production wiring slice completed:
  embedded-runtime now owns the real workflow-service resume port and
  `EmbeddedRuntime` can spawn/manage the auto-resume handle. Standalone runtime
  construction starts the handle beside the snapshot producer. Hosted Tauri
  startup creates the handle through embedded-runtime host construction and
  stores/shuts down only the returned handle; it does not own readiness retry
  policy. Shutdown stops auto-resume before tearing down runtime resources.
- 2026-05-29 re-plan boundary before real dependency-readiness snapshot
  publication: the lifecycle shell has a provider and tracked task owner, but
  no standards-compliant source of readiness work. A real producer must know
  which validated `DependencyEnvironmentRequest` values to probe without
  blocking the synchronous provider, scanning frontend/graph data, deriving
  requests from technical-fit preview facts, or adapting legacy
  `ModelDependencyRequest`/`ModelRefV2`/`model_path` data. Required decision:
  choose the request-source contract before implementing host probes. Options:
  (1) record provider misses into an in-memory request journal for the producer
  to drain; lower integration cost but risks hidden side effects in the
  synchronous provider and weak scheduler correlation; (2) have workflow-service
  or scheduler enqueue explicit readiness work items when runtime tasks enter
  `WaitingDependencyReadiness`; clearer ownership and task correlation, but
  requires a new shared work-item contract and queue lifecycle; (3) have the
  producer periodically scan active scheduler task state; avoids a queue but
  couples producer polling to scheduler internals and is harder to test
  deterministically. Recommendation for the next re-plan: use option 2 with a
  typed backend-owned readiness work-item queue, produced when scheduler task
  state reaches dependency-readiness admission and consumed by the
  embedded-runtime lifecycle producer.
- 2026-05-29 dependency-readiness work-source re-plan selected. Decision:
  implement option 2 as the production path. Workflow-service or scheduler must
  enqueue typed readiness work items exactly at the task transition into
  `WaitingDependencyReadiness`; embedded-runtime or an infrastructure lifecycle
  owner must consume those items, run host package/runtime probes in the async
  shell with tracked task ownership, and publish validated
  `DependencyEnvironmentReadinessSnapshot` values into the shared provider.
  Standards-aligned constraints: the work-item contract is shared integration
  owner work; queue lifecycle, cancellation, retry/backoff, leasing/dedupe, and
  shutdown must be explicit and tested; frontend/Tauri/node-engine/runtime-host
  adapters remain consumers of typed readiness diagnostics only. Option 3 is
  retained only as a later reconciliation/audit loop if queue loss or restart
  recovery requires it. Provider miss journaling must not become the primary
  source of work, and technical-fit preview facts, graph/editor data, reduced
  execution plans, runtime-host load targets, `ModelDependencyRequest`,
  `ModelRefV2`, and path/model-path fields remain forbidden readiness sources.
- 2026-05-29 dependency-readiness work-item contract slice completed. Smallest
  useful vertical slice: define the shared backend-owned work-item DTO and
  in-memory queue contract before wiring scheduler emission or producer
  draining. Allowed write set: `pantograph-dependency-environment-service`
  public API/source README/tests plus this plan and execution-management
  ledger. No-fallback/no-legacy result: work items require validated
  `DependencyEnvironmentRequest` values and task/run/session provenance, do not
  publish snapshots, do not probe hosts, do not record provider misses, and do
  not read frontend/graph/editor/technical-fit/runtime-host/legacy path data.
  Added `DependencyReadinessWorkItem`,
  `DependencyReadinessWorkItemProvenance`, typed provenance/cancellation ids,
  `DependencyReadinessFreshnessPolicy`,
  `DependencyReadinessDiagnosticContext`, and `DependencyReadinessWorkQueue`
  with FIFO dequeue and dedupe by provenance plus validated request key.
  Verification passed: `cargo test -p pantograph-dependency-environment-service
  -- --nocapture`; `cargo check -p pantograph-dependency-environment-service`;
  `cargo check -p pantograph-workflow-service`; `cargo check -p
  pantograph-embedded-runtime`; `cargo fmt -p
  pantograph-dependency-environment-service -- --check`. Verification caveat:
  workflow-service still emits the known unused
  `set_active_run_execution_plan` warning. Remaining follow-up: compose this
  queue through workflow-service/embedded-runtime construction, then emit work
  items exactly when runtime tasks enter `WaitingDependencyReadiness`.
- 2026-05-29 dependency-readiness work-queue composition slice completed.
  Smallest useful vertical slice: wire the shared queue through composition
  roots and the tracked no-probe producer lifecycle without scheduler emission,
  host probes, or snapshot publication. Allowed write set:
  workflow-service dependency-readiness composition helper/tests,
  embedded-runtime producer lifecycle/construction, UniFFI runtime construction,
  crate READMEs, this milestone, and the execution-management ledger.
  No-fallback/no-legacy result: one composition-owned queue is now paired with
  the existing snapshot provider; the producer observes queue length only for
  heartbeat tracing and still cannot fabricate readiness, drain work, probe
  hosts, or adapt technical-fit/frontend/graph/runtime-host/legacy path data.
  Verification passed: `cargo test -p pantograph-workflow-service components_
  -- --nocapture`; `cargo test -p pantograph-embedded-runtime
  dependency_readiness_lifecycle -- --nocapture`; `cargo check -p
  pantograph-uniffi`; `cargo check -p pantograph-workflow-service`; `cargo
  check -p pantograph-embedded-runtime`; `cargo fmt -p
  pantograph-workflow-service -p pantograph-embedded-runtime -p
  pantograph-uniffi -- --check`; `git diff --check`. Verification caveat:
  workflow-service still emits the known unused
  `set_active_run_execution_plan` warning. Remaining follow-up: emit one typed
  work item when each runtime task enters `WaitingDependencyReadiness`.
- 2026-05-29 workflow-service readiness work-item emission slice completed.
  Smallest useful vertical slice: give `WorkflowService` the shared readiness
  work queue and enqueue producer work from the session readiness-admission
  path before provider resolution for runtime tasks in
  `WaitingDependencyReadiness`. Allowed write set: workflow-service service
  config, dependency-readiness composition, session scheduler runner, focused
  session/composition tests, workflow README, this milestone, and
  execution-management ledger. No-fallback/no-legacy result: work items are
  reconstructed from the validated readiness request envelope already consumed
  by the provider boundary; the queue is not populated from provider misses,
  graph/editor/frontend state, technical-fit previews, reduced execution plans,
  runtime-host load targets, `ModelDependencyRequest`, `ModelRefV2`, or
  path/model-path fields. Runtime dispatch remains fail-closed and no host
  probes or snapshot publication were added. Verification passed: `cargo test
  -p pantograph-workflow-service
  workflow_execution_session_runtime_run_requires_dependency_readiness_before_dispatch
  -- --nocapture`; `cargo test -p pantograph-workflow-service components_ --
  --nocapture`; `cargo check -p pantograph-workflow-service`; `cargo check -p
  pantograph-embedded-runtime`; `cargo check -p pantograph-uniffi`; `cargo
  check -p pantograph_rustler --features frontend-http`; `cargo fmt -p
  pantograph-workflow-service -- --check`; `git diff --check`. Verification
  caveat: workflow-service still emits the known unused
  `set_active_run_execution_plan` warning. Remaining follow-up: make the
  embedded-runtime producer drain queued work and publish typed unavailable
  snapshots before adding real host probes.
- 2026-05-29 dependency-readiness unavailable snapshot publication slice
  completed. Smallest useful vertical slice: let the no-probe producer drain
  queued readiness work and publish explicit non-ready snapshots so the
  queue-to-provider path is exercised before host package/runtime probes are
  implemented. Allowed write set: dependency-environment snapshot helper and
  README, embedded-runtime producer lifecycle/tests/Cargo dev-dependency and
  README, this milestone, and execution-management ledger. No-fallback/no-legacy
  result: the producer publishes `Unavailable` readiness diagnostics only; it
  does not mark dependencies ready, does not probe hosts, and does not derive
  work or readiness from provider misses, graph/editor/frontend state,
  technical-fit previews, reduced execution plans, runtime-host load targets,
  `ModelDependencyRequest`, `ModelRefV2`, or path/model-path fields.
  Verification passed: `cargo test -p pantograph-embedded-runtime
  dependency_readiness_lifecycle -- --nocapture`; `cargo test -p
  pantograph-dependency-environment-service -- --nocapture`; `cargo check -p
  pantograph-embedded-runtime`; `cargo fmt -p
  pantograph-dependency-environment-service -p pantograph-embedded-runtime --
  --check`; `git diff --check`. Verification caveat: workflow-service still
  emits the known unused `set_active_run_execution_plan` warning through
  embedded-runtime checks. Remaining follow-up: replace unavailable publication
  with real host package/runtime probes plus retry/backoff/failure diagnostics.
- 2026-05-29 re-plan boundary before real dependency-readiness host probes:
  the queue-to-provider path is now validated, but real host probes cannot be
  implemented standards-compliantly from the current producer input alone. A
  queued `DependencyReadinessWorkItem` carries task/run/session provenance and
  a validated `DependencyEnvironmentRequest`; that request carries action,
  path-free identity, planning request, dependency requirements id, and optional
  environment ref, but it does not carry the concrete requirement and binding
  payload the producer needs to decide which host package/runtime checks to
  run. Implementing probes now would require guessing from the requirements id,
  rereading frontend/graph/editor state, deriving execution readiness from
  technical-fit previews, or adapting legacy `ModelDependencyRequest`/
  `ModelRefV2`/path-shaped dependency payloads, all of which violate the
  no-fallback/no-legacy rule.
  Options for the next re-plan:
  (1) extend `DependencyReadinessWorkItem` to carry a validated dependency
  requirements payload/proof snapshot with the exact requirement and binding
  rows needed by the producer; this keeps the queue self-contained and
  deterministic but broadens the shared work-item contract;
  (2) add a backend-owned dependency requirements registry keyed by
  `DependencyRequirementsId`; workflow-service stores the validated payload
  once and the producer resolves ids through a narrow registry trait; this
  avoids large queue items and supports reuse, but adds registry lifecycle,
  expiry, and missing-payload diagnostics;
  (3) have the producer reconstruct requirements from the planning request at
  drain time; this reduces queue/registry work but risks duplicating planning
  policy in the producer and makes probe behavior harder to reason about;
  (4) reuse legacy model dependency/preflight payloads or technical-fit preview
  facts; rejected because it preserves retired behavior as a successful probe
  source. Recommendation to evaluate next: option 2 if requirements payloads
  need reuse across concurrent tasks/runs, otherwise option 1 for the smallest
  deterministic contract. In either case, real probes must remain in the
  embedded-runtime/infrastructure async shell with tracked lifecycle ownership,
  typed unavailable/failed diagnostics, bounded retry/backoff, and no graph,
  frontend, Tauri, runtime-host, or legacy path-derived probe inputs.
- 2026-05-29 dependency requirements registry re-plan selected. Decision:
  implement option 2 as the real-probe source contract. Add a backend-owned
  dependency requirements registry keyed by `DependencyRequirementsId`.
  Workflow-service must store the validated requirement/binding payload when it
  builds or consumes the dependency readiness source for a runtime task, and
  readiness work items must continue to carry only task/run/session provenance,
  validated request identity, freshness/retry/cancellation policy, diagnostic
  context, and the requirements id. Embedded-runtime or an infrastructure
  lifecycle producer must resolve the requirements id through a narrow registry
  trait before running host package/runtime probes. Missing, stale, mismatched,
  or unavailable registry payloads must publish typed non-ready diagnostics and
  must not fall back to requirement-id string parsing, frontend/graph/editor
  state, technical-fit preview facts, reduced execution plans, runtime-host
  load targets, `ModelDependencyRequest`, `ModelRefV2`, or path/model-path
  fields. Registry ownership and lifecycle constraints: the shared payload
  contract and registry trait are serial integration-owner work; core/shared
  crates may define DTOs and traits but must not depend on app, transport,
  binding, or runtime lifecycle crates; concrete storage, expiry, eviction, and
  shutdown wiring belong in workflow-service composition or another backend
  composition owner; producer probing remains in the embedded-runtime/
  infrastructure async shell with tracked task handles, bounded retry/backoff,
  cancellation, and deterministic tests. Rejected alternatives: self-contained
  queue payloads for production because they duplicate large payloads across
  concurrent tasks/runs; producer-side reconstruction because it duplicates
  planning policy in embedded-runtime; legacy/preview-fact reuse because it
  preserves retired behavior.
- 2026-05-29 dependency requirements registry contract/storage slice
  completed. Smallest useful vertical slice: define the path-free
  `DependencyRequirementsPayload`, `DependencyRequirementsRegistry` lookup
  trait, fresh/stale registry entry status, typed missing/stale/mismatched
  diagnostics, and first in-memory backend registry implementation in
  `pantograph-dependency-environment-service`; expose the registry through
  workflow dependency-readiness composition beside the snapshot provider and
  work queue. Allowed write set: dependency-environment-service registry
  module/tests/README/public exports, workflow-service dependency-readiness
  composition tests, this milestone, and execution-management ledger.
  No-fallback/no-legacy result: payloads can be built only from validated
  dependency-environment results or typed payload constructors, and lookup
  failures return typed diagnostics instead of deriving requirements from
  graph/editor/frontend state, technical-fit previews, reduced execution
  plans, runtime-host load targets, `ModelDependencyRequest`, `ModelRefV2`, or
  path/model-path fields. Verification passed: `cargo test -p
  pantograph-dependency-environment-service`; `cargo test -p
  pantograph-workflow-service dependency_readiness_composition`; `cargo fmt`;
  `git diff --check`. Verification caveat: the focused workflow-service test
  command still emits the known unused `set_active_run_execution_plan`
  warning. Remaining follow-up: workflow-service must store the validated
  payload when dependency-readiness source data is produced or consumed, and
  the embedded-runtime/infrastructure producer must resolve the registry entry
  before publishing unavailable/ready snapshots or running real probes.
- 2026-05-29 dependency readiness producer registry-lookup slice completed.
  Smallest useful vertical slice: require the embedded-runtime
  dependency-readiness snapshot producer to receive the backend requirements
  registry, resolve each queued work item's `DependencyRequirementsId` before
  publishing the no-probe unavailable snapshot, and publish typed unavailable
  diagnostics when the registry payload is missing, stale, invalid, or
  mismatched. Allowed write set: dependency-environment snapshot diagnostic
  helper, embedded-runtime producer lifecycle/tests/README and constructor call
  sites, this milestone, and execution-management ledger.
  No-fallback/no-legacy result: the producer no longer drains work solely from
  the request id and never reconstructs requirement/binding payloads from
  graph/editor/frontend state, technical-fit previews, reduced execution
  plans, runtime-host load targets, `ModelDependencyRequest`, `ModelRefV2`, or
  path/model-path fields. Verification passed: `cargo test -p
  pantograph-embedded-runtime dependency_readiness_lifecycle`; `cargo test -p
  pantograph-dependency-environment-service`; `cargo check -p
  pantograph-uniffi`; `cargo fmt`; `git diff --check`. Verification caveat:
  workflow-service still emits the known unused `set_active_run_execution_plan`
  warning through dependent crate checks. Remaining follow-up: workflow-service
  still needs a standards-compliant source of concrete validated
  requirement/binding payloads to seed the registry before real host probes can
  produce ready evidence.
- 2026-05-29 registry seeding source re-plan selected. Decision: use the
  latest option 1, meaning workflow-service seeds the backend
  `DependencyRequirementsRegistry` from an existing
  `ValidatedDependencyEnvironmentResult` or equivalent validated
  `DependencyRequirementsPayload` produced by the canonical
  dependency-environment boundary. This is not the earlier rejected
  self-contained work-item-payload option; work items stay small and
  task-correlated, while the registry remains the reusable producer lookup
  source. Standards result: the change keeps parse/validate-once semantics,
  keeps embedded-runtime out of planning policy, keeps graph/frontend/Tauri out
  of execution readiness ownership, and keeps async host probing in the
  embedded-runtime/infrastructure shell. Implementation must add a
  workflow-service storage slice that inserts registry payloads only after
  validating the result/payload contract, then queues readiness work using the
  existing requirements id. Missing payload rows, non-current result status,
  id/identity/binding mismatch, stale validation state, or unavailable
  dependency-environment output must fail closed with typed diagnostics and
  must not enqueue a real host probe as if ready. Deferred long-term option:
  a future Pumas/package-facts producer may become the canonical payload source
  if it emits the same validated payload contract; that replacement must not
  require producer-side reconstruction or legacy/path adapters.
- 2026-05-31 runtime-host image projection guardrail slice completed.
  Smallest useful vertical slice: add the backend-owned projection module for
  validated image-generation runtime-host requests while keeping
  `EmbeddedRuntimeHostExecutionPort` fail-closed. Allowed write set:
  embedded-runtime runtime-host image projection module, module index, README,
  this milestone, runtime-host handoff plan, and execution-management ledger.
  No-fallback/no-legacy result: the slice does not call node-engine,
  planned-inference, reduced workflow execution plans, graph paths, Tauri, or a
  compatibility shim; unsupported tasks, unsupported ports, unsupported
  runtimes, invalid devices, missing prompt, and invalid launch handoff facts
  return typed projection errors. Verification passed: `cargo fmt
  --manifest-path crates/pantograph-embedded-runtime/Cargo.toml`; `cargo test
  -p pantograph-embedded-runtime runtime_host_image_execution -- --nocapture`.
  Verification caveat: the focused test command still reports the known
  workflow-service `set_active_run_execution_plan` warning. Remaining
  follow-up: wire the owner-approved full Pumas package-facts source, explicit
  selected backend facts, typed float input options, path-free media output
  projection, and the gateway call behind the runtime-host port.
- 2026-05-31 runtime-host Pumas package-facts resolver slice completed.
  Smallest useful vertical slice: add host-only package-facts resolution from
  validated runtime-host requests while keeping successful execution unwired.
  Allowed write set: embedded-runtime package-facts resolver module, module
  index, README, this milestone, runtime-host handoff plan, and
  execution-management ledger. No-fallback/no-legacy result: package facts are
  resolved only through owner Pumas API using the scheduler-selected model ref;
  the resolver does not read graph paths, reduced execution plans, `ModelRefV2`,
  node-engine preflight output, planned-inference hosts, Tauri state, selector
  summaries, or display metadata. Verification passed: `cargo fmt
  --manifest-path crates/pantograph-embedded-runtime/Cargo.toml`; `cargo test
  -p pantograph-embedded-runtime runtime_host_package_facts -- --nocapture`.
  Verification caveat: the focused test command still reports the known
  workflow-service `set_active_run_execution_plan` warning. Remaining
  follow-up: compose package facts, load-target resolution, and image planning
  projection inside `EmbeddedRuntimeHostExecutionPort`; add path-free media
  artifact output projection; plan explicit selected-backend and typed float
  input contracts.
- 2026-05-31 runtime-host media artifact sink re-plan selected.
  Decision: implement option 2 before wiring successful image gateway
  execution. Add a narrow backend-owned `RuntimeHostMediaArtifactSink` rather
  than injecting the full `WorkflowService` into
  `EmbeddedRuntimeHostExecutionPort`, returning inline base64 media in
  runtime-host outputs, or keeping execution fail-closed as the next
  implementation path. The sink owns only generated-media persistence and
  returns `RuntimeHostExecutionMediaArtifactRef`; the port owns validation,
  Pumas package/load-target resolution, gateway execution, response shaping,
  and typed diagnostics. This keeps persistence mechanics independent from
  runtime execution and keeps Tauri/frontend, scheduler policy, graph editor,
  node-engine, reduced execution plans, `ModelRefV2`, and fake artifact refs
  out of the successful output path. Required verification for the future
  slice: sink tests for deterministic artifact ids/attribution or artifact
  store delegation, missing-sink diagnostics, write-failure diagnostics, and a
  runtime-host response test proving completed image execution returns only
  path-free media artifact refs.
- 2026-05-31 runtime-host media artifact sink slice completed.
  Smallest useful vertical slice: add the backend-owned media artifact sink
  contract and workflow-service-backed image implementation while keeping
  successful gateway execution unwired. Allowed write set: embedded-runtime
  media sink module, module index, README, this milestone, runtime-host
  handoff plan, and execution-management ledger. No-fallback/no-legacy result:
  generated image outputs are persisted only through the backend artifact
  store boundary; invalid base64 and missing artifact-store configuration
  return typed sink errors; no inline media, fake artifact refs, graph paths,
  Tauri/frontend logic, scheduler persistence workaround, planned-inference
  branch, or `ModelRefV2` path was added. Verification passed: `cargo fmt
  --package pantograph-embedded-runtime`; `cargo test -p
  pantograph-embedded-runtime runtime_host_media_artifact_sink --
  --nocapture`. Verification caveat: the focused test command still reports
  the known workflow-service `set_active_run_execution_plan` warning.
  Discovered follow-up: runtime-host and scheduler media refs currently use
  identifier validation for `media_type`, so the sink returns identifier-safe
  values such as `image_png` while artifact descriptors retain the real MIME
  type. A future shared contract cleanup should either rename that field to a
  media type id or allow MIME values without weakening artifact id validation.
  Remaining follow-up: inject the sink into `EmbeddedRuntimeHostExecutionPort`,
  map missing sink/write failures into typed runtime-host diagnostics, call
  the image gateway, and project completed image results into path-free media
  artifact outputs.
- 2026-05-31 runtime-host execution port dependency-seam slice completed.
  Smallest useful vertical slice: make `EmbeddedRuntimeHostExecutionPort`
  depend on narrow load-target and media-artifact sink boundaries while still
  failing closed before successful gateway execution. Allowed write set:
  embedded-runtime execution port, load-target module, README, this milestone,
  runtime-host handoff plan, and execution-management ledger. Implementation:
  added `RuntimeHostLoadTargetResolver`, implemented it for the existing Pumas
  resolver, stored trait-object dependencies in the port, added a full
  dependency-pair constructor, and added a missing-media-sink rejection after
  load-target resolution. No-fallback/no-legacy result: the port still emits
  only typed rejected responses for missing dependencies, load-target errors,
  and unwired runtime execution; no gateway call, planned-inference branch,
  node-engine launch, Tauri persistence logic, graph path, or fake artifact ref
  was added. Verification passed: `cargo fmt --package
  pantograph-embedded-runtime`; `cargo test -p pantograph-embedded-runtime
  runtime_host_execution_port -- --nocapture`. Verification caveat: the
  focused test command still reports the known workflow-service
  `set_active_run_execution_plan` warning. Remaining follow-up: compose
  package-facts resolution, image projection, gateway execution, sink-backed
  output writing, and typed gateway/write diagnostics inside the port.
- 2026-05-31 runtime-host image execution composition slice completed.
  Smallest useful vertical slice: compose package-facts resolution,
  load-target resolution, image projection, gateway execution, and
  sink-backed output writing inside `EmbeddedRuntimeHostExecutionPort`, while
  still leaving production composition wiring as a follow-up. Implementation:
  added `RuntimeHostPackageFactsResolver`, made the port call
  `InferenceGateway::generate_image_from_planning_input`, map successful
  generated images through the media artifact sink, and return completed
  `RuntimeHostExecutionOutputValue::MediaArtifactRef` outputs. Gateway and
  artifact-write failures return typed failed runtime-host responses.
  No-fallback/no-legacy result: the successful path consumes only validated
  scheduler handoff, owner-resolved Pumas package/load-target facts, canonical
  inference gateway planning, and backend artifact-store refs; it does not use
  Tauri/frontend logic, node-engine launch, planned-inference hosts, reduced
  execution plans, graph paths, scheduler persistence workarounds, inline
  media, fake refs, or `ModelRefV2`. Verification passed: `cargo fmt
  --package pantograph-embedded-runtime`; `cargo test -p
  pantograph-embedded-runtime runtime_host_execution_port -- --nocapture`;
  `cargo test -p pantograph-embedded-runtime runtime_host_package_facts --
  --nocapture`; `cargo test -p pantograph-embedded-runtime
  runtime_host_load_target -- --nocapture`; `cargo test -p
  pantograph-embedded-runtime runtime_host_media_artifact_sink --
  --nocapture`. Verification caveat: the focused commands still report the
  known workflow-service `set_active_run_execution_plan` warning. Discovered
  fixed issue: `runtime_host_load_target` tests still expected the old
  `selected_artifact_path`/caller-observed entry path in the canonical
  fixture; the test now asserts those path-shaped fields stay absent.
  Remaining follow-up: wire embedded-runtime production composition through a
  shared backend artifact writer handle, inject package-facts resolver,
  load-target resolver, inference gateway, and artifact-writer-backed media
  sink into the scheduler dispatch runtime-host port, then add session-level
  completed task result coverage.
- 2026-05-31 production artifact-writer composition re-plan selected.
  Decision: use option 2, a shared backend-owned artifact writer handle, before
  enabling hosted production image execution. The current focused image path
  can complete when dependencies are injected, but production composition
  cannot safely pass `Arc<WorkflowService>` into the runtime-host media sink
  because `WorkflowService` owns the runtime-host port before it is wrapped,
  creating a service self-reference or pushing artifact persistence policy into
  the wrong layer. Required next slice: introduce/expose the narrow writer at
  the workflow-service artifact boundary, make workflow-service artifact APIs
  and runtime-host media output use the same writer, construct the writer in
  backend composition before sharing the service, and wire the runtime-host
  port with package-facts resolver, load-target resolver, inference gateway,
  and artifact-writer-backed sink. Rejected/deferred options: late-bound
  delegating runtime-host port is emergency-only mutable lifecycle state;
  workflow-service-owned runtime port factory risks moving infrastructure
  assembly into workflow-service; keeping production fail-closed is only a
  temporary guardrail. Verification must prove no self-reference, no Tauri
  business logic, typed diagnostics for partial wiring, and completed
  runtime-host responses recorded as scheduler task results.
- 2026-05-31 shared backend artifact writer slice completed.
  Smallest useful vertical slice: introduce the shared backend artifact writer
  handle and refactor runtime-host media output to depend on it, without
  enabling hosted production image execution yet. Implementation:
  `WorkflowArtifactWriter` now wraps the backend artifact store and is exposed
  by workflow-service; `WorkflowService` uses it for artifact facade methods
  while preserving diagnostics ownership; `WorkflowServiceRuntimeHostMediaArtifactSink`
  now depends on the writer instead of `Arc<WorkflowService>`. No-fallback/
  no-legacy result: no runtime fallback, graph path, `ModelRefV2`,
  planned-inference branch, Tauri persistence logic, inline media output, or
  fake artifact ref was added. Verification passed: `cargo fmt --package
  pantograph-workflow-service --package pantograph-embedded-runtime`; `cargo
  check -p pantograph-workflow-service`; `cargo check -p
  pantograph-embedded-runtime`; `cargo test -p pantograph-embedded-runtime
  runtime_host_media_artifact_sink -- --nocapture`; `cargo test -p
  pantograph-embedded-runtime runtime_host_execution_port -- --nocapture`;
  `cargo test -p pantograph-workflow-service artifact_store -- --nocapture`.
  Verification caveat: the focused commands still report the known
  workflow-service `set_active_run_execution_plan` warning. Remaining
  follow-up: hosted composition must create or receive the shared writer before
  service sharing, inject the writer-backed media sink into the production
  runtime-host port with package-facts/load-target/gateway dependencies, and
  add session-level completed task result coverage.
- 2026-05-31 runtime-host failed-result scheduler slice completed.
  Smallest useful vertical slice: fix failed runtime-host response handling in
  the task-level session runner before continuing the full production image
  session path. Implementation: failed, unavailable, and invalid
  `RuntimeHostExecutionResponse` values now persist as failed
  `WorkflowSchedulerTaskResult` records and transition the scheduler task to
  terminal-failed with bounded diagnostics. Completed runtime-host responses
  still require completed result status and a completed task transition.
  No-fallback/no-legacy result: no compatibility adapter, graph-path prompt,
  `ModelRefV2`, node-engine planned-inference branch, or Tauri-owned business
  policy was added; failed runtime-host execution fails closed through
  scheduler task state. Verification passed: `cargo fmt -p
  pantograph-workflow-service`; `cargo test -p pantograph-workflow-service
  workflow_execution_session_records_failed_runtime_host_result_as_terminal_task_failure
  -- --nocapture`; `cargo test -p pantograph-workflow-service
  workflow_execution_session_dispatches_ready_runtime_task_through_scheduler_selection
  -- --nocapture`; `cargo check -p pantograph-workflow-service`; and
  `git diff --check` for the touched workflow-service files. Discovered
  boundary: the complete production-composed successful image session cannot
  be finished until Milestone 5d supplies persisted dynamic inference-node
  input/output ports. The static `llm-inference` contract intentionally lacks
  `prompt`; adding the edge is rejected by graph validation, and omitting it
  leaves runtime-host image execution without materialized prompt input. Do
  not add a static prompt compatibility port. Remaining follow-up: complete
  inference-interface resolution/persistence, then add the successful
  scheduler-to-runtime-host image session coverage using the resolved
  descriptor.
- 2026-05-31 missing resource-estimate scheduler guardrail slice completed.
  Smallest useful vertical slice: fail closed before runtime-host dispatch
  when scheduler inference projections do not carry validated resource
  estimate hints. Implementation: task graph schema version 7 adds a typed
  `missing_resource_estimates` projection diagnostic, and runtime-inference
  nodes with empty estimate hints no longer materialize schedulable intents.
  Existing runtime session fixtures now carry explicit RAM/VRAM hints so the
  positive scheduler dispatch path still proves resource claims are present.
  No-fallback/no-legacy result: no zero-resource fallback, graph-path estimate
  inference, node-engine planned-inference branch, `ModelRefV2`, or Tauri-owned
  scheduler policy was added. Verification passed: `cargo fmt -p
  pantograph-workflow-service`; focused task graph tests for missing estimates
  and positive path projection; focused runtime session dispatch test; and
  `cargo check -p pantograph-workflow-service`. Remaining follow-up:
  validation snapshot production must generate conservative model/load and
  execution/context resource estimates from backend-owned facts before the
  production-composed image session path can be completed.
- 2026-05-31 inference validation estimate-hint propagation slice completed.
  Smallest useful vertical slice: carry backend-owned scheduler estimate facts
  through inference interface resolution, validation publication, current
  validation state, and executable snapshot compaction. Implementation:
  resolver facts now expose defaulted scheduler estimate hints; resolution
  projections, node projection records, current scheduler projections, and
  executable validation snapshot nodes preserve those hints. No-fallback/
  no-legacy result: the slice does not invent estimates from graph paths, UI
  state, retired runtime contracts, or zero-resource defaults; absent facts
  remain empty and are blocked by the prior `missing_resource_estimates`
  diagnostic. Verification passed: `cargo fmt -p pantograph-workflow-service`;
  focused projection, publication, current-state projection, and snapshot
  compaction tests; and `cargo check -p pantograph-workflow-service`.
  Remaining follow-up: connect real Pumas/runtime estimate production to
  `InferenceInterfaceResolverFacts` before completing the production-composed
  image session.
- 2026-05-31 production inference facts provider re-plan boundary recorded.
  The estimate-hint contract path is implemented, but production graph
  sessions still use `UnavailableInferenceInterfaceFactsProvider` unless tests
  inject static facts. The next production slice must choose provider
  ownership before editing code. Selected path: implement the six-part
  backend-owned resource-estimate path.
  1. Record the corrected ownership model: Pumas owns static artifact facts;
     Pantograph owns loaded-memory/context estimation, scheduler admission,
     and ledger refinement.
  2. Add a concrete production `InferenceInterfaceFactsProvider` through
     embedded-runtime/backend composition; `workflow-service` consumes the
     provider contract and Tauri/frontend remain transport/presentation only.
  3. Have the provider consume Pumas logical file/component sizes, package
     evidence, runtime/device/task-shape facts, and proven residency facts.
  4. Produce conservative `SchedulerEstimateHint` values with checked
     arithmetic and typed impossible/overflow diagnostics; same-model reuse
     may count only when runtime state proves the model is already resident.
  5. Preserve fail-closed behavior for missing, stale, ambiguous, unsupported,
     or overflowed inputs so task projection remains blocked by
     `missing_resource_estimates`.
  6. After the complete inference path is proven, add diagnostics-ledger
     refinement that compares predicted and observed memory by typed
     model/runtime/device/task signatures.
  Rejected as default: inventing graph/session fallback estimates, adding
  frontend/Tauri policy, treating Pumas as an exact runtime-memory oracle,
  using selector summaries as executable authority, assuming memory will free
  after unrelated running tasks complete, or duplicating runtime/Pumas
  interpretation inside graph session state.
- 2026-06-05 workflow-service memory-impact legacy path cleanup slice
  completed. Smallest useful vertical slice: stop `llm-inference`
  memory-impact classification from treating graph-local `model_path` as
  model identity. Implementation removed `model_path` from the KV-capable
  model-change classifier while preserving canonical `pumas_model_ref` model
  identity handling. No-fallback/no-legacy result: path-shaped graph data can
  no longer produce the model-change reason in this backend classifier, and no
  scheduler/runtime-host/Pumas facts were adapted back into legacy graph
  fields or DTOs. Verification passed: `cargo test -p
  pantograph-workflow-service
  kv_capable_legacy_model_path_change_is_not_model_identity --lib`; `cargo
  test -p pantograph-workflow-service memory_impact --lib`; `cargo fmt -p
  pantograph-workflow-service -- --check`; `cargo check -p
  pantograph-workflow-service`. Remaining follow-up: top-level `model` and
  `model_id` still require a separate caller inventory before cleanup because
  they may represent stable canonical identity rather than legacy path
  semantics.
- 2026-06-05 graph-facing `model_ref` alias cleanup slice completed.
  Smallest useful vertical slice: remove graph-facing `model_ref` aliases from
  Pumas option metadata, Tauri hydration, frontend selection helpers, mock
  graph node definitions, graph validation, graph fingerprint fixtures, and
  automatic edge-insert priority. Implementation keeps backend-internal Pumas
  selector DTO `model_ref` fields where they are source-contract field names,
  but option metadata and graph authoring now expose only `pumas_model_ref` as
  model identity. Non-null `data.model_ref` now produces a typed
  `InvalidPumasModelReference` stale graph diagnostic; edge insertion demotes
  `model_ref` and keeps `pumas_model_ref` preferred. No-fallback/no-legacy
  result: no migration shim, frontend inference, Tauri policy, runtime launch
  input, or compatibility alias was added. Verification passed: focused
  workflow-service contract validation, connection-insert, and graph
  fingerprint tests; `src-tauri` `puma_lib` tests; targeted Node helper/mock
  tests; `cargo fmt` checks for workflow-service/workflow-nodes/src-tauri;
  `cargo check` for workflow-service, workflow-nodes, and src-tauri;
  `npm run typecheck`; targeted ESLint for touched frontend files; strict
  graph-facing alias search; and `git diff --check`. Verification deviation:
  selector option tests inside
  `crates/workflow-nodes/src/input/puma_lib.rs` are not compiled under the
  default `workflow-nodes --lib` target; `cargo test -p workflow-nodes --lib
  -- --list` currently exposes only the puma-lib descriptor/run tests.
  Remaining follow-up resolved in the next descriptor slice.
- 2026-06-05 `unload-model` descriptor model-ref cleanup slice completed.
  Smallest useful vertical slice: remove the graph-facing `model_ref` input
  from the `unload-model` workflow-node descriptor. Implementation updates the
  descriptor to expose only the trigger input and updates the module docs/tests
  to state runtime lifecycle is scheduler/runtime-host-owned, not selected by
  graph model reference ports. No-fallback/no-legacy result: the node remains
  fail-closed if executed and no replacement graph lifecycle command,
  compatibility alias, Tauri/frontend policy, or runtime launch input was
  added. Verification passed: `cargo test -p workflow-nodes unload_model
  --lib`; `cargo fmt -p workflow-nodes -- --check`; `cargo check -p
  workflow-nodes`; strict search for `model_ref` input/port references in
  `crates/workflow-nodes/src/processing/unload_model.rs`.
