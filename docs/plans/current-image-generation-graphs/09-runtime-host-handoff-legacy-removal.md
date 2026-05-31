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

Selected direction: canonical dependency inventory service (the earlier
non-Python readiness option 3), refined with option 2 concrete
per-selected-binding provider dispatch inside that service before the first
real managed-runtime provider. Do not introduce a generic provider registry
yet; reserve that as a later simplification only after multiple real providers
prove that explicit dispatch has become repetitive or error-prone.

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
- **Typed provider source details:** non-Python requirement kinds must add
  typed per-kind detail structs to `pantograph-dependency-planning` before
  their inventory providers are implemented. `RuntimeManagedBinary` must carry
  managed-runtime source identity such as `ManagedBinaryId` plus optional
  selected version/variant/platform scope as typed fields. `RuntimeFeature`
  must carry typed runtime/capability feature identity without using display
  labels. `DeviceToolchain` must carry typed device/toolchain observation
  identity without shell probing. These fields are the provider input contract;
  providers must not recover source identity from generic requirement names.
  A dedicated lower-level inventory contract crate remains an escape hatch only
  if extending `pantograph-dependency-planning` creates an actual dependency
  cycle or ownership conflict.
- **Shared inventory observation contract:** before adding real
  managed-runtime, runtime-feature, or device-toolchain providers, provider
  evidence must be represented as typed observation rows and projected through
  one shared dependency-environment result projector. Providers must not each
  hand-build full `DependencyEnvironmentResult` values or duplicate readiness,
  install-state, stale, and diagnostic mapping policy. Keep the observation
  contract in `pantograph-dependency-planning` unless implementation proves an
  actual dependency cycle or ownership conflict that requires a narrower
  lower-level contract crate.
- **Concrete per-binding provider dispatch:** the inventory dispatcher must
  route each selected binding, plus its referenced requirement row, to the
  concrete dependency inventory provider that owns that dependency kind. It
  must not route the whole payload to one provider. Python and managed-runtime
  selected bindings may therefore be observed by different providers in one
  payload, then merged into one projection. Unsupported or unowned selected
  bindings must produce provider-attributed not-implemented observation rows
  for only those bindings. Provider conflicts, duplicate observations,
  requirement/binding kind mismatches, and unknown referenced requirements are
  typed diagnostics, not fallback behavior.
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

Concrete provider dispatch contract:

- "Provider" means dependency inventory provider, not inference runtime or
  backend. `llama.cpp`, PyTorch, Candle, vLLM, and MLX are runtime/backend
  concepts. Inventory providers observe dependency classes such as Python
  packages, managed-runtime binaries, runtime features, device toolchains, or
  system packages.
- The dispatcher owns provider selection only. It may inspect selected binding
  kind and referenced requirement kind, but it must not interpret provider
  source ids, package names, backend aliases, display names, paths, or graph
  strings as readiness proof.
- A provider-scoped request must carry the immutable work item/request context
  needed for diagnostics plus the selected binding and referenced requirement
  rows that provider owns. Providers return observation rows and diagnostics
  only. They do not build `DependencyEnvironmentResult`, publish snapshots,
  enqueue scheduler work, or mutate graph/session state.
- The inventory service merges provider rows, validates coverage through
  `ValidatedDependencyInventoryObservationProjection`, calls the shared
  projector once, and publishes the resulting snapshot through the existing
  producer lifecycle.
- The first implementation should use explicit fields for the concrete
  providers: Python package provider, managed-runtime provider, and
  not-implemented provider. Do not add a generic provider registry, dynamic
  plugin lookup, ordering rules, or duplicate-provider resolution until a
  later standards review shows that explicit dispatch is causing meaningful
  duplication.
- Initial concrete dispatch must be an explicit typed matrix:
  `DependencyEnvironmentKind::Python` plus
  `DependencyRequirementKind::PythonPackage` goes to the Python package
  provider; `DependencyEnvironmentKind::ManagedBinary` plus
  `DependencyRequirementKind::RuntimeManagedBinary` goes to the managed-runtime
  provider; `RuntimeFeature`, `DeviceToolchain`, and `SystemPackage` remain
  typed not-implemented until their source-owned providers exist. Kind
  mismatches, unowned selected bindings, missing requirement rows, and duplicate
  provider rows are invalid/provider diagnostics, not fallback routing.

Managed-runtime provider matching contract:

- The source of truth is an injected embedded-runtime managed-runtime snapshot
  source backed by `inference::ManagedRuntimeSnapshot` facts. The provider may
  call existing inference managed-runtime APIs only through that source-owned
  boundary. If the source performs file I/O, it must isolate blocking work
  behind the provider boundary and return typed unavailable/stale diagnostics
  instead of blocking scheduler/session code directly.
- `ManagedRuntimeRequirementDetails.managed_binary_id` maps to
  `inference::ManagedBinaryId` by exact key equality with
  `ManagedBinaryId::key()`. Unknown ids are invalid provider input. Do not
  match display names, backend aliases, graph-authored strings, package names,
  or paths.
- If `ManagedRuntimeBindingDetails.managed_binary_id` is present, it must match
  the requirement `managed_binary_id`. If omitted, the requirement id is the
  effective managed binary id.
- Effective `runtime_variant_id`, `version`/`selected_version`, and
  `platform_key` constraints are taken from the binding when present and from
  the requirement otherwise. If both sides specify the same field and disagree,
  the provider emits an invalid observation row for that binding.
- If any version, variant, or platform constraint is present, the provider must
  resolve exactly one matching `ManagedRuntimeVersionStatus` row from the
  matching snapshot. No match is `Missing`; more than one match is `Invalid`
  until the contract is refined. If none of those constraints are present, the
  provider uses the snapshot-level readiness and selection/default facts
  already computed by the managed-runtime source; it must not choose a version
  by display label or ordering.
- `Ready` requires source readiness `Ready`, snapshot `available == true`, and
  for version-scoped observations the matched version row must also be
  executable-ready. `Missing` maps to observation `Missing`. `Unknown`,
  `Downloading`, `Extracting`, and `Validating` map to observation
  `Unavailable` with diagnostics that explain the non-terminal source state.
  `Failed` maps to observation `Failed`. `Unsupported` maps to observation
  `Unavailable` with an unsupported-platform/runtime diagnostic, not to
  provider `NotImplemented`. Provider `NotImplemented` is reserved for
  dependency kinds that do not yet have a real provider.

Runtime-feature and device-toolchain provider-source contract:

- Selected re-plan direction: add one shared provider-source contract for
  `RuntimeFeature` and `DeviceToolchain` before implementing either provider,
  then implement the providers one at a time in validated slices. This is the
  standards-aligned option 3 from the re-plan discussion.
- Contract home: keep the provider-source DTOs in
  `pantograph-dependency-planning` by default because that crate already owns
  dependency source ids, requirement/binding details, observation rows, result
  states, diagnostics, and serde fixtures. The DTOs must not import
  workflow-service, runtime-registry, or inference policy types. If
  implementation proves an actual dependency cycle or ownership conflict, stop
  and re-plan a narrower lower-level inventory contract crate before editing
  manifests or lockfiles.
- Runtime-feature id vocabulary: define a bounded, documented set of
  canonical `RuntimeFeatureSourceId` values before provider implementation.
  The ids represent dependency-readiness features, not display labels or
  runtime backend names. Initial ids should be derived from existing typed
  backend capability concepts such as streaming, device selection, external
  connection support, KV-cache support, custom-code support, component
  preprocessing/postprocessing availability, and request-lifecycle semantics
  only after the contract slice pins their exact wire values and ownership.
- Device-toolchain id vocabulary: define a bounded, documented set of
  canonical `DeviceToolchainSourceId` values before provider implementation.
  The ids represent observable host/runtime toolchain readiness, not shell
  command names or UI labels. Initial ids should be sourced from existing
  typed device/runtime observation facts such as CUDA runtime/device support,
  Metal/MPS availability, llama.cpp device inventory support, and PyTorch
  device probe support only after the contract slice pins their exact wire
  values and ownership.
- Source snapshot DTOs: introduce typed source rows that providers consume
  instead of raw `WorkflowRuntimeCapability`, runtime-registry candidates,
  backend display strings, graph-authored strings, shell output, or generic
  requirement names. Runtime-feature rows must include runtime id, feature id,
  optional runtime variant id, support/readiness state, freshness/correlation
  facts, and bounded diagnostics. Device-toolchain rows must include
  toolchain id, optional runtime id, optional device id/class, readiness state,
  freshness/correlation facts, bounded diagnostics, and optional bounded
  alternatives for explicit invalid/unavailable requests.
- Source ownership: source adapters may project existing typed facts into the
  shared snapshot DTOs, but the projection must live beside the source owner
  or embedded-runtime composition boundary. Runtime-feature source projection
  may consume backend capability facts only as feature evidence; it must not
  consume scheduler ranking decisions or move runtime/device selection policy
  out of the scheduler/runtime registry. Device-toolchain source projection
  may consume inference-owned device probe or managed-runtime device inventory
  facts; it must not run shell probes or infer toolchain readiness from
  package names, graph paths, or runtime display text.
- Provider matching: binding detail ids override requirement ids only when
  present and equal in meaning; conflicting runtime id, feature id, toolchain
  id, runtime variant id, or device id constraints produce `Invalid`
  observation rows. Missing requirement rows, kind mismatches, duplicate
  provider observations, and ambiguous source matches remain typed diagnostics,
  not fallback routing.
- State mapping: a fresh matching supported/ready source row maps to `Ready`.
  A fresh unsupported row maps to `Unavailable` with an unsupported diagnostic.
  A fresh source row whose state is unknown, degraded, probing, or otherwise
  non-terminal maps to `Unavailable` with a source-state diagnostic. A stale
  source row maps to `Unavailable` with a stale diagnostic. No matching source
  row for the requested runtime/feature/toolchain/device scope maps to
  `Missing` with a missing-source diagnostic. Source-reported failures map to
  `Failed`. Provider `NotImplemented` remains reserved for dependency kinds
  without a real provider, not for unsupported source states.
- Alternatives: when an explicit device/toolchain/runtime-feature constraint
  is unavailable but the source has valid alternatives, diagnostics may include
  bounded alternative ids for UI recommendations. The provider must not select
  an alternative or rewrite the requested binding.
- Verification gate: the shared contract slice must add serde round trips,
  `serde(deny_unknown_fields)` rejection, invalid id/detail tests,
  state-mapping tests, stale-source tests, alternatives bounds tests, README
  traceability, and targeted searches proving provider code does not parse
  display names, backend aliases, graph strings, shell output, generic
  requirement names, or scheduler candidate rankings.

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
4. Extend `pantograph-dependency-planning` with typed per-kind requirement and
   binding details for `RuntimeManagedBinary`, `RuntimeFeature`, and
   `DeviceToolchain`. This is a contract-first slice: add validated structs,
   `serde(deny_unknown_fields)` fixtures, unknown-field rejection tests,
   invalid-kind/detail tests, and README traceability before any provider reads
   those fields.
5. Add the shared inventory observation-row contract and projector before real
   non-Python providers. This slice must support mixed selected-binding
   payloads by letting providers emit typed per-binding observations, then
   merging those observations into one validated `DependencyEnvironmentResult`.
   Python and unsupported/not-implemented providers should be adapted to the
   projector first without changing successful Python readiness behavior.
6. Replace payload-wide inventory dispatch with concrete per-selected-binding
   dispatch. This slice should introduce the provider-scoped request shape,
   keep Python behavior unchanged, keep unsupported kinds not-implemented, and
   prove mixed Python plus not-implemented payloads produce one row per
   selected binding through the shared projector.
7. Add the managed-runtime provider from source-owned
   `ManagedRuntimeSnapshot` facts using the matching and state-mapping
   contract above. The provider must support mixed Python plus managed-runtime
   payloads without invoking Python for managed-runtime bindings or rebuilding
   final dependency-environment results locally.
8. Add the shared runtime-feature/device-toolchain provider-source contract.
   This is a contract-first slice: define canonical ids, source snapshot DTOs,
   freshness/correlation fields, state-to-observation mapping, alternative
   diagnostic bounds, serde fixtures, invalid-shape tests, README traceability,
   and no-fallback searches without changing provider behavior.
