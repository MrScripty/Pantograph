# Plan: Pantograph Metrics And Trace Hardening

## Status
In progress

Last updated: 2026-04-18

## Current Source-of-Truth Summary

This document is the dedicated source of truth for the non-premature metrics and
trace hardening work identified after the metrics/trace spine audit. It expands
the roadmap follow-up into a standards-reviewed implementation plan for fixing
contract drift, ambiguous attribution, synthetic timing behavior, and immediate
standards deviations in the metrics/trace area before more concurrency and
workflow-event work lands on top of them.

This plan is intentionally narrower than a broad observability expansion. It
focuses on correctness, contract consistency, and immediate standards
compliance, not on durable telemetry, dashboards, or new metrics families.

## Objective

Harden Pantograph's backend-owned metrics and trace system so snapshot filter
semantics, queue timing, execution attribution, and runtime-diagnostics reads
remain correct under current behavior and upcoming parallel execution, while
refactoring the immediate insertion points to comply with the architecture,
coding, concurrency, testing, interop, documentation, and tooling standards.

## Scope

### In Scope

- Backend-owned workflow trace snapshot contract semantics in
  `crates/pantograph-workflow-service/src/trace`
- Scheduler-to-trace queue attribution and timing behavior in the backend trace
  layer
- Runtime and diagnostics aggregation behavior that currently reads metrics or
  traces through ambiguous session/workflow filters
- Tauri runtime-debug aggregation behavior in `src-tauri/src/llm/commands`
- Refactors required to keep the immediate touched metrics/trace insertion
  points standards-compliant before more logic lands there
- Contract, unit, integration, and documentation updates required to make the
  hardening authoritative and auditable

### Out of Scope

- Durable trace persistence or remote telemetry export
- New metrics families, new dashboards, or metrics-visualization product work
- Scheduler V2 policy redesign beyond the attribution/timing corrections needed
  to keep current metrics truthful
- Parallel demand execution implementation itself
- KV cache implementation
- Broad runtime-policy changes unrelated to metrics/trace correctness

## Inputs

### Problem

The metrics/trace audit identified a set of issues that should be hardened now
because later roadmap phases will depend on these contracts and semantics:

- The combined runtime debug endpoint accepts filters that are not applied
  consistently across diagnostics and trace reads.
- Runtime diagnostics currently select trace runtime metrics by taking the first
  matching trace for a session/workflow filter, which becomes ambiguous once
  multiple runs can exist in the same scope.
- Queue wait timing can be synthesized from snapshot capture time when true
  enqueue/dequeue timestamps are absent, which turns observation time into a
  misleading latency metric.
- Session-based fallback matching can attach queue or trace state to the wrong
  execution once concurrent or repeated runs exist in the same session.
- The immediate metrics/trace insertion points already cross decomposition
  thresholds from `CODING-STANDARDS.md`, so continuing without planned
  extraction would deepen the standards debt in the exact files being hardened.

If these issues are left in place, later work in parallel demand execution,
workflow event contract completion, and scheduler work will build on trace data
that can drift, flatten ambiguity into "first match" behavior, or report queue
latency values that are not actually measured.

### Constraints

- Canonical metrics and trace semantics must remain backend-owned in Rust.
- `src-tauri` may aggregate and transport diagnostics, but it must not become
  the owner of trace filter semantics, execution attribution rules, or timing
  policy.
- Any public contract changes must be additive where practical.
- The plan must harden correctness and immediate standards compliance without
  turning into a broad observability or telemetry program.
- Existing public facades should remain intact unless an explicit break is
  required and documented.
- Hardening must account for near-term concurrency growth from the parallel
  demand execution roadmap work.

### Public Facade Preservation Note

This is a facade-first plan. Existing trace store facades, diagnostics store
facades, and runtime-debug command surfaces remain the public entry points
unless an additive backend-owned contract change is required. The default
implementation choice is extraction and delegation behind the current public
surface, not API breakage.

### Assumptions

- The metrics/trace spine foundation is complete enough that this work is a
  hardening follow-up, not a rewrite of the entire trace system.
