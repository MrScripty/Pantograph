# Current Image Generation Graphs Plan

## Purpose
This directory contains the active implementation plan for canonical image
generation graphs, stale graph diagnostics, single-body media retention,
backend-owned device/runtime selection, and scheduler-owned dynamic task
dispatch.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `plan.md` | Entry point that links the split plan sections and states binding standards rules. |
| `implementation-recovery-sequence.md` | Authoritative active recovery sequence for plan reconciliation, validation repair, canonical `1..N` scheduler dispatch, baseline restoration, and real workflow-editor image generation. |
| `00-objective-scope.md` | Scope, exclusions, and no-fallback/no-legacy boundaries. |
| `01-inputs-contracts.md` | Codebase findings, constraints, assumptions, and affected contracts. |
| `02-image-generation-family-planner.md` | Family planner design, reference-repo guidance, and standards matrix. |
| `03-risks-and-definition-of-done.md` | Risk table and completion criteria. |
| `04-milestones.md` | Milestone index and ordering rules. |
| `05-execution-management.md` | Commit cadence, worker coordination, re-plan triggers, and execution notes. |
| `06-device-runtime-selection.md` | Device policy, runtime variant, adapter boundary, and scheduler decision design. |
| `07-pumas-library-image-generation-facts.md` | Pumas Library producer-fact plan for image-generation family facts, diffusers component evidence, GGUF metadata, summaries, and update cursors. |
| `08-scheduler-owned-dynamic-task-dispatch.md` | Scheduler-owned dynamic task dispatch design for concurrent workflow tasks, batching, resource admission, capability hints, and dispatch decisions. |
| `09-runtime-host-handoff-legacy-removal.md` | Runtime-host handoff replacement plan for removing `model_path`/`ModelRefV2` successful execution paths. |
| `10-task-level-scheduler-orchestration.md` | Option 4 target architecture for durable task-level workflow scheduling, task state, orchestration, and runtime-host dispatch integration. |
| `11-inference-interface-resolution-and-validation.md` | Backend-owned generic inference-node interface resolution and shared validation for graph editor ports, executable publish/admission, and execution materialization. |
| `milestones/` | Per-slice implementation checklists and verification gates. |

## Problem
Pantograph had stale diffusion graph shapes, duplicated media payloads,
scattered backend/device selection paths, and execution-time dependency/runtime
discovery split across node-engine and host code. This plan defines the
canonical graph, inference, artifact, runtime-selection, and scheduler dispatch
work needed to replace those paths.

## Constraints
- Backend services are the source of truth for graph validity, diagnostics,
  model facts, runtime readiness, and execution contracts.
- No fallback or legacy execution behavior is allowed.
- Pumas remains the canonical model source.
- Implementation must follow the standards in the developer-tooling
  `Coding-Standards` repository.

## Decision
Use split plan files so each high-risk area has focused acceptance criteria
while `plan.md` and `04-milestones.md` preserve execution order. Contract gates
come before implementation, then thin vertical slices validate behavior before
adjacent slices expand shared layers.

The
[Implementation Recovery Sequence](implementation-recovery-sequence.md) is the
authoritative active execution overlay while stale milestone statuses and the
older grouped-only dispatch decision are reconciled. Historical execution
notes remain evidence, but they do not override the recovery sequence's
canonical non-empty `1..N` scheduler dispatch decision.

Dynamic scheduler-owned task dispatch is tracked as its own design section and
inserted milestone because concurrent users and batching make whole-workflow
static planning the wrong abstraction. Ready workflow DAG nodes become
schedulable task units; scheduler policy owns queueing, batching, fairness,
resource admission, runtime/device selection, and dependency readiness policy.

Runtime-host handoff and legacy execution removal are split into a follow-on
design section and Milestone 5b. Scheduler handoff contracts alone are not
enough to remove legacy execution because the current runtime nodes still read
`model_path` and emit `ModelRefV2`; the host must consume scheduler handoff,
resolve Pumas-approved load targets at the runtime boundary, and then delete
the old resolver/path contracts.

