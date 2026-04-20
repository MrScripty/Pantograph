# Plan: Pantograph Scheduler V2

## Status
Complete

Last updated: 2026-04-19

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
- `WorkflowSessionQueueItem` and `WorkflowTraceQueueMetrics` now also carry
  additive backend-owned `scheduler_admission_outcome` semantics, so queued
  versus admitted visibility no longer depends on transport-local inference
- the explicit scheduler policy now also owns the first backend starvation-
  protection rule, allowing long-waiting queued runs to accumulate canonical
  promotion credit and surface `starvation_protection` when they legitimately
  overtake newer higher-priority work
- runtime-pressure unload selection now also carries backend-owned target
  workflow and `usage_profile` context plus candidate `usage_profile` facts, so
  capacity rebalance can preserve affine idle runtimes before falling back to
  generic least-recently-used eviction
- runtime-affinity selection now also preserves backend-owned
  `required_backends` and `required_models` facts refreshed from workflow
  capabilities and preflight caches, so unload ranking can avoid evicting idle
  sessions that share the target run's backend and model requirements even
  when they belong to different workflows
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
- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-metrics-trace-spine.md`
- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-runtime-registry-technical-fit-selection.md`
- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-milestone-5-workflow-adapter-integration.md`
- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-milestone-6-diagnostics-documentation-rollout-safety.md`
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
- [x] Extract backend-owned scheduler policy and state boundaries so
      `pantograph-workflow-service::scheduler` owns queue ordering and policy
      primitives while `workflow.rs` remains the public facade.
- [x] Preserve one mutable-state owner for queue transitions and policy
      evaluation in `WorkflowSessionStore`; do not move scheduler truth,
      reorder logic, or admission decisions into Tauri, diagnostics wrappers,
      or other transport layers.
- [x] Define additive backend-owned queue/snapshot/trace vocabulary for
      decision reasons, queue positions, and admission outcomes so transport
      consumers do not infer scheduler state from status combinations.
- [x] Add backend-owned runtime-affinity inputs for unload ranking using
      workflow id, `usage_profile`, `required_backends`, and
      `required_models`, refreshed from workflow capabilities and preflight
      caches before runtime loading.
- [x] Add a backend-owned admission-input model for queued runs that captures
      the canonical facts needed for next-run selection:
      queue ordering state, loaded-runtime posture, affine-runtime reuse
      posture, and any already-known warm-session compatibility facts.
      Keep the implementation in Rust scheduler/workflow-service modules, keep
      transport contracts additive, and do not create a second mutable owner.
- [x] Expand fairness beyond the current starvation-promotion rule with a
      bounded and deterministic policy slice that remains reviewable:
      explicit tie-break semantics, reuse-aware fairness constraints, and
      guardrails that prevent warm-runtime preference from bypassing priority
      or starvation guarantees.
- [x] Extend richer model-dependency affinity beyond the current
      `required_backends`/`required_models` unload-selection basis using
      backend-owned compatibility keys only. Any new affinity identity must be
      normalized in Rust contracts or preflight/session state, not inferred in
      Tauri or frontend code, and must preserve facade-first compatibility.
- [x] Add stable machine-consumable admission and reuse reason vocabulary for
      the stronger admission policy so clients can distinguish queue ordering,
      fairness, affine reuse, and cold-start fallbacks without adapter-local
      heuristics.
- [x] Add focused unit and workflow-service coverage for the remaining
      fairness, admission, and reason-vocabulary slices, including negative
      cases where affine reuse is available but correctly rejected by priority
      or fairness rules.

**Implementation constraints for remaining Milestone 3 work:**
- Backend Rust remains the owner of all scheduler business logic. Tauri and
  other bindings may only forward backend-owned contracts and commands.
- New scheduler-facing fields must be additive on existing contracts unless an
  explicitly documented compatibility break is approved.
- Any new policy helper extracted during this milestone must remain inside the
  scheduler/workflow-service backend boundary and keep `WorkflowSessionStore`
  as the single mutable owner for queue/admission state.