- `workflow_name` is a legitimate diagnostic filter only if it is backed by a
  canonical backend-owned trace contract. If that cannot be justified cleanly,
  the fallback is to remove adapter-local support rather than preserve drift.
- Phase 2 parallel demand execution will increase the likelihood of multiple
  traces per session/workflow, so ambiguous first-match behavior should be
  treated as a real near-term bug.
- The current Tauri diagnostics layer should remain projection-only rather than
  absorbing more backend logic.

### Dependencies

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `IMPLEMENTATION-PLAN-pantograph-metrics-trace-spine.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-2-parallel-demand-execution.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-5-real-workflow-event-contract.md`
- `crates/pantograph-workflow-service/src/trace`
- `src-tauri/src/workflow/diagnostics`
- `src-tauri/src/workflow/headless_diagnostics.rs`
- `src-tauri/src/llm/commands/registry.rs`
- Existing README files for the touched `src/` directories

### Affected Structured Contracts

- `WorkflowTraceSnapshotRequest`
- `WorkflowTraceSnapshotResponse`
- `WorkflowTraceQueueMetrics`
- `WorkflowTraceSummary`
- `WorkflowDiagnosticsSnapshotRequest`
- `RuntimeDebugSnapshotRequest`
- Any additive ambiguity or timing-diagnostic fields needed so consumers can
  distinguish "missing measurement" from measured values

### Affected Persisted Artifacts

- This implementation plan
- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `IMPLEMENTATION-PLAN-pantograph-metrics-trace-spine.md`
- Touched module `README.md` files under:
  - `crates/pantograph-workflow-service/src/trace/`
  - `src-tauri/src/workflow/diagnostics/`
  - `src-tauri/src/llm/`
- Any ADR required if the hardening reveals a new long-lived ownership boundary

### Existing Codebase Non-Compliance In Immediate Surroundings

The immediate hardening insertion points already exceeded the soft
decomposition thresholds from `CODING-STANDARDS.md`, which is why this plan
includes decomposition as implementation work instead of optional cleanup.
After the first Milestone 2 backend trace splits, the current surrounding sizes
are:

- `crates/pantograph-workflow-service/src/trace/store.rs` is approximately 342
  lines
- `crates/pantograph-workflow-service/src/trace/query.rs` is approximately 92
  lines
- `crates/pantograph-workflow-service/src/trace/state.rs` is approximately 298
  lines
- `src-tauri/src/workflow/diagnostics/store.rs` is approximately 785 lines
- `src-tauri/src/llm/commands/registry.rs` is approximately 815 lines

This plan therefore includes decomposition as part of the hardening work rather
than treating it as an optional cleanup task.

### Concurrency / Race-Risk Review

- Execution identity must remain the primary attribution key for traces,
  scheduler items, and runtime metrics.
- Session/workflow filters may legitimately match multiple executions once
  parallel demand execution lands, so read behavior must not rely on incidental
  ordering or "first trace wins" semantics.
- Queue timing must preserve the distinction between measured timestamps and
  snapshot-observed state.
- Any fallback matching retained for compatibility must be narrow, explicit, and
  tested so it cannot misattribute under overlapping runs.
- Hardening should avoid introducing new lock-heavy read paths or repeated
  snapshot churn that would turn inspection into a hidden execution hot path.

### Ownership And Lifecycle Note

- `crates/pantograph-workflow-service` owns canonical trace contracts, request
  validation, filter semantics, and scheduler/runtime attribution rules.
- `src-tauri/src/workflow/diagnostics` owns additive projection overlays only.
- `src-tauri/src/llm/commands` owns transport aggregation and command exposure,
  not trace contract meaning.
- Runtime debug reads remain request/response style command handlers; this plan
  does not introduce new polling loops, timers, or background ownership changes.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Hardening expands into a broad observability initiative | High | Keep scope limited to correctness gaps, immediate standards closure, and near-term roadmap dependency risks |
