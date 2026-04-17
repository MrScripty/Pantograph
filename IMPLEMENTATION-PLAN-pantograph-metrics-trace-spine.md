# Plan: Pantograph Metrics And Trace Spine

## Objective

Add a backend-owned metrics and trace spine for workflow execution so Pantograph
can measure node execution, queue wait, runtime lifecycle, and event delivery
before implementing more aggressive scheduling or incremental execution policy.

## Scope

### In Scope

- Backend-owned execution metrics for `node-engine`, workflow service, and
  runtime lifecycle
- Stable trace and metrics contracts consumed by Tauri/UI without frontend-owned
  business logic
- First-class inspection/snapshot APIs for workflow runs, queue timing, and
  runtime residency decisions
- Event-adapter updates needed to preserve trace semantics across
  backend-to-frontend transport
- Contract, unit, and integration coverage for metrics and trace emission
- README/ADR updates required by touched directories and structured producer
  contracts

### Out of Scope

- Scheduler V2 policy changes
- Parallel execution changes in `demand_multiple`
- KV cache implementation
- New frontend analytics logic or optimistic metrics derivation
- Long-term telemetry export, remote observability, or SaaS monitoring
  infrastructure

## Inputs

### Problem

Pantograph still lacks a canonical, backend-owned spine for answering basic
execution questions:

- How long did a run wait in queue?
- Which nodes consumed the most wall time?
- How much time was spent in runtime warmup versus execution?
- Which scheduler/runtime decisions were made and why?
- Which workflow events were emitted, adapted, dropped, or coalesced?

Without that spine, later roadmap steps such as Scheduler V2, incremental graph
execution, and runtime adapter unification will be harder to validate and
easier to regress.

### Constraints

- Business logic must stay in Rust backend crates, not TypeScript/Svelte.
- `src-tauri` must remain an adapter/composition layer, not the owner of
  workflow policy.
- Contracts added for metrics/traces should be append-only where practical.
- Existing workflow/session facades should be preserved unless a deliberate API
  break is approved.
- Diagnostics must remain machine-consumable; log scraping is not an acceptable
  primary interface.
- New Rust-to-Tauri trace DTOs must freeze serde tag/casing rules before
  adapter work begins, and matching TypeScript consumer contracts must be
  updated in the same logical slice when exposed across the boundary.
- Any new Tauri commands for trace snapshots must validate input at the boundary
  before delegating to backend services.
- The implementation must follow `PLAN-STANDARDS.md`,
  `ARCHITECTURE-PATTERNS.md`, `TESTING-STANDARDS.md`,
  `DOCUMENTATION-STANDARDS.md`, `INTEROP-STANDARDS.md`, and
  `TOOLING-STANDARDS.md`.

### Assumptions

- The next roadmap step is `Metrics/trace spine`, based on roadmap dependency
  order and current repo state.
- Existing Rust-owned diagnostics/session refactors are the base layer to build
  on rather than replace.
- Initial trace retention can be in-memory and bounded; durable persistence is
  not required for milestone one.
- Internal inspection APIs may be Tauri/debug-only at first as long as the
  contracts are backend-owned and versioned explicitly.

### Dependencies

- `crates/node-engine`
- `crates/pantograph-workflow-service`
- `crates/pantograph-embedded-runtime`
- `crates/inference`
- `src-tauri/src/workflow`
- Existing diagnostics store and workflow projection paths already moved behind
  Rust
- Existing contract tests under
  `crates/pantograph-workflow-service/tests/contract.rs`
- README updates in:
  - `crates/node-engine/src/README.md`
  - `crates/pantograph-workflow-service/src/README.md`
  - `crates/pantograph-embedded-runtime/src/README.md`
  - `src-tauri/src/workflow/README.md`
- ADR update if the metrics/trace boundary changes become architectural rather
  than additive

### Affected Structured Contracts

- `WorkflowTraceStatus`
- `WorkflowTraceNodeStatus`
- `WorkflowTraceQueueMetrics`
- `WorkflowTraceRuntimeMetrics`
- `WorkflowTraceNodeRecord`
- `WorkflowTraceSummary`
- `WorkflowTraceSnapshotRequest`
- `WorkflowTraceSnapshotResponse`
- Any later additive Tauri command or workflow event payloads that expose these
  contracts to the existing GUI diagnostics surface

