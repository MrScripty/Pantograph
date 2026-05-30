# Runtime Host Handoff And Legacy Execution Removal

## Objective

Replace the successful `model_path`/`ModelRefV2` execution path with direct
scheduler-to-runtime-host dispatch. The runtime host consumes the
scheduler-owned `SchedulerRuntimeHandoff`, resolves Pumas-approved executable
load targets only at the boundary that needs them, then invokes
runtime-specific execution code without exposing those paths to graph editor,
node-engine task intent, scheduler capability hints, reduced workflow execution
plans, or saved workflow identity.

This is the selected re-plan outcome: split the work into Milestone 5b and use
the runtime-host replacement direction. Do not implement a
`SchedulerRuntimeHandoff` to `ModelRefV2` adapter.

Milestone 5a is closed as scheduler-contract complete. Milestone 5b is the hard
gate for actual legacy deletion: define the canonical runtime-host
request/response and host-owned Pumas load-target resolution first, then wire
dispatch-selected scheduler handoff directly into runtime-host execution before
deleting the old successful `ModelRefV2`/`model_path` execution paths.

2026-05-29 re-plan decision: use the contract-first readiness/handoff
replacement path. The remaining legacy dependency/runtime cleanup is not a
graph-authoring cleanup slice and must not be solved by adapting old
`ModelDependencyRequest`, `ModelRefV2`, `model_path`, direct runtime task, or
node-engine preflight APIs into successful execution. The next implementation
sequence must consume the existing canonical
`DependencyReadinessProofEnvelope` and runtime-host handoff contracts, migrate
scheduler/workflow-service production paths to those contracts in validated
vertical slices, then delete or fail closed old entry points before removing
them. Do not introduce a second readiness proof type.

## Problem

Milestone 5a established scheduler-owned readiness admission and runtime
handoff contracts. The next attempted source replacement found that
`enforce_dependency_preflight` is only one part of the successful legacy path:

- node-engine dependency preflight returns `Option<ModelRefV2>`
- PyTorch, llama.cpp, and audio execution read `model_path`
- runtime nodes emit `ModelRefV2` through `build_model_ref_v2`
- embedded-runtime dependency preflight resolves `ModelRefV2`

Changing only the preflight return type would preserve legacy successful
execution. Converting scheduler handoff back into `ModelRefV2` would be a
compatibility bridge and is not allowed.

A later implementation attempt found an additional boundary: the current
planned image host launches execution from a reduced
`WorkflowExecutionPlanNodeDecision`, and workflow/session execution still
advances by asking node-engine to demand output nodes for the whole run. That
reduced projection can feed diagnostics and inspection, but it is not the
authoritative scheduler handoff. It does not carry the full validated dispatch
state needed by `RuntimeHostExecutionRequest`, including the real
`SchedulerRuntimeHandoff`, dependency environment reference, reservation lease,
batching group, readiness proof, selected dispatch facts, and typed
materialized inference inputs derived from the canonical model-specific
interface descriptor.

Milestone 5b therefore depends on option 4 task-level scheduler orchestration
before its remaining production wiring continues. Scheduler dispatch must call
runtime-host execution directly with the actual task-level handoff, and
workflow progress must be driven by scheduler task state rather than
whole-workflow output-node demand. Do not synthesize a handoff from reduced
workflow execution-plan fields, and do not keep planned inference as an
alternate successful launch path.

The current re-plan boundary is broader than a single preflight return type:
readiness proof production, freshness checks, queue admission, dispatch handoff,
runtime-host load-target resolution, node-engine launch retirement, and old
runtime fixtures must move together through a standards-compliant sequence.
The allowed transition is contract-first and fail-closed. Old APIs may remain
temporarily only as diagnostic-only guards that reject successful execution
with typed replacement guidance; they must not adapt canonical facts back into
legacy success behavior.

## Responsibility Boundaries

- **Scheduler:** owns readiness admission, dispatch selection, task queueing,
  resource admission, dependency policy, batching, retry/defer/fail decisions,
  and the moment a task is handed to a runtime host.
- **Scheduler dispatch orchestrator:** builds `RuntimeHostExecutionRequest`
  only from a validated dispatch-selected `SchedulerRuntimeHandoff` plus
  workflow-service-owned typed materialized inputs, invokes the runtime-host
  execution port, and records the returned task state and diagnostics. It must
  not resolve executable load targets, own scheduler selection policy, or call
  worker APIs directly.
- **Node-engine:** validates graph semantics, submits path-free task intent,
  and consumes scheduler task state/results. It must not launch inference
  runtimes, resolve executable paths, create `ModelRefV2`, choose
  runtimes/devices, inspect reduced execution-plan decisions to execute
  inference, or repair path-shaped inputs.
