# Plan: Pantograph Phase 5 Real Workflow Event Contract

## Status
In progress

Last updated: 2026-04-18

## Current Source-of-Truth Summary

This document is the dedicated source of truth for roadmap Phase 5 real
workflow event contract completion. It expands the short Phase 5 section in
`ROADMAP-pantograph-workflow-graph-scheduling-runtime.md` into a
standards-reviewed implementation plan for completing backend-owned workflow
event semantics across engine producers, runtime/application adapters,
transport boundaries, and read-only GUI consumers.

Phase 5 status should be updated here first once implementation begins. The
roadmap should summarize progress and point back to this file rather than
duplicating milestone detail.

## Objective

Complete the backend-owned workflow event contract so Pantograph preserves
interactive, cancellation, graph-mutation, and incremental-execution semantics
end to end without moving business logic into Tauri, Svelte, or language
binding wrappers, while refactoring the immediate insertion points to comply
with the architecture, coding, testing, interop, concurrency, documentation,
tooling, and dependency standards.

## Scope

### In Scope

- Backend-owned workflow event vocabulary and emission behavior in
  `crates/node-engine`
- Runtime/application ownership of event production and error-to-event mapping
  in `crates/pantograph-embedded-runtime` and related backend crates
- Tauri, UniFFI, Rustler, and other adapter/binding transport parity for the
  backend-owned event contract
- Read-only GUI consumption of backend-owned workflow events in
  `packages/svelte-graph`
- Refactors required to keep immediate insertion points compliant before more
  event-contract work lands
- Contract, acceptance, replay/recovery, and documentation updates required by
  the touched boundaries

### Out of Scope

- Scheduler V2 policy changes
- Parallel execution algorithm changes inside `demand_multiple`
- KV cache implementation
- New frontend-owned workflow state, optimistic event handling, or GUI-side
  event synthesis
- Broad graph invalidation algorithm changes that belong to the incremental
  graph execution phase rather than event-contract completion

## Inputs

### Problem

Pantograph now has prerequisite groundwork for `WaitingForInput`,
`IncrementalExecutionStarted`, `GraphModified`, and `Cancelled`, but the event
contract is still incomplete. Emission coverage is not yet consistent across
all interactive and graph-mutation paths, some transport surfaces still depend
on adapter-local reconstruction or classifier logic, and the immediate
insertion points are already oversized enough that continuing without planned
decomposition would violate the standards and reintroduce frontend or adapter
ownership drift.

Without a dedicated completion plan, more event work would likely deepen the
existing large files, preserve boundary-local policy, and leave headless,
binding, and GUI consumers on slightly different interpretations of the same
run lifecycle.

### Constraints

- Business logic and canonical workflow semantics stay in Rust backend crates.
- `src-tauri` remains a transport/composition layer and must not become the
  owner of workflow lifecycle rules, cancellation classification, or graph
  mutation meaning.
- `packages/svelte-graph` remains a read-only event consumer for backend-owned
  execution state, aside from transient UI state.
- Cross-boundary event DTO changes must be append-only where practical and
  updated in every affected language/binding surface in the same logical slice.
- Boundary handlers may validate raw payloads and translate naming/casing, but
  they must not invent missing backend semantics.
- Existing public facades should remain additive unless an explicit API break
  is approved and documented.
- Oversized insertion files require decomposition review before absorbing more
  Phase 5 behavior.

### Public Facade Preservation Note

Phase 5 is a facade-first plan. Existing workflow execution facades, Tauri
commands, and binding entry points remain in place unless an additive
backend-owned contract change is required and documented. The default
implementation choice is extraction and delegation behind the current public
surface rather than API breakage.

### Assumptions

- The current event vocabulary introduced in `node-engine`, Tauri, and
  `packages/svelte-graph` is the frozen starting point rather than a throwaway
  scaffold.
- `pantograph-workflow-service` remains the owner of canonical trace and
  diagnostics projection derived from workflow events.
