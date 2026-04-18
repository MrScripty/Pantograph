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

### Pass 6: Execution-Core Architecture Fit

Reviewed against:
- `ARCHITECTURE-PATTERNS.md`
- `CODING-STANDARDS.md`

Corrections applied:
- Recorded an explicit Option 2 refactor target: introduce a small internal
  execution-core type under `crates/node-engine/src/engine/` rather than
  continuing to chip away at `DemandEngine::demand_internal` only with
  isolated free helpers.
- Kept the execution core internal to `crates/node-engine` so the public
  `WorkflowExecutor` facade and downstream package boundaries remain
  unchanged.
- Limited the planned touched-area compliance scope to the immediate
  `crates/node-engine/src/engine/` modules and any directly interacting
  readmes/tests; adjacent oversized files are not to be expanded casually
  under this slice.

### Pass 7: Execution-Core Concurrency And Ownership

Reviewed against:
- `CONCURRENCY-STANDARDS.md`
- `CODING-STANDARDS.md`

Corrections applied:
- Required the execution core to own per-call coordination state explicitly,
  rather than distributing lifecycle transitions across `DemandEngine`,
  helper modules, and ad hoc local variables.
- Recorded that lock acquisition order, in-flight tracking, cache state, and
  cleanup semantics must be explicit in the execution-core design before
  bounded parallelism is introduced.
- Prohibited introducing new shared mutable globals or expanding lock scope
  across `.await` points as part of the execution-core refactor.

### Pass 8: Execution-Core Testing And Contract Stability

Reviewed against:
- `TESTING-STANDARDS.md`
- `INTEROP-STANDARDS.md`

Corrections applied:
- Required the execution-core refactor to remain no-behavior-change at the
  public contract level, with existing demand, waiting, caching, and event
  tests kept green while focused execution-core tests are added.
- Recorded that any new internal execution-core APIs remain private to the
  crate; contract freeze still applies to public workflow and event surfaces.
- Required representative recursive-demand tests to continue covering cache
  hits, diamond dependency reuse, waiting-for-input, and event emission after
  the refactor.

### Pass 9: Execution-Core Documentation And Traceability

Reviewed against:
- `DOCUMENTATION-STANDARDS.md`
- `PLAN-STANDARDS.md`

Corrections applied:
- Added a dedicated execution-core subsection to this plan so the Option 2
  refactor is recorded as an explicit source-of-truth decision instead of an
  implicit implementation preference.
- Required `crates/node-engine/src/engine/README.md` to be updated in the same
  logical slice that introduces the execution core, including ownership and
  invariant changes for touched helpers.
- Recorded that the roadmap should summarize the execution-core decision
  briefly rather than duplicating this subsection in detail.

### Pass 10: Security Boundary Applicability

Reviewed against:
- `SECURITY-STANDARDS.md`

Corrections applied:
- Recorded this refactor as an internal execution-path change rather than a new
  external-input boundary; therefore no new validation surface should be
  introduced casually as part of the execution-core extraction.
- Added the requirement that any new execution-core constructor or helper
  continues to trust already-validated internal inputs and does not duplicate
  validation logic inline.
- Constrained the touched-area scope so this refactor does not quietly absorb
  unrelated path/input validation work that belongs at real system boundaries.

### Pass 11: Cross-Platform Applicability

Reviewed against:
- `CROSS-PLATFORM-STANDARDS.md`

Corrections applied:
- Recorded that the planned execution-core refactor must remain platform-neutral
  and must not introduce inline OS branching into `node-engine` business logic.
- Added the requirement that any future platform-specific concurrency or runtime
  behavior needed for parallel demand remains isolated behind thin platform
  modules or existing runtime abstractions, not the execution core.
- Marked cross-platform impact as structural-only for this slice: the internal
  module split must continue to compile on the currently supported targets
  without adding new platform divergence.

### Pass 12: Release And Versioning Applicability

Reviewed against:
- `RELEASE-STANDARDS.md`

Corrections applied:
- Recorded this execution-core change as an internal refactor with no intended
  public API or artifact change; release planning must therefore continue to
  treat Milestone 1 as behavior-preserving unless a later milestone proves
  otherwise.
- Added the requirement that if the execution-core refactor reveals a public
  contract change after all, that change must trigger re-planning and explicit
  release-surface documentation instead of being folded silently into the
  refactor.
- Kept changelog and release concerns out of the current milestone until a
  user-visible behavior or public API difference is actually introduced.

### Pass 13: Language Bindings Applicability

Reviewed against:
- `LANGUAGE-BINDINGS-STANDARDS.md`

Corrections applied:
- Recorded that the execution-core type must remain a private backend-owned
  implementation detail inside `crates/node-engine`, not a new FFI-facing layer
  or binding surface.
