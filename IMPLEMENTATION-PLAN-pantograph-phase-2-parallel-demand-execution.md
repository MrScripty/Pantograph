# Plan: Pantograph Phase 2 Parallel Demand Execution

## Status
In progress

Last updated: 2026-04-18

## Current Source-of-Truth Summary

This document is the dedicated source of truth for roadmap Phase 2 parallel
demand execution. It expands the short Phase 2 roadmap subsection into a
standards-reviewed implementation plan for replacing sequential
`demand_multiple` behavior with bounded backend-owned parallel execution while
preserving deterministic results, stable workflow events, and clean ownership
boundaries.

Phase 2 status should be updated here first once implementation begins. The
roadmap should summarize progress and point back to this file rather than
duplicating milestone detail.

## Objective

Implement bounded parallel demand execution in `crates/node-engine` so
independent workflow branches can execute concurrently, while refactoring the
immediate insertion points to comply with the architecture, coding,
concurrency, testing, documentation, tooling, and interop standards and while
preserving backend-owned execution, cache, and event semantics.

## Scope

### In Scope

- Backend-owned dependency analysis and execution planning for
  `WorkflowExecutor::demand_multiple`
- Bounded parallel execution coordination inside `crates/node-engine`
- Deterministic output merge, cache ownership, and event/trace semantics under
  parallel execution
- Additive execution-budget controls when needed to bound runtime pressure
- Refactors required to keep the immediate `node-engine` insertion points
  compliant before parallel execution logic lands
- Benchmarks, tests, and documentation needed to verify correctness and
  measurable improvement

### Out of Scope

- Scheduler V2 policy changes or queue-admission behavior
- Distributed execution or multi-host execution
- KV cache implementation
- Frontend-owned execution policy or transport-side concurrency decisions
- Broad runtime-selection changes beyond any execution-budget information
  needed to keep local execution bounded

## Inputs

### Problem

`WorkflowExecutor::demand_multiple` still executes requested nodes
sequentially, even when they belong to independent subgraphs. That leaves
available parallelism unused and makes incremental reruns slower than they need
to be. At the same time, the immediate engine insertion points are already
oversized, so landing parallel scheduling logic directly into the current files
would violate the standards and make later debugging of cache, invalidation,
and event-ordering issues much harder.

Without a dedicated plan, the likely failure mode is a large in-place rewrite
inside `engine.rs` that mixes dependency planning, cache ownership, event
emission, and async task coordination under one lock-heavy path.

### Constraints

- Core execution, dependency analysis, and cache ownership stay in Rust backend
  crates.
- Parallelism must be bounded. Unbounded fan-out is not acceptable.
- Deterministic outputs and stable backend-owned event semantics must be
  preserved even when multiple nodes execute concurrently.
- Shared mutable state must remain under a clear single owner and must not be
  held across `.await` points unless an async-aware lock is deliberately
  justified.
- Existing public workflow execution facades should remain additive unless an
  API break is approved and documented.
- The plan must not move execution policy into Tauri or frontend code.
- Existing oversized insertion files require decomposition review before
  parallel execution logic expands further.

### Public Facade Preservation Note

Phase 2 is a facade-first refactor. The public `WorkflowExecutor` surface
remains the entry point, with internal execution planning and coordination
extracted behind it rather than exposed as a new public API unless an additive
configuration surface is explicitly required.

### Assumptions

- `Metrics/trace spine` and `Scheduler V2` now provide enough observability to
  validate parallel execution without first inventing a separate instrumentation
  layer.
- `demand_multiple` parallelism should begin with independent-node execution
  for a single run, not cross-run scheduling.
- Initial bounded parallelism can use repo-standard async primitives already
  available in the Rust stack; no new execution framework is assumed.
- The current event vocabulary is sufficient unless parallel execution reveals
  a missing additive metadata field that must be frozen before implementation.