- Some remaining cancellation and interactive semantics are still represented
  indirectly and need a backend-owned structured form before transport parity
  can be considered complete.
- GUI work for this phase remains read-only consumption of backend-owned
  events; no new TypeScript business-logic owner is acceptable.

### Dependencies

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `IMPLEMENTATION-PLAN-pantograph-metrics-trace-spine.md`
- `crates/node-engine`
- `crates/pantograph-embedded-runtime`
- `crates/pantograph-workflow-service`
- `crates/pantograph-uniffi`
- `crates/pantograph-rustler`
- `src-tauri/src/workflow`
- `packages/svelte-graph`
- Existing README files under the touched `src/` directories

### Affected Structured Contracts

- `node_engine::WorkflowEvent`
- Tauri workflow event transport DTOs in `src-tauri/src/workflow/events.rs`
- Workflow trace and diagnostics projection event mappings
- `packages/svelte-graph` workflow event TypeScript types and execution-state
  consumers
- Buffered/binding event labels and envelopes exposed through UniFFI or other
  wrappers when they mirror workflow events
- Any additive machine-readable cancellation or interactive outcome contracts
  needed to replace boundary-local classifiers

### Affected Persisted Artifacts

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- This implementation plan
- Touched module `README.md` files where ownership or consumer contracts change
- Any ADR required if completion of the workflow event contract reveals a new
  architecture boundary rather than an additive extension
- Any checked-in debug fixtures or event examples added during the work

### Existing Codebase Non-Compliance In Immediate Surroundings

The immediate Phase 5 insertion points already cross decomposition thresholds
from `CODING-STANDARDS.md` and should not absorb more logic unchanged:

- `crates/node-engine/src/engine.rs` is approximately 1398 lines
- `crates/node-engine/src/events.rs` is approximately 545 lines
- `src-tauri/src/workflow/event_adapter.rs` is approximately 609 lines
- `packages/svelte-graph/src/stores/createWorkflowStores.ts` is approximately
  702 lines
- `packages/svelte-graph/src/components/WorkflowToolbar.svelte` is
  approximately 343 lines

Phase 5 must therefore include explicit extraction or decomposition around
event emission, event translation, and frontend execution-state consumption
before more event-contract logic lands in those files.

### Concurrency / Race-Risk Review

- Workflow events can overlap across queued runs, session-backed runs, retries,
  cancellations, and resumed interactive executions, so execution identity must
  remain the primary attribution key.
- Duplicate or replayed terminal events must remain idempotent from the
  backend trace and diagnostics perspective even after new event semantics are
  added.
- Event subscriptions across Tauri channels, GUI listeners, and binding buffers
  must document startup, teardown, and unsubscribe behavior so lifecycle leaks
  do not retain stale runs or duplicate event delivery.
- Interactive wait/resume flows must not split lifecycle ownership between the
  backend producer and a frontend/controller shim.

### Ownership And Lifecycle Note

- `crates/node-engine` owns canonical workflow event vocabulary and producer
  semantics.
- `crates/pantograph-embedded-runtime` owns runtime/application-level
  orchestration that emits or preserves those canonical events.
- `crates/pantograph-workflow-service` owns trace and diagnostics derivation
  from canonical events.
- `src-tauri`, UniFFI, Rustler, and other binding layers remain boundary
  validators and transport adapters only.
- `packages/svelte-graph` owns read-only event consumption and transient UI
  reaction, not canonical execution state.
- Any subscription or buffered transport introduced during implementation must
  state who starts it, who stops it, and how duplicate or restarted consumers
  avoid overlap.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Transport adapters regain ownership of workflow semantics | High | Keep event production and lifecycle meaning in backend crates; adapters only validate and forward |
