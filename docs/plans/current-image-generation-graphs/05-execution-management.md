# Execution Management

## Execution Notes

Update during implementation:

- 2026-05-09: Plan created from read-only investigation. No implementation
  changes have been made as part of this plan creation.
- 2026-05-09: Plan updated after codebase blast-radius review to add the
  contract gate, explicit `diffusers`-to-PyTorch execution normalization,
  saved-graph IO inspector mode, no silent retired-node rewriting, and
  single-body generated media retention.
- 2026-05-09: Plan iterated against local coding standards to add standards
  guardrails for typed Rust contracts, sync-core/async-shell boundaries,
  path/dimension validation, frontend ownership, no polling/optimistic updates,
  decomposition review, contract tests, and stricter worker write-set rules.
- 2026-05-09: Plan updated to make the no-legacy boundary explicit: old
  Pantograph graph shapes are not supported through migrations or compatibility
  shims. Transformers, ComfyUI, and InvokeAI were added as reference-only
  guidance for naming, generation semantics, diffusion family taxonomy, and
  family-specific validation.
- 2026-05-09: Researched Transformers, ComfyUI, InvokeAI, and Pantograph's
  existing Pumas package fact contracts. Added the concrete image-generation
  family planner design, minimum Pumas facts, table-driven family requirements,
  component-role extraction, sync adapter boundaries, and no-name-inference
  rule needed before implementation.
- 2026-05-09: Plan updated after codebase blast-radius review to require
  consolidation with existing runtime/preflight/gateway/node-engine paths,
  exact Pumas missing-facts diagnostics, source-level image body
  de-duplication, task/artifact-aware `diffusers` execution normalization,
  shared saved/run graph inspection projection, and validated-plan-only Python
  worker execution.
- 2026-05-09: Plan iterated against standards again to add concrete compliance
  gates: cross-boundary executable fixtures, test-first vertical slices,
  isolated test roots, public facade preservation, centralized typed constants,
  accessibility/focus checks, Python worker envelope validation, async
  lifecycle ownership checks, path/resource spot checks, and optional worker
  coordination ledger/report paths.
- 2026-05-09: Added device and runtime-variant planning as a first-class
  objective. The plan now requires backend-owned device policy, selected device
  facts, runtime variant readiness, llama.cpp multi-variant managed runtime
  support, and no fallback from explicit unavailable device requests.
- 2026-05-10: Device/runtime planning updated to make every executable backend
  sit behind an adapter boundary. Adapters expose task/model/device/runtime
  facts, feasibility diagnostics, estimates, and backend-specific translation;
  the scheduler owns backend/device ranking, RAM/VRAM placement, queue policy,
  explicit preference validation, and later learned throughput policy from
  ledger/artifact facts.
- 2026-05-10: Re-ran the plan against coding standards and tightened the
  implementation gates for composition-root lifecycle ownership, loopback-only
  local services, bounded queues/listeners, centralized path validation,
  checked resource arithmetic, Rust public API traits/errors, feature/dependency
  checks, interop envelope tests, frontend accessibility, and isolated
  durable-state tests.
- 2026-05-10: Tightened the no-fallback/no-legacy rule across the plan. Old
  backend, runtime, device, technical-fit, frontend, and graph execution paths
  are removal or replacement targets. Auto is a first-class scheduler policy,
  not a backup path, and canonical planning failures must return typed
  diagnostics rather than invoking fallback behavior.
- 2026-05-10: Re-iterated the split plan against the coding standards. Added
  README coverage for the multi-file plan directories, tightened the standards
  matrix for no-fallback/no-legacy and documentation traceability, and expanded
  release verification for worktree hygiene, cross-platform cfg isolation, and
  dependency ownership.