### Dependencies

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `IMPLEMENTATION-PLAN-pantograph-metrics-trace-spine.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-5-real-workflow-event-contract.md`
- `crates/node-engine`
- `crates/pantograph-embedded-runtime`
- Touched README files under `crates/node-engine/src/`

### Affected Structured Contracts

- `WorkflowExecutor::demand_multiple` behavior and any additive execution-budget
  configuration it may require
- `node_engine::WorkflowEvent` only if additive parallel-execution metadata is
  needed
- Trace/metrics observations if new bounded-parallel execution fields are
  surfaced additively

### Affected Persisted Artifacts

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- This implementation plan
- Touched `README.md` files for `crates/node-engine/src/` directories
- Any benchmark fixtures or representative workflow artifacts added to support
  repeatable verification

### Existing Codebase Non-Compliance In Immediate Surroundings

The immediate Phase 2 insertion points already cross decomposition thresholds
from `CODING-STANDARDS.md`:

- `crates/node-engine/src/engine.rs` is approximately 1398 lines
- `crates/node-engine/src/orchestration/executor.rs` is approximately 646
  lines
- `crates/node-engine/src/events.rs` is approximately 545 lines

Phase 2 must therefore start by extracting execution-planning and coordination
responsibilities out of these files rather than layering parallelism directly
onto the existing monolithic paths.

### Concurrency / Race-Risk Review

- Shared dependencies across requested target nodes must be deduplicated so the
  same node does not execute twice concurrently within one run.
- Cache state, version tracking, and in-flight execution state must remain
  consistent under concurrent demand and invalidation.
- Event ordering must stay meaningful even if node execution overlaps; the plan
  must define what is deterministic and what is intentionally concurrent.
- Fail-fast, cancellation, and waiting-for-input behavior must not strand
  partially completed task groups or leave stale in-flight bookkeeping behind.
- Any queue or worker collection introduced to coordinate parallel execution
  must be bounded and owned by a single coordinator.

### Ownership And Lifecycle Note

- `crates/node-engine` owns dependency analysis, execution planning,
  in-flight-task coordination, cache semantics, and canonical execution events.
- `WorkflowExecutor` remains the public facade and composition point for the
  per-run execution coordinator.
- `crates/pantograph-embedded-runtime` remains an executor/runtime dependency
  consumed by `node-engine`; it does not own parallel planning policy.
- Tauri and frontend layers remain consumers of backend-owned execution events
  and trace outputs only.
- Any per-run coordination structure introduced for parallel execution must
  document who creates it, who tears it down, and how cancellation or failure
  releases in-flight state.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Shared dependencies execute more than once under concurrent demand | High | Introduce backend-owned in-flight coordination keyed by node id and execution id |
| Event or output ordering becomes nondeterministic in ways consumers cannot handle | High | Freeze deterministic ordering rules before implementation and keep consumer contracts additive |
| Lock contention or deadlocks erase the performance win | High | Extract planning/coordinator modules, avoid holding sync locks across `.await`, and benchmark representative workflows |
| Parallel fan-out overwhelms runtime capacity | Medium | Add bounded execution-budget controls and conservative defaults |
| Failure, cancellation, or waiting-for-input leaves stale coordinator state | Medium | Add explicit cleanup semantics plus failure/recovery tests |

## Standards Review Passes

### Pass 1: Plan Structure And Source-of-Truth

Reviewed against:
- `PLAN-STANDARDS.md`
- `templates/PLAN-TEMPLATE.md`

Corrections applied:
- Kept this as a dedicated Phase 2 source-of-truth plan instead of a short
  roadmap bullet list.
- Added required inputs, risks, done criteria, milestones, verification, and
  re-plan triggers.
- Included concurrency, ownership, and facade-preservation notes because the
  work changes async execution behavior.

### Pass 2: Architecture And Code Ownership

Reviewed against:
- `ARCHITECTURE-PATTERNS.md`
- `CODING-STANDARDS.md`