| Oversized files absorb more event logic and become harder to review or test | High | Make decomposition an explicit early milestone before parity work lands |
| Cancellation or interactive semantics stay string-classified instead of machine-readable | High | Add backend-owned additive outcome contracts and replace boundary-local classification incrementally |
| GUI or binding consumers drift from Rust event contracts | High | Update every affected consumer contract in the same logical slice and add cross-layer acceptance coverage |
| Replay/recovery paths regress under the richer event vocabulary | Medium | Add duplicate, restart, cancellation, and resume acceptance coverage through real boundaries |

## Standards Review Passes

### Pass 1: Plan Structure And Source-of-Truth

Reviewed against:
- `PLAN-STANDARDS.md`
- `templates/PLAN-TEMPLATE.md`

Corrections applied:
- Kept this as a dedicated Phase 5 source-of-truth plan instead of leaving the
  work as a short roadmap subsection.
- Added required objective, scope, risks, done criteria, milestones, and
  re-plan triggers.
- Included affected contracts, persisted artifacts, concurrency review, and
  facade-preservation notes because the work crosses multiple layers.

### Pass 2: Architecture And Ownership

Reviewed against:
- `ARCHITECTURE-PATTERNS.md`
- `CODING-STANDARDS.md`

Corrections applied:
- Locked business semantics to backend Rust crates and kept Tauri/Svelte as
  transport or display layers only.
- Added explicit decomposition work for oversized insertion points before more
  event logic lands there.
- Recorded single-owner lifecycle rules for interactive wait/resume and event
  subscription flows.

### Pass 3: Interop, Bindings, And Boundary Validation

Reviewed against:
- `INTEROP-STANDARDS.md`
- `LANGUAGE-BINDINGS-STANDARDS.md`

Corrections applied:
- Required additive contract updates to move through every affected boundary in
  the same logical slice.
- Restricted Tauri and binding layers to validation, translation, and forward
  transport only.
- Recorded replacement of string-based cancellation interpretation with
  backend-owned machine-readable contracts when producers can emit them.

### Pass 4: Testing And Concurrency

Reviewed against:
- `TESTING-STANDARDS.md`
- `CONCURRENCY-STANDARDS.md`

Corrections applied:
- Required cross-layer acceptance coverage from backend producer through
  adapter and GUI/binding consumer.
- Added replay, recovery, cancellation, resume, and duplicate-event checks as
  explicit milestone work.
- Required execution-identity preservation and subscription cleanup to avoid
  stale or overlapping consumers.

### Pass 5: Documentation, Tooling, And Dependencies

Reviewed against:
- `DOCUMENTATION-STANDARDS.md`
- `TOOLING-STANDARDS.md`
- `DEPENDENCY-STANDARDS.md`

Corrections applied:
- Planned README updates for touched source directories when ownership or
  consumer contracts change.
- Avoided assuming new third-party dependencies; any new dependency must be
  justified separately and kept out of the plan by default.
- Kept the plan focused on repo-native verification and contract traceability
  rather than ad hoc scripts or undocumented fixtures.

## Definition of Done

- Backend producers emit the real workflow event vocabulary consistently across
  the remaining interactive, cancellation, and graph-mutation paths in scope.
- Tauri, UniFFI, Rustler, and GUI consumers transport or consume the same
  backend-owned event semantics without boundary-local policy reconstruction.
- Any remaining cancellation classification needed by adapters is replaced or
  sharply reduced by explicit backend-owned structured outcomes.
- Oversized immediate insertion points touched by this phase are refactored
  enough that new event logic lands in focused modules rather than deepening
  catch-all files.
- README and roadmap traceability reflect the final ownership and consumer
  contract boundaries.

## Milestones

### Milestone 1: Decompose Event Ownership Insertion Points

**Goal:** Create compliant insertion points before landing more event-contract
logic.

**Tasks:**
- [x] Extract `node-engine` workflow event vocabulary and helper logic from the
      current oversized `events.rs` and `engine.rs` hot spots into focused
      modules without changing the public facade.
- [x] Extract Tauri event translation and diagnostics-bridge helpers from the
      current oversized `src-tauri/src/workflow/event_adapter.rs`.