- Added the requirement that any helper signatures or internal ownership
  changes in this slice must preserve the existing semantics already consumed by
  UniFFI, Rustler, and other bindings through the current backend contracts.
- Marked binding-surface impact as indirect-only for this milestone: if a later
  parallel-demand milestone requires additive event or execution metadata, that
  will need a separate same-slice binding review.

### Pass 14: Frontend And Accessibility Applicability

Reviewed against:
- `FRONTEND-STANDARDS.md`
- `ACCESSIBILITY-STANDARDS.md`

Corrections applied:
- Recorded these standards as not directly applicable to the execution-core
  refactor because the touched area is backend Rust under `crates/node-engine`.
- Added a constraint that the current milestone must not spill into frontend or
  transport-owned UI synchronization work while decomposing the backend engine.
- Preserved the requirement that any future user-visible event-shape change
  still needs same-slice frontend and accessibility review when it crosses a UI
  boundary, but not during this internal refactor.

### Pass 15: Launcher Workflow Applicability

Reviewed against:
- `LAUNCHER-STANDARDS.md`

Corrections applied:
- Recorded launcher standards as indirectly applicable only through future
  verification workflow exposure, not through the internal execution-core
  design itself.
- Added the constraint that no launcher/workflow command changes are part of
  this refactor unless a later milestone adds a canonical perf or benchmark
  entrypoint that must be surfaced intentionally.
- Kept the current verification plan scoped to direct Rust test commands rather
  than inventing launcher changes prematurely.

### Pass 16: Commit And History Applicability

Reviewed against:
- `COMMIT-STANDARDS.md`

Corrections applied:
- Recorded that the execution-core work must continue landing as small
  no-behavior-change atomic refactor slices with matching doc updates in the
  same commits.
- Added the explicit expectation that regression/fix pairs discovered during
  this refactor are cleaned before more slices accumulate.
- Kept the milestone plan aligned with the existing commit cadence note so the
  execution-core work remains auditable and standards-compliant in history as
  well as in code structure.

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
- [x] Extract dependency-planning and target-resolution helpers from
      `crates/node-engine/src/engine.rs` into focused modules.
- [x] Introduce a small internal execution-core type under
      `crates/node-engine/src/engine/` that owns the remaining recursive
      demand flow before bounded parallel coordination lands.
- [x] Extract orchestration/event helper logic from oversized engine-adjacent
      files where needed so the parallel coordinator has a clear insertion
      boundary.
- [x] Update touched `README.md` files if the directory boundary or ownership
      explanation changes.
- [x] Keep the existing `WorkflowExecutor` facade unchanged while moving logic
      behind smaller internal modules.

**Verification:**
- `cargo check -p node-engine`
- Focused no-behavior-change tests for extracted helpers where practical
- Focused recursive-demand regression tests covering cache reuse, diamond
  dependency reuse, waiting-for-input, and demand event emission
- Documentation review against `DOCUMENTATION-STANDARDS.md`

**Status:** Complete

#### Option 2 Execution-Core Refactor Plan

**Objective:** Replace the remaining monolithic recursive execution body in
`DemandEngine::demand_internal` with a compact internal execution-core type
that owns one node-demand operation end to end without changing public
behavior.

**Touched-area compliance scope:**
- `crates/node-engine/src/engine.rs`
- `crates/node-engine/src/engine/README.md`
- New internal `crates/node-engine/src/engine/*` execution-core module(s)
- Existing helper modules directly consumed by the execution core
- Directly related `node-engine` tests that prove no public behavior change

**Planned internal shape:**
- Introduce a private execution-core struct, tentatively
  `DemandExecutionCore<'a>`, under `crates/node-engine/src/engine/`.
- The core should receive references to the existing backend-owned state it
  coordinates:
  `DemandEngine`, `WorkflowGraph`, `TaskExecutor`, `Context`, `EventSink`,
  `ExecutorExtensions`, and the recursive `computing` set.
- `DemandEngine::demand_internal` should become a thin delegator that creates
  the execution core and awaits one method such as `run_node(node_id)`.
- Existing extracted helpers remain narrow collaborators of the execution core
  instead of being re-inlined into `engine.rs`.

**Execution-core responsibilities:**
- Dependency recursion and dependency-output collection
- Input version computation and cache reuse
- Node preparation and waiting-for-input handling
- Demand event emission
- Completed-output cache/version finalization
- In-flight bookkeeping begin/finish semantics

**Execution-core responsibilities that remain out of scope:**
- Public facade/API changes
- Parallel scheduling policy
- Cross-run coordination
- New event contracts or transport behavior
- Orchestration package redesign outside files directly touched by this slice