9. Add the runtime-feature source projection and provider from source-owned
   runtime capability facts. This slice must consume only the shared
   provider-source DTOs, keep scheduler/runtime-registry ranking policy out of
   dependency inventory, and prove unsupported, stale, missing, and ready
   feature states through focused provider tests.
10. Add a dependency-inventory alternative propagation contract before the
   device-toolchain provider. Extend the shared dependency-planning
   observation/status contract so provider-source alternatives can be retained
   on per-binding inventory observations and projected into
   `DependencyBindingStatusRow` results without changing scheduler policy or
   auto-selecting alternatives. This slice must add serde fixtures, validated
   wrappers or `TryFrom` validation, unknown-field rejection, bounds tests,
   README traceability, and focused projector tests proving alternatives
   survive unavailable observations.
11. Add the device-toolchain source projection and provider from source-owned
   device/runtime observation facts. This slice must consume only the shared
   provider-source DTOs, avoid shell probing inside dependency inventory,
   carry explicit unavailable alternatives through the contract from step 10,
   and prove explicit unavailable constraints surface bounded alternatives
   without auto-selecting them.
12. Plan system-package inventory separately before implementation because it
   is platform/package-manager specific and needs a host inventory source
   rather than direct probing in embedded-runtime.
13. Selected system-package path: use option 3, a full staged plan that designs
   the system-package source contract and the inventory simplicity/complection
   boundary together before implementation. This is not a permission to split
   files by size; it is a requirement to identify the independent concerns,
   intentional coupling, accidental coupling risk, and owner for each decision
   before adding package-manager facts.
14. Run an inventory simplicity/complection planning slice before any
   system-package provider code. The slice must decide whether
   provider registration/composition, selected-binding dispatch,
   `cfg(test/standalone)` implementation selection, and fail-closed routing are
   still a single coherent concern or should be separated because the new
   boundary lets maintainers ignore source-specific provider details safely.
   Preserve the existing inventory facade unless the plan records an explicit
   API-breaking reason.
15. Define the system-package source contract before implementation. The
   contract must use typed package ids, platform/package-manager ids, optional
   version/status facts, freshness/correlation fields, bounded diagnostics,
   source-owned alternatives where useful, `serde(deny_unknown_fields)` where
   serialized, and validated wrappers or `TryFrom` conversions for raw source
   rows. The dependency inventory provider may consume this source snapshot but
   must not own package-manager probing policy.
16. Implement system-package inventory in validated slices after the contract:
   first a typed fail-closed/not-implemented provider that proves mixed payloads
   keep system-package diagnostics isolated from other providers, then the real
   host inventory source in platform-specific modules owned by the host/system
   inventory boundary. Real package-manager probing must stay out of graph
   editor, scheduler, shared projector, and provider dispatch code.
17. Selected inventory complection outcome: perform a facade-preserving
   decomposition before system-package provider implementation because the
   current inventory module now couples independent reasoning axes. Keep
   `dependency_inventory.rs` as the public crate-local facade containing
   `DependencyInventoryRequest`, `DependencyInventoryObservation`, the
   `DependencyInventoryProvider` trait, `DependencyInventoryService`, and the
   shared observation-to-result projection. Extract only boundaries that reduce
   reasoning load:
   - `dependency_inventory_dispatch.rs`: provider registration, selected-binding
     dispatch planning, scoped-payload creation, feature-gated dispatch target
     selection, and provider-owned fail-closed not-implemented observations.
   - `dependency_inventory_python.rs`: the Python package provider adapter that
     owns Python probe request selection and projection from Python probe
     outcomes.
   Concrete managed-runtime, runtime-feature, device-toolchain, and future
   system-package providers stay cohesive in their source-owned modules.
   Preserve the existing `DependencyInventoryService` facade and constructor
   behavior; this is a decomposition slice, not an API rewrite.
18. Selected system-package source contract shape: add shared typed contract
   fields before provider implementation. `DependencyRequirementKind::
   SystemPackage` must gain explicit system-package requirement details rather
   than relying on `DependencyRequirementName`. The shared contract should add
   typed `SystemPackageSourceId`, `SystemPackageManagerSourceId`, and
   `HostPlatformSourceId` scalars, plus `SystemPackageRequirementDetails`
   containing at least `package_id`, `package_manager_id`, optional
   `platform_id`, optional architecture/profile fields if source facts require
   them, and any version constraint that cannot be represented by the existing
   requirement-level `version_constraint`. Missing details must validate as a
   typed contract error before execution instead of falling back to generic
   names.
19. Selected system-package provider-source contract shape: add
   `SystemPackageProviderSourceSnapshot` and `SystemPackageProviderSourceRow`
   alongside the existing runtime-feature and device-toolchain provider-source
   contracts. Rows must carry `package_id`, `package_manager_id`,
   `platform_id`, optional architecture/version facts, source-owned state,
   freshness, `checked_at_ms`, bounded diagnostics, and bounded alternatives
   when an explicit package-manager/platform constraint is unavailable but a
   valid alternative is known. Validation must reject duplicate source rows,
   stale rows without diagnostics, invalid source states, oversized
   alternatives, and unknown serialized fields.
20. Selected implementation order after the planning slice:
   - Decomposition-only dispatch/Python-provider extraction with unchanged
     behavior and existing inventory tests.
   - Shared system-package requirement and provider-source contract with serde
     fixtures, invalid-shape tests, README traceability, and no provider
     behavior change.
   - Typed system-package inventory provider that consumes only validated source
     snapshots and initially receives a not-implemented/empty host source in
     production composition, proving mixed payloads remain isolated and
     fail-closed.
   - Host/system inventory source in platform-specific modules is deferred
     future work, not required before returning to scheduler/runtime-host
     execution slices. Only this future source may know package-manager
     commands or platform probing mechanics, and it must publish typed source
     rows rather than shell output.
21. Post-provider re-plan decision: use option 1 for the immediate milestone
   path. System-package readiness remains fail-closed through the typed
   provider and default not-implemented source while the plan returns to the
   scheduler/runtime-host execution path needed for complete inference-run
   testing. Do not add a narrow package-manager probe just because the typed
   provider now exists; that would introduce platform side effects before the
   host/system inventory owner is designed.
22. Documented future option 4: a full host/system package inventory subsystem
   remains required for production-grade system-package readiness. It must be
   planned as its own standards-gated milestone before implementation. The
   future design must cover platform-specific package-manager source modules,
   injected command/probe runners with bounded timeout and output capture,
   no shell interpolation, cache/freshness lifecycle ownership,
   stale/unsupported diagnostics, package-manager/platform support matrix,
   source-owned alternatives, production configuration, and focused fake-runner
   tests. It must preserve the current clean split: host source owns probing,
   caching, freshness, and platform/package-manager semantics; dependency
   inventory owns typed requirement-to-observation matching; shared
   dependency-planning owns result projection; scheduler, graph editor,
   frontend, Tauri, and node-engine consume validated readiness facts only.

Standards gates:

- Apply the coding standards' simplicity/complection test before extracting
  inventory modules. Do not split files merely because they are long; split only
  where the boundary lets maintainers reason about an independent concern
  without also understanding unrelated concerns. Provider source projection,
  provider registration/composition, selected-binding dispatch, shared result
  projection, lifecycle ownership, and diagnostics/fail-closed policy are the
  current reasoning axes to review before adding another provider.
- Keep cohesive provider modules together when their invariants, lifecycle,
  inputs, outputs, and failure behavior are best understood in one place.
  Extract dispatch/composition only if it reduces reasoning load by separating
  provider registration and routing from source-specific observation logic; do
  not introduce organizational-only files.
- For system-package inventory, separate the host package fact source from the
  dependency inventory provider. The host source owns platform/package-manager
  probing, caching, freshness, and stale/unsupported diagnostics; dependency
  inventory owns typed requirement-to-observation matching and shared projector
  input only.
- Use typed ids/enums, `serde(deny_unknown_fields)` on boundary structs,
  bounded diagnostics, validated wrappers or `TryFrom` conversions for raw
  payloads, and explicit freshness/correlation fields.
- Keep provider source ids as typed contract fields. Do not parse
  `DependencyRequirementName`, runtime display names, backend aliases, package
  names, or graph-authored strings to recover managed-runtime, feature, or
  toolchain identity.
- Keep result projection centralized. Concrete providers may return typed
  observation rows, provider diagnostics, freshness/correlation facts, and
  bounded provider alternatives, but dependency-environment
  readiness/install/operation/result-state mapping must live in the shared
  projector so mixed-provider payloads are easy to reason
  about and later consumers can trust the same evidence.
- After the per-binding dispatcher slice, there must be no payload-wide
  Python/not-implemented branch such as "if every selected binding is Python,
  call Python provider, otherwise return not implemented for the whole
  payload." Routing must be per selected binding, then projected once.
- Scheduler, workflow-service, node-engine, graph editor, frontend, and Tauri
  must not import concrete inventory providers or managed-runtime snapshot
  sources. They consume validated payloads, validation/readiness facts,
  diagnostics, and submit/admission state only.
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

- Contract/serde fixtures for inventory requests, observation rows,
  observation-row projection, stale facts, unsupported provider diagnostics,
  invalid kind/binding shape, mixed-kind payloads, and unknown-field rejection.
- Focused provider tests proving Python behavior is preserved through the
  inventory boundary and non-Python kinds fail closed until their source-owned
  providers exist.
- Producer lifecycle tests proving snapshots are published only from inventory
  observations and not from reconstructed graph/path/package-name data.
- Targeted searches proving the snapshot producer does not call concrete
  probes directly after migration and does not interpret non-Python generic
  requirement names locally.
- Targeted searches proving payload-wide dispatch helpers such as
  `selected_payload_is_python_only` are removed or reduced to test-only
  historical references after the concrete per-selected-binding dispatcher
  lands.
- Dispatcher tests proving payload-wide Python/not-implemented branching is
  replaced by per-selected-binding routing, mixed selected-binding payloads
  collect rows from multiple concrete providers, and unknown/mismatched
  bindings fail closed with typed diagnostics.
- Managed-runtime provider tests proving exact managed binary id matching,
  version/variant/platform narrowing, ambiguous-match rejection, missing
  runtime/version diagnostics, readiness-state mapping, and no use of display
  names, backend aliases, graph strings, paths, or package names.
- `cargo fmt -- --check`, focused crate tests, `cargo check` for touched
  crates, line-count review, README traceability updates, and `git diff
  --check`.

Implementation progress:

- 2026-05-30 slice: added the embedded-runtime dependency inventory service
  boundary and Python package provider adapter. The readiness snapshot producer
  now depends on `DependencyInventoryService` for fresh payload checks instead
  of carrying or invoking a package probe runner directly. The Python provider
  still uses the existing no-shell package probe internally, preserving
  explicit Python environment/profile behavior and default-host selection only
  when the canonical payload has no explicit environment/profile identity.
- Verification for the slice: `cargo fmt -- --check`,
  `cargo test -p pantograph-embedded-runtime dependency_inventory`, and
  `cargo test -p pantograph-embedded-runtime dependency_readiness_lifecycle`,
  `cargo check -p pantograph-embedded-runtime`, direct-probe search,
  line-count review, and `git diff --check` passed. The only warning observed
  during `cargo check` is the pre-existing
  `set_active_run_execution_plan` dead-code warning in
  `pantograph-workflow-service`.
- 2026-05-30 follow-up slice: added explicit inventory dispatch for selected
  non-Python requirement or binding kinds. Python-only payloads still route to
  the Python provider; non-Python payloads now publish provider-owned
  `NotImplemented` readiness diagnostics and binding statuses without invoking
  the Python probe or interpreting generic names as readiness evidence.
- Verification for the follow-up slice: `cargo fmt -- --check`,
  `cargo test -p pantograph-embedded-runtime dependency_inventory`, and
  `cargo test -p pantograph-embedded-runtime dependency_readiness_lifecycle`,
  `cargo check -p pantograph-embedded-runtime`, direct-probe search,
  line-count review, and `git diff --check` passed.