- If new affinity or admission facts come from technical-fit or preflight
  assessment, store normalized backend-owned values in Rust state rather than
  recomputing them in adapters.
- If the remaining policy work forces further decomposition under `src/`,
  update the touched module README(s) in the same logical slice.

**Verification:**
- Scheduler policy is represented by focused backend-owned abstractions instead
  of ad hoc branching inside one service file
- Machine-consumable scheduler reasons and errors are exposed from backend
  contracts and do not require adapter-local reconstruction
- Admission and fairness decisions are computed from backend-owned Rust state,
  not duplicated in Tauri, diagnostics, or frontend code
- New contract fields remain additive and are documented in the touched
  scheduler source-of-truth files
- Unit and workflow-service tests cover both positive reuse/fairness outcomes
  and the refusal paths that preserve deterministic priority behavior

**Execution progress:**
- The backend-owned scheduler module boundary and explicit priority/FIFO policy
  abstraction are landed.
- Transport-visible scheduler decision reasons are landed on
  `WorkflowSessionQueueItem`.
- Queue items now also expose additive canonical `queue_position` diagnostics
  for running and pending items.
- Queue items and trace queue metrics now also expose additive backend-owned
  `scheduler_admission_outcome` values so queued versus admitted state is
  machine-consumable without reconstructing it from item status.
- The first starvation-protection promotion rule is now backend-owned in the
  scheduler policy and covered by unit plus workflow-service tests.
- Runtime-pressure unload selection now also consumes backend-owned target
  workflow and `usage_profile` affinity inputs, and preserves less-affine idle
  sessions first in workflow-service and embedded-runtime tests.
- Scheduler affinity now also consumes backend-owned `required_backends` and
  `required_models` refreshed from workflow capabilities and preflight caches,
  and preserves shared-backend or shared-model idle runtimes before unrelated
  sessions during rebalance.
- Scheduler runtime-affinity ranking now also folds `usage_profile`,
  `required_backends`, and `required_models` into an explicit backend-owned
  compatibility identity, so cross-workflow reclaim keeps same-profile,
  same-dependency idle runtimes resident before less-compatible sessions.
- Store-owned queue transitions now also build a canonical internal admission
  input from queue ordering, loaded-runtime posture, affine reuse posture, and
  warm-session compatibility facts before delegating next-run selection to the
  backend scheduler policy.
- Admitted runs now also expose backend-owned `warm_session_reused`,
  `runtime_reload_required`, and `cold_start_required` reasons so queue,
  snapshot, and trace consumers no longer depend on a generic
  `admitted_for_execution` label.
- Admission selection now also applies a bounded warm-reuse fairness window in
  the highest-priority, non-starved band, so a compatible warm candidate can
  bypass at most the next cold candidate without overtaking higher-priority or
  starved work.
- Focused unit and workflow-service coverage now also locks the positive warm-
  reuse path plus negative cases where reuse is correctly rejected by
  starvation or window guardrails.

**Status:** Complete

### Milestone 4: Runtime-Aware Admission, Reuse, And Diagnostics

**Goal:** Connect Scheduler V2 policy to runtime-registry facts, reuse paths,
and observable diagnostics without violating ownership boundaries.

**Tasks:**
- [x] Consume backend-owned runtime-registry facts for runtime-affinity,
      residency pressure, reclaim candidates, and warm reuse decisions
- [x] Improve rebalance behavior when loaded-session capacity is exhausted
- [x] Add queue ETA and admission diagnostics derived from canonical scheduler
      state
- [x] Preserve additive trace integration so scheduler decisions remain
      observable through the metrics/trace spine
- [x] Keep Tauri and bindings transport-only while forwarding any new scheduler
      diagnostics fields

**Verification:**
- Scheduler reuse and rebalance decisions are derived from backend-owned
  runtime facts
- ETA and admission diagnostics are visible through canonical scheduler and
  trace contracts
- No scheduler policy is added to Tauri or other wrappers

**Execution progress:**
- Canonical scheduler snapshots now expose an additive backend-owned
  `runtime_registry` diagnostics block covering target runtime identity,
  runtime-registry reclaim candidates, and next warmup/reuse posture.