- **Runtime host / embedded runtime:** consumes `RuntimeHostExecutionRequest`,
  validates the embedded scheduler handoff, resolves Pumas-approved load
  targets, manages runtime execution, and records diagnostics.
- **Inference/runtime crates:** execute with host-owned executable facts and
  runtime-specific request contracts. They do not read graph-owned
  `model_path` fields.
- **Graph editor/frontend:** displays capability/task/diagnostic facts and
  optional typed constraints. It must not pass executable paths or final
  scheduler decisions as graph inputs.

## Implementation Direction

Use the clean replacement path:

1. Confirm the existing canonical `DependencyReadinessProofEnvelope` and
   runtime-host handoff contracts cover the remaining production execution
   paths before editing those paths. The proof must carry typed dependency
   readiness, selected environment identity, descriptor/runtime/device
   freshness, Pumas model/artifact identity, correlation ids, and bounded
   diagnostics without graph paths or executable load targets.
2. Define host-facing execution input that consumes a dispatch-selected
   `SchedulerRuntimeHandoff`, scheduler dispatch decision, dependency
   readiness proof, and workflow-service-owned materialized runtime inputs.
   It must carry no `ModelRefV2` and no reduced execution-plan projection.
3. Create the shared dependency-readiness snapshot provider in the backend
   composition root before `WorkflowService` is shared, wire the same provider
   into runtime readiness admission and graph-session dependency actions, and
   keep async host package/runtime probing in an embedded-runtime or
   infrastructure lifecycle owner. The selected next architecture is a
   canonical dependency inventory service: the snapshot producer asks the
   inventory boundary for dependency observations, and concrete providers own
   Python package, managed-runtime, runtime-feature, device-toolchain, and
   system-package source integration. Until that inventory path publishes
   validated snapshots, missing readiness must fail closed.
4. Add runtime-host load-target resolution from Pumas refs/artifact identity to
   executable facts at the host boundary only.
5. Add a scheduler-owned runtime-host execution port and dispatch orchestrator.
   The orchestrator is the only successful caller of runtime-host execution and
   must pass the actual validated `SchedulerRuntimeHandoff`.
6. Complete task-level scheduler orchestration from
   `10-task-level-scheduler-orchestration.md` so production session execution
   has durable task state, task results, and dispatch-selected handoff at the
   task boundary.
7. Retire planned-inference launch from node-engine. Inference nodes become
   scheduler task-intent producers and consumers of scheduler task state/results
   rather than callers of `PlannedInferenceExecutionHost`.
8. Replace PyTorch, llama.cpp, and audio node execution so successful execution
   no longer reads graph `model_path`, reduced execution-plan projections, or
   emits `ModelRefV2`.
9. Replace node-engine dependency preflight output with typed readiness proof
   or scheduler task state consumption. Old preflight APIs must fail closed
   with typed diagnostics until their callers are replaced; they must not
   translate canonical readiness back into `ModelDependencyRequest`,
   `ModelRefV2`, or path-shaped request fields.
10. Delete `ModelDependencyResolver`, `ModelDependencyRequest`, `ModelRefV2`,
   `build_model_ref_v2`, path repair helpers, direct old runtime task success
   fixtures, and path-shaped tests once their successful production callers are
   gone.

Fail-closed behavior is allowed only as a temporary guardrail when a required
canonical handoff is missing. It is not a compatibility mode and must emit
typed diagnostics.

## No-Fallback Requirements

- Do not adapt `SchedulerRuntimeHandoff`, `DependencyPreflightResult`, or Pumas
  load-target facts back into `ModelRefV2`.
- Do not synthesize `SchedulerRuntimeHandoff` from `WorkflowExecutionPlan`,
  `WorkflowExecutionPlanNodeDecision`, backend execution projections, graph
  inputs, or node-engine request context.
- Do not accept `model_path`, `modelPath`, `local_load_path`, or executable
  package paths as graph/node-engine successful execution identity.
- Do not leave `PlannedInferenceExecutionHost` or
  `EmbeddedPlannedInferenceExecutionHost` as alternate successful inference
  launch branches after scheduler-to-runtime-host dispatch is wired.
- Do not leave old resolver calls as alternate successful branches after the
  host handoff is wired.
- Do not let runtime adapters choose scheduler runtime/device policy.
- Do not preserve tests or fixtures that validate path-shaped success behavior.

## Replacement Sequence

1. **Host input contract:** define a runtime-host execution request/response
   shape that consumes `SchedulerRuntimeHandoff`, selected runtime/device
   facts, dependency environment ref, and Pumas model/artifact identity.