### Affected Persisted Artifacts

- None for Milestone 1; the initial trace spine is contract- and memory-owned
  only
- Any later checked-in JSON fixtures or trace snapshot examples must be treated
  as structured artifacts and validated with the repo tooling required by
  `TOOLING-STANDARDS.md`

### Concurrency / Race-Risk Review

- Trace aggregation will consume overlapping workflow/session activity, so all
  producer-facing contracts must carry stable execution or session identifiers.
- Bounded in-memory trace retention must have a single owner responsible for
  append, eviction, and cleanup so overlapping runs cannot race retention
  policy.
- Replay, cancellation, and retry paths must remain idempotent from the trace
  reader’s perspective even if upstream event delivery duplicates a producer
  event.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Metrics logic leaks into Tauri/Svelte adapters | High | Keep contracts and aggregation in Rust crates; adapter only forwards snapshots/events |
| Event timing becomes nondeterministic under concurrent runs | High | Require execution/session IDs on every metric-bearing event and test concurrent attribution |
| Trace collection adds lock contention or slows execution | High | Use append-only, bounded, low-contention collectors and benchmark overhead |
| Contract drift between engine, service, runtime, and UI adapters | High | Freeze serde/tag/casing rules first and update matching TS consumers in the same slice |
| Oversized diagnostics modules absorb multiple responsibilities | Medium | Perform decomposition review when files exceed size/responsibility thresholds and extract by ownership boundary |
| Recovery/cancel/retry paths produce inconsistent traces | Medium | Add replay/recovery/idempotency checks and duplicate-event handling verification |
| JSON fixtures or trace snapshot examples drift from producer contracts | Medium | Add staged validation hooks for any checked-in schema-backed trace artifacts |

## Definition of Done

- Every workflow run has a backend-owned trace summary with total duration,
  queue wait, per-node timing, and runtime lifecycle timing.
- Queue wait and runtime warmup are inspectable without log scraping.
- Trace/event payloads remain attributable under overlapping workflow runs.
- Tauri/UI consumers read backend snapshots rather than deriving metrics
  locally.
- Contract, unit, integration, and cross-layer acceptance coverage exist for
  trace creation, aggregation, transport adaptation, and recovery/cancel paths.
- Touched module boundaries have README updates or ADR coverage required by the
  standards.

## Milestones

### Milestone 1: Freeze Contracts And Ownership

**Goal:** Define the metrics/trace ownership model and DTOs before implementation
spreads.

**Tasks:**
- [x] Define engine-level metric/event DTOs and run-level trace summary
      contracts.
- [x] Define runtime lifecycle metric DTOs and scheduler/queue timing payloads.
- [x] Freeze serde tag/casing and field semantics for any Rust-to-Tauri trace
      DTOs before adapter work starts.
- [x] Add request-shape validation helpers for trace snapshot filters so later
      Tauri commands can validate at the boundary without inventing frontend
      policy.
- [x] Record facade-preservation decision: preserve current workflow/session
      public facades and extend them additively.
- [x] Record lifecycle ownership for trace buffers, retention, and cleanup.
- [x] Identify touched directories that require README or ADR updates and record
      the expected traceability artifacts up front.

**Verification:**
- Architecture review against `ARCHITECTURE-PATTERNS.md`
- Interop contract review against `INTEROP-STANDARDS.md`
- Documentation traceability review against `DOCUMENTATION-STANDARDS.md`
- Contract serialization tests for new DTO casing/tag behavior

**Status:** Completed

### Milestone 2: Engine Metrics Foundation

**Goal:** Make `node-engine` emit canonical per-node and per-run timing
records.

**Tasks:**
- [x] Add node execution timing capture around task demand/execution paths.
- [x] Add per-run aggregation hooks for total wall time, node counts, and
      failure/cancel completion states.
- [x] Ensure engine events include stable execution/task identifiers needed for
      trace attribution.
- [x] Add low-overhead internal metrics structures rather than ad hoc logging.
- [x] Perform decomposition review if any new metrics module crosses file-size
      or responsibility thresholds from `CODING-STANDARDS.md`.

**Verification:**
- `cargo test -p node-engine`
- Focused unit tests for node timing emission and failure/cancel paths
- Overhead review to ensure metrics collection does not materially alter
  baseline behavior