- `pantograph-workflow-service` now accepts an optional backend diagnostics
  provider so canonical queue ownership stays in the service while
  host-specific runtime-registry facts remain injectable and transport-agnostic.
- `pantograph-embedded-runtime` now supplies those facts from the runtime
  registry and gateway lifecycle, while Tauri continues forwarding the shared
  backend contract without adding scheduler policy.
- Canonical scheduler snapshots now also expose additive backend-owned
  `next_admission_wait_ms` and `next_admission_not_before_ms` fields. These
  remain intentionally bounded to earliest-known admission time: immediate
  admission returns `0`, runtime-admission polling returns the canonical poll
  delay, and capacity or active-run blockers stay `None` instead of guessing.
- Canonical scheduler snapshots now also expose additive backend-owned
  `next_admission_bypassed_queue_id` visibility when warm-reuse fairness
  intentionally selects a non-head queue item, so adapters and diagnostics can
  explain the remaining fairness behavior without reconstructing queue policy.

**Status:** Complete

### Milestone 5: Close-Out, Acceptance, And Source-Of-Truth Reconciliation

**Goal:** Finish Scheduler V2 with standards-compliant tests, documentation,
and source-of-truth alignment.

**Tasks:**
- [x] Add cross-layer acceptance coverage for scheduler snapshots, queue
      commands, and transport-visible decision/error payloads
- [x] Add replay/recovery/idempotency coverage for scheduler interactions with
      reclaim, cleanup, and restart paths where relevant
- [x] Update roadmap, READMEs, and ADR text if ownership or accepted
      consequences changed during implementation
- [x] Validate any new checked-in fixtures or structured examples through the
      repo's existing tooling expectations
- [x] Record completion status and any intentionally deferred follow-up work in
      this plan

**Verification:**
- Acceptance coverage exercises at least one end-to-end scheduler-visible path
- Source-of-truth documents and touched READMEs accurately describe the landed
  scheduler ownership and diagnostics behavior

**Execution progress:**
- Added transport-facing acceptance coverage proving additive scheduler
  `runtime_registry` diagnostics survive the headless diagnostics projection
  without Tauri or wrappers reinterpreting scheduler policy.
- Added cleanup-path recovery coverage ensuring stale-session cleanup is
  idempotent after removal and cannot delete workflow sessions that still hold
  queued scheduler work.
- Extended that cleanup recovery coverage to the background stale-session
  worker so autonomous cleanup preserves queued scheduler work as well.
- Added hosted embedded-runtime recovery coverage proving that, after an
  embedding workflow restores the inference runtime, the backend-owned
  scheduler runtime-registry diagnostics provider still reports reclaim and
  `reuse_loaded_runtime` facts for the next admission request.
- Added companion reclaim coverage proving that, after keep-alive-driven
  runtime unload stops the last loaded runtime, the same backend-owned
  diagnostics provider flips the next admission posture to
  `start_runtime`/`no_loaded_instance`.
- Workflow error envelopes now also expose additive backend-owned
  `details.scheduler` payloads for scheduler-capacity failures, including
  stable reason codes plus session or loaded-runtime counts, and the Tauri and
  HTTP transport adapters now forward those details without translating
  scheduler policy locally.
- Refreshed the roadmap to remove stale pre-implementation Scheduler V2
  wording, record the now-landed transport and recovery coverage, and align
  the recommended execution order with Scheduler V2 already being underway.
- Reviewed the touched scheduler/runtime ownership READMEs and ADR references
  during close-out; no additional wording changes were required because the
  current backend-vs-adapter boundary text remained accurate after the
  standards-oriented refactors.
- Milestone 5 did not introduce new checked-in fixtures or structured examples
  outside the existing backend tests, so no additional fixture-specific
  validation or regeneration steps were required during close-out.
- Scheduler V2 is complete at the current roadmap scope. Later roadmap phases
  should treat the scheduler contracts and policy boundaries landed here as
  frozen unless a new re-plan explicitly approves additional scheduler work.