- [x] Extract GUI execution-event handling from the oversized
      `createWorkflowStores.ts` and `WorkflowToolbar.svelte` paths into
      focused read-only consumers or reducers.
- [x] Update touched `README.md` files if the directory boundary or ownership
      explanation changes.

**Verification:**
- `cargo check -p node-engine`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run typecheck`
- Focused no-behavior-change tests for extracted event helpers where practical

**Status:** Completed

### Milestone 2: Freeze The Completion Matrix

**Goal:** Define the remaining event-semantic gaps before implementation
spreads.

**Tasks:**
- [x] Inventory every remaining producer path that should emit
      `WaitingForInput`, `Cancelled`, `GraphModified`, or
      `IncrementalExecutionStarted`, and record which paths are currently
      missing or partial.
- [x] Freeze any additive event payload fields needed for machine-readable
      cancellation, interactive pause metadata, or graph-mutation attribution.
- [x] Record the intended consumer behavior for Tauri, headless bindings, and
      GUI readers so the backend contract remains the source of truth.
- [x] Update roadmap/README references so the new source-of-truth file is the
      authoritative planning entry.

**Verification:**
- Contract review against `INTEROP-STANDARDS.md`
- Serialization and type-alignment checks for any additive DTO changes
- Documentation review against `DOCUMENTATION-STANDARDS.md`

**Status:** Completed

### Frozen Completion Matrix

| Contract | Current backend producer coverage | Current transport / consumer parity | Remaining gap frozen for implementation |
| -------- | --------------------------------- | ----------------------------------- | --------------------------------------- |
| `WaitingForInput` | `node-engine::DemandEngine::demand` emits `WorkflowEvent::WaitingForInput` for unresolved `human-input` nodes before returning `NodeEngineError::WaitingForInput`. | Tauri translates and traces it directly; `packages/svelte-graph` now reduces it through the store helper; UniFFI buffers preserve the canonical event name; Rustler forwards the raw serialized engine event. | Non-streaming embedded/frontend-http workflow runners still surface interactive mismatch through final error envelopes rather than a streamed event contract. Keep that behavior explicit until a dedicated headless streaming boundary exists. |
| `GraphModified` | `WorkflowExecutor` emits `GraphModified` for `mark_modified`, `update_node_data`, `add_node`, `add_edge`, `remove_edge`, and `restore_graph_snapshot`. | Tauri translation, diagnostics projection, and the shared graph GUI all consume the canonical event; UniFFI and Rustler preserve the backend shape without adapter-local reinterpretation. | Additional backend-owned graph mutation paths must either route through `WorkflowExecutor` or add equivalent backend-owned emission; no new adapter-local graph mutation synthesis is permitted. |
| `IncrementalExecutionStarted` | `WorkflowExecutor::demand_multiple` emits the event before executing the requested task set. | Tauri translation, diagnostics projection, and the shared graph GUI consume the canonical event; UniFFI and Rustler preserve it without renaming drift. | Incremental execution outside the current `demand_multiple` path, including future scheduler-driven or parallel demand entry points, must emit the same backend-owned contract instead of relying on diagnostics-only inference. |
| `Cancelled` | `node-engine::WorkflowEvent` now includes canonical `WorkflowCancelled`, and backend Rust emitters can publish explicit cancellation semantics without encoding them as failure strings. Non-streaming workflow-service surfaces now also expose a first-class `cancelled` error envelope instead of flattening user-driven cancellation into `runtime_timeout`. | Tauri translation and diagnostics now consume `WorkflowCancelled` directly without classifying `WorkflowFailed` messages; UniFFI buffered events preserve the canonical cancellation type name; Rustler continues forwarding raw backend event JSON and therefore inherits the backend-owned variant directly; frontend HTTP mapping now preserves the backend-owned `cancelled` error code instead of rewriting it into timeout semantics. | Broaden canonical cancellation emission to any remaining backend-owned producer paths that can still terminate without publishing `WorkflowCancelled`, and extend acceptance coverage across the remaining non-streaming command/binding surfaces that should preserve the new `cancelled` envelope. |

### Frozen Additive Payload Direction

- `Cancelled` now has a backend-owned structured outcome through
  `WorkflowCancelled`; adapters and bindings must forward that contract instead
  of inferring cancellation from free-form error text.
- `WaitingForInput` already carries the minimum machine-readable fields needed
  for the current GUI and diagnostics readers: `workflow_id`, `execution_id`,
  `node_id`, and optional prompt/message text.
- `GraphModified` remains keyed by `workflow_id`, `execution_id`, and
  `dirty_tasks`; future full-graph snapshots remain additive via the existing
  optional transport field rather than changing the engine contract.
- `IncrementalExecutionStarted` remains keyed by `workflow_id`,
  `execution_id`, and the incremental task set; future scheduler metadata must
  be additive and backend-owned if introduced.

### Frozen Consumer Behavior

- Tauri remains a validator/transport layer: it may translate naming and casing
  and project diagnostics state, but it must not own cancellation
  classification because backend-owned cancellation outcomes now exist.
- `packages/svelte-graph` remains a read-only consumer of backend-owned
  workflow events. It may mirror runtime outputs into transient node data for
  execution UX, but it must not synthesize missing workflow events or become a
  second owner of lifecycle semantics.
- UniFFI must continue exposing canonical backend event names for buffered
  workflow events and must adopt the future backend-owned cancelled contract in
  the same slice that introduces it.
- Rustler continues forwarding serialized backend events directly; additive
  backend contract changes therefore remain the only accepted way to expand its
  event language.

### Milestone 3: Complete Backend Event Emission

**Goal:** Make backend producers emit the canonical event contract
consistently.

**Tasks:**
- [ ] Add or normalize event emission for the remaining interactive paths
      beyond the current human-input pause path where backend semantics still
      degrade or flatten.
- [x] Add or normalize graph-mutation and incremental-run events for the
      remaining backend-owned execution and edit-session paths that still do
      not emit them consistently.
- [x] Introduce additive backend-owned structured cancellation outcomes where
      needed so transport boundaries no longer depend on free-form message
      classification.
- [ ] Keep emission ownership in backend crates and avoid adapter-generated
      semantic events.

**Verification:**
- `cargo test -p node-engine`
- `cargo test -p pantograph-embedded-runtime`
- Focused tests for interactive pause, cancellation, graph-mutation, and
  incremental-run producer paths

**Status:** In progress

### Milestone 4: Transport And Consumer Parity

**Goal:** Keep every boundary aligned with the backend-owned event contract.

**Tasks:**
- [ ] Update Tauri workflow event transport, diagnostics mapping, and headless
      readers so they forward additive backend fields without reinterpretation.
- [ ] Update UniFFI, Rustler, and any other affected wrappers so mirrored event
      labels and envelopes remain aligned with the backend contract.
- [ ] Update `packages/svelte-graph` TypeScript contracts and read-only event
      consumers in the same logical slices as Rust DTO changes.
- [ ] Validate boundary payloads at the boundary and keep generated bindings
      or mirrored types synchronized in the same commits.

**Verification:**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Targeted Tauri workflow adapter tests
- Targeted binding tests where the event envelope changes
- `npm run typecheck`

**Status:** Not started

### Milestone 5: Acceptance, Recovery, And Source-of-Truth Close-Out

**Goal:** Prove the completed event contract survives real execution paths and
leave accurate documentation behind.

**Tasks:**
- [ ] Add cross-layer acceptance checks from backend producer through Tauri or
      binding transport into GUI or headless event consumers.
- [ ] Add replay, duplicate-event, restart, cancellation, and resume coverage
      for the completed event vocabulary.
- [ ] Update touched README files, the roadmap, and any ADR references required
      by the standards.
- [ ] Reconcile this plan’s status and completion summary so it remains the
      authoritative source of truth.

**Verification:**
- Targeted Rust crate tests for touched backend packages
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run typecheck`
- Documentation review against `DOCUMENTATION-STANDARDS.md`