| A filter-contract fix breaks existing internal callers that depend on drift | High | Preserve facades, make contract changes additive where practical, and update all affected callers in the same slice |
| Ambiguity fixes reveal hidden assumptions in diagnostics consumers | High | Add cross-layer tests for multi-run session/workflow cases before changing selection behavior |
| Decomposition work obscures behavior changes or makes review harder | Medium | Split by responsibility boundary first and keep contract-preserving refactors separate from semantic changes where possible |
| Queue timing hardening removes values some internal consumers were reading | Medium | Replace synthetic timing with explicit absence or diagnostics so consumers can handle missing measurements intentionally |

## Standards Review Passes

### Pass 1: Plan Structure And Source-of-Truth

Reviewed against:
- `PLAN-STANDARDS.md`
- `templates/PLAN-TEMPLATE.md`

Corrections applied:
- Kept this as a dedicated follow-up plan instead of burying the work inside the
  roadmap or the already-completed metrics spine plan.
- Included required objective, scope, inputs, risks, milestones, verification,
  re-plan triggers, and completion criteria.
- Added concurrency and facade-preservation notes because the work affects
  async-adjacent attribution and contract reads across multiple layers.

### Pass 2: Architecture And Ownership

Reviewed against:
- `ARCHITECTURE-PATTERNS.md`
- `CODING-STANDARDS.md`

Corrections applied:
- Kept canonical trace and timing semantics in
  `crates/pantograph-workflow-service`.
- Restricted Tauri changes to projection and transport aggregation.
- Added decomposition milestones for the oversized trace, diagnostics, and
  runtime-debug command files before more hardening logic lands there.

### Pass 3: Interop And Contract Consistency

Reviewed against:
- `INTEROP-STANDARDS.md`
- `ARCHITECTURE-PATTERNS.md`

Corrections applied:
- Required one canonical filter story across trace, diagnostics, and runtime
  debug reads.
- Recorded that adapter-only filter semantics are not acceptable if the backend
  trace contract cannot express them.
- Required additive ambiguity/timing semantics when consumers need to
  distinguish "missing measurement" from "measured zero."

### Pass 4: Concurrency, Timing, And Parallel-Readiness

Reviewed against:
- `CONCURRENCY-STANDARDS.md`
- `TESTING-STANDARDS.md`

Corrections applied:
- Elevated session/workflow multi-run ambiguity to an explicit milestone target
  instead of leaving it as a future note.
- Required exact timestamp semantics for queue wait calculations.
- Added lock-ownership review for the touched trace and diagnostics stores so
  decomposition does not preserve avoidable synchronization debt in the same
  files.
- Added multi-run, replay, duplicate-event, and ambiguous-filter cases to the
  verification plan so Phase 2 can rely on these reads safely.

### Pass 5: Documentation, Tooling, And Traceability

Reviewed against:
- `DOCUMENTATION-STANDARDS.md`
- `TOOLING-STANDARDS.md`

Corrections applied:
- Included module README updates for all touched metrics/trace boundaries.
- Required the roadmap and prior metrics plan to be synchronized once the
  hardening work changes milestone truth or closes follow-up items.
- Kept validation scoped to targeted crates/modules rather than proposing broad
  repo-wide churn.

### Pass 6: Anti-Premature-Hardening Scope Check

Reviewed against:
- The audit findings and current roadmap dependency order

Corrections applied:
- Excluded durable persistence, telemetry export, dashboard work, and new metric
  families because they are not required to resolve the identified correctness
  issues.
- Kept the plan focused on filter consistency, attribution correctness, queue
  timing truthfulness, and immediate standards closure.
- Mapped every audit finding to at least one milestone below so the plan closes
  the identified gaps rather than only documenting them.

## Definition of Done

- Trace, diagnostics, and runtime-debug reads use one consistent backend-owned
  filter model.
- Session/workflow-scoped reads no longer rely on ambiguous first-match
  semantics when multiple runs may exist.
- Queue wait timing is only computed from actual measured timestamps or is
  explicitly absent/diagnosed as incomplete.
- Execution attribution no longer depends on broad session-id fallback matching
  that can attach state to the wrong run.
- Immediate touched metrics/trace files are decomposed enough to clear the
  current standards debt or to leave only justified, documented thresholds in
  untouched areas.
- Tests and module documentation cover the new semantics and preserve backend
  ownership boundaries.

## Milestones

### Milestone 1: Freeze Hardening Contracts And Boundaries

