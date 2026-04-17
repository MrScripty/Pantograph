# Plan: Pantograph Scheduler V2

## Status
Active

Last updated: 2026-04-16

## Current Source-of-Truth Summary

This document is the dedicated source of truth for Scheduler V2. It expands
the short Phase 4 section in
`ROADMAP-pantograph-workflow-graph-scheduling-runtime.md` into a
standards-reviewed implementation plan for runtime-aware workflow scheduling,
queue fairness, warm-runtime reuse, and scheduler diagnostics.

Scheduler V2 planning and status should now be updated here first. The roadmap
remains the cross-target summary, while milestone sequencing, immediate
compliance refactors, and acceptance gates for scheduler work are tracked in
this dedicated plan.

The accurate implementation baseline at the current checkpoint is:

- `crates/pantograph-workflow-service/src/scheduler/` now exists as the
  dedicated backend-owned scheduler module boundary, with scheduler-facing DTOs
  and the in-memory workflow-session queue/store extracted out of
  `workflow.rs`
- the initial explicit queue policy object now also exists in
  `crates/pantograph-workflow-service/src/scheduler/policy.rs`, freezing the
  current priority-then-FIFO behavior behind a backend-owned policy boundary
- `WorkflowSessionQueueItem` now carries an additive machine-consumable
  `scheduler_decision_reason`, and the backend trace layer prefers that
  scheduler-owned reason when it is present
- `WorkflowSessionQueueItem` now also carries additive canonical
  `queue_position` diagnostics, so scheduler snapshots expose backend-owned
  ordering facts instead of forcing adapters or trace readers to infer them
- the explicit scheduler policy now also owns the first backend starvation-
  protection rule, allowing long-waiting queued runs to accumulate canonical
  promotion credit and surface `starvation_protection` when they legitimately
  overtake newer higher-priority work
- `crates/pantograph-workflow-service/src/workflow.rs` still owns the current
  workflow-service facade, runtime orchestration, and session command entry
  points, but it no longer has to be the long-term home for scheduler DTOs and
  queue/store mutation logic
- current queue selection is intentionally simple: priority ordering with FIFO
  tie-breaks, one active run per session, and immediate idle unload when
  `keep_alive` is disabled
- `crates/pantograph-workflow-service/src/graph/session.rs` still owns the
  edit-session scheduler snapshot path and therefore must stay aligned with
  workflow-session scheduler contracts
- `crates/pantograph-workflow-service/src/trace/store.rs` and
  `src/trace/scheduler.rs` already expose additive scheduler trace data, but
  full Scheduler V2 policy observability is not complete yet
- runtime-registry Milestones 5 and 6 are complete and should be treated as
  frozen boundaries that Scheduler V2 consumes rather than reopens
- the immediate insertion area is already partially non-compliant with the
  coding standards because the files that currently hold queue and scheduler
  behavior are above decomposition thresholds

## Objective

Implement a backend-owned Scheduler V2 that replaces the current simple queue
and keep-alive policy with explicit scheduler policy objects, fair and
observable admission decisions, runtime-affinity and warm-session reuse, and
machine-consumable diagnostics, while refactoring the immediate workflow
service surroundings into standards-compliant module boundaries before more
policy is added.

## Scope

### In Scope

- scheduler policy extraction and refactor work required to keep the immediate
  workflow-service insertion area standards compliant
- backend-owned queue admission, fairness, starvation protection, runtime
  affinity, and warm-session reuse policy
- scheduler decision vocabulary, ETA diagnostics, and stable machine-consumable
  scheduler error payloads
- runtime-aware rebalance behavior when loaded-session capacity is exhausted
- workflow-session and edit-session scheduler snapshot alignment where the
  contracts overlap
- trace, diagnostics, README, roadmap, and source-of-truth updates required by
  the touched scheduler boundaries
- additive binding and transport updates only where new backend-owned
  scheduler DTO fields must cross Tauri or other wrappers

### Out of Scope

- distributed or multi-host scheduling
- frontend-owned scheduler policy, TypeScript-side queue logic, or optimistic
  scheduler state reconstruction
