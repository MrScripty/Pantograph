# Plan: Current Image Generation Graphs And Stale Graph Diagnostics

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

## Standards Rule

The standards constraints in
`02-image-generation-family-planner.md#standards-guardrails` and
`02-image-generation-family-planner.md#standards-compliance-matrix`, plus the
device/runtime constraints in `06-device-runtime-selection.md`, are binding
for every milestone. The directory READMEs in this plan are also part of the
documentation-traceability contract for the split plan structure. If
implementation needs to violate one of these constraints, stop and re-plan
before editing production code.