- Remaining follow-ups: add source-owned managed-runtime, runtime-feature, and
  device-toolchain inventory providers; add serde/fixture coverage when the
  inventory request/observation contract becomes shared or externally
  serialized; and separately plan system-package inventory before
  implementation.
- Re-plan boundary before the next provider slice: code search found
  source-owned managed-runtime snapshots and runtime capability facts, but the
  plan does not yet define the typed mapping from dependency requirement rows
  to `ManagedBinaryId`, runtime feature ids, or device/toolchain observation
  ids. Before implementing managed-runtime, runtime-feature, or
  device-toolchain providers, decide the source id fields, selected-binding
  detail shape, freshness/correlation semantics, and exact result-state
  projection for ready, missing, unavailable, unsupported, and stale facts.
  Do not implement these providers by parsing generic requirement names,
  scanning runtime capability display text, shell-probing devices/toolchains,
  or treating managed-runtime readiness as scheduler runtime selection.
- 2026-05-30 re-plan decision: use the `pantograph-dependency-planning`
  contract-first path for non-Python inventory sources. Add typed per-kind
  detail structs and fixtures in the shared dependency-planning crate before
  implementing managed-runtime, runtime-feature, or device-toolchain providers.
  This keeps source identity in validated contracts, avoids new crate and
  lockfile churn, preserves current ownership, and matches the standards for
  typed boundaries, README/fixture traceability, narrow modules, sync
  validation with async provider I/O, and no fallback behavior. Revisit a
  dedicated inventory contract crate only if implementation proves a dependency
  cycle or ownership conflict.
- 2026-05-30 contract slice: extended `pantograph-dependency-planning` with
  typed non-Python requirement and binding detail structs for managed-runtime,
  runtime-feature, and device-toolchain source identity. The slice is
  contract-only: inventory providers still return typed not-implemented
  diagnostics until they are implemented against source-owned facts. Validation
  requires typed details for supported non-Python requirement kinds, rejects
  mismatched detail/kind combinations, and rejects unknown legacy detail fields.
- Verification for the contract slice: `cargo fmt -- --check`,
  `cargo test -p pantograph-dependency-planning dependency_environment_result`,
  `cargo test -p pantograph-dependency-planning`, and
  `cargo check -p pantograph-dependency-planning` passed. `git diff --check`
  also passed. Line-count review found production modules under the
  decomposition target (`environment/payload.rs` 486 lines,
  `environment/scalar.rs` 416 lines), but `tests/contract.rs` remains a broad
  existing contract suite over 1k lines. Keep future dependency-environment
  contract tests in focused files instead of growing that file further.
- 2026-05-30 follow-up: updated the embedded-runtime dependency-inventory
  non-Python dispatch fixture to include typed managed-runtime details. This
  preserves the new source-detail contract in downstream tests and avoids
  treating generic requirement names as provider source identity.
- 2026-05-30 provider observation contract re-plan decision: selected the
  observation-contract path before real managed-runtime provider work. The
  next implementation slice must add typed inventory observation rows and a
  single shared projector that builds validated dependency-environment
  results. This makes mixed Python plus non-Python payloads a first-class
  contract and keeps provider evidence reusable for scheduler admission, graph
  validation, diagnostics, and future stale-shape UX without duplicating
  projection policy in each provider. The default contract home remains
  `pantograph-dependency-planning`; stop and re-plan only if implementation
  proves a real dependency cycle or ownership conflict.
- 2026-05-30 observation projection contract slice: added typed inventory
  observation rows plus the shared dependency-environment result projector in
  `pantograph-dependency-planning`. The contract supports mixed selected
  bindings, requires explicit provider evidence for every selected binding,
  rejects unselected/duplicate observation rows, requires stale diagnostics,
  and keeps provider result-state projection centralized before real
  managed-runtime, runtime-feature, or device-toolchain providers are added.
  Verification passed with focused observation projection tests, the full
  dependency-planning crate test suite, crate check, formatting, and
  line-count review.
- 2026-05-30 embedded-runtime observation projector adoption slice: adapted
  the dependency inventory service so Python package provider output and
  unsupported/not-implemented provider output are emitted as typed
  per-binding observation rows, then projected through the shared
  `pantograph-dependency-planning` projector before readiness snapshots are
  published. This removes provider-local full-result construction from the
  successful Python and not-implemented paths while preserving the no-shell
  Python probe behavior. Provider `ProbeNotImplemented` now aggregates to a
  top-level `NotImplemented` readiness result instead of an `Unavailable`
  result with only row-level not-implemented status, so downstream callers see
  the exact provider contract state. The production inventory module tests
  were split into `dependency_inventory_tests.rs` to keep module sizes under
  the decomposition target.
- 2026-05-30 concrete per-binding inventory dispatch slice: replaced the
  payload-wide Python/not-implemented branch with a concrete dispatch plan that
  routes each selected binding to the provider domain implied by its typed
  binding and requirement kinds. Python package bindings are checked through
  the Python provider; matched non-Python domains emit provider-attributed
  not-implemented rows until their source-owned providers exist; mismatched
  binding/requirement kinds emit typed invalid observations. Mixed Python plus
  managed-runtime payloads now project one readiness snapshot from combined
  provider observations without invoking Python for managed-runtime bindings
  or parsing generic requirement names for not-implemented provider identity.
  Verification passed with formatting, focused inventory tests, lifecycle
  tests, crate check, targeted helper-removal search, line-count review,
  README traceability update, and `git diff --check`. The only warning
  observed remains the pre-existing `set_active_run_execution_plan` dead-code
  warning in `pantograph-workflow-service`.
- 2026-05-30 managed-runtime inventory provider slice: added a source-owned
  managed-runtime inventory provider for `RuntimeManagedBinary` bindings. The
  provider consumes injected `ManagedRuntimeSnapshot` facts, maps typed
  `managed_binary_id` values by exact `ManagedBinaryId::key()` equality,
  rejects binding/requirement id or constraint conflicts as invalid, projects
  missing version/variant/platform matches as `ArtifactMissing`, and maps ready
  source facts to ready inventory observations. Standalone composition wires
  the snapshot source from `app_data_dir` behind a blocking-isolated provider
  boundary; default non-standalone composition keeps managed-runtime bindings
  fail-closed as not implemented unless a source is injected. Verification
  passed with formatting, focused inventory tests, lifecycle tests, default and
  standalone crate checks, no-display/path/backend parsing search, line-count
  review, README traceability update, and `git diff --check`. The only warning
  observed remains the pre-existing `set_active_run_execution_plan` dead-code
  warning in `pantograph-workflow-service`.
- 2026-05-30 runtime-feature/device-toolchain provider re-plan boundary:
  runtime-feature and device-toolchain providers must not be implemented until
  their source-owned contracts are planned. The next plan update must define
  the canonical feature/toolchain id vocabulary, the injected source snapshot
  APIs, the owner of each source-to-observation mapping, and the exact
  readiness/diagnostic projection for supported, unsupported, unavailable,
  unknown, and stale states. Do not derive provider evidence from runtime
  display names, backend aliases, generic requirement names, scheduler ranking
  candidates, graph strings, shell output, or ad hoc capability field parsing.
- 2026-05-30 shared runtime-feature/device-toolchain provider-source contract
  slice: added contract-only source snapshot DTOs, canonical feature/toolchain
  and device-class vocabularies, source state/freshness fields, bounded
  alternatives, and validated wrappers in `pantograph-dependency-planning`.
  The slice adds fixture and public API tests for serde, unknown-field
  rejection, unknown id rejection, stale diagnostics, and alternative bounds.
  It intentionally does not implement provider behavior, source projection, or
  embedded-runtime dispatch changes. Runtime-feature provider implementation
  remains the next slice and must consume these DTOs rather than raw
  workflow-service capability DTOs or runtime-registry candidate facts.
- 2026-05-30 runtime-feature inventory provider slice: added runtime-feature
  dependency inventory observation from the shared provider-source snapshot
  contract. Source projection and observation mapping live in separate
  internal modules: standalone composition projects inference backend
  capability facts into `RuntimeFeatureProviderSourceSnapshot` rows, then the
  inventory provider consumes those rows. Runtime-feature provider matching
  uses typed runtime id, feature id, and optional variant id constraints, maps
  source states to typed observation rows and diagnostics, and does not consume
  workflow-service capability DTOs, runtime-registry scheduler candidates,
  graph strings, display names, shell output, or generic requirement names.
- 2026-05-30 device-toolchain alternatives re-plan: device-toolchain provider
  implementation is blocked until bounded provider alternatives can move from
  `DeviceToolchainProviderSourceRow` through `DependencyInventoryObservationRow`
  and into `DependencyBindingStatusRow`. The selected approach is a
  contract-first dependency-planning slice that adds bounded alternatives to
  per-binding observations/status rows and projector tests before implementing
  device-toolchain source projection/provider behavior. The provider slice
  must then consume source-owned device/runtime facts only, avoid shell probing
  inside dependency inventory, and surface alternatives as diagnostics/evidence
  without scheduler auto-selection.
- 2026-05-30 dependency-planning alternatives propagation slice: extended
  `DependencyInventoryObservationRow` and `DependencyBindingStatusRow` with
  bounded `DependencyProviderSourceAlternative` evidence, reused the shared
  provider-source alternative validator, and updated the shared projector to
  copy alternatives through without changing result-state mapping. Added a
  fixture and tests proving unavailable observations preserve alternatives and
  unbounded alternatives fail validation. Existing embedded-runtime providers
  now emit empty alternatives explicitly. This completes the contract
  prerequisite for the device-toolchain provider slice; the next slice must
  consume source-owned device/runtime facts and must not introduce shell
  probing or scheduler auto-selection in dependency inventory.
- 2026-05-30 device-toolchain inventory provider slice: added device-toolchain
  source projection and provider modules in embedded-runtime. The source
  adapter projects inference backend-owned runtime-variant device-class facts
  into `DeviceToolchainProviderSourceSnapshot` rows; the provider consumes
  that contract, matches typed toolchain/runtime/device constraints, and
  carries bounded alternatives into per-binding statuses without selecting
  them. Concrete `device_id` requests fail closed unless source-owned rows
  include concrete device ids. The slice does not shell-probe drivers or
  toolchains and does not consume scheduler/runtime-registry ranking facts,
  graph strings, display names, backend aliases, package names, paths, or
  generic requirement names. Follow-up: before adding system-package inventory,
  run a standards-driven simplicity/complection review of the central inventory
  dispatcher. Split provider registration/composition or selected-binding
  routing only where the new boundary lets maintainers ignore unrelated
  provider-source details safely; do not split solely because of line count.
- 2026-05-30 inventory dispatch/Python-provider decomposition slice:
  `dependency_inventory.rs` now remains the crate-local inventory facade and
  shared observation-to-result projection; `dependency_inventory_dispatch.rs`
  owns provider registration, selected-binding routing, scoped payloads,
  feature-gated dispatch target selection, and provider-owned fail-closed
  not-implemented observations; and `dependency_inventory_python.rs` owns the
  Python package provider adapter. This slice was organization-only: no
  system-package readiness behavior, package-manager probing, shell parsing,
  scheduler policy, graph/path/Pumas-name inference, compatibility shim, or
  legacy preflight behavior was added. Verification passed with the focused
  embedded-runtime inventory/readiness tests and standalone checks; the known
  workflow-service `set_active_run_execution_plan` warning remains.
- 2026-05-30 shared system-package contract slice: added typed system-package
  requirement/binding details, typed system package/package-manager/platform
  source ids, system-package provider-source snapshot/row DTOs, a validated
  provider-source wrapper, fixture coverage, and validation tests before any
  provider behavior. `DependencyRequirementKind::SystemPackage` now requires
  explicit system-package details and rejects package-name-shaped legacy fields
  rather than relying on generic requirement names. Provider-source validation
  rejects duplicate system-package rows, stale rows without diagnostics,
  unknown serialized fields, and oversized alternatives. The slice does not add
  package-manager probing, scheduler policy, graph/path/Pumas-name inference,
  compatibility shims, or legacy preflight behavior. Follow-up: add the typed
  system-package provider against validated source snapshots.
