# Plan: Pantograph Phase 6 Incremental Graph Execution And Node Memory

## Status
Active

Last updated: 2026-04-18

## Current Source-of-Truth Summary

This document is the dedicated source of truth for roadmap Phase 6. Phase 6 is
no longer only "partial graph invalidation." With the clarified requirements,
Phase 6 now covers the backend-owned execution-memory substrate that makes
incremental graph execution, debug inspection, keep-alive workflow sessions,
input reinjection, and scheduler-driven unload/restore viable without pushing
core graph policy into Tauri or the frontend.

The roadmap remains the cross-target summary. This file now owns:

- the backend-owned node-memory and session-checkpoint contract
- graph-edit reconciliation rules that preserve compatible node memory across
  graph mutations
- incremental rerun semantics that reuse preserved node memory and cached
  outputs where safe
- restore/reload behavior for keep-alive, temporary runtime unload, and resumed
  workflow sessions
- standards-driven refactors required in the immediate touched backend,
  transport, and package files before more policy lands there

The accurate baseline at the start of this revised plan is:

- `crates/node-engine/src/engine.rs` already provides demand-driven execution,
  output caching, version tracking, and `GraphModified` /
  `IncrementalExecutionStarted` event emission, but it does not yet expose a
  first-class backend-owned node-memory contract.
- `WorkflowExecutor` already owns an execution-scoped `Context` and a
  `DemandEngine`, but current behavior is output-cache-oriented rather than
  memory-contract-oriented.
- `crates/pantograph-workflow-service/src/graph/session.rs` already owns
  backend graph-edit sessions and dirty-task derivation, but it remains
  oversized and currently treats undo/redo as broad invalidation rather than
  graph-plus-memory reconciliation.
- `crates/pantograph-workflow-service/src/workflow.rs` and
  `crates/pantograph-workflow-service/src/scheduler/` already own keep-alive,
  runtime reuse, reclaim, and queue semantics, but they do not yet track a
  persistent workflow-session memory/checkpoint artifact.
- `src-tauri/src/workflow/execution_manager.rs` still owns execution handles
  containing a `WorkflowExecutor` and undo stacks, but it does not yet own a
  standards-reviewed boundary for checkpointing and restoring backend-owned
  node memory.
- `crates/pantograph-workflow-service/src/trace/` and
  `src-tauri/src/workflow/diagnostics/` already project dirty-task and
  incremental-run facts, but they do not yet expose backend-owned node-memory
  inspection snapshots.
- `packages/svelte-graph/src/stores/createWorkflowStores.ts`,
  `packages/svelte-graph/src/components/WorkflowGraph.svelte`, and
  `src/services/workflow/WorkflowService.ts` remain immediate non-compliant
  insertion areas that must be kept healthy if they are touched by the rollout.

## Objective

Implement a standards-compliant backend-owned node-memory and session-
checkpoint system that allows Pantograph to:

- preserve compatible per-node execution state across graph edits
- run only the affected downstream closure after input injection or graph
  mutation
- inspect node state after execution for debugging and system-building
- keep workflow sessions warm without requiring full recomputation on each
  invoke
- tolerate scheduler-driven runtime unload/restore and temporary host/runtime
  churn without losing logical workflow state

The implementation must leave the immediate touched files in a clean state and
must preserve backend ownership of graph semantics.

## Architecture Model

Phase 6 now treats incremental graph execution as a workflow-session state
system, not only as a dirty-task calculation feature. The architecture to be
implemented is:

- one backend-owned workflow session contains:
  graph revision metadata,
  node-memory records,
  output cache entries,
  checkpoint metadata,
  runtime-residency metadata
- each node-memory record is owned per workflow session and per node id
- graph mutation and explicit input injection produce a backend-owned
  reconciliation result that decides which node-memory records are preserved,
  invalidated, remapped, or dropped
- incremental reruns start from those mutation/injection points and only
  resolve the affected downstream closure while preserved node-memory records
  remain available to unchanged nodes
- keep-alive, reclaim, unload, and restore move a session between runtime
  residency states without changing the authoritative owner of the session's
  logical node state
- trace, diagnostics, Tauri, and the GUI consume read-only projections of this
  backend-owned state

The implementation must make these boundaries explicit enough that later
durable-persistence or binding work can build on the same model instead of
inventing a second execution-state system.

## Scope

### In Scope

- backend-owned node-memory records keyed by workflow session and node identity
- backend-owned session checkpoint artifacts that can survive keep-alive,
  runtime unload, restore, and resumed invocation
- explicit workflow-session residency states:
  active,
  warm,
  checkpointed-but-unloaded,
  restored
- graph-edit reconciliation rules for preserving, invalidating, or dropping
  node memory as graph structure changes