**Status:** Complete

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
- 2026-04-16: Added additive backend-owned
  `scheduler_admission_outcome` fields to workflow-session queue items and
  trace queue metrics, updated workflow-service, embedded-runtime, and Tauri
  fixtures, and verified that scheduler-facing diagnostics no longer have to
  infer queued versus admitted state from status/reason combinations.
- 2026-04-16: Extended scheduler runtime-affinity inputs with backend-owned
  `required_models`, refreshed that affinity basis from workflow capabilities
  and preflight cache updates before runtime loading, and verified that
  capacity rebalance preserves shared-model idle runtimes ahead of unrelated
  model sessions.
- 2026-04-16: Extended the same scheduler affinity basis with backend-owned
  `required_backends`, so rebalance now preserves idle sessions that share the
  target run's backend requirements before candidates that only overlap on
  model id or workflow shape.
- 2026-04-16: Added a backend-owned internal admission-input model in
  `scheduler/policy.rs` and `scheduler/store.rs`, so queue admission now
  evaluates canonical queue/runtime/warm-compatibility facts through an
  explicit policy boundary instead of leaving them implicit in the store
  mutation path.
- 2026-04-16: Replaced the generic admitted execution reason with
  backend-owned `warm_session_reused`, `runtime_reload_required`, and
  `cold_start_required` admission reasons across queue, workflow-service, and
  trace projections.
- 2026-04-16: Folded scheduler unload affinity onto an explicit internal
  compatibility identity spanning `usage_profile`, `required_backends`, and
  `required_models`, so cross-workflow reclaim can preserve more reusable idle
  runtimes without shifting ownership into adapters.
- 2026-04-16: Added a bounded warm-reuse fairness window for highest-priority,
  non-starved admission candidates, plus focused policy and workflow-service
  coverage for positive reuse, starvation guardrails, and out-of-window
  rejection cases.
- 2026-04-17: Added additive backend-owned scheduler snapshot diagnostics for
  loaded-session pressure, reclaimable-capacity visibility, and next-admission
  prediction, then threaded those canonical fields through embedded-runtime and
  Tauri diagnostics/event transport without moving scheduler policy ownership
  out of Rust.
- 2026-04-17: Extended the workflow trace spine so scheduler snapshot
  diagnostics can flow through backend-owned trace events and queue metrics,
  keeping runtime-capacity and next-admission visibility machine-consumable in
  trace snapshots instead of leaving those fields trapped in transport-local
  diagnostics state.
- 2026-04-17: Changed workflow-session admission so runs no longer dequeue into
  an immediate `SchedulerBusy` failure when loaded-session capacity is fully
  occupied by active work; the selected candidate now stays queued with
  `waiting_for_runtime_capacity` until a real rebalance or release path opens.
- 2026-04-17: Added a backend-owned runtime-registry admission dry-run seam so
  workflow-session runs can remain queued with `waiting_for_runtime_admission`
  when the active runtime cannot currently accept another reservation, instead
  of dequeuing and then failing after runtime load begins.
- 2026-04-17: Added backend-owned scheduler ETA lower-bound diagnostics via
  additive `next_admission_wait_ms` and `next_admission_not_before_ms`
  snapshot fields, exposing only earliest-known admission timing from
  canonical state instead of transport-side or speculative estimates.

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
- Scheduler V2 implementation slices are complete through Milestone 5 close-out
  plus the post-close-out bounded ETA and fairness-observability additions

### Deviations

- None yet

### Follow-Ups

- No internal Scheduler V2 follow-up is currently scheduled. Later roadmap
  phases should build on this boundary rather than reopening scheduler policy
  ownership without an explicit re-plan.

### Verification Summary

- Standards-reviewed the plan against structure, architecture, documentation,
  concurrency, testing, interop, bindings, dependency, tooling, and security
  requirements

### Traceability Links

- Module README updated: `crates/pantograph-workflow-service/src/scheduler/README.md`
- ADR added/updated: N/A at planning time
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A at planning
  time