- 2026-05-30 typed system-package inventory provider slice: added a
  source-owned system-package inventory provider for `SystemPackage` bindings.
  The provider consumes only validated `SystemPackageProviderSourceSnapshot`
  facts, matches explicit package/package-manager/platform/architecture details,
  maps source state to observation rows, preserves source alternatives, and
  returns typed invalid diagnostics for missing or contradictory details.
  Standalone composition now routes system-package bindings through this
  provider with a not-implemented source until real host inventory exists;
  tests inject ready and unavailable sources. No package-manager probing, shell
  parsing, graph/path/package-name inference, Python probe reuse, scheduler
  policy, compatibility shim, or legacy preflight behavior was added.
- 2026-05-30 workflow-service dispatch fact mapping slice: mapped validated
  `WorkflowRuntimeDispatchCandidateFactBundle` facts one-to-one into
  `SchedulerDispatchCandidate` values. This is a pure workflow-service
  contract mapping from already-canonical selected runtime/device/model,
  reservation, resource-fit, trait, and batching facts. It does not configure
  the production embedded-runtime provider, acquire runtime-registry leases,
  release reservations, synthesize candidates from graph paths or reduced
  execution plans, or adapt facts back into `ModelRefV2`.
- 2026-05-30 reservation lifecycle re-plan decision: use option 3, a shared
  reservation lifecycle contract with an embedded-runtime implementation.
  Workflow-service emits typed scheduler/application outcomes for leases that
  came from dispatch candidate facts; embedded-runtime implements the concrete
  runtime-registry release, retention-hint mutation, and reclaim/reconcile
  behavior. Do not let workflow-service import runtime-registry internals, and
  do not hide lease cleanup in an opaque provider-local state machine.

### Runtime Dispatch Reservation Lifecycle Contract

Production embedded-runtime provider wiring is blocked until runtime-registry
reservation lifecycle ownership is explicit and tested. The selected option-3
split is:

- **Shared contract owner:** define a narrow reservation lifecycle DTO/port
  contract consumable by workflow-service and implemented by embedded-runtime.
  The contract carries validated scheduler lease ids, workflow/run/task
  correlation, dispatch candidate ids when available, lifecycle outcome,
  terminal/error diagnostics, and idempotency/correlation ids. It carries no
  graph paths, `ModelRefV2`, reduced execution-plan fields, runtime-host load
  targets, or provider-private source facts.
- **Workflow-service application owner:** records which validated candidate
  facts were submitted to scheduler selection and emits lifecycle outcomes for
  no-selection, request validation failure, selected dispatch start,
  runtime-host dispatch failure, runtime-host terminal success/failure,
  cancellation, retry/defer, session close, and duplicate/replayed lifecycle
  observations. It does not call runtime-registry directly and does not choose
  release/reclaim policy.
- **Embedded-runtime infrastructure owner:** implements the lifecycle port
  against `SharedRuntimeRegistry`, releases or updates retention for owned
  leases, runs the existing release-and-reconcile path where reclaim is
  needed, and returns typed diagnostics for unknown, already-released,
  mismatched, or failed release attempts.
- **Scheduler owner:** continues to require reservation facts on dispatch
  candidates and uses those facts only for dispatch selection/handoff. It does
  not own runtime-registry I/O or release side effects.

Required lifecycle outcomes:

- unselected candidate after scheduler no-selection or different candidate
  selected
- candidate request rejected before selection validation completes
- selected candidate dispatch started
- selected candidate runtime-host dispatch rejected before host execution
- runtime-host terminal completed
- runtime-host terminal failed
- workflow/session cancellation
- retry/defer supersession
- session close or active-run cleanup
- duplicate/replayed lifecycle event

Standards gates:

- Keep validation synchronous and side effects behind the embedded-runtime port
  implementation. Do not hold workflow/session locks across async release or
  reclaim calls.
- Use correct-by-construction Rust APIs: typed lease ids, outcome enums,
  bounded diagnostics, `serde(deny_unknown_fields)` where serialized,
  validated wrappers or `TryFrom`, `#[must_use]` on lifecycle application
  results, and idempotency keys for replay.
- Keep the boundary simple by separating independent concerns: workflow-service
  owns task/session outcome emission; embedded-runtime owns registry release
  and retention side effects; scheduler owns selection policy; runtime host
  owns execution.
- Do not add compatibility shims, fallback release guesses, graph-path
  recovery, `ModelRefV2` adapters, reduced-plan handoff synthesis, or provider
  display-string parsing.
- Add focused tests for every required outcome plus duplicate/replay behavior,
  unknown lease diagnostics, selected-vs-unselected release behavior, terminal
  success/failure cleanup, cancellation/session-close cleanup, and no registry
  cleanup calls from workflow-service directly.

Staged implementation:

1. Add the shared reservation lifecycle contract and fixture/validation tests.
   This is contract-only and must not wire production provider emission.
   Completed 2026-05-30 in `pantograph-runtime-host-contracts` with typed
   lifecycle event/application DTOs, validated wrappers, lifecycle port trait,
   fixture coverage, and path-shaped field rejection.
2. Add workflow-service lifecycle emission around dispatch selection and
   runtime-host dispatch using the shared port with a typed unavailable
   default that fails closed when production lifecycle wiring is absent.
   Completed 2026-05-30 in `pantograph-workflow-service`: the scheduler task
   orchestrator now emits `dispatch_started`, unselected-candidate, terminal
   completion/failure, and dispatch-rejected lifecycle events through the
   shared reservation lifecycle port; service construction exposes explicit
   lifecycle-port injection while the default port fails closed before
   runtime-host dispatch.
3. Add the embedded-runtime lifecycle port implementation backed by
   runtime-registry release/retention/reconcile APIs.
   Completed 2026-05-30 in `pantograph-embedded-runtime`: added the staged
   embedded reservation lifecycle port over `SharedRuntimeRegistry`, mapping
   runtime-registry scheduler lease ids to release-and-reconcile behavior,
   idempotent duplicate release applications, typed owner-mismatch
   diagnostics, and dispatch-started no-op applications.
4. Wire embedded-runtime composition to provide both the final dispatch
   candidate provider and reservation lifecycle port together. Do not allow one
   without the other in production.
5. Only after lifecycle verification passes, join Pumas package facts, runtime
   capability facts, and runtime resource facts into real resource-backed
   candidate bundles.

2026-05-30 workflow-service lifecycle emission slice verification:

- `cargo fmt -- --check`
- `cargo test -p pantograph-workflow-service reservation_lifecycle --lib`
- `cargo test -p pantograph-workflow-service workflow_execution_session_dispatches_ready_runtime_task_through_scheduler_selection --lib`
- `cargo check -p pantograph-workflow-service` (passes with existing
  `set_active_run_execution_plan` dead-code warning)
- `git diff --check`

Discovered follow-up: production embedded-runtime composition must inject the
reservation lifecycle port together with the final resource-backed dispatch
candidate provider. Workflow-service now fails closed if that lifecycle owner
is absent, so the next slice must implement the embedded-runtime port before
real resource-backed candidates can be enabled.

2026-05-30 embedded-runtime lifecycle port slice verification:

- `cargo fmt -- --check`
- `cargo test -p pantograph-embedded-runtime reservation_lifecycle --lib`
- `cargo check -p pantograph-embedded-runtime` (passes with existing
  workflow-service `set_active_run_execution_plan` dead-code warning)
- `git diff --check`

Discovered follow-up: the embedded lifecycle port is intentionally staged with
`#[allow(dead_code)]` until production composition wires it together with the
final dispatch candidate provider. The next slice must remove that staging
allowance by injecting the port wherever real resource-backed candidate bundles
are enabled.

2026-05-30 re-plan boundary: composition wiring cannot be completed as a
simple connection slice because hosted embedded-runtime construction receives
an already shared `Arc<WorkflowService>`, while the workflow-service
reservation lifecycle port is currently a builder-style dependency that must
be supplied before sharing. Continuing without a design decision would either
wire the lifecycle port without the final candidate provider, violating the
"configure both together" rule, or add ad hoc mutable configuration to
workflow-service. The next plan update must choose a standards-compliant
composition direction before implementation continues.

Options to resolve:

1. Add a workflow-service setter for the lifecycle port using interior
   mutability around scheduler orchestration. This is the smallest surface but
   weakens the current constructor-owned dependency shape and risks runtime
   reconfiguration of dispatch behavior after sessions exist.
2. Change embedded-runtime constructors so hosted/standalone composition owns
   `WorkflowService` before it is wrapped in `Arc`, then build one canonical
   workflow service with dependency readiness, candidate provider, runtime-host
   port, and lifecycle port configured together. This preserves explicit
   composition and avoids mutable post-share wiring, but it is a broader
   constructor migration.
3. Introduce a small embedded-runtime workflow-service factory/composition
   object that returns `Arc<WorkflowService>` only after all required paired
   runtime dispatch dependencies are present. This keeps pairing explicit and
   avoids exposing mutable setters, but it adds a new composition boundary that
   must replace current direct `Arc<WorkflowService>` construction paths.

Selected decision: use option 3. Add a focused embedded-runtime
workflow-service composition factory that returns `Arc<WorkflowService>` only
after all paired runtime dispatch dependencies are configured. The factory is
not a new policy owner: it may create and connect concrete dependencies, but
it must not choose scheduler policy, runtime/device ranking, Pumas
package-fact policy, runtime-registry release policy, or graph validation
behavior.

Standards alignment:

- Simplicity/complection: the new boundary separates runtime composition from
  workflow-service task orchestration, scheduler policy, runtime-registry
  lifecycle side effects, and Tauri/app transport. This is justified because
  it lets each owner change independently while preserving the paired-provider
  invariant.
- Composition root: concrete runtime dispatch candidate providers,
  runtime-host ports, dependency-readiness components, and reservation
  lifecycle ports must be selected at the embedded-runtime composition
  boundary before the workflow service is shared.
- State/lifecycle ownership: workflow-service remains the application emitter
  for lifecycle events; embedded-runtime remains the infrastructure owner for
  runtime-registry release/reconcile side effects; runtime-registry remains
  the retention/reclaim policy owner. The composition factory only wires these
  owners together.
- No mutable post-share wiring: do not add production setters that mutate
  dispatch dependencies on `Arc<WorkflowService>` after sessions can exist.
  Tests may still build workflow services directly when they intentionally use
  default fail-closed dependencies.
- No fallback/legacy: the factory must not synthesize candidates from graph
  paths, reduced execution plans, frontend state, display strings,
  `ModelRefV2`, or provider-private cleanup state.

Next implementation sequence:

1. Add a narrow embedded-runtime workflow-service composition module that owns
   construction of a fully wired `WorkflowService` and documents why the
   boundary exists.
   Completed 2026-05-30: `pantograph-embedded-runtime` now has a focused
   workflow-service composition module that attaches dependency-readiness
   components and capacity limits before sharing the service.
2. Move standalone workflow-service construction through that module while
   preserving the public embedded-runtime facade shape.
   Completed 2026-05-30: standalone runtime construction now creates the
   dependency-readiness snapshot producer from the composition-owned
   components and receives the shared workflow service from the same
   composition boundary.
3. Add hosted construction support that accepts runtime-registry/gateway
   inputs early enough to configure the reservation lifecycle port before the
   service is wrapped in `Arc`.
   Started 2026-05-30: the composition module now accepts a host-customized
   unshared `WorkflowService` and applies dependency-readiness wiring plus
   loaded-runtime capacity configuration before returning `Arc<WorkflowService>`.
   The remaining hosted work is to pass runtime-registry/gateway dispatch
   dependencies into this boundary and reject partial production wiring.
4. Wire the final runtime dispatch candidate provider and reservation
   lifecycle port through the same composition path. The plan still forbids
   enabling non-empty resource-backed candidates until both dependencies are
   present.
   Started 2026-05-30: the composition factory now exposes a single typed
   `EmbeddedWorkflowServiceDispatchDependencies` bundle for runtime dispatch
   candidate provider, runtime-host execution port, and reservation lifecycle
   port. Partial production wiring is not represented by the factory API. The
   remaining work is to supply the real resource-backed implementations through
   that bundle.
5. Remove the staged `#[allow(dead_code)]` from the embedded lifecycle port
   when production composition exercises it.