- reopening runtime-registry ownership boundaries already closed in completed
  Milestones 5 and 6
- KV cache policy, parallel node execution, or incremental graph invalidation
  beyond scheduler-facing compatibility points
- persistence or external storage for queue state unless a later re-plan
  explicitly approves it

## Inputs

### Problem

Pantograph's current scheduler behavior is intentionally conservative and is
embedded directly in large workflow-service modules. That was acceptable for
simple queueing and keep-alive behavior, but it is not a compliant or safe
place to add the next layer of runtime-aware policy. Scheduler V2 needs to add
explicit admission policy, fairness, runtime reuse, rebalance behavior, ETA
diagnostics, and stable machine-consumable reasons, yet the immediate
surroundings already violate decomposition expectations and would regress
further if new policy were appended in place.

Without a dedicated Scheduler V2 plan, implementation would likely:

- append more queue and runtime policy directly into `workflow.rs`
- let edit-session and workflow-session scheduler contracts drift
- push scheduler reasoning into Tauri or diagnostics wrappers
- ship new scheduler payloads without enough traceability to explain policy
  decisions or capacity outcomes

### Constraints

- Core scheduler policy remains backend-owned in Rust service/runtime crates;
  Tauri and any bindings remain transport-only.
- `pantograph-workflow-service` remains the owner of canonical workflow queue,
  session, and scheduler facts.
- `pantograph-runtime-registry` remains the owner of runtime-registry facts,
  retention/reclaim policy, and backend-owned runtime snapshots consumed by
  the scheduler.
- Public workflow-service facades should remain additive unless a documented
  break is explicitly approved.
- Scheduler V2 must not assume distributed coordination or durable queue state.
- Any new source directory created during the refactor must include a
  standards-compliant `README.md`.
- New scheduler diagnostics must be machine-consumable and must not rely on
  adapter-local guesses when backend state is ambiguous.

### Public Facade Preservation Note

Scheduler V2 is a facade-first refactor. The default implementation path is
internal extraction and delegation behind the existing workflow-service public
surface. New scheduler behavior should appear as additive contract fields and
clearer decision semantics rather than broad public API breakage.

### Assumptions

- Runtime-registry Milestones 5 and 6 remain the frozen boundary for runtime
  identity, reclaim, recovery, and diagnostics transport.
- Scheduler V2 should consume the metrics/trace spine rather than invent a
  second observability path.
- Existing workflow-session queue commands and scheduler snapshot transport are
  the baseline contracts that Scheduler V2 will extend rather than replace.
- No new datastore is required for the first Scheduler V2 implementation
  slices; in-memory ownership remains acceptable unless new requirements prove
  otherwise.
- The scheduler should continue to treat edit sessions and workflow sessions as
  distinct domains while keeping any shared scheduler-facing DTO semantics
  aligned.