- incremental rerun behavior that starts from injection/edit points and
  implicitly resolves only the affected downstream closure
- node-level inspection snapshots that remain available after execution for
  debugger/system-builder use
- selective input reinjection rules for persistent sessions so later invocations
  can update inputs and rerun only the affected suffix of the graph
- explicit separation between output cache, node memory, and runtime/resource
  handles
- backend-owned inspection and diagnostics contracts for node memory and graph
  mutation impact
- scheduler/runtime integration required so temporary runtime unload/restore
  does not destroy logical workflow state
- refactors in the immediate touched backend, transport, and package files so
  the changed areas end standards compliant
- roadmap, README, and source-of-truth reconciliation for the broadened Phase 6

### Out of Scope

- a codebase-wide refactor outside the touched areas
- distributed multi-host memory synchronization
- arbitrary durable history/versioning for every checkpoint beyond the bounded
  retention explicitly approved here
- expanding public language-binding surface area beyond additive transport
  forwarding needed by touched contracts
- reopening Scheduler V2 or Runtime Adapter Unification ownership boundaries
  beyond the specific checkpoint/reconcile integration this phase requires
- KV cache as the generic workflow-memory substrate

## Inputs

### Problem

Pantograph currently has enough machinery to perform partial recomputation, but
not enough machinery to treat workflow state as a durable, inspectable,
backend-owned graph substrate:

- output cache reuse exists, but output cache is not the same as node memory
- execution-scoped context exists, but it is not a frozen contract for node
  memory persistence, restore, or post-edit reconciliation
- graph-editing already computes dirty tasks, but dirty tasks alone are not
  enough to preserve pre-edit node state and continue efficiently
- keep-alive sessions and runtime reuse already exist, but they do not yet
  guarantee backend-owned node-state continuity across unload/restore
- diagnostics already expose traces, but they do not yet expose explicit
  backend-owned node-memory inspection

Without revising the Phase 6 plan around a real execution-memory substrate,
implementation would drift into one of four bad outcomes:

- expanding output-cache internals until they silently become de facto memory
  ownership without a contract
- keeping node memory as accidental graph-flow context entries with no
  checkpoint, compatibility, or restore rules
- re-deriving graph or memory policy in Tauri/frontend because those are the
  easiest visible insertion points
- shipping keep-alive and restore behaviors that preserve runtime/process state
  but lose the workflow's logical node state

### Constraints

- Backend Rust owns graph semantics, node-memory semantics, and checkpoint
  semantics.
- `src-tauri` remains transport/composition only.
- The frontend and graph package may visualize node-memory facts, but they must
  not become the owner of node-memory lifecycle or graph reconciliation rules.
- Existing facades should remain additive unless a documented break is
  explicitly approved.
- The plan must separate:
  output cache,
  node memory,
  runtime handles/resources.
- Complex graph mutations may fall back to full checkpoint invalidation where
  exact partial preservation is not yet proven.
- Any new source directory created under `src/` or equivalent roots requires a
  standards-compliant `README.md`.

### Required Semantic Distinctions

The implementation must keep these concepts separate and documented:

- `output cache`:
  a reusable computed result fast-path owned by execution logic
- `node memory`:
  the inspectable logical state for one node in one workflow session, including
  compatibility metadata and any backend-approved private node state
- `session checkpoint`:
  a bounded backend-owned artifact that preserves enough workflow-session state
  to survive reclaim, unload, restore, or resumed invocation
- `runtime residency`:
  whether a runtime/process is currently attached to the workflow session

The checkpoint may contain node-memory data or references to it, but runtime
process handles and ephemeral host resources must stay outside the serializable
logical session-state contract.

### Public Facade Preservation Note

This remains a facade-first plan. The default implementation path is internal
extraction and additive contract growth behind existing workflow-service,
execution-manager, node-engine, Tauri, and graph-package surfaces.

### Assumptions

- A workflow session is the correct top-level owner for preserved node memory.
- `node_id` remains the primary identity key for memory reconciliation, with
  node type and schema/version checks guarding compatibility.
- Output cache can remain a fast-path optimization, but it should no longer be
  the only reusable execution-state mechanism.
- Runtime-specific state that cannot be serialized should be represented
  indirectly in node memory and restored through backend runtime/session
  orchestration rather than embedded directly in checkpoint payloads.
- Some initial persistence may remain bounded and backend-owned before a later
  phase approves broader durable storage.
- Graph edits will not preserve memory by best-effort guesswork alone; each
  preservation path must be explainable through explicit compatibility rules
  that can also be projected into diagnostics.
- Some nodes may initially support only coarse compatibility or full
  invalidation, and that is acceptable if the fallback remains backend-owned
  and explicit.