**Status:** Completed

### Milestone 3: Workflow Service Trace Aggregation

**Goal:** Aggregate engine timing into workflow/session traces with queue and
scheduling context.

**Tasks:**
- [x] Add run/session trace collectors in
      `crates/pantograph-workflow-service`.
- [x] Record queue admission time, dequeue time, execution start, execution
      finish, and cancel/unload transitions.
- [x] Add snapshot/read APIs for recent run traces and queue timing summaries.
- [x] Keep aggregation in the service/application layer rather than Tauri
      transport code.
- [x] Add duplicate-event/idempotency safeguards where aggregation consumes
      repeatable or replayable event streams.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- Focused tests for queue-wait timing, run lifecycle aggregation, bounded
  history retention, and duplicate-event handling
- Concurrency review against `CONCURRENCY-STANDARDS.md`

**Status:** Completed

### Milestone 4: Runtime Lifecycle Metrics

**Goal:** Measure runtime warmup, readiness, reuse, and teardown in
backend-owned runtime layers.

**Tasks:**
- [x] Add runtime lifecycle timing hooks in
      `crates/pantograph-embedded-runtime` and `crates/inference`.
- [x] Capture warmup/load/reuse/failure/eviction decision reasons as structured
      records.
- [x] Attach runtime lifecycle metrics to the workflow trace summary without
      moving runtime policy into adapters.
- [x] Ensure runtime metrics are attributable to workflow run/session
      identifiers where applicable.
- [x] Perform decomposition review if runtime trace modules exceed size or
      responsibility thresholds.

**Verification:**
- `cargo test -p pantograph-embedded-runtime`
- Targeted tests for runtime warmup/reuse/failure timing paths
- Contract review confirming runtime decision payloads are machine-consumable

**Status:** Completed

### Milestone 5: Adapter And Diagnostics Integration

**Goal:** Expose backend-owned traces through Tauri without reintroducing
frontend business logic.

**Tasks:**
- [x] Extend `src-tauri/src/workflow/event_adapter.rs` and related commands to
      forward trace/metrics snapshots.
- [x] Preserve event semantics needed for diagnostics views and future scheduler
      work.
- [x] Keep any GUI work read-only: render backend trace state, do not compute
      or reconcile it in TypeScript.
- [x] Update matching TypeScript consumer contracts in the same slice for any
      new Tauri-exposed DTOs.
- [x] Validate any new snapshot/filter request payloads at the Tauri command
      boundary.

**Verification:**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Targeted Tauri contract tests for snapshot/event transport
- `npm run typecheck` for matching TS consumer contracts

**Status:** Completed

### Milestone 6: Hardening, Docs, And Acceptance Coverage

**Goal:** Close the phase with standards-compliant tests and documentation.

**Tasks:**
- [x] Add cross-layer acceptance checks from workflow run start through trace
      snapshot consumption.
- [x] Add cancellation, retry/recovery, and replay/idempotency checks for trace
      consistency.
- [x] Update README files for touched backend directories with ownership and
      contract notes.
- [x] Add or update ADR/docs if metrics ownership or event contracts materially
      change architecture.
- [x] If JSON fixtures or trace snapshot examples are committed, add fast
      validation per `TOOLING-STANDARDS.md`.

