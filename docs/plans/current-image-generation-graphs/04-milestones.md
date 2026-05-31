# Milestones

Use these milestone files as the implementation checklist. Each milestone owns
one validated vertical slice or one release/guardrail gate. Update the
milestone status in its file and summarize progress in
[Execution Management](05-execution-management.md).

## Milestone Index

0. [Contract Gate](milestones/00-contract-gate.md)
   - Freeze cross-layer contracts before implementation.
   - Define validated DTOs, boundary fixtures, execution normalization, and
     decomposition decisions.

1. [Current Juggernaut Graph Slice](milestones/01-current-juggernaut-graph-slice.md)
   - Keep one current Juggernaut workflow.
   - Use canonical `puma-lib -> llm-inference -> image-output`.

2. [Retired Node Producers Removed](milestones/02-retired-node-producers-removed.md)
   - Stop producers, probes, templates, and current app paths from emitting or
     rewriting retired `diffusion-inference` graphs.

3. [Backend Stale Graph Diagnostics](milestones/03-backend-stale-graph-diagnostics.md)
   - Add backend-owned stale graph diagnostics and bounded submit/admission
     reasons.

4. [IO Inspector Stale Graph Presentation](milestones/04-io-inspector-stale-graph-presentation.md)
   - Render stale saved graphs and selected stale node details in the IO
     inspector without frontend-inferred diagnostics.

5. [Device And Runtime Variant Selection](milestones/05-device-and-runtime-variant-selection.md)
   - Define canonical device policy, backend adapter candidate facts, runtime
     variant readiness, and deterministic scheduler-selected device resolution
     for CPU/CUDA on Linux/Windows and Metal/MPS on macOS.

5a. [Scheduler-Owned Dynamic Task Dispatch](milestones/05a-scheduler-owned-dynamic-task-dispatch.md)
   - Replace whole-workflow static execution assumptions with durable
     task-level scheduler queueing, capability hints, batching, resource
     admission, dependency readiness policy, and dispatch decisions.

5c. [Task-Level Scheduler Orchestration](milestones/05c-task-level-scheduler-orchestration.md)
   - Integrate option 4 durable per-task workflow orchestration so scheduler
     task state, not node-engine output demand or reduced execution plans,
     advances workflow progress.
   - Before continuing runtime dispatch, classify newly exposed retired
     whole-run/session/runtime-load/artifactization surfaces as delete,
     canonical scheduler/runtime-host reattach, or scheduler task-result
     conversion work. Do not preserve them as compatibility branches.

5d. [Inference Interface Resolution And Validation](milestones/05d-inference-interface-resolution-and-validation.md)
   - Add the backend-owned resolver/validator that turns connected model
     references into typed generic inference-node ports, defaults,
     availability, options, and diagnostics.
   - Graph editor draft validation, executable publish/admission validation,
     scheduler task materialization, and pre-dispatch validation must consume
     the same resolved interface descriptor so task execution cannot drift from
     the ports shown to users. Draft save remains editable graph persistence
     and does not create scheduler authority.

5b. [Runtime Host Handoff And Legacy Execution Removal](milestones/05b-runtime-host-handoff-legacy-removal.md)
   - Replace successful `model_path`/`ModelRefV2` runtime execution with
     direct scheduler-to-runtime-host execution that consumes scheduler
     handoff and resolves Pumas-approved load targets only at the host
     boundary.
   - Consume the canonical `DependencyReadinessProofEnvelope` before
     production runtime dispatch, then delete or fail closed old
     `ModelDependencyRequest`, `ModelRefV2`, node-engine preflight, direct
     runtime task, and path-shaped success paths without compatibility
     adapters.

P-a. [Pumas Library Contract Start](07-pumas-library-image-generation-facts.md)
   - Start Pumas P0-P1 immediately after Pantograph Milestone 0 freezes the
     expected facts contract.
   - Establish Pumas package-facts module ownership, selected-artifact identity,
     DTO/cache contract, cache freshness statuses, and shared fixture shape.

P-b. [Pumas Library Producer-Fact Completion](07-pumas-library-image-generation-facts.md)
   - Complete Pumas P2-P5 before Pantograph Milestone 5a consumes production
     model facts for scheduler dispatch, before Milestone 5c integrates
     production task-level orchestration, before Milestone 5d resolves
     model-specific inference interfaces, before Milestone 5b resolves
     runtime-host load targets, and before Milestone 6 consumes real
     image-generation package facts.
   - Provide richer diffusers component facts, image-family evidence, GGUF
     metadata, package-fact summaries, update cursors, and SQLite cache
     migration/backfill.
   - Publish or otherwise pin the Pumas version/commit Pantograph consumes for
     Milestone 5a, Milestone 5c, Milestone 5d, Milestone 5b, and Milestone 6.

6. [PyTorch/Diffusers Image Generation Execution Slice](milestones/06-pytorch-diffusers-image-generation-execution-slice.md)
   - Implement deterministic PyTorch/diffusers image-generation planning,
     worker execution, and single-body artifact retention.