Corrections applied:
- Kept execution planning and concurrency ownership inside `crates/node-engine`
  rather than adapters or frontend code.
- Added decomposition milestones for the oversized immediate insertion points
  before parallel logic lands.
- Preserved the public `WorkflowExecutor` facade and moved complexity behind
  focused backend modules.

### Pass 3: Concurrency And Async Safety

Reviewed against:
- `CONCURRENCY-STANDARDS.md`

Corrections applied:
- Required bounded parallelism rather than unbounded task spawning.
- Added explicit ownership for in-flight node coordination and cleanup on
  cancellation or failure.
- Recorded that shared mutable state must not be held under blocking or sync
  locks across `.await`.

### Pass 4: Testing And Interop Stability

Reviewed against:
- `TESTING-STANDARDS.md`
- `INTEROP-STANDARDS.md`

Corrections applied:
- Required cross-layer acceptance checks so parallel execution is validated
  from backend producer through transport consumer, not only through unit
  tests.
- Planned replay/recovery and waiting-for-input coverage for concurrent demand
  paths.
- Limited contract changes to additive event or metrics fields if needed, with
  same-slice consumer updates when a boundary contract changes.

### Pass 5: Documentation, Tooling, And Dependencies

Reviewed against:
- `DOCUMENTATION-STANDARDS.md`
- `TOOLING-STANDARDS.md`
- `DEPENDENCY-STANDARDS.md`

Corrections applied:
- Planned README updates where extracted execution modules change directory
  ownership or consumer contracts.
- Kept the plan dependency-neutral by default; no new third-party runtime or
  scheduling library is assumed.
- Included benchmark and verification work as first-class close-out criteria
  instead of leaving performance claims implicit.

## Definition of Done

- `WorkflowExecutor::demand_multiple` can execute independent nodes in bounded
  parallel groups while preserving correctness.
- Shared dependencies are executed once per run and reused safely across
  concurrent downstream demand.
- Event, trace, failure, cancellation, and waiting-for-input semantics remain
  stable and attributable under parallel execution.
- Immediate oversized insertion points touched by the phase are decomposed
  enough that the final implementation lands in focused modules.
- Benchmarks and acceptance checks show either measurable improvement or a
  documented reason why a specific path remains sequential.

## Milestones

### Milestone 1: Decompose The Execution Hot Path

**Goal:** Create compliant engine boundaries before landing concurrency logic.

**Tasks:**
- [ ] Extract dependency-planning and target-resolution helpers from
      `crates/node-engine/src/engine.rs` into focused modules.
- [ ] Extract orchestration/event helper logic from oversized engine-adjacent
      files where needed so the parallel coordinator has a clear insertion
      boundary.
- [ ] Update touched `README.md` files if the directory boundary or ownership
      explanation changes.
- [ ] Keep the existing `WorkflowExecutor` facade unchanged while moving logic
      behind smaller internal modules.

**Verification:**
- `cargo check -p node-engine`
- Focused no-behavior-change tests for extracted helpers where practical
- Documentation review against `DOCUMENTATION-STANDARDS.md`

**Status:** In progress

### Milestone 2: Freeze Parallel Execution Semantics

**Goal:** Define the concurrency contract before implementation spreads.

**Tasks:**
- [ ] Define how independent target nodes are grouped, ordered, and deduped
      before execution begins.
- [ ] Define bounded execution-budget semantics, including default behavior and
      any additive configuration surface.
- [ ] Define deterministic result merge and event-order expectations under
      concurrency.
- [ ] Define cleanup semantics for failure, cancellation, and
      waiting-for-input paths.

**Verification:**
- Architecture review against `ARCHITECTURE-PATTERNS.md`
- Concurrency review against `CONCURRENCY-STANDARDS.md`
- Contract review if additive event or metrics fields are required

**Status:** Not started

