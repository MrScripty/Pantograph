# Runtime Host Handoff And Legacy Execution Removal

## Objective

Replace the successful `model_path`/`ModelRefV2` execution path with a
runtime-host boundary that consumes scheduler-owned handoff facts. The runtime
host resolves Pumas-approved executable load targets only at the boundary that
needs them, then invokes runtime-specific execution code without exposing those
paths to graph editor, node-engine task intent, scheduler capability hints, or
saved workflow identity.

This is the selected re-plan outcome: split the work into Milestone 5b and use
the runtime-host replacement direction. Do not implement a
`SchedulerRuntimeHandoff` to `ModelRefV2` adapter.

Milestone 5a is closed as scheduler-contract complete. Milestone 5b is the hard
gate for actual legacy deletion: define the canonical runtime-host
request/response and host-owned Pumas load-target resolution first, then delete
the old successful `ModelRefV2`/`model_path` execution paths.

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

## Responsibility Boundaries

- **Scheduler:** owns readiness admission, dispatch selection, task queueing,
  resource admission, dependency policy, batching, and retry/defer/fail
  decisions.
- **Node-engine:** validates graph semantics, submits path-free task intent,
  and consumes scheduler task state. It must not resolve executable paths,
  create `ModelRefV2`, choose runtimes/devices, or repair path-shaped inputs.
- **Runtime host / embedded runtime:** consumes `SchedulerRuntimeHandoff` and
  scheduler dispatch decisions, resolves Pumas-approved load targets, manages
  runtime execution, and records diagnostics.
- **Inference/runtime crates:** execute with host-owned executable facts and
  runtime-specific request contracts. They do not read graph-owned
  `model_path` fields.
- **Graph editor/frontend:** displays capability/task/diagnostic facts and
  optional typed constraints. It must not pass executable paths or final
  scheduler decisions as graph inputs.

## Implementation Direction

Use the clean replacement path:

1. Define host-facing execution input that consumes `SchedulerRuntimeHandoff`
   plus scheduler dispatch decision and carries no `ModelRefV2`.
2. Add runtime-host load-target resolution from Pumas refs/artifact identity to
   executable facts at the host boundary only.
3. Replace PyTorch, llama.cpp, and audio node execution so successful execution
   no longer reads graph `model_path` or emits `ModelRefV2`.
4. Replace node-engine dependency preflight output with typed readiness proof
   or scheduler handoff consumption, then delete `ModelDependencyResolver`,
   `ModelDependencyRequest`, `ModelRefV2`, and path-shaped fixtures/tests.

Fail-closed behavior is allowed only as a temporary guardrail when a required
canonical handoff is missing. It is not a compatibility mode and must emit
typed diagnostics.

## No-Fallback Requirements

- Do not adapt `SchedulerRuntimeHandoff`, `DependencyPreflightResult`, or Pumas
  load-target facts back into `ModelRefV2`.
- Do not accept `model_path`, `modelPath`, `local_load_path`, or executable
  package paths as graph/node-engine successful execution identity.
- Do not leave old resolver calls as alternate successful branches after the
  host handoff is wired.
- Do not let runtime adapters choose scheduler runtime/device policy.
- Do not preserve tests or fixtures that validate path-shaped success behavior.

## Replacement Sequence

1. **Host input contract:** define a runtime-host execution request/response
   shape that consumes `SchedulerRuntimeHandoff`, selected runtime/device
   facts, dependency environment ref, and Pumas model/artifact identity.
2. **Pumas load-target resolution:** add the host-owned service that resolves
   executable load targets from Pumas at runtime dispatch time and maps Pumas
   unavailable states to typed diagnostics.
3. **Runtime execution migration:** update PyTorch, llama.cpp, and audio
   execution paths to use host-owned executable facts instead of graph
   `model_path`.
4. **Node-engine preflight replacement:** replace `Option<ModelRefV2>` output
   with typed readiness/handoff facts and fail closed if scheduler handoff is
   absent.
5. **Legacy deletion:** remove `ModelDependencyResolver`,
   `ModelDependencyRequest`, `ModelRefV2`, `build_model_ref_v2`, path repair
   helpers, frontend `modelPath` dependency actions, and path-shaped
   success fixtures.

## Verification Strategy

- Contract fixtures for host execution request/response and Pumas load-target
  diagnostics.
- Boundary tests proving graph, node-engine, scheduler hints, and saved
  workflow payloads reject executable path fields.
- Runtime-host tests proving Pumas load targets are resolved only inside the
  host boundary and unavailable states fail with typed diagnostics.
- Node-engine tests proving affected runtime nodes fail closed without
  scheduler handoff and do not call `ModelDependencyResolver`.
- Runtime migration tests for PyTorch, llama.cpp, and audio paths proving
  successful execution uses host-owned executable facts and emits non-legacy
  outputs.
- Deletion checks proving `ModelDependencyResolver`, `ModelDependencyRequest`,
  `ModelRefV2`, `build_model_ref_v2`, and successful `model_path` fixtures are
  gone or replaced by canonical contracts.
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