### Dependencies

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-2-parallel-demand-execution.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-5-real-workflow-event-contract.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-5-follow-on-completion.md`
- `IMPLEMENTATION-PLAN-pantograph-scheduler-v2.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-7-runtime-adapter-unification.md`
- `IMPLEMENTATION-PLAN-pantograph-metrics-trace-spine.md`
- `crates/node-engine/src/engine`
- `crates/pantograph-workflow-service/src/graph`
- `crates/pantograph-workflow-service/src/scheduler`
- `crates/pantograph-workflow-service/src/trace`
- `src-tauri/src/workflow`
- `packages/svelte-graph/src/stores`
- `packages/svelte-graph/src/components`
- `src/services/workflow`
- standards in
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`

### Affected Systems

- `node-engine`: execution memory contract, cache separation, incremental
  rerun consumption, checkpoint integration
- `pantograph-workflow-service graph`: edit-session graph reconciliation,
  mutation impact ownership, memory compatibility rules
- `pantograph-workflow-service workflow/scheduler`: keep-alive session
  lifecycle, unload/reclaim integration, restore admission
- `pantograph-workflow-service trace`: node-memory inspection and restore/
  reinjection diagnostics
- `pantograph-embedded-runtime` and adjacent runtime-coordination boundaries:
  restore/rebind sequencing where runtime residency changes intersect
  checkpointed workflow-session state
- `src-tauri workflow`: transport DTO forwarding, execution-manager ownership
  refactor, thin adapter exposure
- `packages/svelte-graph` and `src/services/workflow`: read-only presentation of
  backend-owned memory/mutation facts plus session/execution ownership hygiene

### Affected Structured Contracts

- `WorkflowGraphEditSessionGraphResponse`
- backend-owned graph mutation impact DTOs
- backend-owned node-memory snapshot/checkpoint DTOs
- backend-owned workflow session checkpoint lifecycle requests/responses
- `WorkflowEvent::GraphModified`
- `WorkflowEvent::IncrementalExecutionStarted`
- trace and diagnostics DTOs that surface memory/checkpoint inspection facts
- any additive Tauri/backend/frontend transport DTOs needed to forward those
  facts without re-owning them

### Affected Persisted Artifacts

- this dedicated Phase 6 plan
- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- touched module `README.md` files in:
  - `crates/node-engine/src/engine`
  - `crates/pantograph-workflow-service/src/graph`
  - `crates/pantograph-workflow-service/src/trace`
  - `src-tauri/src/workflow`
  - `packages/svelte-graph/src/stores`
  - `packages/svelte-graph/src/components`
  - `src/services/workflow`
- any checked-in fixtures or examples added to pin node-memory or checkpoint
  payloads

### Existing Codebase Non-Compliance In Immediate Surroundings

The immediate Phase 6 insertion area already exceeds decomposition thresholds
and must not absorb more behavior in place:

- `crates/pantograph-workflow-service/src/graph/session.rs` is approximately
  1149 lines and currently mixes DTOs, mutable session state, mutation
  orchestration, undo/redo, execution markers, and graph conversion
- `crates/node-engine/src/engine.rs` still centralizes multiple execution
  concerns even after earlier extractions and will need targeted further splits
  if node-memory ownership lands there
- `src-tauri/src/workflow/execution_manager.rs` currently mixes execution
  handle ownership, undo/redo, and lifecycle concerns in one file
- `packages/svelte-graph/src/components/WorkflowGraph.svelte` is approximately
  1215 lines and must not absorb more stateful policy
- `src/services/workflow/WorkflowService.ts` is approximately 895 lines and
  must not become the owner of memory/checkpoint logic
- `packages/svelte-graph/src/stores/createWorkflowStores.ts` still contains
  local graph mutation paths adjacent to the Phase 6 ownership boundary
- `src-tauri/src/workflow/connection_intent.rs` and
  `src-tauri/src/workflow/types.rs` remain oversized and must not become a
  dumping ground for backend graph-memory logic

This plan therefore requires local extraction and cleanup in the exact touched
areas. It does not authorize repo-wide sweeping refactors.

### Concurrency / Race-Risk Review

- Graph edits, input reinjection, partial reruns, undo/redo, keep-alive
  reuse, unload/reclaim, and restore can overlap.
- There must be one backend-owned owner for mutable workflow-session memory.
- Output cache invalidation and node-memory invalidation must remain coherent;
  the system must not reuse a cached output whose node memory has already been
  invalidated by a graph edit.
- Scheduler reclaim and restore must not race with in-flight checkpoint writes
  or produce double-restore behavior.
- Session memory must be isolated per workflow session; parallel sessions must
  not share mutable node-memory state.