6. Add tests proving production composition fails closed or refuses to build
   when only one of the paired dispatch dependencies is present, and proving
   direct test construction still uses fail-closed defaults.

2026-05-30 workflow-service composition factory slice verification:

- `cargo fmt -- --check`
- `cargo test -p pantograph-embedded-runtime workflow_service_composition --lib`
- `cargo check -p pantograph-embedded-runtime` (passes with existing
  workflow-service `set_active_run_execution_plan` dead-code warning)

Discovered follow-up: hosted construction still receives an already shared
workflow service. The next slice must add hosted construction support that
accepts runtime-registry/gateway inputs before service sharing, then wire the
final dispatch candidate provider and reservation lifecycle port together.

2026-05-30 hosted-service composition helper slice verification:

- `cargo fmt -- --check`
- `cargo test -p pantograph-embedded-runtime workflow_service_composition --lib`
- `cargo check -p pantograph-embedded-runtime` (passes with existing
  workflow-service `set_active_run_execution_plan` dead-code warning)

Discovered follow-up: this slice proves hosted callers can preserve their
store/ledger/artifact customization without sharing the service first, but it
does not yet move Tauri/UniFFI startup through the factory or configure the
runtime-registry-backed lifecycle port. Those remain blocked until the
production dispatch dependency pair is represented explicitly.

2026-05-30 paired dispatch dependency factory slice verification:

- `cargo fmt -- --check`
- `cargo test -p pantograph-embedded-runtime workflow_service_composition --lib`
- `cargo check -p pantograph-embedded-runtime` (passes with existing
  workflow-service `set_active_run_execution_plan` dead-code warning)

Discovered follow-up: the factory API now requires dispatch dependencies as a
complete typed bundle, but the real runtime-registry resource-backed candidate
provider and runtime-host execution port are not yet supplied. Keep
resource-backed candidate sets disabled until those concrete implementations
are wired through this bundle.

2026-05-30 re-plan trigger: final resource-backed provider assembly has no
selected owner. The embedded-runtime sources for Pumas package facts,
runtime-registry capability facts, and runtime-registry resource reservation
facts are staged independently, and workflow-service has the provider trait
that consumes validated `WorkflowRuntimeDispatchCandidateFactBundle` values.
The missing boundary is the assembler that joins those facts for one scheduler
task, validates freshness and compatibility, reserves resources, and returns
typed workflow-service provider diagnostics. Implementing that assembler
inside `EmbeddedWorkflowServiceComposition` would violate the factory's
selected responsibility because it would move dispatch fact policy and
resource-fit assembly into construction code. Implementing it inside
workflow-service would violate source ownership because workflow-service would
need embedded-runtime/Pumas/runtime-registry facts. Stop before wiring
production candidates until this owner is selected.

Standards-aligned options:

1. Add a focused embedded-runtime `runtime_dispatch_candidate_provider` module.
   This provider owns joining embedded-runtime fact sources into
   `WorkflowRuntimeDispatchCandidateFactBundle`, while the composition factory
   only wires the provider as part of the paired dependency bundle. This keeps
   policy out of construction and keeps embedded/Pumas/registry facts out of
   workflow-service.
2. Put candidate assembly in the runtime-registry layer and make
   embedded-runtime adapt Pumas/package facts into registry input first. This
   centralizes runtime/resource policy but risks complecting model-library
   evidence with runtime residency/admission ownership.
3. Keep the current staged sources and add only a fail-closed production
   provider that emits typed "provider not implemented" diagnostics. This is
   safe and small, but it does not progress real inference dispatch and would
   leave resource-backed candidate wiring blocked.

Recommendation: option 1. It is the cleanest composition-root split: a
dedicated embedded-runtime provider owns fact assembly; workflow-service owns
the provider trait and scheduler selection; runtime-registry owns resource
reservation/release; and the composition factory only enforces paired wiring.

Selected decision: use option 1. Add a focused embedded-runtime
`runtime_dispatch_candidate_provider` module that implements the
workflow-service `WorkflowRuntimeDispatchCandidateProvider` trait and owns
joining staged embedded-runtime facts into validated
`WorkflowRuntimeDispatchCandidateFactBundle` values. The provider is a
behavior module, not a construction module. `EmbeddedWorkflowServiceComposition`
must continue to only attach dependencies and enforce paired wiring.

Standards alignment for the selected option:

- Simplicity/complection: candidate assembly is separated from construction,
  scheduler selection policy, runtime-registry lifecycle/release behavior, and
  runtime-host worker execution. Readers can reason about fact assembly without
  reading workflow-service orchestration or composition-root code.
- Composition-root boundary: the composition factory selects and wires the
  concrete provider, runtime-host port, and lifecycle port as a complete
  dependency bundle. It does not join facts or choose candidates.
- Source ownership: embedded-runtime may consume Pumas package facts,
  runtime-registry capability facts, and runtime-registry reservation facts
  because those sources are embedded-runtime infrastructure boundaries.
  Workflow-service receives only validated scheduler-facing candidate bundles.
- Scheduler ownership: scheduler/workflow-service still owns dispatch
  selection, ranking, task state, batching policy, and typed no-candidate
  diagnostics. The provider supplies facts; it does not pick the winner.
- Runtime-registry ownership: runtime-registry remains the owner of admission,
  reservation ids, release, retention, and reclaim policy. The provider may
  request reservations and project returned leases; it must not invent
  alternate admission or release behavior.
- Async boundary: because `WorkflowRuntimeDispatchCandidateProvider` is
  synchronous, the first provider slice must not perform async Pumas lookups
  or block inside scheduler selection. If package facts are not already
  available through a staged snapshot/cache, return typed unavailable
  diagnostics and no candidates.
- No fallback/legacy: the provider must not recover candidates from graph
  paths, reduced execution plans, frontend state, display strings,
  `ModelRefV2`, dependency-preflight compatibility paths, or provider-private
  cleanup state.

Next implementation sequence:

1. Add `runtime_dispatch_candidate_provider.rs` with a fail-closed provider
   skeleton that implements `WorkflowRuntimeDispatchCandidateProvider`, returns
   typed provider diagnostics, and does not emit non-empty candidates until all
   required source facts are supplied.
   Completed 2026-05-30: `pantograph-embedded-runtime` now has an initial
   fail-closed dispatch candidate provider boundary that implements the
   workflow-service provider trait and emits typed no-candidate diagnostics
   instead of candidates while source facts are unavailable.
2. Add focused provider tests for missing Pumas/package facts, missing runtime
   capability facts, missing resource facts, path-carrying model refs, and
   no-candidate diagnostics.
   Started 2026-05-30: focused tests cover missing staged package/runtime/
   resource facts and path-carrying Pumas model refs. Later slices must expand
   coverage when source-input contracts are added.
3. Wire the fail-closed provider through
   `EmbeddedWorkflowServiceDispatchDependencies` together with the runtime-host
   execution port and reservation lifecycle port, proving the composition
   factory enforces paired production wiring.
   Started 2026-05-30: the composition bundle test now uses the real
   `EmbeddedRuntimeDispatchCandidateProvider` with paired runtime-host and
   reservation lifecycle ports, proving the provider fits the existing paired
   dependency API. Hosted production construction still remains unchanged until
   concrete runtime-host wiring is supplied.
4. Add the source-input contract for already-available/staged Pumas package
   facts so the provider can remain synchronous. If implementation discovers
   that package facts can only be fetched asynchronously at selection time,
   stop and re-plan the provider trait or snapshot lifecycle instead of
   blocking in scheduler selection.
   Completed 2026-05-30: `EmbeddedRuntimeDispatchCandidateProvider` now
   accepts an `EmbeddedRuntimeDispatchCandidateSourceSnapshot` containing
   already-collected Pumas package and runtime capability outcomes. The
   provider maps those typed source diagnostics into scheduler dispatch
   diagnostics synchronously and still emits no candidates while runtime
   resource facts are unavailable.
5. Join Pumas package facts with runtime-registry capability facts to produce
   candidate fact drafts with typed compatibility diagnostics, but no resource
   leases yet.
   Completed 2026-05-30: the provider now joins projected Pumas accepted
   backend hints with runtime-registry backend keys into internal path-free
   candidate drafts and returns typed incompatible-runtime diagnostics when no
   runtime capability matches. It still emits no scheduler candidates until
   reservation facts are supplied.
6. Add runtime-registry reservation through
   `RuntimeDispatchResourceFactsSource`, project returned leases into
   `SchedulerResourceReservation`, and emit candidates only when reservation
   succeeds.
7. Verify lifecycle pairing by proving every emitted reservation lease is
   handled by the embedded reservation lifecycle port on unselected,
   dispatch-rejected, completed, failed, cancellation, retry/defer, and
   session-close paths.
8. Only after provider and lifecycle tests pass, remove the staged
   `#[allow(dead_code)]` lifecycle-port allowance and enable non-empty
   resource-backed candidate sets in hosted production composition.

2026-05-30 fail-closed dispatch candidate provider slice verification:

- `cargo fmt -- --check`
- `cargo test -p pantograph-embedded-runtime runtime_dispatch_candidate_provider --lib`
- `cargo check -p pantograph-embedded-runtime` (passes with existing
  workflow-service `set_active_run_execution_plan` dead-code warning)

Discovered follow-up: the provider is intentionally staged with
`#[allow(dead_code)]` until it is wired through
`EmbeddedWorkflowServiceDispatchDependencies`. It still emits no non-empty
candidate sets; the next slice must connect the fail-closed provider through
the paired composition dependency bundle before any real fact-joining work.

2026-05-30 paired fail-closed provider composition slice verification:

- `cargo fmt -- --check`
- `cargo test -p pantograph-embedded-runtime workflow_service_composition --lib`
- `cargo test -p pantograph-embedded-runtime runtime_dispatch_candidate_provider --lib`
- `cargo check -p pantograph-embedded-runtime` (passes with existing
  workflow-service `set_active_run_execution_plan` dead-code warning)

Discovered follow-up: this slice proves the fail-closed provider can be wired
through the complete dependency bundle, but it intentionally uses rejecting
test ports and does not change hosted production composition. The next
production slice still needs a real runtime-host port or a selected
standards-compliant fail-closed embedded-runtime runtime-host port before
hosted construction can pass a complete bundle.

2026-05-30 dispatch provider source-snapshot slice verification:

- `cargo fmt -- --check`
- `cargo test -p pantograph-embedded-runtime runtime_dispatch_candidate_provider --lib`
- `cargo check -p pantograph-embedded-runtime` (passes with existing
  workflow-service `set_active_run_execution_plan` dead-code warning)

Implementation notes: added a synchronous
`EmbeddedRuntimeDispatchCandidateSourceSnapshot` input contract for
already-collected Pumas package-facts and runtime capability-facts outcomes.
The provider projects typed source diagnostics into scheduler dispatch
diagnostics and keeps returning an empty candidate set until runtime resource
reservation facts are supplied. This preserves the no-fallback rule: the
provider does not perform async Pumas lookups, does not block in scheduler
selection, and does not recover candidate evidence from graph paths, reduced
plans, display strings, `ModelRefV2`, or dependency-preflight compatibility
state.

Remaining follow-up: add source lifecycle wiring that produces the
source-snapshot before dispatch selection, then join projected Pumas package
facts with runtime capability facts into candidate drafts. Resource-backed
candidates remain disabled until reservation acquisition and lifecycle pairing
are implemented.

2026-05-30 dispatch provider candidate-draft slice verification:

- `cargo fmt -- --check`
- `cargo test -p pantograph-embedded-runtime runtime_dispatch_candidate_provider --lib`
- `cargo check -p pantograph-embedded-runtime` (passes with existing
  workflow-service `set_active_run_execution_plan` dead-code warning)

Implementation notes: added internal
`EmbeddedRuntimeDispatchCandidateDraft` assembly that matches Pumas accepted
backend hints to runtime-registry backend keys after normalization. The
provider reports a typed `IncompatibleRuntimeRequirement` diagnostic when
projected source facts are available but no runtime capability matches. Drafts
are not emitted as scheduler candidates yet because reservation facts and
release lifecycle coverage are still required.