Option 4 task-level scheduler orchestration is now the target bridge between
Milestone 5a contracts and the remaining Milestone 5b runtime-host deletion
work. The scheduler must own durable per-task state and dispatch progress
before runtime-host execution is wired into production session execution. This
prevents reduced workflow execution plans or node-engine output demand from
remaining successful runtime launch paths.

Generic inference interface resolution is split into Milestone 5d because the
ports shown by the graph editor, the workflow validator, scheduler task input
materialization, and pre-dispatch runtime validation must all come from one
backend-owned model-specific descriptor. This keeps the generic inference node
simple while avoiding image-only port tables, scheduler-owned execution
payloads, and graph-visible Pumas package facts.

Pumas is split across the execution order. Pumas P0-P1 starts after Pantograph
Milestone 0 so the package-facts contract is available early. Pumas P2-P5 may
run in parallel with Pantograph Milestones 1-5, but must complete and be pinned
before Pantograph Milestone 5a consumes production model facts for scheduler
dispatch, before Milestone 5c integrates production task-level orchestration,
before Milestone 5d resolves model-specific inference interfaces, before
Milestone 5b resolves runtime-host load targets, and before Milestone 6
consumes real image-generation package facts.

## Alternatives Rejected
- Single large plan file: rejected because the plan already spans graph,
  inference, runtime, frontend, persistence, and diagnostics concerns.
- Compatibility-first migration: rejected because Pantograph does not preserve
  old graph/runtime/device execution shapes.

## Invariants
- Old graph, backend, runtime, device, and worker execution methods are removed
  or replaced, not used as fallbacks.
- Frontend and Tauri render or transport backend-owned facts; they do not define
  execution semantics.
- Shared contracts are frozen before parallel implementation begins.
- Workflows are durable DAG runs; ready nodes are scheduled as task units, not
  as one static all-at-once workflow execution object.

## Revisit Triggers
- A canonical contract cannot represent a required backend/device/runtime fact.
- A standards gate requires changing milestone order or write ownership.
- A vertical slice cannot be verified without adding fallback behavior.

## Dependencies
**Internal:** Pantograph workflow service, node engine, inference crate,
embedded runtime, diagnostics ledger, ArtifactStore, Tauri IPC, frontend
workbench components, and tracked workflow fixtures.

**External:** Pumas Library package facts, Python/PyTorch diffusers execution,
llama.cpp managed runtime artifacts, and reference guidance from Transformers,
ComfyUI, and InvokeAI.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- `docs/adr/ADR-011-scheduler-only-workflow-execution.md`
- `docs/adr/ADR-013-workflow-version-registry-and-run-snapshots.md`
- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`

## Usage Examples
Start with `implementation-recovery-sequence.md`, then use `plan.md`,
`04-milestones.md`, and the milestone files for historical detail and
owner-specific checklists. During recovery implementation, update the recovery
sequence after each validated slice with verification results, deviations,
discovered issues, and follow-ups.

## API Consumer Contract
- Supported inputs: plan readers should treat
  `implementation-recovery-sequence.md` as the active execution entry point
  until its reconciliation milestone is complete.
- Outputs: implementation tasks, verification gates, re-plan triggers, and
  standards constraints.
- Lifecycle: update the relevant milestone status and execution notes after
  each verified slice.
- Error behavior: if implementation discovers a conflict with standards or the
  no-fallback rule, stop and re-plan before production edits continue.
- Compatibility: this plan intentionally allows breaking changes to old
  Pantograph graph/runtime/device shapes.

## Structured Producer Contract
- Stable fields: milestone titles, task checkboxes, verification bullets, risk
  mitigations, re-plan triggers, and execution notes.
- Volatile fields: status, verification results, worker reports, and follow-up
  issues.
- Default semantics: unchecked tasks are not started unless a status note says
  otherwise.
- Ordering: milestone order in `implementation-recovery-sequence.md` is
  binding during recovery; Milestone 0 will reconcile it into the older plan
  set.
- Compatibility: old plan assumptions are replaced by explicit execution notes
  rather than kept as parallel interpretations.
- Regeneration/migration: split files may be reorganized only with this README
  and `plan.md` updated in the same documentation slice.