- Persistent session reinvocation must not observe partially reconciled
  node-memory state from an overlapping edit or restore transition.
- Any background checkpoint, cleanup, or restore coordination introduced in
  this phase must declare start/stop ownership and prevent duplicate workers.

### Ownership And Lifecycle Note

- `node-engine` owns execution-time read/write behavior for node memory and the
  distinction between cache reuse and explicit node-state reuse.
- `pantograph-workflow-service` owns workflow-session memory stores,
  graph-edit reconciliation, checkpoint retention policy, and compatibility
  decisions when graphs change.
- Scheduler/runtime code owns when sessions are kept warm, unloaded, reclaimed,
  or restored, but it consumes backend-owned checkpoint rules instead of
  inventing them locally.
- Trace/diagnostics code owns read-only inspection projections only.
- Tauri/frontend code own transport and visualization only.

### Execution-State Model

The execution-state model to implement and validate through this plan is:

1. A workflow session owns a graph revision and a node-memory set.
2. Each successful node execution may update:
   output cache,
   node-memory record,
   inspection metadata.
3. A graph edit or explicit input reinjection produces a reconciliation result
   with:
   preserved nodes,
   invalidated nodes,
   dropped nodes,
   affected downstream closure.
4. Incremental execution uses that reconciliation result to rerun only what is
   needed.
5. Keep-alive or scheduler unload converts the session into a checkpointed
   state without erasing logical node memory.
6. Restore reattaches runtime residency and resumes from the reconciled
   session-state substrate instead of forcing blanket recomputation.

### Graph-Edit Compatibility Classes

At minimum, Phase 6 must define backend-owned compatibility classes for:

- `preserve-as-is`:
  same node id, same node type, compatible schema, compatible input contract
- `preserve-with-input-refresh`:
  node memory survives, but downstream closure becomes dirty because upstream
  inputs changed
- `drop-on-identity-change`:
  node removed, renamed without an explicit remap rule, or node type changed
- `drop-on-schema-incompatibility`:
  node contract changed in a way that invalidates prior memory
- `fallback-full-invalidation`:
  a mutation class the system cannot safely reconcile yet

These classes must be surfaced in backend-owned diagnostics so later debugging
or persistence work can explain why a node reran or was preserved.

### Persistent Session Expectations

Phase 6 must support the following backend-owned workflow-session behavior:

- a host can keep a workflow session alive after a run
- later invocations can request a fresh full run or selectively update inputs
- selective input updates implicitly rerun the affected suffix of the graph
  without recomputing unaffected nodes
- scheduler reclaim or temporary unload can remove runtime residency while
  retaining the workflow session's logical node state
- restore can rehydrate runtime residency and continue from compatible
  checkpoint state
- debugger/system-builder tools can inspect node memory after execution even if
  the runtime is later unloaded

## Standards Review Passes

### Draft Pass

Initial draft revised from:

- the original Phase 6 incremental invalidation plan
- the clarified requirement for per-node memory persistence and restore
- direct inspection of current node-engine, graph-session, scheduler, trace,
  execution-manager, and graph-package insertion points

### Pass 1: Plan And Documentation Standards

Reviewed against:

- `PLAN-STANDARDS.md`
- `DOCUMENTATION-STANDARDS.md`

Resulting requirements:

- use one dedicated Phase 6 plan as the source of truth for the broadened scope
- include explicit dependencies, risks, milestones, re-plan triggers, and
  completion criteria
- update roadmap and touched READMEs in the same slices that change ownership
  or source of truth

### Pass 2: Architecture And Coding Standards

Reviewed against:

- `ARCHITECTURE-PATTERNS.md`
- `CODING-STANDARDS.md`

Resulting requirements:

- backend Rust remains the single owner of graph state, node memory, and
  checkpoint semantics
- immediate oversized files must be decomposed before absorbing more policy
- frontend and Tauri must not own graph or memory lifecycle rules
- stateful flow ownership must stay singular and explicit

### Pass 3: Concurrency And Testing Standards

Reviewed against:

- `CONCURRENCY-STANDARDS.md`
- `TESTING-STANDARDS.md`

Resulting requirements:

- one backend owner for mutable workflow-session memory
- isolated per-session state and explicit protection of shared mutable memory
- replay/recovery/idempotency tests for checkpoint restore and scheduler-driven
  unload/restore
- cross-layer acceptance coverage from backend graph change/input injection
  through transport and GUI/package consumers

### Pass 4: Frontend, Interop, Cross-Platform, Dependency, And Security Standards

Reviewed against:

- `FRONTEND-STANDARDS.md`
- `INTEROP-STANDARDS.md`
- `CROSS-PLATFORM-STANDARDS.md`
- `DEPENDENCY-STANDARDS.md`
- `SECURITY-STANDARDS.md`