**Goal:** Define the target semantics up front so contract fixes and
decomposition do not drift across layers.

**Tasks:**
- [x] Freeze the canonical filter model across
      `WorkflowTraceSnapshotRequest`,
      `WorkflowDiagnosticsSnapshotRequest`, and
      `RuntimeDebugSnapshotRequest`.
- [x] Decide the `workflow_name` story explicitly:
      either add backend-owned trace support for it or remove adapter-local
      support from combined runtime-debug trace reads.
- [x] Freeze queue timing semantics so `queue_wait_ms` only represents measured
      values, never snapshot-capture fallbacks.
- [x] Freeze execution-attribution rules so `execution_id` remains authoritative
      and any compatibility fallback is narrow, documented, and testable.
- [x] Define how session/workflow-scoped runtime-metrics reads behave when
      multiple matching traces exist: resolved execution set, explicit
      ambiguity, or another backend-owned deterministic rule.
- [x] Record the planned extraction boundaries for
      `trace/store.rs`, `workflow/diagnostics/store.rs`, and
      `llm/commands/registry.rs`.

**Planned extraction boundaries:**
- `crates/pantograph-workflow-service/src/trace/store.rs`:
  keep `WorkflowTraceStore` as the facade in `store.rs`, extract request
  filtering and runtime-selection helpers into `trace/query.rs`, extract
  retained run-state mutation and restart/replay reconciliation into
  `trace/state.rs`, and keep event-application delegation in the existing
  `runtime.rs` and `scheduler.rs` modules.
- `src-tauri/src/workflow/diagnostics/store.rs`:
  keep `WorkflowDiagnosticsStore` as the projection facade in `store.rs`,
  extract overlay bookkeeping and retained-event pruning into
  `diagnostics/overlay.rs`, extract trace-attempt reconciliation into
  `diagnostics/attempts.rs`, and keep trace adaptation in `diagnostics/trace.rs`
  so projection assembly stops carrying overlay mutation details.
- `src-tauri/src/llm/commands/registry.rs`:
  keep command entrypoints in `registry.rs`, extract request normalization and
  validation into `llm/commands/registry/request.rs`, extract runtime-debug
  snapshot aggregation into `llm/commands/registry/debug.rs`, and extract the
  current test module into `llm/commands/registry/tests.rs` so transport wiring
  does not own the aggregation internals.

**Verification:**
- Contract and ownership review against `ARCHITECTURE-PATTERNS.md` and
  `INTEROP-STANDARDS.md`
- Plan-to-audit closure review confirming every audit finding is addressed by a
  milestone task
- Documentation traceability review against `DOCUMENTATION-STANDARDS.md`

**Status:** Completed

### Milestone 2: Decompose Immediate Insertion Points

**Goal:** Extract the touched oversized files into smaller ownership-focused
modules before semantic hardening deepens them.

**Tasks:**
- [x] Split `crates/pantograph-workflow-service/src/trace/store.rs` into focused
      modules for request/filter handling, run-state mutation/restart logic, and
      store facade behavior while preserving the public trace facade.
- [x] Split `src-tauri/src/workflow/diagnostics/store.rs` so overlay-state
      bookkeeping, trace-attempt resolution, and projection assembly no longer
      live in one monolithic store file.
- [ ] Split `src-tauri/src/llm/commands/registry.rs` so request normalization,
      runtime-debug aggregation, command wrappers, and tests are not coupled in
      one file.
- [ ] Review lock ownership and primitive choice in the touched trace and
      diagnostics stores, migrating to the repo-preferred synchronization model
      when the touched code can do so without broad unrelated churn.
- [ ] Keep existing public entry points stable and route behavior through the
      extracted modules instead of introducing adapter-local duplicates.

**Verification:**
- File-size and responsibility review against `CODING-STANDARDS.md`
- Targeted package tests for the touched modules after extraction
- Concurrency review against `CONCURRENCY-STANDARDS.md`, including lock scope
  and poisoning/cascade behavior in the touched stores
- README review to ensure extracted boundaries remain documented

**Status:** In progress

### Milestone 3: Harden Backend Trace Filter And Timing Semantics