**Standards-driven refactor requirements for touched areas:**
- `engine.rs` must continue shrinking in responsibility; the execution-core
  slice must remove orchestration detail from the file rather than merely move
  code around without changing ownership shape.
- The new execution-core module must have one clear responsibility: own
  recursive node-demand orchestration for one run path.
- Helper/module boundaries inside `crates/node-engine/src/engine/` must remain
  coherent: execution core coordinates, helpers perform narrow work.
- No touched README may become stale relative to helper ownership.
- No touched test should be weakened; new tests should pin the execution-core
  invariants if they are not already covered.
- No new external-input validation, platform branching, binding-facing surface,
  frontend/state ownership, launcher workflow, or release-surface changes may
  be folded into this refactor without triggering re-planning.

**Ordered tasks:**
1. Define the internal execution-core struct and its constructor inputs,
   keeping all dependencies backend-owned and private to `node-engine`.
2. Move the remaining recursive orchestration from
   `DemandEngine::demand_internal` into one execution-core method while
   preserving existing helper calls and early-return semantics.
3. Refactor any touched helper signatures only where needed to make ownership
   clearer; avoid expanding scope into unrelated modules.
4. Add or adjust focused tests for execution-core-owned invariants if current
   tests do not already pin them directly.
5. Update `crates/node-engine/src/engine/README.md`, this plan, and the
   roadmap summary in the same logical slice.

**Verification for this option:**
- `cargo check -p node-engine`
- `cargo test -p node-engine test_demand_caching`
- `cargo test -p node-engine test_demand_diamond_graph`
- `cargo test -p node-engine test_demand_events`
- `cargo test -p node-engine test_workflow_executor_human_input_emits_waiting_for_input`
- `cargo test -p node-engine test_workflow_executor_human_input_continues_with_response`
- Additional focused tests for any new private execution-core invariants added
  during the refactor

**Re-plan triggers for this option:**
- The execution core needs new public API surface instead of remaining
  internal.
- The refactor reveals that helper sprawl cannot be controlled without
  touching additional oversized files outside the declared touched-area scope.
- The execution core cannot own per-call state cleanly without introducing
  broader concurrency or contract changes that belong in Milestone 2 instead.
- File ownership or responsibility drift would make the touched area less
  standards-compliant than before the refactor.

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

**Status:** In progress

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

**Status:** In progress

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
- 2026-04-18: Eighth Milestone 1 decomposition slice landed in
  `crates/node-engine/src/engine/execution_events.rs`, extracting task-start,
  waiting-for-input, and task-completed event emission out of
  `DemandEngine::demand_internal` so backend-owned demand event choreography
  now lives behind a focused helper before bounded parallel coordination work.
- 2026-04-18: Ninth Milestone 1 decomposition slice landed in
  `crates/node-engine/src/engine/inflight_tracking.rs`, extracting recursive
  in-flight node bookkeeping and cycle-detection cleanup out of
  `DemandEngine::demand_internal` so the remaining recursive core owns less
  coordination-state detail before bounded parallel coordination work.
- 2026-04-18: Tenth Milestone 1 decomposition slice landed in
  `crates/node-engine/src/engine/execution_core.rs`, moving the remaining
  recursive node-demand orchestration behind a private execution-core owner so
  `DemandEngine::demand_internal` is now a thin delegator and Milestone 1's
  decomposition boundary is complete.
- 2026-04-18: First Milestone 2 semantic-freeze slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, introducing a private
  multi-demand plan that preserves caller-visible requested-target order
  separately from the current sequential execution-target list so later bounded
  scheduling can change internal coordination without re-entangling the facade
  event payload contract.
- 2026-04-18: Second Milestone 2 semantic-freeze slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, introducing a private
  multi-demand result collector that freezes deterministic last-write-wins
  result-map merge semantics separately from the execution loop that currently
  populates it.
- 2026-04-18: First Milestone 3 coordinator-prep slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, introducing a private
  multi-demand coordinator owner that still runs sequentially but now owns the
  execution loop the future bounded scheduler will replace.
- 2026-04-18: Third Milestone 2 semantic-freeze slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, introducing a private
  execution-budget contract with a current default of one in-flight target so
  later bounded scheduling has an explicit budget owner before additive runtime
  configuration is introduced.
- 2026-04-18: Fourth Milestone 2 semantic-freeze slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, teaching the private
  multi-demand plan to separate requested targets from minimal root execution
  targets so redundant top-level requests covered by other requested
  dependents are pruned before coordinator execution begins.