- 2026-05-10: Updated milestone ordering so Pumas is not treated as the final
  plan step. Pumas P0-P1 starts after Pantograph Milestone 0 to freeze the
  package-facts producer contract early. Pumas P2-P5 may run in parallel with
  Pantograph Milestones 1-5, but Pumas fact extraction, summaries, update
  cursors, selected-artifact semantics, and cache migration/backfill must be
  complete and pinned before Pantograph Milestone 6 begins real PyTorch/diffusers
  image execution.

## Commit Cadence Notes

- Commit the plan as its own documentation slice.
- During implementation, inspect `git status` before starting each slice.
- Do not begin implementation with dirty source, test, config, lockfile,
  generated, or build artifacts unless the user explicitly accepts those
  changes for the slice. Markdown plan files may be dirty only while the plan is
  being updated.
- Commit after each logical slice is complete and verified.
- Keep saved workflow cleanup, stale diagnostics contracts, frontend rendering,
  and PyTorch/diffusers execution as separate commits unless a vertical slice
  requires a small cross-layer commit.
- Follow commit format/history cleanup rules from `COMMIT-STANDARDS.md`.

## Optional Worker Assignment

Use only if implementation begins from a clean integration commit and slices
can be assigned without overlapping write sets. Shared contracts, saved
workflow files, generated TypeScript/Rust DTOs, lockfiles, READMEs, ADRs, and
this plan are serial integration-owner work unless explicitly reassigned in a
worker wave.

If workers are used, create
`docs/plans/current-image-generation-graphs/coordination-ledger.md` before the
wave starts. Worker reports go under
`docs/plans/current-image-generation-graphs/worker-reports/` and must list
changed paths, tests run, standards concerns, and any discovered out-of-scope
issues without editing shared contracts directly.

| Owner/Agent | Scope | Primary Write Set | Allowed Adjacent Write Set | Forbidden/Shared Files | Output Contract | Handoff Checkpoint |
| ----------- | ----- | ----------------- | -------------------------- | ---------------------- | --------------- | ------------------ |
| Integration Owner | Milestone 0 contracts, saved workflow cleanup, generated DTO alignment, README/ADR updates, worker integration | `docs/plans/current-image-generation-graphs/`, `.pantograph/workflows/`, shared DTO modules, generated type outputs, module READMEs | Tests that directly validate the serial contract changes | None; this owner coordinates shared files | Atomic commits with standards spot-check notes | Before any worker wave and after each worker integration |
| Worker A | Backend stale graph diagnostic DTOs and validation after contracts are frozen | `crates/pantograph-workflow-service/src/graph/`, related graph tests | Narrow workflow-service tests that consume graph diagnostics | Saved workflow files, generated frontend types, inference backend files, lockfiles | Tests plus report listing changed files and any standards concerns | After Milestone 3 tests pass in worker workspace |
| Worker B | Frontend IO inspector stale graph rendering after backend DTO shape is frozen | `src/components/workbench/`, `src/services/workflow/` presenter/type consumers, related frontend tests | `packages/svelte-graph/` only if presenter extraction requires it and integration owner approves | Backend DTO definitions, saved workflow files, lockfiles, generated files unless assigned | Tests plus report listing changed files, selectors used, and lifecycle cleanup checks | After backend DTO contract is frozen and generated/handwritten TS types are available |
| Worker C | Device policy, backend adapter candidate facts, and runtime variant contracts after Milestone 0 is frozen | `crates/inference/src/device.rs`, backend adapter contract modules, managed runtime contracts/tests, runtime registry technical-fit tests | Frontend device selector consumers only if generated DTO shape is frozen | Saved workflow files, PyTorch worker image execution, graph diagnostics, lockfiles | Tests plus report listing variant/device DTOs, adapter candidate facts, no-fallback checks, scheduler/inference ownership boundaries, standards gate results, path/resource validation, lifecycle owner, feature/dependency impact, and affected runtime state paths | Before PyTorch/diffusers execution planning consumes backend/device decisions |
| Worker D | PyTorch/diffusers execution planner and backend bridge after execution and device contracts are frozen | `crates/inference/src/`, `crates/node-engine/src/core_executor/`, `crates/inference/torch/`, related tests | `crates/pantograph-embedded-runtime/src/` only for execution-normalization consumers approved by integration owner | Saved workflow files, frontend components, shared graph diagnostics, lockfiles, device contract files unless assigned | Tests plus report listing changed files, worker bridge shape, no-fallback checks, and decomposition decisions | After canonical graph, execution planner, and device-resolution contracts are confirmed |