Resulting requirements:

- frontend consumes backend-owned inspection data without polling-based policy
  reconstruction
- additive DTO changes must validate at transport boundaries
- no inline platform-specific business logic should be introduced for checkpoint
  paths
- no new third-party dependency is justified for core checkpoint or memory data
  structures unless a later re-plan proves a real gap
- any external input to checkpoint restore or injection boundaries validates
  once at the boundary
- touched package/frontend files must stay presentation-only; if they need more
  logic for new diagnostics, extract read-only helpers rather than expanding
  monolithic stores/components

## Definition of Done

- Pantograph has a backend-owned workflow-session node-memory contract
- graph edits reconcile compatible node memory instead of forcing blanket
  recomputation in every case
- input reinjection can rerun only the downstream closure from the injection
  points while preserving unaffected node memory
- keep-alive sessions and temporary runtime unload/restore preserve logical
  workflow state through backend-owned checkpoints
- debugger/diagnostics consumers can inspect backend-owned post-run node state
- touched backend, transport, and package files end in a standards-compliant
  shape with updated READMEs

## Milestones

### Milestone 1: Freeze The Node-Memory And Checkpoint Contract

**Goal:** Define the backend-owned contracts and module ownership before
implementation spreads across graph, execution, scheduler, and diagnostics
layers.

**Tasks:**
- [ ] Define explicit backend-owned concepts for `output cache`, `node memory`,
      and `session checkpoint` and record their invariants.
- [ ] Define additive DTOs for node-memory snapshots, memory compatibility, and
      session checkpoint summaries.
- [ ] Decide and document the authoritative identity model:
      session id,
      node id,
      node type,
      schema/version compatibility markers.
- [ ] Define the backend-owned workflow-session residency states and record
      which state transitions do and do not require checkpoint reconciliation.
- [ ] Define the minimum compatibility classes used by graph edit, undo/redo,
      explicit input reinjection, and resumed invocation paths.
- [ ] Extract any minimum scaffolding needed so the new contracts do not land
      directly into already oversized files.
- [ ] Update the roadmap and touched READMEs to record this broadened Phase 6
      ownership boundary.

**Verification:**
- Documentation/source-of-truth review against plan and documentation
  standards
- Focused compile/test review only if code scaffolding lands in this slice

**Status:** Complete

### Milestone 2: Refactor Backend Graph And Execution Ownership Boundaries

**Goal:** Make the immediate backend insertion areas healthy enough to absorb
the new system safely.

**Tasks:**
- [x] Extract node-memory and checkpoint helpers out of
      `crates/pantograph-workflow-service/src/graph/session.rs`.
- [x] Extract execution-memory and checkpoint-adapter helpers out of the
      immediate `node-engine` insertion area as needed.
- [x] Extract or isolate any runtime-residency/checkpoint coordination helpers
      from adjacent workflow-service or runtime-boundary modules before more
      phase logic lands there.
- [x] Refactor `src-tauri/src/workflow/execution_manager.rs` so execution handle
      ownership, undo/redo, and checkpoint lifecycle do not remain collapsed in
      one module if this file is touched.
- [x] Keep `workflow.rs` facade-first and avoid re-owning graph memory policy
      there.
- [x] Update README files for any new source directories/modules created by the
      extraction.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- `cargo test -p node-engine --lib`
- README review for touched backend source directories

**Status:** Completed

### Milestone 3: Implement Workflow-Session Node Memory

**Goal:** Add a real backend-owned per-node memory store and integrate it with
execution.

**Tasks:**
- [x] Add workflow-session memory records keyed by session id and node id.
- [x] Define memory record fields for identity, last input fingerprint, last
      output snapshot, private node state, status, and inspection metadata.
- [x] Make the per-node memory contract explicit enough that a node's logical
      memory can be inspected after execution without depending on live runtime
      process state.
- [x] Integrate execution-time writes so successful demand paths update
      backend-owned node memory through workflow-session contracts.
- [x] Integrate execution-time reads/writes so nodes can consume and update
      their memory through backend-owned contracts.
- [x] Keep runtime handles and non-serializable process state out of the memory
      payload; represent them through indirect references and restore rules.
- [x] Preserve strict separation between output cache reuse and explicit node
      memory reuse.
- [x] Add backend-owned tests proving memory isolation across concurrent
      workflow sessions and repeated runs against the same session.

**Verification:**
- `cargo test -p node-engine --lib`
- Focused `cargo test -p pantograph-workflow-service` coverage for graph-memory
  store ownership and state isolation

**Status:** Completed

### Milestone 4: Add Graph-Edit Reconciliation And Incremental Reinjection