- 2026-04-18: Second Milestone 3 coordinator-prep slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, replacing the flat root
  execution-target list with an explicit private execution-batch schedule so
  the coordinator now runs against a schedule shape that later bounded
  scheduling can widen instead of a bare vector.
- 2026-04-18: Fifth Milestone 2 semantic-freeze slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, teaching the private batch
  planner to group independent root targets into the same batch while
  separating roots whose transitive dependency closures overlap, freezing the
  current parallel-eligibility rule before real concurrent execution lands.
- 2026-04-18: Third Milestone 3 coordinator-prep slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, introducing an explicit
  private batch-execution outcome owner and pinning stop-after-failed-batch
  behavior so later concurrent execution must preserve cleanup semantics
  instead of relying on incidental loop short-circuiting.
- 2026-04-18: Sixth Milestone 2 semantic-freeze slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, making waiting-for-input
  interruption an explicit private batch-execution result so later bounded
  execution must stop later batches on interactive pause just as it does on
  terminal failure.
- 2026-04-18: Fourth Milestone 3 coordinator-prep slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, introducing a private
  batch-dispatch plan that applies the explicit execution budget to each
  overlap-safe batch so the coordinator now owns budget-window dispatch even
  before real concurrent execution is enabled.
- 2026-04-18: Fifth Milestone 3 coordinator-prep slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, introducing explicit
  per-window completion and interruption outcomes so later concurrent window
  execution can preserve the same backend-owned stop semantics currently
  enforced sequentially.
- 2026-04-18: Sixth Milestone 3 coordinator-prep slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, moving direct
  `DemandEngine` demand/cache access behind a private window runner so later
  concurrent window execution can change engine ownership without reopening
  batch planning and outcome contracts.
- 2026-04-18: Seventh Milestone 2 semantic-freeze slice landed in
  `crates/node-engine/src/engine/multi_demand.rs`, replacing raw dispatch
  window vectors with explicit window plans that mark whether each window is
  eligible for bounded concurrent execution under the current batch and budget
  rules.
- 2026-04-18: The plan now also records explicit applicability passes for the
  remaining standards files in the coding-standards repo, including which
  standards are directly constraining this backend refactor and which are
  currently out of direct scope but still impose no-spillover constraints.

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

- Milestone 1 decomposition is complete with focused no-behavior-change
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
- Demand event emission for started, waiting, and completed states now also
  lives behind a focused helper instead of remaining embedded in the recursive
  demand path.
- In-flight cycle detection and cleanup now also live behind a focused helper
  instead of remaining embedded in the recursive demand path.
- The remaining recursive demand orchestration now also lives behind a private
  execution-core owner, leaving `DemandEngine::demand_internal` as a thin
  delegator instead of the last monolithic recursive execution body.
- Multi-demand target requests are now normalized behind a private plan object
  that keeps caller-visible requested-target order separate from the current
  sequential execution-target list ahead of bounded parallel coordination.
- Multi-demand result aggregation now also lives behind a private collector so
  deterministic merge semantics are explicit before concurrent completion paths
  are introduced.
- The current sequential multi-demand traversal now also runs behind a private
  coordinator owner so later bounded scheduling can change one internal owner
  instead of reopening facade orchestration.
- Multi-demand execution budget semantics are now also explicit behind a
  private contract with a current default of one in-flight target.
- Multi-demand planning now also separates caller-visible requested targets
  from minimal root execution targets so later bounded scheduling can start
  from the smallest required top-level drive set.
- The coordinator now also consumes an explicit execution-batch schedule, even
  though the current schedule still contains only one sequential batch.
- The execution-batch schedule now reflects dependency-overlap eligibility:
  independent roots may share a batch, while roots with shared transitive
  dependencies are separated into deterministic sequential batches.
- Batch execution cleanup semantics are now explicit: the current coordinator
  stops after the first failed batch and does not continue into later batches.
- Batch interruption semantics now also explicitly cover waiting-for-input:
  the current coordinator stops later batches when an earlier batch pauses for
  interactive input.
- The coordinator now also applies the explicit execution budget through a
  private batch-dispatch plan, even though each dispatch window still executes
  sequentially for now.
- The coordinator now also owns explicit per-window completion and
  interruption outcomes rather than only batch-level aggregation.
- Direct engine access now also lives behind a private window runner rather
  than being spread across coordinator methods.
- Dispatch windows now also carry an explicit parallel-eligibility decision so
  future concurrent execution branches from a frozen backend-owned contract
  instead of recomputing eligibility ad hoc.

### Deviations

- None yet.

### Follow-Ups

- Continue with Milestone 2 semantic-freeze work so bounded parallel
  coordination can land on top of the completed Milestone 1 execution-core and
  helper boundaries without reopening the public contract surface.

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