Worker rules:

- Each worker uses an isolated worktree or temporary clone from the same clean
  integration commit.
- Workers may read broadly but must not edit outside their primary or approved
  adjacent write set.
- If required changes fall outside the assigned write set, workers record them
  in the report instead of editing them.
- Integration owner reviews reports, verifies write sets, integrates one worker
  at a time, resolves conflicts in a separate integration commit, runs the
  wave's verification, and updates this plan before starting another wave.
- Worker reports are stored under the plan's `worker-reports/` directory if
  workers are used.

## Re-Plan Triggers

- Pumas 0.6.0 selected-model detail cannot resolve the Juggernaut model id.
- Existing PyTorch worker diffusion support cannot load Pumas diffusers
  directory package facts without a contract change.
- Runtime readiness cannot distinguish PyTorch base availability from
  diffusers dependency availability.
- Runtime readiness cannot represent multiple runtime variants for one
  managed backend release without duplicating binary-management ownership.
- Backend probes cannot provide enough facts to distinguish explicit device
  unavailability from auto-selection behavior.
- Backend adapter candidate facts force scheduler ranking, queue policy, or
  learned placement decisions into the inference crate.
- Scheduler admission needs to inspect raw backend command strings to choose a
  backend/runtime/device.
- A backend adapter requires a global runtime, untracked task, unbounded queue,
  unbounded listener, non-loopback local service, or process lifecycle outside a
  composition-root owner.
- Runtime roots, executable paths, dynamic-library paths, Pumas package paths,
  artifacts, or worker-visible paths cannot be validated through shared
  allowed-root handling.
- Resource estimate, dimensions, token/context, byte-range, or output-size
  calculations cannot be expressed with checked arithmetic and typed failures.
- Runtime feature/dependency changes cannot pass default, no-default-features,
  and all-features checks for affected public crates.
- IO inspector cannot consume graph diagnostics without creating a duplicate
  graph read model.
- Tests reveal that saved workflow JSON still embeds large generated image
  bodies after graph execution.
- Candle is already selected by an existing backend policy for diffusion
  package facts.
- Fixing the graph requires changing public inference request/result contracts.
- Standards spot checks reveal new production `unwrap()`/`expect()` use,
  stringly public planner state, unvalidated paths/dimensions, unowned async
  tasks, unbounded queues, frontend polling loops, or large-file threshold
  crossings without decomposition review.
- A worker needs to edit outside its assigned write set or a shared contract
  changes after worker implementation begins.

## Recommendations

- Prefer `runtime_hint = "pytorch"` for the first saved Juggernaut workflow
  for clarity. Accept `runtime_hint = "diffusers"` only through centralized
  normalization that resolves it to PyTorch execution until there is a
  separately registered Diffusers backend.
- Keep stale graph diagnostics separate from workflow-run diagnostics unless
  a stale graph reaches submission, admission, or execution. This avoids
  polluting the run ledger with editor/load validation facts.
- Use Tiny SD Turbo as the first executable image-generation vertical slice,
  then validate Juggernaut after the small-model path proves the backend,
  artifact, and UI contracts.

## Completion Summary

### Completed

- Not started.

### Deviations

- None.

### Follow-Ups

- None yet.

### Verification Summary

- Not run.

### Traceability Links

- Module README updated: pending implementation.
- ADR added/updated: N/A unless implementation changes runtime backend
  ownership boundaries.
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: pending.