Remaining follow-up: add runtime-registry reservation through
`RuntimeDispatchResourceFactsSource`, project reservation leases into scheduler
resource facts, and only then emit validated scheduler candidate bundles.

2026-05-30 re-plan trigger: resource-backed scheduler candidate emission cannot
be implemented without changing the candidate reservation contract. The staged
`RuntimeDispatchResourceFactsSource` correctly returns
`Vec<SchedulerResourceReservation>` for one runtime-registry lease because a
single runtime dispatch may reserve RAM, VRAM, or later additional resource
claims together. `WorkflowRuntimeDispatchCandidateFact`,
`SchedulerDispatchCandidate`, and `SchedulerDispatchDecision` currently carry
only one `SchedulerResourceReservation` / reservation lease id. Selecting one
reservation would silently drop resource claims; duplicating a candidate per
reservation would misrepresent one runtime execution as several scheduler
candidates. The provider also cannot invent a selected device id when the task
intent does not explicitly constrain one because current runtime capability
facts do not expose device candidate facts.

Standards-aligned options:

1. Keep the current single-reservation contract and initially emit candidates
   only for explicit-device tasks whose resource source returns exactly one
   reservation. This is the smallest code change, but it bakes in a special
   case and would either reject common RAM+VRAM reservations or tempt future
   fallback behavior.
2. Change the workflow-service and scheduler dispatch-selection contracts to
   carry `Vec<SchedulerResourceReservation>` per candidate/decision, with
   validation that all reservations belong to the same workflow run, task, and
   reservation lease group. This preserves the source-of-truth shape from the
   runtime-registry, avoids dropped claims, and lets runtime-host lifecycle
   release one lease while scheduler diagnostics can still show every resource
   claim.
3. Introduce a composite scheduler reservation summary containing one lease id
   plus aggregate resource-claim details, while keeping detailed reservations
   internal to runtime-registry. This narrows scheduler surface area but adds a
   second resource representation that can drift from runtime-registry facts.
4. Defer resource-backed candidate emission until a larger runtime/device
   capability source is designed that supplies selected device candidates and
   multi-resource reservation facts together. This avoids a partial contract,
   but blocks complete inference-run dispatch longer.

Recommendation: option 2. It is the cleanest standards-compliant contract
change because it keeps the runtime-registry reservation fact shape intact,
avoids special-case fallbacks, and separates concerns: runtime-registry owns
claims and leases, provider projects all claim reservations, scheduler selects
one candidate, and runtime-host lifecycle releases the selected lease.

Selected decision: use option 2. Before the provider emits resource-backed
candidates, update the workflow-service and scheduler dispatch contracts so a
candidate and selected decision carry all resource reservations for the chosen
runtime-registry lease. The contract must remain explicit and validated:

- `WorkflowRuntimeDispatchCandidateFact` must carry
  `Vec<SchedulerResourceReservation>` instead of one reservation.
- `SchedulerDispatchCandidate` must carry the same reservation vector. Empty
  reservation vectors are invalid for eligible candidates and must produce
  typed `MissingReservation` diagnostics.
- `SchedulerDispatchDecision` must preserve the selected lease id and carry the
  selected reservation vector so diagnostics and task state can explain every
  claim. All reservations in the vector must belong to the same workflow run,
  task, and reservation lease id.
- Validation must reject mixed workflow runs, mixed tasks, mixed lease ids,
  duplicate resource claims for the same device/resource kind, empty vectors,
  and path-carrying model refs.
- Runtime-host lifecycle still releases by the selected lease id. It must not
  release per resource claim or invent additional release semantics.
- The provider must not emit candidates for unconstrained tasks until runtime
  capability facts expose selected device candidates. It may support explicit
  device tasks first only if that path uses the same reservation-vector
  contract and returns typed diagnostics for missing device facts.

Standards alignment for the selected option:

- Simplicity/complection: one candidate represents one runtime execution and
  one runtime-registry lease, while the reservation vector represents the
  concrete resource claims for that execution. This avoids splitting a single
  runtime decision across fake per-resource candidates.
- Source of truth: runtime-registry remains the owner of resource claims,
  reservation ids, retention, and release. Scheduler and workflow-service only
  carry validated facts needed for selection, diagnostics, and task state.
- Contract-first boundary: update scheduler/workflow-service contracts and
  tests before changing embedded-runtime provider emission.
- No fallback/legacy: do not collapse multiple reservations to the first row,
  synthesize aggregate strings, duplicate candidates per claim, or infer device
  ids from graph/runtime names.

Next implementation sequence:

1. Completed 2026-05-30: add scheduler contract tests for
   multi-reservation candidates and selected decisions, including same
   lease/task/run validation and rejection of mixed lease ids or duplicate
   device/resource claims.
2. Completed 2026-05-30: update `SchedulerDispatchCandidate`,
   `SchedulerDispatchDecision`, selection policy, and dispatch-selection
   validation to use reservation vectors while preserving typed
   `MissingReservation` diagnostics for empty vectors.
3. Completed 2026-05-30: update workflow-service
   `WorkflowRuntimeDispatchCandidateFact` and `dispatch_candidate` projection
   to carry reservation vectors one-to-one into scheduler candidates.
4. Completed 2026-05-30: update workflow-service tests and scheduler/runtime
   host fixtures that build candidate facts, scheduler candidates, or selected
   dispatch decisions.
5. Completed 2026-05-30 for explicit-device tasks: update
   `EmbeddedRuntimeDispatchCandidateProvider` to call
   `RuntimeDispatchResourceFactsSource` for matched drafts, pass through every
   returned reservation, and emit validated scheduler candidate bundles.
   The default fail-closed provider path remains in place when no resource
   facts source is injected.
6. Re-plan trigger recorded 2026-05-30: wiring resource-backed provider
   construction into production composition requires deciding who owns the live
   Pumas package facts and runtime capability facts at scheduler-dispatch time.
   `WorkflowRuntimeDispatchCandidateProvider` is synchronous, but Pumas package
   fact collection currently depends on async selector access. Do not wire a
   test-only static source snapshot into production composition.
7. Selected 2026-05-30 immediate path: use option 1 as a short-turnaround
   bridge. Add a versioned dispatch source-fact snapshot owned by
   embedded-runtime composition. Async Pumas/runtime source owners refresh the
   snapshot before dispatch, while the synchronous workflow-service provider
   reads only validated fresh, path-free facts.
   Completed 2026-05-30: `pantograph-embedded-runtime` now has
   `EmbeddedRuntimeDispatchSourceFactSnapshotStore`, which refreshes Pumas
   package facts and runtime capability facts through their source owners,
   versions the snapshot, validates freshness/model-ref/path-free constraints,
   and returns typed lifecycle diagnostics instead of exposing stale or
   mismatched source facts to the synchronous dispatch provider.
8. Bridge guardrail: the option 1 snapshot must use the same validated
   dispatch source-fact shape that the later option 3
   readiness/admission-attached snapshot will persist. Missing, stale,
   version-mismatched, or incomplete snapshots must return typed diagnostics
   and no candidates; do not introduce a provider-private cache contract or
   static production snapshot.
9. Next implementation: wire resource-backed provider construction into the
   embedded runtime composition boundary that owns the runtime registry,
   runtime-host port, reservation lifecycle port, validated dispatch
   source-fact snapshot, and runtime-registry resource source.
   Completed 2026-05-30: `EmbeddedWorkflowServiceDispatchDependencies` now has
   a resource-backed constructor that builds the snapshot store, dispatch
   candidate provider, runtime-registry resource source, runtime-host port, and
   reservation lifecycle port as one paired dependency bundle. Hosted
   production wiring remains blocked until the canonical embedded runtime-host
   execution port and snapshot refresh lifecycle are available together.
10. Follow-on: add the canonical embedded runtime-host execution port needed
   for a complete inference path.
   Completed 2026-05-30 as a fail-closed boundary:
   `EmbeddedRuntimeHostExecutionPort` validates dispatch-selected
   runtime-host requests, requires host-owned Pumas load-target resolution,
   returns typed rejected responses for missing/unavailable load targets, and
   does not call legacy node-engine, whole-run, `ModelRefV2`, graph-path, or
   reduced-plan execution paths.
11. Follow-on: add a workflow-service pre-dispatch source-refresh port and
   embedded-runtime implementation so async source owners can refresh the
   versioned snapshot before the synchronous candidate provider runs.
   Completed 2026-05-30: `WorkflowRuntimeDispatchSourceRefresher` is called
   after dependency-readiness admission and before candidate collection, while
   `EmbeddedRuntimeDispatchSourceFactRefresher` refreshes the shared snapshot
   store that the resource-backed provider reads. Workflow-service owns only
   the orchestration point; Pumas/runtime-registry source ownership remains in
   embedded-runtime.
12. After the first complete inference path works end-to-end, implement option
   3 by promoting the validated dispatch source-fact snapshot into
   readiness/admission task state with persistence, freshness, drift, and
   invalidation diagnostics before relying on restart/replay,
   duplicate-dispatch prevention, cancellation recovery, durable multi-run
   scheduling, or production-grade recovery semantics.
13. Follow-on: add a device-source slice so unconstrained tasks receive
   selected device candidates from runtime capability facts instead of provider
   guesses.

2026-05-30 re-plan trigger: hosted embedded-runtime startup cannot yet install
the resource-backed dispatch dependency bundle without changing ownership
boundaries. `EmbeddedRuntime::hosted_with_default_python_runtime` receives an
already shared `Arc<WorkflowService>`, but
`EmbeddedWorkflowServiceDispatchDependencies::resource_backed` must be applied
before the service is shared. The required Pumas owner selector access is also
resolved from runtime extensions after the current service handoff, while the
runtime registry is attached later through `with_runtime_registry`. Mutating an
already shared service to install dispatch dependencies would complect runtime
composition with service internals and violate the composition-root standard.

Standards-aligned options:

1. Add an embedded-runtime workflow-service factory/composition input for
   hosted startup. The host provides config, gateway, runtime registry, Pumas
   selector access, and optional preconfigured stores before `WorkflowService`
   is wrapped in `Arc`; embedded-runtime installs dependency-readiness,
   resource-backed dispatch dependencies, runtime-host port, reservation
   lifecycle port, and diagnostics provider in one composition root. This is
   the recommended path because it keeps business logic out of Tauri/frontends
   and avoids post-share mutation.
2. Add mutating setters to `WorkflowService` for dispatch dependencies after
   sharing. Do not use this path unless a later design proves the setters can
   be single-use, race-free, and initialized before any session run; otherwise
   it creates lifecycle complection and weakens reasoning about active runs.
3. Limit resource-backed dispatch wiring to standalone runtime for now. This
   is standards-compliant only as a scoped standalone slice, but it does not
   unblock hosted complete inference-run testing and must not be presented as
   production hosted wiring.
4. Defer hosted wiring and keep fail-closed dispatch diagnostics. This
   preserves correctness but stops progress toward complete inference runs.

Selected decision: option 1. Add a hosted workflow-service
composition/factory boundary so all runtime dispatch dependencies are
installed before sharing the service. The slice must not move Pumas facts,
runtime registry policy, runtime-host execution, or dependency-readiness
business logic into Tauri or workflow-service.

Next implementation sequence:

1. Add an embedded-runtime factory/input DTO for hosted workflow-service
   construction. Inputs must be explicit: config capacity, gateway,
   extensions/Pumas selector access source, runtime registry, runtime-host
   load-target resolver inputs, dependency-readiness composition, and optional
   preconfigured stores. Do not accept an already shared `Arc<WorkflowService>`
   on the new resource-backed path.
   Completed 2026-05-31 for the composition boundary:
   `EmbeddedHostedWorkflowServiceFactoryInput` now carries explicit gateway,
   runtime registry, runtime-registry lifecycle controller, owner Pumas
   selector access, capacity, and dispatch snapshot freshness inputs. The
   resource-backed path rejects read-only and local-client Pumas access with a
   typed workflow-service invalid-request diagnostic instead of installing a
   partial dispatch stack.