**Goal:** Preserve compatible node memory across graph edits and rerun only the
affected downstream closure.

**Tasks:**
- [ ] Implement graph-edit reconciliation rules for:
      same node compatible,
      node removed,
      node type changed,
      incompatible schema changed,
      edge/input topology changed.
- [ ] Make input injection mark the downstream closure dirty and implicitly
      rerun from those injection points.
- [ ] Support repeated invocations against a kept-alive session where callers
      selectively update inputs and expect only the affected suffix to rerun.
- [ ] Allow explicit fallback to full invalidation for mutation paths whose
      compatibility cannot be proven exactly.
- [ ] Align `GraphModified` and `IncrementalExecutionStarted` payloads with the
      new mutation-impact and memory-reuse semantics.
- [ ] Ensure undo/redo restores graph state and reconciles node memory rather
      than only clearing cache.
- [ ] Add backend-owned diagnostics facts that explain which nodes were
      preserved, invalidated, or rerun after graph mutation or input
      reinjection.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- `cargo test -p node-engine --lib`
- Cross-layer acceptance checks for:
  graph edit then rerun,
  input injection then downstream-only rerun,
  undo/redo memory reconciliation

**Status:** In progress

### Milestone 5: Integrate Keep-Alive, Scheduler Unload, And Restore

**Goal:** Ensure logical workflow state survives runtime churn.

**Tasks:**
- [ ] Add bounded backend-owned session checkpoint retention for keep-alive
      workflow sessions.
- [ ] Integrate scheduler/runtime reclaim and restore paths with the checkpoint
      contract so temporary unload does not discard node memory.
- [ ] Define restore ordering and idempotency rules so runtime restoration and
      checkpoint restoration cannot race or replay inconsistently.
- [ ] Ensure persistent workflow sessions can resume without recomputing the
      entire graph when compatible checkpoint state exists.
- [ ] Define how parallel workflow sessions and temporarily unloaded sessions
      retain isolated logical memory while runtime residency is reclaimed and
      later restored.
- [ ] Update touched scheduler/runtime READMEs where the new checkpoint
      semantics cross existing lifecycle boundaries.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- Focused restore/reclaim/replay tests in touched runtime or Tauri workflow
  modules
- Recovery/idempotency checks per testing standards

**Status:** Not started

### Milestone 6: Add Inspection And Diagnostics Surfaces

**Goal:** Make node memory inspectable and traceable without moving ownership
out of the backend.

**Tasks:**
- [ ] Extend backend trace/diagnostics contracts with additive node-memory and
      checkpoint-inspection facts.
- [ ] Add thin Tauri transport forwarding for those diagnostics.
- [ ] Refactor touched package/frontend files so they remain read-only
      presenters of backend-owned memory and mutation diagnostics.
- [ ] Expose enough inspection data for debugger/system-builder tooling to see
      the pre-edit preserved node state, the post-edit reconciliation result,
      and the post-rerun node-memory state without reconstructing backend
      policy in the GUI.
- [ ] Keep `WorkflowGraph.svelte`, `createWorkflowStores.ts`, and
      `WorkflowService.ts` healthy by extracting any touched helper logic before
      more stateful behavior lands there.
- [ ] Add README updates documenting the inspection contract and the "backend is
      source of truth" rule for node memory.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- `npm run typecheck`
- `npm run test:frontend`
- Cross-layer acceptance check from backend node-memory snapshot to UI-facing
  diagnostics projection

**Status:** Not started

### Milestone 7: Close Out Source Of Truth And Rollout Safety

**Goal:** Leave the roadmap, plan, and touched systems in a reconciled state.

**Tasks:**
- [ ] Reconcile this plan and the roadmap status after each landed slice.
- [ ] Record any residual work that belongs to Phase 3 KV cache or a later
      persistence-focused phase instead of leaving it ambiguous inside Phase 6.
- [ ] Finalize touched READMEs and any needed ADR links.
- [ ] Close the plan with a completion summary that points to the final
      backend-owned node-memory and checkpoint boundaries.

**Verification:**
- Source-of-truth review for roadmap, plan, and touched READMEs
- Final repo checks appropriate to the touched files per testing/tooling
  standards

**Status:** Not started

## Rollout Strategy

The rollout order is intentionally backend-first:

1. Freeze contracts and extract immediate backend seams.
2. Land node-memory and checkpoint ownership in backend Rust.
3. Add graph-edit reconciliation and repeated-invocation reinjection behavior.
4. Integrate keep-alive, scheduler reclaim, unload, and restore behavior.
5. Expose diagnostics and inspection to Tauri/frontend as thin readers.

The rollout must also preserve touched-area health:

