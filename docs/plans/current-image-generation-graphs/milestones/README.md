# Current Image Generation Milestones

## Purpose
This directory contains the per-milestone implementation checklists for the
current image-generation graph and device/runtime selection plan.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `00-contract-gate.md` | Serial contract-freeze gate before implementation or parallel work. |
| `01-current-juggernaut-graph-slice.md` | Saved Juggernaut graph repair slice. |
| `02-retired-node-producers-removed.md` | Removal of retired direct diffusion graph producers. |
| `03-backend-stale-graph-diagnostics.md` | Backend-owned stale graph diagnostic contract and projection. |
| `04-io-inspector-stale-graph-presentation.md` | IO inspector saved-graph display slice. |
| `05-device-and-runtime-variant-selection.md` | Device policy, runtime variant, and scheduler-facing candidate slice. |
| `05a-scheduler-owned-dynamic-task-dispatch.md` | Scheduler-owned dynamic task dispatch, capability hints, task queueing, batching, resource admission, and dispatch decisions. |
| `05b-runtime-host-handoff-legacy-removal.md` | Runtime-host handoff wiring and removal of successful `model_path`/`ModelRefV2` execution paths. |
| `06-pytorch-diffusers-image-generation-execution-slice.md` | Deterministic PyTorch/diffusers execution slice. |
| `07-candle-future-capability-guardrail.md` | Candle non-selection guardrail for image generation. |
| `08-release-build-and-user-validation.md` | Final verification, release build, and smoke-test gate. |

## Problem
The work crosses graph persistence, backend validation, inference execution,
device selection, frontend rendering, diagnostics, and release packaging.
Separate milestone files keep each vertical slice auditable and reduce the risk
of broad unverified horizontal changes.

## Constraints
- Milestone 0 must freeze shared contracts before implementation expands.
- Device/runtime contracts must land before scheduler-owned dispatch slices
  that consume them.
- Dynamic task dispatch must replace whole-workflow static planning
  assumptions before future execution slices depend on scheduler runtime/device,
  dependency readiness, resource admission, batching, or multi-user fairness.
- Runtime-host handoff must replace successful `model_path`/`ModelRefV2`
  execution before real PyTorch/diffusers image execution is implemented.
- No fallback or legacy execution paths may remain reachable.
- Each milestone must define verification before implementation is considered
  complete.

## Decision
Use one file per milestone, with goal, tasks, verification, and status. The
ordering and dependency rules remain centralized in `../04-milestones.md`.

## Alternatives Rejected
- Embedding all milestone tasks in one file: rejected because the detailed
  device/runtime and execution slices need independent standards gates.
- Free-form execution notes only: rejected because each slice needs explicit
  acceptance criteria before implementation begins.

## Invariants
- Milestones are executed in dependency order unless the parent plan is updated.
- Shared contracts, generated DTOs, lockfiles, saved workflow files, and READMEs
  are serial integration-owner work unless reassigned in `05-execution-management.md`.
- Verification results are recorded before a milestone is marked complete.

## Revisit Triggers
- A milestone cannot be validated with its listed tests or fixtures.
- A worker needs to edit outside an assigned write set.
- A shared contract changes after dependent implementation has started.

## Dependencies
**Internal:** Parent plan files, Pantograph source crates, frontend services,
tracked workflow fixtures, and release/build tooling.

**External:** Coding standards, Pumas Library contracts, Python/PyTorch
diffusers runtime, and managed llama.cpp runtime artifacts.

## Related ADRs
- None identified as of 2026-05-10.
- Reason: These files are execution checklists for a plan, not committed
  architecture decisions.
- Revisit trigger: A milestone introduces a durable architecture decision that
  should outlive this plan.

## Usage Examples
Before implementing Milestone 5, read `../06-device-runtime-selection.md`.
Before implementing Milestone 5a, read
`../08-scheduler-owned-dynamic-task-dispatch.md`. Before implementing Milestone
5b, read `../09-runtime-host-handoff-legacy-removal.md`. Update
`../05-execution-management.md` with verification and standards notes after
each slice passes.

## API Consumer Contract
- Supported inputs: milestone files are consumed by implementers and reviewers.
- Outputs: scoped tasks, verification gates, and current status.
- Lifecycle: update status and parent execution notes after a validated slice.
- Error behavior: when verification cannot pass, record the issue and re-plan.
- Compatibility: milestone tasks may remove old behavior where the parent plan
  marks it non-canonical.

## Structured Producer Contract
- Stable fields: title, goal, tasks, verification, and status.
- Volatile fields: status notes, discovered issues, and verification results.
- Default semantics: unchecked items are pending.
- Ordering: numeric filename order reflects execution order unless
  `../04-milestones.md` says otherwise.
- Compatibility: old milestone wording is superseded by later execution notes
  in `../05-execution-management.md`.
- Regeneration/migration: adding, removing, or renumbering milestones requires
  updating this README and `../04-milestones.md` together.