**Status:** Not started

## Execution Notes

Update during implementation:
- 2026-04-17: Draft created after roadmap reconciliation and direct inspection
  of `node-engine`, Tauri workflow transport, and `packages/svelte-graph`
  insertion points.
- 2026-04-17: The plan now explicitly includes decomposition work because the
  immediate event-contract insertion points already exceed coding-standards
  thresholds.
- 2026-04-17: First Milestone 1 decomposition slice landed in
  `crates/node-engine/src/events/`, keeping `crate::events` as the stable
  facade while moving workflow-event contract and sink implementations into
  focused modules with dedicated README coverage.
- 2026-04-17: Second Milestone 1 decomposition slice landed in
  `crates/node-engine/src/engine/`, moving graph-mutation and incremental-run
  event helpers behind focused internal modules so later workflow-event
  completion work no longer needs to deepen `engine.rs` directly.
- 2026-04-17: Third Milestone 1 decomposition slice landed in
  `src-tauri/src/workflow/event_adapter/`, separating pure node-engine event
  translation from diagnostics-store bridge logic while keeping the stable
  `workflow::event_adapter` facade intact. The same slice also aligned
  cancellation-shaped node-engine failures with backend trace projection so the
  translated `Cancelled` event and diagnostics status remain consistent.
- 2026-04-17: Fourth Milestone 1 decomposition slice landed in
  `packages/svelte-graph/src/stores/workflowExecutionEvents.ts`, moving
  backend workflow-event reduction out of `WorkflowToolbar.svelte` and into a
  focused read-only store helper with targeted tests and README updates so the
  GUI keeps subscription ownership without re-owning event semantics.