**Goal:** Make the backend trace layer the single truthful owner of filter,
attribution, and queue timing semantics.

**Tasks:**
- [x] Implement the Milestone 1 filter decision in the backend trace request
      contract and validation path.
- [x] Normalize trace filter handling in one backend-owned place rather than
      letting adapters normalize one request shape and forward another.
- [x] Remove or sharply constrain `session_id`-based fallback matching that can
      alias an execution read to the wrong trace or queue item.
- [x] Change scheduler snapshot handling so missing `enqueued_at_ms` or
      `dequeued_at_ms` does not synthesize `queue_wait_ms` from
      `captured_at_ms`.
- [x] Add any additive diagnostics or selection fields needed to represent
      incomplete timing or unresolved attribution without fabricating data.
- [x] Keep replay, restart, and duplicate-event semantics intact while the trace
      matching and timing rules are tightened.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- Focused tests for filter normalization, workflow-name handling, queue timing
  absence semantics, multi-run session matching, and duplicate-event/restart
  behavior
- Concurrency review against `CONCURRENCY-STANDARDS.md`

**Status:** Completed

### Milestone 4: Harden Diagnostics And Runtime-Debug Aggregation

**Goal:** Make the Tauri-facing diagnostics and runtime-debug surfaces consume
the hardened backend semantics without reintroducing local ambiguity.

**Tasks:**
- [x] Update diagnostics and runtime-debug request handling to reuse the frozen
      backend-owned filter semantics from Milestone 1 and Milestone 3.
- [x] Remove first-match runtime-metrics selection for session/workflow-only
      reads when multiple traces can match.
- [ ] Make combined diagnostics-plus-trace runtime-debug responses operate on
      the same resolved execution criteria or return explicit ambiguity instead
      of silently merging mismatched slices.
- [x] Harden the selection helpers in
      `src-tauri/src/workflow/headless_diagnostics.rs` so runtime metric
      projection does not collapse multi-run scope into incidental ordering.
- [x] Preserve Tauri as a projection/transport boundary and keep any new
      selection logic backend-owned or explicitly delegated to a backend-owned
      helper.
- [ ] Add compatibility-preserving adapters only where required to keep existing
      command surfaces stable.

**Verification:**
- Targeted tests in `src-tauri` for whitespace normalization, `workflow_name`
  behavior, `include_trace` alignment, and ambiguous session/workflow reads
- Cross-layer checks covering backend trace read plus Tauri runtime-debug
  aggregation
- Integration-test isolation review so diagnostics/runtime-debug tests do not
  share mutable process-global or durable state
- Architecture review confirming no new backend-owned semantics moved into
  Tauri modules

**Status:** In progress

### Milestone 5: Documentation, Roadmap Reconciliation, And Close-Out

**Goal:** Leave the metrics/trace hardening work as the new authoritative basis
for later roadmap phases.

**Tasks:**
- [ ] Update touched module READMEs with the new filter, timing, attribution,
      and consumer-contract semantics.