### Milestone 3: Land The Bounded Parallel Coordinator

**Goal:** Replace sequential `demand_multiple` execution with a safe
backend-owned coordinator.

**Tasks:**
- [ ] Implement an internal coordinator that plans independent work, enforces a
      bounded parallelism limit, and deduplicates shared dependencies.
- [ ] Keep in-flight coordination backend-owned and release it correctly on
      success, failure, cancellation, or waiting-for-input.
- [ ] Avoid holding inappropriate locks across `.await` points and keep the
      coordinator’s shared mutable state under a clear owner.
- [ ] Preserve the sequential fallback path for graphs that cannot safely
      parallelize.

**Verification:**
- `cargo test -p node-engine`
- Focused tests for independent-branch parallelism, shared-dependency dedupe,
  failure cleanup, and waiting-for-input behavior

**Status:** Not started

### Milestone 4: Preserve Event, Trace, And Consumer Semantics

**Goal:** Make parallel execution observable without breaking downstream
consumers.

**Tasks:**
- [ ] Verify canonical workflow events remain attributable and meaningful under
      overlapping node execution.
- [ ] Add any strictly necessary additive event or metrics metadata in
      backend-owned contracts and update affected consumers in the same logical
      slice.
- [ ] Confirm metrics/trace aggregation still records correct node/run timing
      under bounded parallel execution.
- [ ] Keep Tauri and frontend consumers read-only with respect to backend-owned
      execution state.

**Verification:**
- Targeted `cargo test -p pantograph-embedded-runtime`
- Targeted Tauri workflow adapter tests if any additive event fields cross that
  boundary
- `npm run typecheck` if mirrored TypeScript contracts change

**Status:** Not started

### Milestone 5: Benchmarks, Acceptance Coverage, And Source-of-Truth Close-Out

**Goal:** Prove correctness and performance, then leave accurate traceability
behind.

**Tasks:**
- [ ] Add representative workflow benchmarks or benchmark-like reproducible
      harnesses that compare the sequential baseline with bounded parallel
      execution.
- [ ] Add cross-layer acceptance coverage for a real workflow path that
      exercises parallel independent branches through backend execution and
      consumer-visible events.
- [ ] Update touched README files, this plan, and the roadmap so status and
      ownership stay aligned with implementation reality.
- [ ] Record any workflows that intentionally remain sequential and why.

**Verification:**
- Targeted Rust crate tests for touched packages
- Repeated benchmark or harness runs to detect state leakage or unstable timing
- Documentation review against `DOCUMENTATION-STANDARDS.md`

**Status:** Not started

## Execution Notes

Update during implementation:
- 2026-04-17: Draft created after roadmap reconciliation and direct inspection
  of the current sequential `demand_multiple` implementation and the oversized
  surrounding `node-engine` modules.
- 2026-04-17: The plan now explicitly includes decomposition work because the
  immediate execution hot path already exceeds coding-standards thresholds.
- 2026-04-17: First Milestone 1 decomposition slice landed in
  `crates/node-engine/src/events/`, reducing one shared `node-engine` hot spot
  before dependency-planning and bounded-parallel coordinator work begins.
- 2026-04-17: Second Milestone 1 decomposition slice landed in
  `crates/node-engine/src/engine/`, extracting the current sequential
  multi-demand path and incremental graph-event helpers so the future bounded
  parallel coordinator has a focused insertion boundary.
- 2026-04-18: Third Milestone 1 decomposition slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, moving the
  `WorkflowExecutor::demand_multiple` graph-read, event-emit, and engine-lock
  choreography behind the dedicated multi-demand helper boundary so later
  bounded-parallel coordination does not need to be inserted back into
  `engine.rs`.
- 2026-04-18: Fourth Milestone 1 decomposition slice landed in
  `crates/node-engine/src/engine/dependency_inputs.rs`, extracting dependency
  output to node input mapping and Puma-Lib model-path context propagation out
  of `DemandEngine::demand_internal` so later planners and coordinators can
  consume a narrower execution core.