- backend refactors land before policy expansion in oversized backend modules
- runtime-boundary changes remain backend-owned and additive
- Tauri stays a thin transport/composition layer
- package/frontend slices extract read-only helpers instead of growing
  state-owning monoliths

This sequence prevents Tauri, the frontend, or bindings from becoming the
accidental owner of unfinished memory semantics.

## Risks And Mitigations

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Output cache and node memory become conflated during implementation | High | Freeze the contract first and keep separate DTOs/modules for cache and memory |
| Restore/reclaim races corrupt checkpoint state | High | Single owner for checkpoint lifecycle plus replay/idempotency tests |
| Oversized touched files become worse before the system stabilizes | High | Require local extraction before adding policy in those files |
| Runtime unload/restore preserves process state but loses logical node state | High | Make scheduler/runtime integration consume explicit session checkpoints |
| Graph edits preserve incompatible node memory and yield incorrect reruns | High | Freeze compatibility classes and add diagnostics plus explicit fallback-to-full-invalidation paths |
| Persistent-session reinvocation and scheduler unload interact in the wrong order | High | Define workflow-session residency states and restore ordering before implementation spreads |
| Frontend or Tauri starts re-deriving memory semantics for display | Medium | Limit them to transport and read-only diagnostics projections |
| Persistent session requirements reveal a broader durability problem than this phase can safely solve | Medium | Keep bounded retention explicit and re-plan durable expansion if needed |

## Execution Notes

Update during implementation:
- 2026-04-18: Original incremental-invalidations-only Phase 6 plan expanded to
  a node-memory and session-checkpoint rollout after clarifying requirements.
- 2026-04-18: Milestone 1 completed. `node-engine` now owns the initial
  workflow-session residency, node-memory, graph-memory-impact, and checkpoint
  contract types behind `engine/session_state.rs`, and
  `pantograph-workflow-service::graph` now forwards an additive
  `workflow_session_state` snapshot contract that marks current graph mutation
  behavior as conservative fallback full invalidation until later
  reconciliation slices land.
- 2026-04-18: Milestone 2 started. Graph snapshot response assembly now lives
  in `graph/session_contract.rs` instead of `graph/session.rs`, and checkpoint-
  summary assembly now lives behind `node-engine::engine::session_state`
  instead of expanding `WorkflowExecutor` inline.
- 2026-04-18: Milestone 2 continued. Graph edit-session runtime/lifecycle state
  now lives behind `graph/session_runtime.rs` instead of keeping active
  execution metadata, queue projection, and run counters inside
  `graph/session.rs`.
- 2026-04-18: Milestone 2 continued. Tauri execution-state lifecycle and
  undo/redo projection now live behind `src-tauri/src/workflow/execution_manager/`
  instead of keeping manager and per-execution state concerns collapsed inside
  `execution_manager.rs`.
- 2026-04-18: Milestone 2 continued. `node-engine` graph mutation, snapshot,
  and restore helpers now live behind `engine/graph_state.rs` instead of
  keeping those Phase 6 insertion points inline inside `engine.rs`.
- 2026-04-18: Milestone 2 continued. Graph edit-session request/response DTOs
  and local undo/session-kind types now live behind `graph/session_types.rs`
  instead of keeping those contract declarations inline at the top of
  `graph/session.rs`.
- 2026-04-18: Milestone 2 continued. Graph embedding-metadata sync, graph-to-
  engine conversion, and node-data merge helpers now live behind
  `graph/session_graph.rs` instead of keeping those shared utility paths inline
  at the bottom of `graph/session.rs`.
- 2026-04-18: Milestone 2 continued. `node-engine` workflow-session residency
  and checkpoint-summary facade wiring now lives behind
  `engine/workflow_session.rs` instead of keeping those Phase 6 executor
  helpers inline inside `engine.rs`.
- 2026-04-18: Milestone 2 continued. Workflow-service session runtime loading,
  preflight caching, and affinity refresh helpers now live behind
  `workflow/session_runtime.rs` instead of keeping that Phase 6 runtime
  coordination inline inside `workflow.rs`.
- 2026-04-18: Milestone 3 started. `node-engine::engine::session_state` now
  owns a backend-only per-session node-memory store keyed by session id and
  node id, `WorkflowExecutor` now exposes thin node-memory inspection/update
  facades through `engine/workflow_session.rs`, and checkpoint summaries now
  report preserved-node counts from that store without implying full
  checkpoint/restore support yet.
- 2026-04-18: Milestone 3 continued. `graph/session_contract.rs` now accepts
  explicit backend-owned workflow-session state projections so graph-session
  responses can surface real node-memory snapshots and checkpoint summaries
  when wiring reaches that boundary, while current callers still preserve the
  fallback active/empty behavior.
