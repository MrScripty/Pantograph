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

8. [Task-Level Scheduler Orchestration](10-task-level-scheduler-orchestration.md)
   - Option 4 target architecture
   - Current whole-run execution gap
   - Task graph, task state, and result materialization boundaries
   - Scheduler, node-engine, runtime-host, frontend, and ledger effects
   - Staged implementation and verification plan

9. [Inference Interface Resolution And Validation](11-inference-interface-resolution-and-validation.md)
   - Generic inference-node model interface resolution
   - Shared descriptor contract for port discovery and validation
   - Graph editor draft validation, save validation, and execution revalidation
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

## Standards Rule

The standards constraints in
`02-image-generation-family-planner.md#standards-guardrails` and
`02-image-generation-family-planner.md#standards-compliance-matrix`, plus the
device/runtime constraints in `06-device-runtime-selection.md`, are binding
for every milestone. The directory READMEs in this plan are also part of the
documentation-traceability contract for the split plan structure. If
implementation needs to violate one of these constraints, stop and re-plan
before editing production code.