2. **Readiness proof consumption:** use the canonical
   `DependencyReadinessProofEnvelope` as the scheduler/workflow-service-facing
   proof that queue admission and dispatch must consume. It must be produced
   from backend validation summaries, dependency-planning facts, selected
   runtime/device facts, descriptor fingerprints, and explicit user
   constraints. It must be bounded, typed, fresh, path-free, and rejected when
   required evidence is missing, stale, ambiguous, or unavailable. If a
   remaining production caller needs fields outside the envelope, stop and
   re-plan the shared contract instead of creating an adapter-local proof.
3. **Pumas load-target resolution:** add the host-owned service that resolves
   executable load targets from Pumas at runtime dispatch time and maps Pumas
   unavailable states to typed diagnostics.
4. **Scheduler execution port:** add a narrow runtime-host execution port at
   the scheduler/application boundary. The port accepts only
   `RuntimeHostExecutionRequest` and returns `RuntimeHostExecutionResponse`.
   The scheduler dispatch orchestrator owns request ids, cancellation/retry
   correlation, and recording response diagnostics; runtime host owns Pumas
   path resolution and worker execution.
5. **Direct dispatch wiring:** update workflow/session execution so a
   dispatch-selected scheduler handoff invokes runtime-host execution directly.
   This step depends on option 4 task-level scheduler orchestration so the
   handoff comes from durable task state rather than whole-workflow reduced
   plans. The reduced `WorkflowExecutionPlan` remains available for inspection
   and diagnostics only; it must not be used to launch inference.
6. **Node-engine launch retirement:** remove node-engine planned-inference
   launch ownership for runtime inference nodes. Affected inference nodes must
   submit or reference schedulable task intent and consume scheduler task
   results/state. Missing scheduler task state fails closed with typed
   diagnostics.
7. **Runtime execution migration:** update PyTorch, llama.cpp, and audio
   execution paths to use host-owned executable facts instead of graph
   `model_path`.
8. **Node-engine preflight replacement:** replace `Option<ModelRefV2>` output
   with typed readiness/task-state facts and fail closed if scheduler-owned
   readiness or task state is absent.
9. **Legacy deletion:** remove `ModelDependencyResolver`,
   `ModelDependencyRequest`, `ModelRefV2`, `build_model_ref_v2`, path repair
   helpers, `PlannedInferenceExecutionHost`,
   `EmbeddedPlannedInferenceExecutionHost`, frontend `modelPath` dependency
   actions, and path-shaped success fixtures.

## Dependency Inventory Service Replan

Selected direction: option 3, canonical dependency inventory service.

The next dependency-readiness source should be a service boundary that owns
cross-kind dependency observation instead of adding more direct probe adapters
to the dependency-readiness snapshot producer. The producer remains the async
lifecycle owner for queued readiness work and snapshot publication, but it
should ask a dependency inventory service to check a validated
`DependencyRequirementsPayload` against the selected runtime/device/environment
context. Concrete inventory providers own source integration and return typed
observations; the producer projects those observations into
dependency-environment snapshots.

Required ownership:

- **Shared dependency contract:** `pantograph-dependency-planning` remains the
  owner for dependency-environment requirement rows, binding rows, result
  states, diagnostics, and serde fixtures unless implementation discovers a
  dependency-cycle reason to introduce a narrower lower-level contract crate.
  If that happens, stop and re-plan the crate boundary before editing
  manifests or lockfiles.
- **Workflow-service/scheduler:** consume dependency-readiness snapshots and
  proofs only. They must not depend on concrete inventory providers, perform
  host probing, infer package names, or interpret dependency requirement names
  as policy.
- **Embedded-runtime/infrastructure:** compose the inventory service and its
  concrete providers. Provider implementations may perform I/O only inside
  their source-owned boundary and must return typed unavailable, stale,
  invalid, unsupported, or not-implemented diagnostics.
- **Graph editor/node-engine/frontend/Tauri:** display validation/readiness
  facts and submit typed user constraints only. They must not own inventory
  state, probe configuration, backend package-manager policy, executable load
  targets, or optimistic backend readiness.

Initial provider ownership:

- **Python package provider:** migrate the existing env-map plus no-shell
  package-readiness probe behind the dependency inventory service first. This
  proves the architecture without changing successful Python readiness
  behavior.
- **Managed runtime provider:** consume managed-runtime inventory facts for
  `RuntimeManagedBinary`. It must not scan paths or infer binary readiness from
  graph data.
- **Runtime feature provider:** consume runtime-registry/capability facts for
  `RuntimeFeature`. It must not move runtime selection policy out of the
  scheduler/runtime registry.
- **Device toolchain provider:** consume device/runtime observation facts for
  `DeviceToolchain`. It must not shell-probe drivers/toolchains ad hoc inside
  dependency planning.
- **System package provider:** remains typed not-implemented until a
  host/system package inventory owner, platform support matrix, and
  package-manager contract are planned. Do not implement this by running local
  shell commands or parsing distro-specific tool output in the snapshot
  producer.

Staged implementation:

1. Add a focused dependency inventory contract and provider trait with typed
   request context, observation rows, freshness/correlation fields, and
   diagnostics. Validation and dispatch should be synchronous and
   correct-by-construction; provider I/O stays async behind the provider
   boundary.
2. Move the existing Python package-readiness path behind the inventory
   service. The behavior must remain no-fallback: explicit Python environments
   use only configured env-map ids, and default-host Python is selected only
   when the canonical payload has no explicit environment/profile identity.
3. Register typed unsupported/not-implemented providers for non-Python
   requirement kinds so mixed payloads fail closed with provider-attributed
   diagnostics instead of stringly local interpretation.
4. Add managed-runtime, runtime-feature, and device-toolchain providers one at
   a time from their source-owned facts. Each provider slice must add focused
   serde fixtures, README ownership updates, and no-fallback tests.
5. Plan system-package inventory separately before implementation because it
   is platform/package-manager specific and likely needs a host inventory
   source rather than direct probing in embedded-runtime.

Standards gates:

- Keep modules below the decomposition target where practical. Split inventory
  contracts, provider dispatch, Python provider, managed-runtime provider,
  runtime-feature provider, and device-toolchain provider into named files
  instead of growing the snapshot producer or Python runtime files.
- Use typed ids/enums, `serde(deny_unknown_fields)` on boundary structs,
  bounded diagnostics, validated wrappers or `TryFrom` conversions for raw
  payloads, and explicit freshness/correlation fields.
- Do not add third-party dependencies for provider dispatch. If a provider
  genuinely needs a new dependency, record dependency ownership, transitive
  cost, feature impact, and verification before editing manifests.
- Do not preserve dual successful paths. After Python readiness is behind the
  inventory service, direct producer-to-package-probe calls should be removed
  or reduced to private provider implementation details.
- Do not recover from missing inventory facts by inspecting graph paths, Pumas
  package names, generic requirement names, shell output, Python package probes
  for non-Python kinds, or old dependency preflight.

Required verification:

- Contract/serde fixtures for inventory requests, observations, stale facts,
  unsupported provider diagnostics, invalid kind/binding shape, mixed-kind
  payloads, and unknown-field rejection.
- Focused provider tests proving Python behavior is preserved through the
  inventory boundary and non-Python kinds fail closed until their source-owned
  providers exist.
- Producer lifecycle tests proving snapshots are published only from inventory
  observations and not from reconstructed graph/path/package-name data.
- Targeted searches proving the snapshot producer does not call concrete
  probes directly after migration and does not interpret non-Python generic
  requirement names locally.
- `cargo fmt -- --check`, focused crate tests, `cargo check` for touched
  crates, line-count review, README traceability updates, and `git diff
  --check`.

## Verification Strategy

- Contract fixtures for host execution request/response and Pumas load-target
  diagnostics.
- Boundary tests proving graph, node-engine, scheduler hints, and saved
  workflow payloads reject executable path fields.
- Runtime-host tests proving Pumas load targets are resolved only inside the
  host boundary and unavailable states fail with typed diagnostics.
- Scheduler dispatch tests proving runtime-host execution requests are built
  only from dispatch-selected `SchedulerRuntimeHandoff` values plus
  workflow-service-owned typed materialized inputs, reject reduced
  execution-plan projections as launch input, and record typed runtime-host
  responses against scheduler task state.
- Node-engine tests proving affected runtime nodes fail closed without
  scheduler task state and do not call `ModelDependencyResolver` or
  `PlannedInferenceExecutionHost`.
- Runtime migration tests for PyTorch, llama.cpp, and audio paths proving
  successful execution uses host-owned executable facts and emits non-legacy
  outputs.
- Deletion checks proving `ModelDependencyResolver`, `ModelDependencyRequest`,
  `ModelRefV2`, `build_model_ref_v2`, `PlannedInferenceExecutionHost`,
  `EmbeddedPlannedInferenceExecutionHost`, and successful `model_path`
  fixtures are gone or replaced by canonical contracts.
- Milestone-order check proving legacy deletion did not start before the
  runtime-host execution request/response and host-owned Pumas load-target
  resolution contracts existed.

## Re-Plan Triggers

- A runtime cannot execute without graph-visible executable paths.
- Pumas load-target resolution cannot produce typed unavailable diagnostics.
- Node-engine cannot fail closed without a compatibility bridge.
- Runtime host execution requires scheduler policy to move out of scheduler.
- Deleting `ModelRefV2` breaks unrelated non-runtime contracts that have not
  been planned for replacement.
- Runtime execution cannot be triggered from scheduler dispatch without making
  node-engine or graph editor aware of executable load targets.
- Existing workflow/session execution cannot represent pausable,
  task-by-task scheduler dispatch without preserving whole-workflow planned
  inference as a runtime launch path.