2. Build the workflow service through `EmbeddedWorkflowServiceComposition`
   before sharing. The factory must attach dependency-readiness components,
   resource-backed dispatch dependencies, the embedded runtime-host execution
   port, reservation lifecycle port, and scheduler diagnostics provider in one
   composition root.
   Completed 2026-05-31 for the factory slice:
   `EmbeddedWorkflowServiceComposition::resource_backed_hosted` builds the
   Pumas load-target resolver, embedded runtime-host execution port,
   reservation lifecycle port, resource-backed dispatch dependency bundle,
   and scheduler diagnostics provider before returning the shared workflow
   service. `WorkflowService::with_scheduler_diagnostics_provider` was added
   so diagnostics can be installed during construction instead of through
   post-share mutation.
3. Update hosted startup to use the factory only when it has the required
   runtime registry and Pumas selector access. Missing required construction
   inputs must produce typed initialization diagnostics or keep the existing
   fail-closed service path; do not silently install partial dispatch
   dependencies.
   Remaining: hosted startup still receives or constructs the workflow
   service outside this factory. The next slice must move hosted startup onto
   this factory or introduce a final embedded-runtime lifecycle bundle that
   returns the service plus any producer handles needed by the hosted runtime.
   Until then, `workflow_service_composition` carries a temporary staged
   production `#[allow(dead_code)]` allowance.
4. Add focused tests proving the hosted factory builds a shared workflow
   service with the paired dispatch refresher/provider/runtime-host/lifecycle
   dependencies before sessions can run, and proving Tauri/frontends do not own
   runtime dispatch business logic.
   Completed 2026-05-31 for factory behavior: embedded-runtime tests prove the
   resource-backed hosted factory builds a shared service with owner Pumas
   access and rejects non-owner Pumas selector access.
5. After this factory slice, continue to the first complete inference path by
   wiring runtime-specific execution behind `EmbeddedRuntimeHostExecutionPort`.

2026-05-31 re-plan trigger: the hosted workflow-service factory exists, but
hosted startup still constructs and manages `Arc<WorkflowService>` before
Pumas owner selector access exists, then passes that already shared service
into `EmbeddedRuntime::hosted_with_default_python_runtime` and attaches
runtime-registry state later. The next slice therefore cannot be a direct
call-site swap. It must change hosted composition ownership so the
resource-backed workflow service is built before sharing and before commands
can run against partial dispatch state.

Standards-aligned hosted startup options:

1. Keep Tauri as the late composition root and reorder startup so Pumas
   extensions initialize before `WorkflowService` is shared. Tauri would then
   call the embedded-runtime factory and manage the returned service. This is
   short, but risky: `src-tauri` would coordinate Pumas access, workflow
   service stores, runtime registry, dependency readiness, and runtime
   dispatch lifecycle in one place. It is standards-compliant only if Tauri is
   kept to infrastructure wiring and no runtime-dispatch policy or Pumas fact
   interpretation moves there.
2. Introduce the backend-owned hosted composition bundle as the target design.
   Embedded-runtime receives explicit infrastructure inputs from the host,
   creates/configures the workflow service, installs dependency readiness,
   resource-backed dispatch dependencies, runtime-host execution,
   reservation lifecycle, scheduler diagnostics, and lifecycle sidecars, then
   returns `SharedWorkflowService` plus owned handles for Tauri to manage.
   This best matches composition-root, single-owner lifecycle, and
   simplicity/complection standards.
3. Stage option 2 in two validated slices. First add the hosted composition
   bundle contract and lifecycle ownership in embedded-runtime without
   migrating every Tauri caller. Then migrate Tauri startup/headless runtime
   construction onto that bundle and enable successful resource-backed hosted
   dispatch only once all required inputs are available before sharing. This
   keeps slices small while preserving option 2 as the target design.
4. Defer hosted wiring and continue lower-level runtime-host execution work
   while hosted dispatch stays fail-closed. This is correct but does not
   unblock hosted complete inference-run testing.

Selected decision: option 3, with option 2 as the target architecture. Do not
implement option 1's Tauri-owned late builder unless the embedded-runtime
bundle proves impossible and a new re-plan explicitly accepts that ownership.
Do not use post-share dispatch setters as an implementation shortcut.

Next staged implementation sequence:

1. Add an embedded-runtime hosted composition input/output bundle. Inputs must
   be explicit infrastructure/configuration values such as app paths, gateway,
   runtime registry, owner Pumas selector access or setup source, workflow
   service store/configuration inputs, capacity, and app-shell event sinks.
   The output must return the `SharedWorkflowService` plus lifecycle handles
   owned by the hosted runtime composition, including dependency-readiness
   producer handles when they are started.
   Completed 2026-05-31 for the embedded-runtime bundle contract:
   `EmbeddedHostedWorkflowServiceCompositionInput` and
   `EmbeddedHostedWorkflowServiceCompositionOutput` now model the pre-share
   resource-backed hosted composition boundary. The output returns the shared
   service and dependency-readiness producer handle, and tests prove successful
   bundle creation plus typed rejection for invalid producer configuration.
2. Split service construction from lifecycle sidecars without exposing partial
   successful dispatch. Dispatch dependencies and scheduler diagnostics must
   be installed before sharing. Any sidecar that necessarily needs the shared
   service, such as projection refresh workers, must be started by the same
   hosted composition owner before commands are exposed and must not mutate
   runtime dispatch dependencies.
   Partially completed 2026-05-31: dispatch dependencies and scheduler
   diagnostics are installed before sharing, and the dependency-readiness
   producer starts only after service construction succeeds so failed service
   construction does not leak a background lifecycle handle. App-shell
   sidecars such as projection refresh workers remain for the Tauri migration
   slice.
3. Migrate `src-tauri/src/app_setup.rs` and
   `src-tauri/src/workflow/headless_runtime.rs` so Tauri supplies
   infrastructure inputs and manages returned handles, but does not own Pumas
   fact resolution, runtime-registry policy, dependency-readiness production,
   or runtime-host dispatch decisions.
   Re-plan boundary reached 2026-05-31 before source edits: current Tauri
   startup shares and manages `WorkflowService` before `setup`, creates the
   gateway inside `setup`, and initializes Pumas selector access in the
   asynchronous `executor-extension-init` task. The hosted bundle needs owner
   Pumas selector access, gateway/controller, runtime registry, configured
   unshared workflow-service stores, and a runtime handle before sharing.
   Migrating this safely requires a concrete startup ownership transition:
   either synchronous infrastructure initialization before workflow-service
   state is managed, or a higher-level embedded-runtime host setup source that
   acquires Pumas selector access and returns lifecycle handles. The plan must
   also define how Tauri stores and shuts down the returned
   dependency-readiness producer handle without making Tauri the business owner
   of dependency-readiness production.
   Selected 2026-05-31: use staged backend-owned hosted startup composition.
   Do not use the narrow Tauri reorder as the implementation path. Add a
   small embedded-runtime startup composition boundary first; it must receive
   host infrastructure/configuration, initialize or validate owner Pumas
   selector access before workflow-service sharing, build executor extensions
   and dependency resolver wiring through backend-owned helpers, invoke the
   hosted composition bundle, and return `SharedWorkflowService`,
   `SharedExtensions`, the model dependency resolver, and lifecycle handles.
   The follow-up Tauri slice may manage those returned values and attach
   app-shell event sinks, but it must not own Pumas fact interpretation,
   dependency-readiness production policy, runtime-registry policy, or runtime
   dispatch decisions.
   Completed 2026-05-31 for the backend startup boundary: embedded-runtime now
   exports `EmbeddedHostedStartupCompositionInput`,
   `EmbeddedHostedStartupCompositionOutput`,
   `EmbeddedHostedStartupPumasSelectorSource`, and
   `EmbeddedWorkflowServiceComposition::resource_backed_hosted_startup`. The
   boundary validates owner Pumas selector access before workflow-service
   sharing, installs shared extensions, KV-cache, and model dependency
   resolver wiring, invokes the hosted composition bundle, and returns
   lifecycle handles for the host to manage. Tauri migration remains pending.
   Completed 2026-05-31 for the Tauri startup migration: `app_setup.rs` now
   calls the hosted startup composition before managing workflow state and
   manages the returned workflow service, extensions, dependency resolver, and
   dependency-readiness producer handle. Tauri now supplies infrastructure
   inputs only and no longer owns the asynchronous Pumas/executor-extension
   initialization task. `app_lifecycle.rs` shuts down the returned
   dependency-readiness producer handle during window close. Headless runtime
   construction still needs the follow-up migration away from
   `hosted_with_default_python_runtime`.
4. Narrow or replace `EmbeddedRuntime::hosted_with_default_python_runtime`.
   It must not remain the successful resource-backed hosted path while it
   accepts an already shared `WorkflowService`. During migration it may remain
   only as a non-resource-backed/fail-closed helper for existing tests, with
   clear follow-up to delete or rename it after callers move to the bundle.
   Completed 2026-05-31 for Tauri headless runtime callers:
   embedded-runtime now exposes `EmbeddedRuntime::from_hosted_composition` for
   services that were already composed before sharing, and
   `src-tauri/src/workflow/headless_runtime.rs` uses it instead of
   `hosted_with_default_python_runtime`. The new constructor preserves
   preconfigured workflow-service capacity and does not mutate scheduler
   diagnostics or runtime-dispatch dependencies after sharing. Remaining work:
   narrow, rename, or delete `hosted_with_default_python_runtime` for the
   embedded-runtime test/non-resource-backed path so it cannot be confused with
   the canonical resource-backed hosted composition entry point.
   Completed 2026-05-31 for helper narrowing: the legacy helper is now the
   `#[cfg(test)] pub(crate)`
   `EmbeddedRuntime::test_hosted_with_default_python_runtime` constructor, and
   all remaining call sites are embedded-runtime tests. Production hosted
   runtime construction now flows through the startup/workflow-service
   composition boundary plus `EmbeddedRuntime::from_hosted_composition`.
5. Add focused tests proving hosted composition cannot expose successful
   resource-backed dispatch without owner Pumas access and runtime registry,
   that Tauri/headless runtime construction does not install dispatch
   dependencies after sharing, and that missing hosted inputs produce typed
   initialization diagnostics or fail-closed service behavior.
6. After hosted composition uses the bundle, remove the temporary staged
   `#[allow(dead_code)]` on `workflow_service_composition` and continue to the
   first complete inference path behind `EmbeddedRuntimeHostExecutionPort`.

2026-05-31 first complete inference path re-plan decision: use the
image-generation-first runtime-host executor path. The next implementation
slice must keep `EmbeddedRuntimeHostExecutionPort` as the thin
validation/correlation/Pumas load-target boundary, then delegate successful
image-generation execution to a focused embedded-runtime module. That module
owns only the projection from a validated `RuntimeHostExecutionRequest`,
workflow-service-owned materialized task inputs, and the resolved Pumas load
target into the canonical `inference::InferenceGateway` image execution API,
plus the projection from inference results into
`RuntimeHostExecutionResponse` media-artifact outputs and typed diagnostics.
If the input/output projection is still ambiguous at implementation time, add
a projection-only micro-slice first and keep the port fail-closed until the
projection is verified.

Options disposition for this re-plan:

1. Selected now: add the image-generation-first embedded-runtime executor as
   the smallest useful vertical slice. Unsupported task kinds must return
   typed unsupported/runtime-unavailable diagnostics and must not call legacy
   execution paths.
2. Deferred: add a generic runtime-host execution router with handlers keyed
   by task kind or modality. This remains valid when multiple concrete
   modality handlers exist, but it is not the first slice because it adds
   abstraction before a second handler proves the boundary.
3. Allowed only as a replacement refactor: reuse code from
   `PlannedInferenceExecutionHost` or `planned_inference_host` only by moving
   the useful behavior into the canonical runtime-host executor owner. Do not
   preserve the planned-inference contract as an alternate successful launch
   branch or compatibility shim.
4. Allowed only as a preparatory guardrail: add a typed projection-only slice
   before execution if the canonical gateway input/output mapping is unclear.
   The projection must remain path-free, backend-owned, and tested, and the
   runtime-host port must continue returning typed unavailable diagnostics
   until the execution call is wired.