- [ ] Reconcile
      `IMPLEMENTATION-PLAN-pantograph-metrics-trace-spine.md`
      and `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
      so the hardening follow-up is represented accurately.
- [ ] Confirm no out-of-scope observability expansion slipped into the
      implementation.
- [ ] Record any remaining deferred work explicitly, limited to issues that are
      truly downstream of later roadmap phases rather than gaps left behind by
      this hardening slice.

**Verification:**
- Documentation review against `DOCUMENTATION-STANDARDS.md`
- Focused verification summary referencing the milestone checks already run
- Source-of-truth review confirming roadmap and plan status agree

**Status:** Not started

## Execution Notes

Update during implementation:
- 2026-04-17: Drafted dedicated hardening plan from the metrics/trace audit.
- 2026-04-18: Began Milestone 1 by freezing the shared filter model, adding the
  backend-owned `workflow_name` trace filter, and aligning request trimming plus
  blank-filter rejection across trace, diagnostics, and runtime-debug reads.
- 2026-04-18: Completed the queue-timing semantics slice by removing
  `captured_at_ms` fallback synthesis from backend scheduler trace snapshots,
  documenting authoritative-only queue timing, and adding a regression test for
  missing queue timestamps.
- 2026-04-18: Completed the execution-attribution slice by removing session-id
  fallback queue-item matching from backend scheduler trace snapshots,
  documenting execution-first attribution, and adding a regression test that
  prevents another run in the same session from leaking queue metrics into the
  active trace.
- 2026-04-18: Completed the multi-run runtime-metrics slice by adding a
  backend-owned unique-match selection helper for trace runtime metrics,
  exposing matched execution ids for ambiguity, and updating Tauri diagnostics
  to refuse multi-run session/workflow first-match selection.
- 2026-04-18: Completed the Milestone 1 extraction-boundary recording by
  capturing explicit module targets for the oversized backend trace store,
  Tauri diagnostics store, and runtime-debug registry command file before
  decomposition begins.
- 2026-04-18: Began Milestone 2 by extracting backend trace query and runtime
  selection helpers out of `trace/store.rs` into `trace/query.rs` while
  preserving `WorkflowTraceStore` as the public facade.
- 2026-04-18: Continued Milestone 2 by extracting backend trace run-state
  creation and event-application helpers out of `trace/store.rs` into
  `trace/state.rs` while preserving existing trace event semantics.
- 2026-04-18: Reconciled the hardening plan and roadmap after the queue-timing,
  execution-attribution, unique-match runtime selection, and backend trace
  decomposition slices so Milestones 2 through 4 reflect current repo state.
- 2026-04-18: Continued Milestone 2 by extracting Tauri diagnostics overlay
  state, pruning, and event-to-overlay mutation helpers out of
  `workflow/diagnostics/store.rs` into `workflow/diagnostics/overlay.rs` while
  keeping trace-attempt coordination in the facade for the next slice.
- 2026-04-18: Continued Milestone 2 by extracting Tauri diagnostics
  trace-attempt lookup, execution-id reconciliation, and overlay reset/record
  decisions out of `workflow/diagnostics/store.rs` into
  `workflow/diagnostics/attempts.rs` so the facade only orchestrates backend
  trace snapshots and overlay application.

## Commit Cadence Notes

- Commit when a logical hardening slice is complete and verified.
- Keep extraction-only commits separate from semantic hardening commits where
  that separation improves reviewability.
- Follow `COMMIT-STANDARDS.md` for format and cleanup.

## Re-Plan Triggers

- The chosen unified filter model would require an API break larger than an
  additive contract change.
- A diagnostics consumer relies on synthetic queue timing or ambiguous
  first-match behavior in a way that materially changes rollout sequencing.
- Decomposition reveals an additional backend-owned boundary that should move
  out of Tauri rather than stay in the existing modules.
- Parallel demand execution lands first and changes the minimum acceptable
  multi-run semantics before this hardening work is finished.

## Recommendations

- Recommendation 1: implement Milestone 1 before any more Phase 2 or Phase 5
  work that consumes session/workflow-scoped trace reads, because the current
  ambiguity and filter drift will otherwise spread into new code paths.
- Recommendation 2: keep decomposition coupled to the touched hardening slices
  rather than running a generic cleanup pass, because that resolves the current
  standards debt without turning this into a broad refactor program.

## Completion Criteria

- All audit findings listed in this plan are resolved or explicitly reclassified
  as downstream-only follow-up with justification.
- The touched metrics/trace boundaries comply with the standards closely enough
  that no immediate oversized-file or ownership drift remains in the modified
  paths.
- Later roadmap work can depend on metrics/trace reads without inheriting known
  filter, attribution, or synthetic-timing bugs.

## Completion Summary

### Completed

- None yet.

### Deviations

- None yet.

### Follow-Ups

- None yet. Populate only if a re-plan trigger is hit or a downstream-only item
  remains after implementation.

### Verification Summary

- None yet.

### Traceability Links

- Module README updated:
  `crates/pantograph-workflow-service/src/trace/README.md`
- Module README updated:
  `src-tauri/src/workflow/diagnostics/README.md`
- Module README updated:
  `src-tauri/src/llm/README.md`
- ADR added/updated: N/A unless implementation reveals a new ownership boundary
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`