**Verification:**
- Run all targeted Rust crate tests touched by the slice
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run typecheck` and targeted frontend checks if UI diagnostics readers
  change
- Documentation review against `DOCUMENTATION-STANDARDS.md`
- Artifact validation review against `TOOLING-STANDARDS.md`

**Status:** Completed

## Execution Notes

Update during implementation:
- 2026-04-12: Plan created for roadmap step 1 after completing Rust ownership
  refactors for Puma-Lib hydration and dependency-environment actions.
- 2026-04-12: Plan updated to explicitly freeze interop contracts, documentation
  deliverables, replay/recovery coverage, decomposition-review checkpoints, and
  artifact validation expectations before implementation begins.
- 2026-04-12: The first implementation slice is contracts-and-ownership only in
  `pantograph-workflow-service`, with README traceability updates before engine
  or adapter behavior expands.
- 2026-04-12: Milestone 1 implementation includes request-boundary validation
  in the Rust contract layer so later Tauri commands do not need to invent
  validation semantics locally.
- 2026-04-12: First implementation slice landed in
  `crates/pantograph-workflow-service` with backend-owned trace DTOs,
  snake_case contract tests, request-filter validation, and README ownership
  updates.
- 2026-04-12: Second implementation slice added a backend-owned
  `WorkflowTraceStore` plus canonical trace-event and graph-context types in
  `pantograph-workflow-service`, leaving Tauri adapter cutover as the next
  isolated slice.
- 2026-04-12: Third implementation slice changed
  `src-tauri/src/workflow/diagnostics.rs` into a projection/overlay adapter
  over `WorkflowTraceStore`, preserving the existing GUI diagnostics contract
  while removing Tauri ownership of canonical run/node timing state.
- 2026-04-12: Fourth implementation slice added a direct
  `workflow_get_trace_snapshot` inspection command plus TypeScript contract
  mirrors so backend-owned trace reads no longer depend on the diagnostics
  projection shape.
- 2026-04-12: Fifth implementation slice threaded scheduler/runtime snapshot
  payloads through `WorkflowTraceEvent` and moved snapshot interpretation into
  `pantograph-workflow-service`, so queue observation timing and runtime
  readiness decisions are recorded in the canonical Rust trace store instead of
  remaining adapter-local.
- 2026-04-12: Sixth implementation slice added backend-owned runtime lifecycle
  snapshots in `crates/inference`, threaded those facts through Tauri workflow
  runtime snapshot events, and merged authoritative warmup/instance metadata
  into the canonical trace store.
- 2026-04-12: Seventh implementation slice tightened scheduler trace
  attribution by carrying `session_id` through `WorkflowTraceEvent` and
  preferring queue items that match the active execution/session before falling
  back to session-level backlog state.
- 2026-04-12: Eighth implementation slice added authoritative enqueue/dequeue
  timestamps to `WorkflowSessionQueueItem`, populated them in workflow session
  scheduling, mirrored the additive contract to TypeScript, and taught the
  trace store to prefer those producer timestamps over snapshot observation
  time.
- 2026-04-12: Ninth implementation slice extended authoritative queue timing
  to graph edit sessions so direct edit-run scheduler snapshots emit concrete
  start timestamps instead of placeholder `None` values.
- 2026-04-12: Tenth implementation slice taught the trace store to preserve
  authoritative enqueue timestamps when the first scheduler observation is
  already `Running`, and added diagnostics acceptance coverage so backend-owned
  queue timestamps are verified through the Tauri diagnostics adapter into the
  canonical trace store.
- 2026-04-12: Eleventh implementation slice added diagnostics-store helpers for
  runtime and scheduler snapshot events, then moved headless snapshot capture
  and the interactive runtime-error path onto those helpers so overlay updates
  cannot diverge from canonical trace recording when execution identity is
  available.
- 2026-04-12: Twelfth implementation slice added a backend-owned
  `trace_execution_id` to scheduler snapshots so adapters can attribute
  snapshot observations to the active or uniquely-visible queued run without
  guessing from `session_id`.
- 2026-04-12: Thirteenth implementation slice added explicit service and
  contract coverage for `trace_execution_id`, including the ambiguous
  multi-pending case where the backend must leave the field unset.
- 2026-04-12: Fourteenth implementation slice extracted the headless scheduler
  snapshot attribution path into a small Rust helper and added adapter-level
  tests that prove it records traces under `trace_execution_id` when present
  and falls back to the requested session identity on scheduler errors.
- 2026-04-12: Fifteenth implementation slice extracted the headless runtime
  snapshot recording path into a matching helper and added adapter-level tests
  that freeze the two intended behaviors: identified executions append
  canonical runtime trace events, while execution-less diagnostics reads only
  refresh the overlay snapshot.
- 2026-04-12: Sixteenth implementation slice updated the workflow-service and
  Tauri workflow READMEs so the new `trace_execution_id` contract and
  adapter-side fallback rules are documented at the ownership boundaries where
  later scheduler/runtime work will build on them.
- 2026-04-12: Seventeenth implementation slice added an adapter-level joining
  test proving the headless scheduler and runtime snapshot helpers merge onto
  the same canonical trace when the backend provides a shared
  `trace_execution_id`.
- 2026-04-12: Eighteenth implementation slice surfaced the backend-owned
  scheduler `trace_execution_id` into the diagnostics projection and existing
  GUI scheduler view so operators can inspect the current trace target without
  frontend inference.
- 2026-04-13: Twenty-ninth implementation slice added canonical `session_id`
  attribution to workflow trace summaries and diagnostics projections so
  session-scoped trace reads remain correct even when queued/run execution ids
  diverge from the scheduling session identity.
- 2026-04-13: Thirtieth implementation slice added headless workflow adapter
  coverage for backend-owned `session_id` attribution so direct trace snapshot
  reads and diagnostics projections are exercised at the Tauri boundary.
- 2026-04-13: Thirty-first implementation slice added headless diagnostics
  clear-history helper coverage so the adapter reader path proves it clears run
  history while preserving backend-owned scheduler and runtime snapshots.
- 2026-04-13: Thirty-second implementation slice added direct Tauri channel
  transport coverage for workflow event emission so the adapter `send()` path
  is exercised with a real IPC channel instead of only translation helpers.
- 2026-04-13: Thirty-third implementation slice added backend-owned llama.cpp
  runtime reuse detection so identical sidecar mode/model/device starts return
  structured reuse outcomes instead of forcing a fresh runtime start.
- 2026-04-17: Thirty-fourth implementation slice moved ordinary node-engine
  trace timestamp capture into
  `pantograph_workflow_service::WorkflowTraceStore`, so the Tauri workflow
  event adapter now forwards execution events into backend-owned trace timing
  instead of stamping canonical run/node chronology locally.
- 2026-04-17: Thirty-fifth implementation slice made `node-engine`
  `WorkflowEvent` carry additive backend-owned `occurred_at_ms` timestamps and
  updated the Tauri workflow adapter to preserve those producer timestamps when
  projecting diagnostics overlays into the backend trace spine.
- 2026-04-17: Thirty-sixth implementation slice hardened
  `pantograph_workflow_service::WorkflowTraceStore` so duplicate terminal
  run/node events are treated as idempotent replay, preserving first terminal
  timestamps and durations inside the backend trace owner instead of pushing
  de-duplication policy into adapters.
- 2026-04-17: Thirty-seventh implementation slice added direct Tauri workflow
  adapter coverage proving duplicate terminal node-engine events still resolve
  to one canonical trace outcome because replay/idempotency remains owned by
  the backend trace store rather than the transport layer.
- 2026-04-17: Thirty-eighth implementation slice taught the backend-owned
  diagnostics store to reset execution overlays when the canonical trace resets
  into a new attempt, preventing stale node progress/event history from
  leaking across retry-shaped `RunStarted` transitions.
- 2026-04-17: Thirty-ninth implementation slice added direct Tauri workflow
  adapter coverage proving restarted `node-engine` executions preserve that
  backend-owned reset behavior through transport adaptation and do not retain
  stale diagnostics overlay state from the prior attempt.
- 2026-04-17: Fortieth implementation slice added cancellation-to-restart
  coverage across the workflow trace store, diagnostics store, and Tauri
  workflow adapter so retry semantics are frozen for cancelled executions as
  well as failed or completed attempts.

## Commit Cadence Notes

- Commit when a logical slice is complete and verified.
- Keep contract-definition, engine metrics, service aggregation, runtime
  metrics, and adapter integration in separate atomic commits where practical.
- Follow commit format/history cleanup rules from `COMMIT-STANDARDS.md`.

## Optional Subagent Assignment

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| None | None | None | Revisit only if engine metrics and docs can proceed independently without boundary risk |

## Re-Plan Triggers

- Metrics collection overhead is high enough to change engine/service
  sequencing.
- Existing diagnostics contracts cannot support additive extension.
- Runtime lifecycle metrics require new ownership boundaries not assumed here.
- Queue timing or event attribution proves ambiguous under concurrent runs.
- A documentation/ADR requirement reveals an architectural break rather than an
  additive change.
- Checked-in trace fixtures or snapshot payloads require new tooling hooks not
  covered by current repo validation.

## Recommendations (Only If Better Option Exists)

- Recommendation 1: Land Milestone 1 as a contracts-only commit before any
  producer implementation. This reduces cross-layer drift and makes later
  review of metrics semantics much easier.
- Recommendation 2: Defer any metrics GUI beyond thin read-only diagnostics
  consumption until backend contracts stabilize. This keeps the roadmap step
  compliant with backend-owned-data rules and avoids another TS refactor later.

## Completion Summary

### Completed

- Milestone 1 completed: contracts, ownership boundaries, serde/tagging rules,
  trace snapshot request validation, retention ownership, and traceability
  notes were frozen before the later producer and adapter slices expanded.
- Milestone 2 completed: `node-engine` now emits additive producer-owned run
  and task timing through canonical workflow events, including stable execution
  and task identifiers plus backend-owned timestamps.
- Milestone 3 completed: `pantograph-workflow-service` owns canonical run
  trace aggregation, queue timing, snapshot reads, session attribution, and
  duplicate/replay safeguards for terminal and restart-shaped transitions.
- Milestone 4 completed: runtime lifecycle metrics, reuse facts, warmup
  timings, and structured decision reasons now flow through backend-owned
  inference/runtime contracts into the canonical trace summary.
- Milestone 5 completed: Tauri workflow commands, diagnostics, and event
  adapters now transport backend-owned trace/runtime data read-only, preserve
  attribution semantics, and validate trace snapshot requests at the boundary.
- Milestone 6 completed: targeted recovery, cancellation, replay, clear-
  history, duplicate-terminal, restart, and cancellation-restart acceptance
  coverage has been added across the service, diagnostics store, and adapter
  layers, with touched README/plan documentation kept in sync.

### Deviations

- Current runtime producers do not yet emit true warmup/reuse/instance-lifetime
  facts for every runtime path; the inference gateway and dedicated embedding
  sidecar now record authoritative lifecycle state, but remaining adapter-
  specific runtime paths still need to converge on the same producer contract.

### Follow-Ups

- Extend runtime lifecycle producers beyond the current inference-gateway,
  backend-start-outcome, embedding-sidecar, PyTorch loaded-model reuse, and
  Python-sidecar registry-observation paths so every remaining runtime host
  populates the same authoritative
  `WorkflowTraceRuntimeMetrics` fields.
- Continue converging producer metadata beyond shared identity/alias mapping so
  warmup, reuse, degraded-state, and reconnect facts are emitted through the
  same registry-ready capability contract family across all remaining hosts.
- Extend direct command-path acceptance coverage beyond the current embedding
  runtime, scheduler, trace snapshot, and diagnostics projection refresh paths
  so the remaining diagnostics reader and transport paths are also exercised at
  the adapter boundary.
- Extend queue attribution beyond current execution/session matching so traces
  can distinguish concurrent queued runs more precisely when richer run
  identity surfaces are available from producers.
- Replace the current centralized cancellation-message classifier with a fully
  machine-readable producer contract once every workflow producer can emit
  explicit cancellation outcomes directly.
- Decide whether later detailed metrics inspection belongs in the existing
  diagnostics command surface or a dedicated trace/metrics module.
- Extend snapshot-path acceptance coverage beyond current diagnostics-store
  helper tests so headless command adapters are exercised directly.

### Verification Summary

- `cargo test -p pantograph-workflow-service contract`
- `cargo test -p pantograph-workflow-service trace`
- `cargo test -p pantograph-workflow-service workflow_session_queue_items_include_authoritative_timestamps`
- `cargo test -p inference gateway`
- `cargo test --manifest-path src-tauri/Cargo.toml diagnostics::`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run typecheck`
- `cargo check -p pantograph-workflow-service`
- `cargo test --manifest-path src-tauri/Cargo.toml diagnostics::`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run typecheck`
- Reviewed `PLAN-STANDARDS.md`, `PLAN-TEMPLATE.md`,
  `ARCHITECTURE-PATTERNS.md`, `CODING-STANDARDS.md`,
  `TESTING-STANDARDS.md`, `DOCUMENTATION-STANDARDS.md`,
  `CONCURRENCY-STANDARDS.md`, `TOOLING-STANDARDS.md`,
  `INTEROP-STANDARDS.md`, and the Pantograph roadmap document

### Traceability Links

- Module README updated: N/A
- ADR added/updated: N/A
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A

## Brevity Note

Keep implementation slices small. Expand only where concurrency risk, contract
stability, or ownership clarity requires more detail.