- 2026-04-18: Milestone 3 continued. `WorkflowExecutor` now supports explicit
  binding to a logical workflow session id through `engine/workflow_session.rs`
  and `engine/session_state.rs`, creating the backend-owned identity seam that
  later execution-path node-memory reads/writes can use without inferring
  session identity from transport-local execution ids.
- 2026-04-18: Milestone 3 continued. The single-demand executor path now
  projects bound-session node memory from backend cache state after successful
  execution, which records real output snapshots for every cached node reached
  by the run without reopening the already-dirty multi-demand coordinator yet.
- 2026-04-18: Milestone 3 continued. The multi-demand executor path now also
  projects bound-session node memory from backend cache state after successful
  execution, and focused `node-engine` coverage now pins that both executor
  demand paths record backend-owned node-memory snapshots without moving
  ownership into adapters.
- 2026-04-18: Milestone 3 continued. Focused `node-engine` coverage now also
  proves node-memory isolation across workflow sessions and replacement
  semantics across repeated runs against the same bound session, closing the
  Milestone 3 state-isolation test slice without claiming memory-consumption or
  restore behavior that has not landed yet.
- 2026-04-18: Milestone 3 continued. `DemandEngine` now also preserves backend-
  resolved input snapshots alongside cached outputs, and Phase 6 cache
  projection now emits canonical input fingerprints plus inspection metadata
  from those real execution-time inputs for both sequential and bounded-
  parallel multi-demand paths.
- 2026-04-18: Milestone 3 continued. Backend execution now also injects prior
  serializable node-memory snapshots into task inputs under a reserved
  `_node_memory` contract for bound workflow sessions, so reruns can consume
  backend-owned prior memory without moving that state into adapters or Tauri.
- 2026-04-18: Milestone 3 completed. `session_state.rs` now defines an
  explicit indirect runtime/process-state reference contract plus restore
  strategy enum for non-serializable state, cache-derived node-memory
  projection keeps those references empty by default, and the Phase 6 source of
  truth now treats Milestone 3 as complete.
- 2026-04-18: Milestone 4 started. Edit-session graph mutations, undo, and
  redo now project backend-owned node-memory compatibility decisions from
  before/after graph diffs instead of defaulting every mutation response to
  fallback full invalidation when the richer graph context is already known.
- 2026-04-18: Keep-alive and queued workflow-session runs now carry the logical
  `workflow_session_id` through backend run options, and the embedded runtime
  reuses a backend-owned session executor for repeated session runs so
  unchanged inputs can carry forward while selectively updated inputs only
  invalidate the affected suffix. Runtime unload still clears that executor
  until Milestone 5 checkpoint/restore work lands.

## Commit Cadence Notes

- Commit when a logical slice is complete and verified.
- Commit code and documentation for the same slice together.
- Follow `COMMIT-STANDARDS.md`.
- Do not list verification commands in commit messages.

## Re-Plan Triggers

- node-memory compatibility rules prove insufficient for one or more major node
  classes
- durable checkpoint requirements exceed the bounded retention model approved
  here
- scheduler/runtime restore integration requires reopening a boundary that is
  currently frozen by the completed Phase 7 work
- touched frontend or Tauri boundaries cannot remain additive without a larger
  transport contract redesign

## Recommendations

- Treat node memory as the substrate and output cache as an optimization.
  This keeps the architecture aligned with the required persistent and
  inspectable workflow semantics.
- Treat workflow-session residency and checkpointing as part of the same Phase 6
  design problem as graph incrementality.
  Otherwise partial reruns, keep-alive sessions, and scheduler unload/restore
  will each grow their own incompatible state model.
- Keep Phase 6 durable-state ambition bounded.
  Build the workflow-session checkpoint system first; only broaden to richer
  long-term persistence in a later re-plan if the retained-session model proves
  insufficient.

## Completion Summary

### Completed

- None yet.
- Reason: this document is the revised planning baseline for the expanded Phase
  6 scope.
- Revisit trigger: first implementation slice lands.

### Deviations

- The earlier narrower Phase 6 plan is superseded by this broader node-memory
  and checkpoint plan.
- Reason: clarified requirements materially changed scope and affected systems.

### Follow-Ups

- None yet.
- Reason: follow-on work should be identified from implemented slices, not
  guessed now.

### Verification Summary

- Planning/documentation-only change; no code verification run.

### Traceability Links

- Roadmap source of truth:
  `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- Adjacent plans:
  `IMPLEMENTATION-PLAN-pantograph-scheduler-v2.md`
  `IMPLEMENTATION-PLAN-pantograph-phase-7-runtime-adapter-unification.md`
  `IMPLEMENTATION-PLAN-pantograph-metrics-trace-spine.md`