- 2026-04-17: Milestone 2 is now frozen around the current codebase: direct
  producer/consumer coverage is recorded for `WaitingForInput`,
  `GraphModified`, `IncrementalExecutionStarted`, and `Cancelled`, and the
  remaining hard gap is explicit that canonical cancellation semantics still
  stop at Tauri/diagnostics classification instead of originating in
  backend-owned workflow events.
- 2026-04-17: First Milestone 3 backend-emission slice landed by adding
  canonical `node_engine::WorkflowEvent::WorkflowCancelled`, updating backend
  Rust emitters to prefer that explicit outcome over cancellation-shaped
  failure strings, and removing Tauri-side cancellation inference for
  node-engine events while carrying the same contract through diagnostics and
  UniFFI buffered events.
- 2026-04-17: Second Milestone 3 cancellation slice landed in
  `pantograph-workflow-service`, `pantograph-embedded-runtime`, and
  `pantograph-frontend-http-adapter`, adding a first-class backend-owned
  `cancelled` error envelope for non-streaming workflow APIs so user-driven
  cancellation no longer shares the `runtime_timeout` contract with actual
  timeout paths.
- 2026-04-17: Third Milestone 3 backend-emission slice landed in
  `crates/node-engine/src/orchestration/executor.rs`, so orchestration runs
  now emit a canonical backend terminal workflow event even when execution
  exits through real `Err(NodeEngineError)` paths such as missing Start nodes
  or other executor failures. Cancellation-to-terminal-event selection is now
  owned inside the backend executor helper rather than left to adapter
  reconstruction once a cancellable orchestration path is wired through it.
- 2026-04-17: Milestone 5 acceptance coverage also gained a binding-focused
  slice: UniFFI frontend-HTTP workflow-run tests now assert that backend-owned
  `cancelled` envelopes survive the binding boundary intact, and Rustler now
  has a serializer-parity test confirming that its workflow error envelope
  helper preserves the same `cancelled` code/message contract.
