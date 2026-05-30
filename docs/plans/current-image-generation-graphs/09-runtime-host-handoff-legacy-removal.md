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
8. Add runtime-feature and device-toolchain providers one at a time from their
   source-owned facts. Each provider slice must add focused observation
   fixtures, result-projection tests, README ownership updates, and no-fallback
   tests.
9. Plan system-package inventory separately before implementation because it
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
- Keep provider source ids as typed contract fields. Do not parse
  `DependencyRequirementName`, runtime display names, backend aliases, package
  names, or graph-authored strings to recover managed-runtime, feature, or
  toolchain identity.
- Keep result projection centralized. Concrete providers may return typed
  observation rows, provider diagnostics, and freshness/correlation facts, but
  dependency-environment readiness/install/operation/result-state mapping must
  live in the shared projector so mixed-provider payloads are easy to reason
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