### Dependencies

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `IMPLEMENTATION-PLAN-pantograph-metrics-trace-spine.md`
- `IMPLEMENTATION-PLAN-pantograph-runtime-registry-technical-fit-selection.md`
- `IMPLEMENTATION-PLAN-pantograph-milestone-5-workflow-adapter-integration.md`
- `IMPLEMENTATION-PLAN-pantograph-milestone-6-diagnostics-documentation-rollout-safety.md`
- `crates/pantograph-workflow-service`
- `crates/pantograph-runtime-registry`
- `crates/pantograph-embedded-runtime`
- `src-tauri/src/workflow`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- standards in
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`

### Affected Structured Contracts

- `WorkflowSessionSummary`
- `WorkflowSessionQueueItemStatus`
- `WorkflowSessionQueueItem`
- `WorkflowSessionQueueListRequest` and `WorkflowSessionQueueListResponse`
- `WorkflowSchedulerSnapshotRequest` and `WorkflowSchedulerSnapshotResponse`
- `WorkflowSessionQueueCancelRequest` and `WorkflowSessionQueueCancelResponse`
- `WorkflowSessionQueueReprioritizeRequest` and
  `WorkflowSessionQueueReprioritizeResponse`
- `WorkflowSessionKeepAliveRequest` and `WorkflowSessionKeepAliveResponse`
- `WorkflowErrorCode` scheduler-facing variants
- additive scheduler diagnostics and trace DTOs for ETA, decision reasons,
  admission outcomes, runtime-affinity basis, and rebalance visibility
- any Tauri or binding transport wrappers that forward those backend-owned
  DTOs without becoming policy owners

### Affected Persisted Artifacts

- this dedicated Scheduler V2 plan
- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- README files added or updated in touched scheduler-related source
  directories
- ADR updates only if the implementation materially changes an accepted
  architecture boundary
- any checked-in fixtures, snapshots, or examples added to document scheduler
  payloads or queue diagnostics

### Existing Codebase Non-Compliance In Immediate Surroundings

The immediate Scheduler V2 insertion area already exceeds coding-standards
decomposition thresholds and should not absorb more behavior without planned
refactor:

- `crates/pantograph-workflow-service/src/workflow.rs` is approximately 6447
  lines and still owns queue state, keep-alive behavior, scheduler DTOs, and
  admission logic together
- `crates/pantograph-workflow-service/src/graph/session.rs` is approximately
  1029 lines and still contains edit-session scheduler snapshot behavior that
  must stay aligned with the canonical scheduler contracts
- `crates/pantograph-workflow-service/src/trace/store.rs` is approximately 582
  lines and is already carrying trace-side scheduler projection work
- current scheduler policy is encoded in store methods inside `workflow.rs`
  rather than behind a dedicated scheduler module boundary

Scheduler V2 must therefore include compliance refactors before or alongside
the first policy expansion:

- extract scheduler state, DTO helpers, and policy into a dedicated
  `crates/pantograph-workflow-service/src/scheduler/` module tree or an
  equivalent focused boundary
- keep `workflow.rs` as a facade and orchestration entry point rather than the
  long-term owner of every scheduler concern
- extract any new edit-session scheduler projection helpers out of
  `graph/session.rs` if additional policy logic would otherwise deepen that
  file
- add `README.md` files for any new source directories created during the
  extraction

### Concurrency / Race-Risk Review

- Queue admission, reprioritize, cancel, keep-alive changes, stale-session
  cleanup, runtime reclaim, recovery, and scheduler snapshot reads can all
  overlap.
- Scheduler V2 must preserve a single owner for mutable queue and admission
  state; policy decisions cannot be split across workflow service, runtime
  registry, and adapter-local caches.
- If background queue advancement or cleanup workers evolve as part of
  Scheduler V2, the plan must state who starts them, who stops them, and how
  duplicate admission or stale unload actions are prevented.
- Runtime-affinity and reuse decisions must be derived from synchronized
  backend-owned runtime facts and session state, not host-local guesses.
- ETA and decision-reason diagnostics must describe real canonical state and
  must remain deterministic under concurrent queue movement.

### Ownership And Lifecycle Note

- `pantograph-workflow-service` owns canonical workflow queue state, scheduler
  policy application, admission ordering, and scheduler-facing DTO semantics.
- `pantograph-runtime-registry` owns runtime-registry state, reclaim/retention
  facts, and runtime observations that the scheduler consumes.
- `pantograph-embedded-runtime` owns producer-aware execution helpers and
  runtime translation needed to turn scheduler decisions into runtime actions.
- `src-tauri` remains the composition root and transport host; it may start or
  stop backend-owned workers but must not become the owner of scheduler
  policy, queue truth, or runtime-affinity logic.
- Bindings remain transport wrappers over backend-owned scheduler contracts.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Scheduler policy is appended directly into `workflow.rs` again | High | Make boundary extraction a required early milestone before substantial policy work lands |
| Edit-session and workflow-session scheduler payloads drift | High | Keep shared DTO semantics backend-owned and add explicit contract-alignment tasks and tests |
| Adapters start reconstructing ETA, affinity, or decision reasons | High | Restrict adapters to forwarding backend-owned DTOs and validate one end-to-end path |
| Fairness or rebalance logic introduces races or duplicate dequeues | High | Keep one mutable-state owner, document lifecycle/worker ownership, and add concurrency-oriented tests |
| Runtime-registry completed boundaries are reopened by scheduler work | Medium | Treat runtime-registry output as an input dependency and isolate new logic to scheduler policy consumers |
| New scheduler payload fields drift across bindings | Medium | Update wrappers together and keep contract changes additive and explicitly documented |

## Standards Review Passes

### Pass 1: Plan Structure And Source-of-Truth

Reviewed against:
- `templates/PLAN-TEMPLATE.md`
- `PLAN-STANDARDS.md`

Corrections applied:
- Created a dedicated Scheduler V2 plan instead of leaving Phase 4 as a short
  roadmap subsection.
- Added required affected-contract, persisted-artifact, concurrency,
  ownership, and facade-preservation notes because Scheduler V2 crosses queue,
  runtime, diagnostics, and adapter boundaries.
- Declared this document the Scheduler V2 source of truth so roadmap updates
  can stay summary-level.

### Pass 2: Architecture And Ownership

Reviewed against:
- `ARCHITECTURE-PATTERNS.md`
- `CODING-STANDARDS.md`

Corrections applied:
- Locked scheduler policy ownership to backend Rust crates and kept Tauri and
  bindings transport-only.
- Added explicit refactor scope for oversized insertion files instead of
  assuming new policy can be appended in place.
- Preserved the existing public facade by making extraction and delegation the
  default implementation path.

### Pass 3: Documentation And Source-Tree Compliance

Reviewed against:
- `DOCUMENTATION-STANDARDS.md`

Corrections applied:
- Required README coverage for any new scheduler source directory.
- Recorded roadmap and plan updates as part of the implementation, not as
  optional follow-up.
- Required touched queue/scheduler boundaries to document ownership rather than
  leaving module purpose implicit.

### Pass 4: Concurrency And Lifecycle Safety

Reviewed against:
- `CONCURRENCY-STANDARDS.md`

Corrections applied:
- Required a single owner for mutable queue and admission state.
- Added explicit lifecycle requirements for any background cleanup, poll, or
  admission worker touched by Scheduler V2.
- Recorded overlap risks for cancel, reprioritize, keep-alive, reclaim,
  recovery, and snapshot reads so tests can target them intentionally.

### Pass 5: Testing And Observability

Reviewed against:
- `TESTING-STANDARDS.md`

Corrections applied:
- Required unit coverage for policy decisions and queue ordering.
- Required cross-layer acceptance coverage from backend-owned scheduler state
  through transport-visible diagnostics and error payloads.
- Required replay/recovery/idempotency checks where scheduler state interacts
  with runtime reclaim, cleanup, and restart flows.

### Pass 6: Interop, Bindings, Dependency, Tooling, And Security

Reviewed against:
- `INTEROP-STANDARDS.md`
- `LANGUAGE-BINDINGS-STANDARDS.md`
- `DEPENDENCY-STANDARDS.md`
- `TOOLING-STANDARDS.md`
- `SECURITY-STANDARDS.md`

Corrections applied:
- Restricted bindings and Tauri surfaces to validation, conversion, and
  transport of backend-owned scheduler DTOs.
- Assumed no new dependency unless an implementation milestone proves an
  explicit need; policy should fit inside existing Rust crate boundaries.
- Required validation of raw scheduler requests at the boundary and stable
  machine-consumable error semantics rather than implicit adapter behavior.
- Required any new checked-in structured artifacts to use existing repo
  validation hooks or a narrowly justified addition.

## Definition Of Done

- Scheduler V2 has a dedicated backend-owned module boundary rather than
  continuing as embedded logic inside `workflow.rs`.
- Queue admission, fairness, runtime-affinity, reuse, and rebalance decisions
  are explicit policy objects or equivalent focused abstractions.
- Scheduler diagnostics expose additive machine-consumable decision reasons,
  ETA/admission facts, and stable error semantics.
- Workflow-session and edit-session scheduler-facing payloads remain aligned
  where contracts overlap.
- Tauri and other wrappers forward backend-owned scheduler contracts without
  reconstructing policy.
- The immediate touched scheduler source-tree surroundings comply with README
  and decomposition standards.
- Tests cover policy behavior, concurrency-sensitive queue flows, and at least
  one transport-visible acceptance path.

## Milestones

### Milestone 1: Freeze Scheduler V2 Baseline And Gates

**Goal:** Lock the real starting point, dependency gate, and contract inventory
before policy changes begin.

**Tasks:**
- [x] Reconcile this plan, the roadmap, and the metrics/trace/runtime-registry
      plans so Scheduler V2 dependencies are stated consistently
- [x] Inventory the current scheduler-facing DTOs, queue commands, and
      edit-session overlap points that become the baseline compatibility set
- [x] Record the prerequisite gate for remaining runtime-producer convergence
      and the minimum metrics/trace visibility required before Scheduler V2
      policy work is allowed to proceed
- [x] Record any immediate contract ambiguities or missing machine-consumable
      reason fields that Milestone 2 or 3 must resolve

**Verification:**
- Plan, roadmap, and dependency notes agree on the Scheduler V2 starting point
- Baseline contract inventory is documented and names the real owner module for
  each touched scheduler contract

**Status:** Complete

### Milestone 2: Refactor Scheduler Boundaries To Compliance

**Goal:** Extract scheduler ownership out of oversized workflow-service files so
Scheduler V2 policy has a standards-compliant home.

**Tasks:**
- [x] Create a dedicated scheduler module tree under
      `crates/pantograph-workflow-service/src/` with a `README.md`
- [x] Move queue-state and scheduler-policy-adjacent logic out of
      `workflow.rs` into focused scheduler modules while preserving the public
      facade
- [x] Extract or isolate any new edit-session scheduler projection logic that
      would otherwise deepen `graph/session.rs`
- [x] Keep trace-side scheduler helpers focused and avoid recreating scheduler
      policy inside `trace/store.rs`
- [x] Update existing READMEs to document the new scheduler ownership boundary

**Verification:**
- `workflow.rs` no longer grows as the implementation home for Scheduler V2
- New scheduler source directories have `README.md` coverage
- Public workflow-service entry points remain additive or are explicitly
  documented if a compatibility change is unavoidable

**Status:** Complete

### Milestone 3: Introduce Backend-Owned Scheduler Policy Objects

**Goal:** Replace the implicit simple queue rules with explicit scheduler
policy, fairness, and machine-consumable decision semantics.

**Tasks:**
- [ ] Introduce focused scheduler policy abstractions for admission, ordering,
      fairness, starvation protection, and warm-session reuse
- [ ] Define stable decision-reason, admission-outcome, and scheduler-error
      vocabularies for transport-visible payloads
- [ ] Add runtime-affinity inputs based on workflow id, model dependency, and
      `usage_profile`
- [ ] Keep one mutable-state owner for queue transitions and policy evaluation
- [ ] Add unit coverage for fairness, starvation prevention, and decision
      reasoning

**Verification:**
- Scheduler policy is represented by focused backend-owned abstractions instead
  of ad hoc branching inside one service file
- Machine-consumable scheduler reasons and errors are exposed from backend
  contracts and do not require adapter-local reconstruction

**Execution progress:**
- The backend-owned scheduler module boundary and explicit priority/FIFO policy
  abstraction are landed.
- Transport-visible scheduler decision reasons are landed on
  `WorkflowSessionQueueItem`.
- Queue items now also expose additive canonical `queue_position` diagnostics
  for running and pending items.
- The first starvation-protection promotion rule is now backend-owned in the
  scheduler policy and covered by unit plus workflow-service tests.
- Remaining Milestone 3 work is the deeper policy expansion: broader fairness
  policy and runtime-affinity-oriented admission inputs.

**Status:** In progress

### Milestone 4: Runtime-Aware Admission, Reuse, And Diagnostics

**Goal:** Connect Scheduler V2 policy to runtime-registry facts, reuse paths,
and observable diagnostics without violating ownership boundaries.

**Tasks:**
- [ ] Consume backend-owned runtime-registry facts for runtime-affinity,
      residency pressure, reclaim candidates, and warm reuse decisions
- [ ] Improve rebalance behavior when loaded-session capacity is exhausted
- [ ] Add queue ETA and admission diagnostics derived from canonical scheduler
      state
- [ ] Preserve additive trace integration so scheduler decisions remain
      observable through the metrics/trace spine
- [ ] Keep Tauri and bindings transport-only while forwarding any new scheduler
      diagnostics fields

**Verification:**
- Scheduler reuse and rebalance decisions are derived from backend-owned
  runtime facts
- ETA and admission diagnostics are visible through canonical scheduler and
  trace contracts
- No scheduler policy is added to Tauri or other wrappers

**Status:** Not started

### Milestone 5: Close-Out, Acceptance, And Source-Of-Truth Reconciliation

**Goal:** Finish Scheduler V2 with standards-compliant tests, documentation,
and source-of-truth alignment.

**Tasks:**
- [ ] Add cross-layer acceptance coverage for scheduler snapshots, queue
      commands, and transport-visible decision/error payloads
- [ ] Add replay/recovery/idempotency coverage for scheduler interactions with
      reclaim, cleanup, and restart paths where relevant
- [ ] Update roadmap, READMEs, and ADR text if ownership or accepted
      consequences changed during implementation
- [ ] Validate any new checked-in fixtures or structured examples through the
      repo's existing tooling expectations
- [ ] Record completion status and any intentionally deferred follow-up work in
      this plan

**Verification:**
- Acceptance coverage exercises at least one end-to-end scheduler-visible path
- Source-of-truth documents and touched READMEs accurately describe the landed
  scheduler ownership and diagnostics behavior

**Status:** Not started

## Execution Notes

Update during implementation:
- 2026-04-16: Created the dedicated Scheduler V2 source-of-truth plan and
  recorded the immediate compliance refactors required before more policy work
  lands.
- 2026-04-16: Extracted workflow-session scheduler DTOs and the in-memory
  queue/store into `crates/pantograph-workflow-service/src/scheduler/`, added
  the required README coverage, and kept `workflow.rs` as the public facade
  through re-exports rather than further deepening the service file.
- 2026-04-16: Added the first explicit backend-owned queue policy object plus
  a stable scheduler decision-reason vocabulary, and threaded additive
  `scheduler_decision_reason` fields through queue items so trace/diagnostics
  can prefer backend-owned scheduler reasons over generic matched-item
  fallbacks.

## Commit Cadence Notes

- Commit when a logical scheduler slice is complete and verified.
- Keep refactor and behavior slices atomic enough that queue-ownership moves,
  policy additions, and transport updates can be reviewed independently.
- Follow commit format and history-cleanup rules from
  `COMMIT-STANDARDS.md`.

## Re-Plan Triggers

- Runtime-producer convergence is found incomplete in a way that prevents
  trustworthy scheduler-affinity or reuse decisions.
- Metrics/trace coverage is insufficient to explain new scheduler decisions or
  ETA output.
- Scheduler V2 requires durable queue state, distributed ownership, or another
  architecture change that exceeds the assumptions in this plan.
- Public contract breakage becomes unavoidable rather than additive.
- Immediate surrounding modules prove to need a larger refactor than the
  extraction work captured here.

## Completion Summary

### Completed

- Dedicated Scheduler V2 source-of-truth planning file created
- Roadmap cross-reference updated to point at this plan

### Deviations

- None yet

### Follow-Ups

- Reconcile Scheduler V2 dependency readiness against the current metrics/trace
  spine and any remaining runtime-producer convergence tasks before
  implementation starts

### Verification Summary

- Standards-reviewed the plan against structure, architecture, documentation,
  concurrency, testing, interop, bindings, dependency, tooling, and security
  requirements

### Traceability Links

- Module README updated: Pending Scheduler V2 implementation boundary
- ADR added/updated: N/A at planning time
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A at planning
  time