- 2026-04-17: Another Milestone 3 backend-emission slice landed in
  `crates/node-engine/src/orchestration/executor.rs`, where orchestration
  data-graph execution now propagates backend-owned `WaitingForInput` and
  `Cancelled` outcomes instead of flattening them into the generic data-node
  error handle. Canonical interactive and cancellation semantics therefore
  remain owned by backend producers when subgraph execution pauses or stops.
- 2026-04-17: The follow-on backend slice landed in
  `crates/pantograph-embedded-runtime/src/lib.rs`, so embedded data-graph
  execution now returns real `WaitingForInput` outcomes instead of converting
  them into synthetic `*.error` outputs. The Tauri orchestration adapter was
  reduced to a passthrough for that backend-owned result instead of rewriting
  it at the boundary.
- 2026-04-17: Milestone 5 transport coverage was tightened in
  `src-tauri/src/workflow/headless_workflow_commands.rs` so the headless error
  envelope helper is now explicitly pinned for the backend-owned interactive
  `invalid_request` contract emitted by non-streaming workflow runs.
- 2026-04-17: Binding acceptance coverage now also pins the same interactive
  `invalid_request` envelope through the UniFFI frontend-HTTP workflow-run
  surface and the Rustler workflow-error serializer helper, matching the
  previously-added `cancelled` envelope coverage on those boundaries.
- 2026-04-18: Another Milestone 3 backend-emission slice landed in
  `pantograph-workflow-service::graph`, where edit-session graph mutation
  responses now carry an additive backend-owned canonical `GraphModified`
  event with deterministic dirty-task ordering and session-scoped identity
  instead of leaving graph-mutation semantics to adapter or GUI inference.
  UniFFI runtime coverage now also pins that additive response contract so the
  binding-facing JSON surface can observe it before Tauri transport parity
  work begins in Milestone 4.

## Commit Cadence Notes

- Commit when a logical slice is complete and verified.
- Keep decomposition, contract-freeze, backend-emission, transport-parity, and
  acceptance/doc slices separate where practical.
- Follow commit format/history cleanup rules from `COMMIT-STANDARDS.md`.

## Optional Subagent Assignment

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| None | None | None | Revisit only if Phase 5 implementation is split into disjoint backend and GUI consumer slices |

## Re-Plan Triggers

- A required event semantic cannot be represented additively and would force a
  breaking wire-contract change.
- A remaining producer path turns out to belong to scheduler or incremental
  graph execution ownership instead of Phase 5.
- Decomposition of the oversized insertion points reveals a different module
  boundary than assumed here.
- A binding surface cannot preserve the backend contract without a separate ADR
  or codegen change.

## Recommendations (Only If Better Option Exists)

- Recommendation 1: Land the decomposition work before semantic completion
  work. This keeps Phase 5 from deepening already-oversized modules and makes
  event-contract review substantially easier.
- Recommendation 2: Treat machine-readable cancellation outcomes as part of the
  contract-completion work, not as a later cleanup. That change reduces the
  amount of transport-specific interpretation that would otherwise linger.

## Completion Summary

### Completed

- None yet. Phase 5 implementation has not started from this plan.

### Deviations

- None yet.

### Follow-Ups

- None yet. Follow-ups will be recorded here once implementation lands.

### Verification Summary

- Reviewed `PLAN-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`,
  `CODING-STANDARDS.md`, `INTEROP-STANDARDS.md`,
  `LANGUAGE-BINDINGS-STANDARDS.md`, `TESTING-STANDARDS.md`,
  `CONCURRENCY-STANDARDS.md`, `DOCUMENTATION-STANDARDS.md`, and
  `DEPENDENCY-STANDARDS.md`
- Inspected current insertion points in `crates/node-engine`,
  `src-tauri/src/workflow`, and `packages/svelte-graph`

### Traceability Links

- Module README updated: N/A
- ADR added/updated: N/A
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A

## Brevity Note

Keep Phase 5 implementation slices small. Expand detail only where event
ownership, interop stability, or replay/recovery risk requires it.