7. [Candle Future Capability Guardrail](milestones/07-candle-future-capability-guardrail.md)
   - Keep Candle unavailable for image-generation execution until executable
     Candle diffusion support exists.

8. [Release Build And User Validation](milestones/08-release-build-and-user-validation.md)
   - Run affected tests, build frontend/release binary, and record final
     standards/manual smoke checks.

## Execution Order

Milestone 0 is mandatory before parallel or broad implementation. After
Milestone 0, start Pumas P0-P1 so the external package-facts contract, selected
artifact semantics, cache freshness statuses, and fixture shape are frozen early
enough for Pantograph planner work to target the real producer contract.
Milestones 1 and 2 should be completed as one early graph-shape correction
slice because a fixed saved workflow is not complete while tracked producers
can still emit the retired graph shape. Milestones 3 and 4 may be implemented
after the stale diagnostic DTO is frozen. Milestone 5 must land before
scheduler-owned dispatch slices because it freezes the adapter facts the
scheduler will rank. Milestone 5a must then replace whole-workflow static
planning assumptions with dynamic task-level scheduling before future real
execution work depends on dependency readiness, runtime/device selection,
resource admission, batching, or multi-user fairness. Milestone 5c must then
integrate option 4 durable task-level orchestration into workflow/session
execution. Milestone 5d must then add the canonical inference interface
resolver/validator before the remaining Milestone 5b runtime-host wiring
proceeds, because runtime task dispatch needs the same typed model-specific
input/output contract that the graph editor and workflow validators expose.
Milestone 5b must then replace runtime launch ownership with direct
scheduler-to-runtime-host dispatch and remove successful `model_path`,
`ModelRefV2`, and node-engine planned-inference execution contracts before
Milestone 6 implements real PyTorch/diffusers image generation. Its first
remaining production slices must follow the 2026-05-29 contract-first
readiness/handoff re-plan: consume the canonical
`DependencyReadinessProofEnvelope`, keep queue-admission and pre-dispatch
freshness checks, materialize runtime-host requests from durable scheduler task
state, reject old APIs with diagnostics if reached, and then delete the
retired dependency/runtime contracts. The 2026-05-31 refinement requires a
classification slice before further deletion: every remaining
`model_path`/`modelPath`, `ModelDependencyRequest`, `ModelDependencyResolver`,
`ModelRefV2`, and `build_model_ref_v2` reference must be recorded as runtime
execution replacement, canonical scheduler/runtime-host state, app
configuration/embedding/RAG, stale diagnostic fixture, tooling/probe, or
frontend/Tauri transport. Runtime execution references must convert to
scheduler task state/results and runtime-host responses or fail closed with
typed diagnostics; if the canonical contracts lack required facts, stop and
re-plan a shared contract extension.

Pumas P2-P5 may proceed in parallel with Pantograph Milestones 1-5, but Pumas
producer-fact completion is a hard gate before Milestone 5a, Milestone 5c,
Milestone 5d, Milestone 5b, and Milestone 6 consume production
image-generation model facts, integrate production task orchestration, resolve
model-specific inference interfaces, or resolve runtime-host load targets.
Milestone 6 must wait for the
execution planner contracts, backend normalization boundary, scheduler-facing
candidate facts, device-resolution decision from Milestone 0 and Milestone 5,
and scheduler-owned dynamic task dispatch from Milestone 5a plus canonical
inference interface resolution from Milestone 5d plus direct
scheduler-to-runtime-host handoff and legacy removal from Milestone 5b. It
must also
consume the Pumas image-generation facts defined in
[Pumas Library Image Generation Facts](07-pumas-library-image-generation-facts.md)
from a pinned Pumas release or commit. If those facts, summaries, selected
artifact semantics, or cache migration/backfill are not available, stop with a
re-plan note instead of implementing name-derived or fallback behavior.
Milestone 8 is the final build and validation gate.

Recommended high-level order:

```text
Pantograph M0
Pumas P0-P1
Pantograph M1-M5 and Pumas P2-P5 in parallel
Pumas release/pin
Pantograph M5a
Pantograph M5c
Pantograph M5d
Pantograph M5b
Pantograph M6
Pantograph M7-M8
```

Milestone 5a may start in contract/design slices before all runtime execution
work is complete, but production execution slices must not preserve
`ModelDependencyResolver`, `ModelDependencyRequest`, `ModelRefV2`, or
`model_path` as successful scheduler/dependency handoff paths.
Milestone 5b owns removal of those successful runtime execution paths and must
not introduce a scheduler-handoff-to-`ModelRefV2` bridge, synthesize handoff
from reduced workflow execution-plan projections, preserve node-engine
planned-inference launch as an alternate successful execution branch, or bypass
Milestone 5c task-level orchestration by making whole-workflow output demand
the runtime launch driver.