Completed 2026-05-31 for the option 4 guardrail: embedded-runtime now has
`runtime_host_image_execution.rs`, a backend-only projection from validated
runtime-host image requests, scheduler dispatch decisions, full Pumas package
facts, and Pumas load targets into canonical image-generation planning input.
It rejects unsupported task kinds, unsupported materialized input ports,
unsupported runtime ids, invalid device ids, missing prompt, and invalid launch
handoff facts with typed errors. `EmbeddedRuntimeHostExecutionPort` still
fails closed after Pumas load-target resolution, so this slice does not create
a successful execution branch or preserve planned-inference behavior.
Remaining before enabling the gateway call: add or select the runtime-host
source for full Pumas package facts, avoid long-term backend-id inference from
runtime ids by carrying explicit selected backend facts through scheduler
dispatch, extend the runtime-host input contract for typed float image options
when those ports are exposed, and implement path-free media artifact result
projection.

Completed 2026-05-31 for the full Pumas package-facts source guardrail:
embedded-runtime now has `runtime_host_package_facts.rs`, which resolves full
Pumas package facts only from the scheduler-selected model ref in a validated
runtime-host request. It decodes Pumas facts into the inference contract,
strips Pumas-only model-ref contract-version fields, and fails closed for
missing dispatch decisions, Pumas lookup errors, decode failures, stale
package-facts contracts, and selected-artifact mismatches. This removes the
package-facts source blocker for the next runtime-host composition slice, but
does not wire a successful gateway call. Remaining before execution: compose
package facts plus load target plus image projection in the runtime-host port,
add path-free media artifact output projection, and plan durable selected
backend and float input contracts.

2026-05-31 media artifact output re-plan decision: use a narrow backend-owned
runtime-host media artifact sink before successful image execution is wired.
The sink is the selected option 2 from the re-plan discussion. It should expose
only the operation the runtime-host executor needs: accept generated image
output plus task/run/node/port attribution, persist through the backend-owned
artifact store or workflow-service artifact boundary, and return a
`RuntimeHostExecutionMediaArtifactRef`. `EmbeddedRuntimeHostExecutionPort`
must receive the sink as a dependency and remain responsible for response
shaping, but it must not own workflow-service persistence internals, invent
artifact ids, return inline base64 media as scheduler task results, or call
Tauri/frontend code. Missing sink and artifact write failures must fail closed
with typed runtime-host diagnostics.

Options disposition for the media-output boundary:

1. Rejected for now: inject full `WorkflowService` into the runtime-host port.
   This is the fastest path but complects request validation, Pumas resolution,
   inference execution, artifact persistence, and response shaping.
2. Selected: add `RuntimeHostMediaArtifactSink` as a narrow dependency. This
   follows the standards' simplicity/complection guidance because persistence
   can change independently from runtime execution and the port can be tested
   without exposing workflow-service internals.
3. Rejected for now: extend runtime-host output contracts to carry inline
   encoded media. This weakens path-free artifact-ref ownership, risks large
   scheduler task-result payloads, and moves retention decisions into the wrong
   boundary.
4. Temporary guardrail only: keep execution fail-closed until the sink exists.
   This remains acceptable between slices but is not the next implementation
   path.

Standards alignment: this decision follows the simplicity/complection rule by
separating validation and runtime side effects, transport mapping and domain
execution, lifecycle ownership and request handling, and diagnostics policy and
recovery behavior. Tauri remains an infrastructure/app-shell composition
caller only; scheduler keeps dispatch policy only; graph editor and node-engine
do not own Pumas load targets or runtime execution business logic.

Completed 2026-05-31 for the media artifact sink slice:
`pantograph-embedded-runtime` now has
`runtime_host_media_artifact_sink.rs`, defining the
`RuntimeHostMediaArtifactSink` contract plus a workflow-service-backed image
implementation. The sink decodes generated `inference::EncodedImage` payloads,
writes retained image bytes through the backend artifact store, preserves
workflow/run/node/port/model/runtime attribution, uses deterministic artifact
ids, and returns path-free `RuntimeHostExecutionMediaArtifactRef` values. It
fails closed with typed sink errors for malformed base64 and unavailable
artifact persistence. This does not wire successful gateway execution yet and
does not introduce inline media outputs, fake refs, Tauri/frontend logic,
scheduler persistence workarounds, planned-inference branches, graph paths, or
`ModelRefV2` adapters. Verification passed: `cargo fmt --package
pantograph-embedded-runtime`; `cargo test -p pantograph-embedded-runtime
runtime_host_media_artifact_sink -- --nocapture`. Verification caveat: the
focused test command still reports the known workflow-service
`set_active_run_execution_plan` warning. Discovered follow-up: runtime-host
and scheduler media refs currently validate `media_type` as an identifier, so
the sink returns identifier-safe values such as `image_png` while artifact
descriptors retain the real MIME type; a later shared contract cleanup should
rename that field to a media type id or allow MIME values without weakening
artifact id validation. Remaining before execution: inject the sink into
`EmbeddedRuntimeHostExecutionPort`, map missing sink/write failures into typed
runtime-host diagnostics, call the image gateway, and project completed image
results into path-free media artifact outputs.

Completed 2026-05-31 for the execution port dependency-seam slice:
`EmbeddedRuntimeHostExecutionPort` now depends on narrow load-target and
media-artifact sink boundaries. The existing Pumas load-target resolver
implements the new `RuntimeHostLoadTargetResolver` trait, and the port stores
trait-object dependencies so tests and future composition can exercise the
runtime-host boundary without exposing workflow-service persistence internals
or Pumas implementation details. After load-target resolution, missing media
sink configuration now fails closed with a typed rejected runtime-host
response; when both dependencies exist, the port still stops at the existing
runtime-unavailable guardrail because gateway execution remains unwired.
Verification passed: `cargo fmt --package pantograph-embedded-runtime`; `cargo
test -p pantograph-embedded-runtime runtime_host_execution_port --
--nocapture`. Verification caveat: the focused test command still reports the
known workflow-service `set_active_run_execution_plan` warning. Remaining
before execution: compose package facts, image projection, gateway execution,
sink-backed output writing, and typed gateway/write diagnostics inside the
port.

Completed 2026-05-31 for the image execution composition slice:
`EmbeddedRuntimeHostExecutionPort` now composes the first complete
image-generation execution path when all canonical dependencies are injected.
It resolves package facts through the new `RuntimeHostPackageFactsResolver`
trait, projects the validated runtime-host request through the existing image
planning boundary, calls
`InferenceGateway::generate_image_from_planning_input`, writes generated images
through `RuntimeHostMediaArtifactSink`, and returns completed
`RuntimeHostExecutionOutputValue::MediaArtifactRef` outputs. Gateway failures
and media artifact write failures become typed failed runtime-host responses.
The slice also corrected `runtime_host_load_target` tests to assert the
canonical runtime-host request fixture stays path-free by leaving
`selected_artifact_path` and `caller_observed_entry_path` absent. Verification
passed: `cargo fmt --package pantograph-embedded-runtime`; `cargo test -p
pantograph-embedded-runtime runtime_host_execution_port -- --nocapture`;
`cargo test -p pantograph-embedded-runtime runtime_host_package_facts --
--nocapture`; `cargo test -p pantograph-embedded-runtime
runtime_host_load_target -- --nocapture`; `cargo test -p
pantograph-embedded-runtime runtime_host_media_artifact_sink -- --nocapture`.
Verification caveat: the focused commands still report the known
workflow-service `set_active_run_execution_plan` warning. Remaining production
follow-up is now governed by the production artifact-writer composition
re-plan below.

2026-05-31 production artifact-writer composition re-plan decision: use option
2, a shared backend-owned artifact writer handle, before enabling hosted
production image execution. The runtime-host image path already works in
focused tests when its dependencies are injected, but hosted composition
cannot safely inject the current workflow-service-backed media sink by passing
`Arc<WorkflowService>` back into the runtime-host port. That would make
`WorkflowService` depend on a port that depends on the same `WorkflowService`,
or force persistence policy into Tauri/embedded-runtime composition code.

Selected design:

1. Introduce or expose a narrow artifact writer contract at the backend
   workflow-service artifact boundary. The handle should cover only the
   artifact write/read descriptor operations needed by workflow-service
   artifact APIs and runtime-host generated-media persistence.
2. Construct the artifact writer in the embedded-runtime/backend composition
   root before `WorkflowService` is wrapped in `Arc`.
3. Inject the same writer into `WorkflowService` artifact operations and into
   the runtime-host media artifact sink, refactoring or replacing the current
   full-service-backed sink as needed.
4. Construct `EmbeddedRuntimeHostExecutionPort` with the Pumas package-facts
   resolver, Pumas load-target resolver, inference gateway, and the
   artifact-writer-backed media sink.
5. Keep scheduler dispatch and task-result recording as the only production
   successful caller of runtime-host execution.

Options disposition:

1. Deferred emergency bridge: install a late-bound delegating runtime-host port
   and initialize it after `Arc<WorkflowService>` exists. This is faster but
   adds mutable lifecycle state and conflicts with the existing no mutable
   post-share wiring guardrail unless separately replanned.
2. Selected: shared backend artifact writer handle. This keeps artifact
   persistence business logic backend-owned, avoids a self-reference, keeps
   Tauri as app shell, and follows the standards' simplicity/complection
   guidance by separating artifact persistence from runtime execution and
   composition mechanics.
3. Rejected for this slice: workflow-service-owned runtime port factory. It
   can avoid the self-reference but risks moving runtime-host infrastructure
   assembly into workflow-service.
4. Temporary guardrail only: keep production image execution fail-closed until
   the shared writer exists. This remains valid between slices but is not a
   replacement for the production composition work.

Verification for the next slice must prove the same backend writer is used by
workflow-service artifact APIs and runtime-host media output, no
`WorkflowService` self-reference or Tauri persistence policy is introduced,
partial composition fails closed with typed diagnostics, and completed
runtime-host image responses are recorded as scheduler task results.

Completed 2026-05-31 for the shared backend artifact writer slice:
`pantograph-workflow-service` now exposes `WorkflowArtifactWriter`, a cloneable
backend artifact-store handle created before `WorkflowService` is shared.
`WorkflowService` artifact APIs use the handle internally and still own their
diagnostics wrapping, while `WorkflowServiceRuntimeHostMediaArtifactSink`
depends on the writer instead of `Arc<WorkflowService>`. Focused runtime-host
tests prove the service and sink can share one writer and that completed image
execution still returns path-free media artifact refs. This removes the
service self-reference blocker for the next hosted production composition
slice, but does not yet enable hosted production image execution. Remaining
follow-up: construct/inject the shared writer in hosted embedded-runtime
composition, build the runtime-host port with package-facts resolver,
load-target resolver, inference gateway, and writer-backed sink, and add
session-level task-result coverage.

Completed 2026-05-31 for hosted production runtime-host composition:
`EmbeddedWorkflowServiceComposition::resource_backed_hosted` and
`resource_backed_hosted_bundle` now require a configured
`WorkflowArtifactWriter` before `WorkflowService` is wrapped in `Arc`, then
build `EmbeddedRuntimeHostExecutionPort` with the Pumas load-target resolver,
Pumas package-facts resolver, inference gateway, and writer-backed
`WorkflowServiceRuntimeHostMediaArtifactSink`. This keeps artifact persistence
business logic backend-owned, avoids a `WorkflowService` self-reference, and
keeps Tauri as an app-shell caller. Focused composition tests prove successful
hosted construction requires explicit artifact-store setup and missing writer
wiring fails closed with typed diagnostics. Remaining follow-up: add
session-level coverage that scheduler task execution records completed
runtime-host image responses through the hosted composition path, then remove
the remaining planned-inference/node-engine launch branches.

## Verification Strategy

- Contract fixtures for host execution request/response and Pumas load-target
  diagnostics.
- Boundary tests proving graph, node-engine, scheduler hints, and saved
  workflow payloads reject executable path fields.
- Runtime-host tests proving Pumas load targets are resolved only inside the
  host boundary and unavailable states fail with typed diagnostics.
- Backend artifact-writer composition tests proving workflow-service artifact
  operations and runtime-host media output use the same backend-owned writer
  handle without a `WorkflowService` self-reference, without Tauri business
  logic, and with typed diagnostics for missing or partial writer wiring.
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
