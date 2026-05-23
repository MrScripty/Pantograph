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
batching group, readiness proof, and selected dispatch facts.

Milestone 5b therefore depends on option 4 task-level scheduler orchestration
before its remaining production wiring continues. Scheduler dispatch must call
runtime-host execution directly with the actual task-level handoff, and
workflow progress must be driven by scheduler task state rather than
whole-workflow output-node demand. Do not synthesize a handoff from reduced
workflow execution-plan fields, and do not keep planned inference as an
alternate successful launch path.

## Responsibility Boundaries

- **Scheduler:** owns readiness admission, dispatch selection, task queueing,
  resource admission, dependency policy, batching, retry/defer/fail decisions,
  and the moment a task is handed to a runtime host.
- **Scheduler dispatch orchestrator:** builds `RuntimeHostExecutionRequest`
  only from a validated dispatch-selected `SchedulerRuntimeHandoff`, invokes
  the runtime-host execution port, and records the returned task state and
  diagnostics. It must not resolve executable load targets or call worker
  APIs directly.
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

1. Define host-facing execution input that consumes `SchedulerRuntimeHandoff`
   plus scheduler dispatch decision and carries no `ModelRefV2`.
2. Add runtime-host load-target resolution from Pumas refs/artifact identity to
   executable facts at the host boundary only.
3. Add a scheduler-owned runtime-host execution port and dispatch orchestrator.
   The orchestrator is the only successful caller of runtime-host execution and
   must pass the actual validated `SchedulerRuntimeHandoff`.
4. Complete task-level scheduler orchestration from
   `10-task-level-scheduler-orchestration.md` so production session execution
   has durable task state, task results, and dispatch-selected handoff at the
   task boundary.
5. Retire planned-inference launch from node-engine. Inference nodes become
   scheduler task-intent producers and consumers of scheduler task state/results
   rather than callers of `PlannedInferenceExecutionHost`.
6. Replace PyTorch, llama.cpp, and audio node execution so successful execution
   no longer reads graph `model_path`, reduced execution-plan projections, or
   emits `ModelRefV2`.
7. Replace node-engine dependency preflight output with typed readiness proof
   or scheduler task state consumption, then delete `ModelDependencyResolver`,
   `ModelDependencyRequest`, `ModelRefV2`, and path-shaped fixtures/tests.

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
2. **Pumas load-target resolution:** add the host-owned service that resolves
   executable load targets from Pumas at runtime dispatch time and maps Pumas
   unavailable states to typed diagnostics.
3. **Scheduler execution port:** add a narrow runtime-host execution port at
   the scheduler/application boundary. The port accepts only
   `RuntimeHostExecutionRequest` and returns `RuntimeHostExecutionResponse`.
   The scheduler dispatch orchestrator owns request ids, cancellation/retry
   correlation, and recording response diagnostics; runtime host owns Pumas
   path resolution and worker execution.
4. **Direct dispatch wiring:** update workflow/session execution so a
   dispatch-selected scheduler handoff invokes runtime-host execution directly.
   This step depends on option 4 task-level scheduler orchestration so the
   handoff comes from durable task state rather than whole-workflow reduced
   plans. The reduced `WorkflowExecutionPlan` remains available for inspection
   and diagnostics only; it must not be used to launch inference.
5. **Node-engine launch retirement:** remove node-engine planned-inference
   launch ownership for runtime inference nodes. Affected inference nodes must
   submit or reference schedulable task intent and consume scheduler task
   results/state. Missing scheduler task state fails closed with typed
   diagnostics.
6. **Runtime execution migration:** update PyTorch, llama.cpp, and audio
   execution paths to use host-owned executable facts instead of graph
   `model_path`.
7. **Node-engine preflight replacement:** replace `Option<ModelRefV2>` output
   with typed readiness/task-state facts and fail closed if scheduler-owned
   readiness or task state is absent.
8. **Legacy deletion:** remove `ModelDependencyResolver`,
   `ModelDependencyRequest`, `ModelRefV2`, `build_model_ref_v2`, path repair
   helpers, `PlannedInferenceExecutionHost`,
   `EmbeddedPlannedInferenceExecutionHost`, frontend `modelPath` dependency
   actions, and path-shaped success fixtures.

## Verification Strategy

- Contract fixtures for host execution request/response and Pumas load-target
  diagnostics.
- Boundary tests proving graph, node-engine, scheduler hints, and saved
  workflow payloads reject executable path fields.
- Runtime-host tests proving Pumas load targets are resolved only inside the
  host boundary and unavailable states fail with typed diagnostics.
- Scheduler dispatch tests proving runtime-host execution requests are built
  only from dispatch-selected `SchedulerRuntimeHandoff` values, reject reduced
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