- 2026-04-18: Fifth Milestone 1 decomposition slice landed in
  `crates/node-engine/src/engine/single_demand.rs`, moving the
  `WorkflowExecutor::demand` graph-read and engine-lock choreography behind a
  focused helper so both single-demand and multi-demand facade entry points
  now live under the same internal engine boundary.
- 2026-04-18: Sixth Milestone 1 decomposition slice landed in
  `crates/node-engine/src/engine/node_preparation.rs`, extracting static node
  data injection and human-input pause detection out of
  `DemandEngine::demand_internal` so the remaining execution core can be
  narrowed further before bounded parallel coordination begins.
- 2026-04-18: Seventh Milestone 1 decomposition slice landed in
  `crates/node-engine/src/engine/output_cache.rs`, extracting fresh-cache
  resolution and completed-output cache/version finalization out of
  `DemandEngine::demand_internal` so the remaining recursive demand core owns
  less state-transition detail ahead of the coordinator work.

## Commit Cadence Notes

- Commit when a logical slice is complete and verified.
- Keep decomposition, semantic-freeze, coordinator, event/trace, and
  benchmark/doc slices separate where practical.
- Follow commit format/history cleanup rules from `COMMIT-STANDARDS.md`.

## Optional Subagent Assignment

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| None | None | None | Revisit only if implementation is split into disjoint engine and consumer-contract slices |

## Re-Plan Triggers

- Shared-dependency deduplication requires a different ownership model than the
  current per-run executor boundary assumes.
- Parallel execution reveals a missing event or trace contract that cannot be
  added append-only.
- Benchmarks show no measurable improvement or unacceptable runtime pressure
  under the bounded approach.
- Waiting-for-input or cancellation semantics become ambiguous under
  concurrency and require a separate ADR or upstream event-contract change.

## Recommendations (Only If Better Option Exists)

- Recommendation 1: Land the decomposition slice before the first concurrency
  slice. Parallel execution bugs are materially easier to reason about when the
  coordination logic is not buried in the current monolithic engine file.
- Recommendation 2: Start with conservative bounded parallelism and explicit
  deterministic grouping rules rather than trying to maximize throughput on the
  first implementation pass.

## Completion Summary

### Completed

- Milestone 1 decomposition has begun with focused no-behavior-change
  extraction slices in `crates/node-engine/src/events/` and
  `crates/node-engine/src/engine/`.
- Focused engine helpers now own the multi-demand facade choreography and
  dependency-input assembly that bounded parallel planning will need to reuse.
- The executor-facing single-demand path also now delegates through an engine
  helper, reducing the remaining inline hot-path logic in `engine.rs`.
- Node preparation for `_data` injection and `human-input` waiting semantics
  now also lives behind a focused helper instead of remaining embedded in the
  recursive demand path.
- Output-cache hit and completion-finalization rules now also live behind a
  focused helper instead of remaining embedded in the recursive demand path.

### Deviations

- None yet.

### Follow-Ups

- Continue Milestone 1 by extracting the remaining dependency-planning and
  target-resolution logic out of `engine.rs` before bounded parallel
  coordination lands.

### Verification Summary

- Reviewed `PLAN-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`,
  `CODING-STANDARDS.md`, `CONCURRENCY-STANDARDS.md`,
  `TESTING-STANDARDS.md`, `INTEROP-STANDARDS.md`,
  `DOCUMENTATION-STANDARDS.md`, and `DEPENDENCY-STANDARDS.md`
- Inspected current insertion points in `crates/node-engine` and the roadmap
  sections those changes will update

### Traceability Links

- Module README updated: N/A
- ADR added/updated: N/A
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A

## Brevity Note

Keep Phase 2 implementation slices small and measurable. Expand detail only
where concurrency, determinism, or ownership risk requires it.
