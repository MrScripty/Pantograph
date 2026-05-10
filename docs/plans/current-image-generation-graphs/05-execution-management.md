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
- 2026-05-10: Committed the plan directory as the initial documentation slice.
  Milestone 0 then completed as a contract-gate documentation slice with write
  scope limited to this plan. The frozen decisions name the planned graph
  diagnostic DTO, shared graph inspection projection, task/artifact-aware
  `diffusers` to PyTorch execution normalization boundary, missing-facts
  diagnostic fields, wire-format rules, fixture names, isolated test roots,
  decomposition decisions, and concrete fallback/legacy paths found by code
  search.
- 2026-05-10: Completed Milestone 1 and the current graph-shape parts of
  Milestone 2 as one vertical slice. The canonical tracked Juggernaut workflow
  is `.pantograph/workflows/juggernaut-x-v10-sdxl.json`; duplicate ignored
  local Juggernaut/Tiny SD workflow files were removed from the workspace.
  Tracked image-generation saved workflows now use
  `puma-lib -> llm-inference -> image-output`, retain stable model ids only,
  and do not persist local Pumas paths or derived dependency snapshots. Current
  graph canonicalization no longer rewrites retired inference nodes into
  executable `llm-inference`; compatibility migration helpers were deleted and
  graph persistence tests now prove retired shapes remain available for stale
  diagnostics without migration records.
- 2026-05-10: Completed the remaining Milestone 2 Pumas selector/probe slice.
  Diffusion task labels from execution descriptors, package summaries, and
  selector rows now project to graph-facing `image_generation`, while factual
  package metadata such as `pipeline_tag: text-to-image`,
  `recommended_backend: diffusers`, and runtime engine hints remain available
  for downstream diagnostics and planning. This preserves the no-fallback rule:
  the change is deterministic task normalization for current graph intent, not
  a legacy direct-diffusion compatibility route.
- 2026-05-10: Started Milestone 3 with the backend graph diagnostic DTO and
  classifier foundation. `WorkflowGraphDiagnostic` is now owned by the graph
  service facade with typed code/severity/scope fields, bounded message/detail
  payloads, and contract-validation classification for retired node types,
  unknown node types, effective-definition failures, missing edge endpoints,
  missing handles, incompatible ports, capacity errors, and cycles. The older
  `Vec<String>` graph validator remains as a string projection of the
  structured diagnostics for existing binding consumers. Contract-validation
  tests were split into `contract_validation_tests.rs`, keeping the production
  validation module below the standards decomposition threshold after the DTO
  addition.

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

- Initial plan documentation committed.
- Milestone 0 contract gate completed. Production behavior is unchanged; the
  slice freezes contracts and identifies the implementation/test fixtures for
  later vertical slices.
- Milestone 1 completed.
- Milestone 2 completed for tracked workflow/template cleanup, canonicalization
  split, no-rewrite persistence behavior, Pumas diffusion selector/probe task
  projection, and documentation.
- Milestone 3 partially completed for the backend diagnostic DTO, bounded
  payload contract, and structured graph contract-validation classifier.

### Deviations

- Milestone 0 did not add executable tests because it is a pre-implementation
  contract-freeze slice. It names the first failing acceptance test or fixture
  for every implementation milestone before source changes begin.
- Milestone 2 landed in two commits: the first handled graph-shape cleanup and
  no-rewrite canonicalization, and the second handled Pumas selector/probe
  diffusion task projection.
- `crates/workflow-nodes/src/input/puma_lib.rs` remains above the standards
  decomposition-review threshold at 1885 lines. Extraction is deferred because
  this slice needed a narrow graph-task projection change and a module split
  would have expanded the write set beyond the current milestone.

### Follow-Ups

- Pumas P0-P1 should start after Milestone 0 because the Pantograph expected
  package-facts contract is now frozen.
- Investigate and either keep or clean the unrelated dirty plan edit in
  `07-pumas-library-image-generation-facts.md`; it was present during this
  slice and was not staged for the graph-shape commit.
- Plan the next vertical slice for Milestone 3 backend stale graph diagnostics,
  unless the unrelated dirty Pumas plan edit requires integration or cleanup
  first.
- Continue Milestone 3 by adding the shared saved-graph/read-model inspection
  projection that returns `WorkflowGraphDiagnostic` alongside the graph
  snapshot, then wire session/load consumers in a separate validated slice.

### Verification Summary

- `git status --short` inspected before the slice. The user approved ignoring
  untracked SQLite WAL/SHM files and the unrelated proposal markdown.
- Code search verified the relevant existing targets for
  `ConservativeFallback`, override fallback candidates, raw `runtime_hint`,
  retired `diffusion-inference` producers/templates/tests, raw device auto
  behavior, frontend synthetic fallback paths, and artifact single-body
  retention paths.
- No build or unit tests were run for Milestone 0 because the slice changed
  only plan documentation and recorded future acceptance tests.
- `cargo test -p pantograph-workflow-service graph::` passed for the graph
  canonicalization, persistence, registry, and session graph surface touched by
  the slice.
- `node --experimental-strip-types --test src/services/workflow/templateService.test.ts`
  passed for bundled templates and tracked saved image-generation workflow
  fixtures.
- `rg -n "diffusion-inference" crates src packages .pantograph/workflows -g '!target'`
  reports guardrail/test/doc references only after tracked workflow cleanup.
- `cargo test -p workflow-nodes --features model-library puma_lib` passed for
  Pumas selector/probe diffusion task projection, including a live selector
  options path for an imported diffusers bundle.
- A follow-up `rg -n "diffusion-inference" crates src packages .pantograph/workflows -g '!target'`
  reports only `.pantograph/workflows/README.md`; no tracked saved workflow,
  bundled template, or executable producer emits the retired node.
- `cargo test -p pantograph-workflow-service graph::` passed for the Milestone
  3 DTO/classifier foundation, including serde round-trip, bounded diagnostic
  details, retired and unknown node classification, and missing edge
  endpoint/handle diagnostics.

### Traceability Links

- Module README updated: N/A for Milestone 0 because no production module
  ownership changed.
- ADR added/updated: N/A unless implementation changes runtime backend
  ownership boundaries.
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: pending.
