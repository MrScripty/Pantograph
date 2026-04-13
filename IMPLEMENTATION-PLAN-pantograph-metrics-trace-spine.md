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
- [ ] Define engine-level metric/event DTOs and run-level trace summary
      contracts.
- [ ] Define runtime lifecycle metric DTOs and scheduler/queue timing payloads.
- [ ] Freeze serde tag/casing and field semantics for any Rust-to-Tauri trace
      DTOs before adapter work starts.
- [ ] Add request-shape validation helpers for trace snapshot filters so later
      Tauri commands can validate at the boundary without inventing frontend
      policy.
- [ ] Record facade-preservation decision: preserve current workflow/session
      public facades and extend them additively.
- [ ] Record lifecycle ownership for trace buffers, retention, and cleanup.
- [ ] Identify touched directories that require README or ADR updates and record
      the expected traceability artifacts up front.

**Verification:**
- Architecture review against `ARCHITECTURE-PATTERNS.md`
- Interop contract review against `INTEROP-STANDARDS.md`
- Documentation traceability review against `DOCUMENTATION-STANDARDS.md`
- Contract serialization tests for new DTO casing/tag behavior

**Status:** In progress

### Milestone 2: Engine Metrics Foundation

**Goal:** Make `node-engine` emit canonical per-node and per-run timing
records.

**Tasks:**
- [ ] Add node execution timing capture around task demand/execution paths.
- [ ] Add per-run aggregation hooks for total wall time, node counts, and
      failure/cancel completion states.
- [ ] Ensure engine events include stable execution/task identifiers needed for
      trace attribution.
- [ ] Add low-overhead internal metrics structures rather than ad hoc logging.
- [ ] Perform decomposition review if any new metrics module crosses file-size
      or responsibility thresholds from `CODING-STANDARDS.md`.

**Verification:**
- `cargo test -p node-engine`
- Focused unit tests for node timing emission and failure/cancel paths
- Overhead review to ensure metrics collection does not materially alter
  baseline behavior

**Status:** Not started

### Milestone 3: Workflow Service Trace Aggregation

**Goal:** Aggregate engine timing into workflow/session traces with queue and
scheduling context.

**Tasks:**
- [ ] Add run/session trace collectors in
      `crates/pantograph-workflow-service`.
- [ ] Record queue admission time, dequeue time, execution start, execution
      finish, and cancel/unload transitions.
- [ ] Add snapshot/read APIs for recent run traces and queue timing summaries.
- [ ] Keep aggregation in the service/application layer rather than Tauri
      transport code.
- [ ] Add duplicate-event/idempotency safeguards where aggregation consumes
      repeatable or replayable event streams.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- Focused tests for queue-wait timing, run lifecycle aggregation, bounded
  history retention, and duplicate-event handling
- Concurrency review against `CONCURRENCY-STANDARDS.md`

**Status:** Not started

### Milestone 4: Runtime Lifecycle Metrics

**Goal:** Measure runtime warmup, readiness, reuse, and teardown in
backend-owned runtime layers.

**Tasks:**
- [ ] Add runtime lifecycle timing hooks in
      `crates/pantograph-embedded-runtime` and `crates/inference`.
- [ ] Capture warmup/load/reuse/failure/eviction decision reasons as structured
      records.
- [ ] Attach runtime lifecycle metrics to the workflow trace summary without
      moving runtime policy into adapters.
- [ ] Ensure runtime metrics are attributable to workflow run/session
      identifiers where applicable.
- [ ] Perform decomposition review if runtime trace modules exceed size or
      responsibility thresholds.

**Verification:**
- `cargo test -p pantograph-embedded-runtime`
- Targeted tests for runtime warmup/reuse/failure timing paths
- Contract review confirming runtime decision payloads are machine-consumable

**Status:** Not started

### Milestone 5: Adapter And Diagnostics Integration

**Goal:** Expose backend-owned traces through Tauri without reintroducing
frontend business logic.

**Tasks:**
- [ ] Extend `src-tauri/src/workflow/event_adapter.rs` and related commands to
      forward trace/metrics snapshots.
- [ ] Preserve event semantics needed for diagnostics views and future scheduler
      work.
- [ ] Keep any GUI work read-only: render backend trace state, do not compute
      or reconcile it in TypeScript.
- [ ] Update matching TypeScript consumer contracts in the same slice for any
      new Tauri-exposed DTOs.
- [ ] Validate any new snapshot/filter request payloads at the Tauri command
      boundary.

**Verification:**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Targeted Tauri contract tests for snapshot/event transport
- `npm run typecheck` for matching TS consumer contracts

**Status:** Not started

### Milestone 6: Hardening, Docs, And Acceptance Coverage

**Goal:** Close the phase with standards-compliant tests and documentation.

**Tasks:**
- [ ] Add cross-layer acceptance checks from workflow run start through trace
      snapshot consumption.
- [ ] Add cancellation, retry/recovery, and replay/idempotency checks for trace
      consistency.
- [ ] Update README files for touched backend directories with ownership and
      contract notes.
- [ ] Add or update ADR/docs if metrics ownership or event contracts materially
      change architecture.
- [ ] If JSON fixtures or trace snapshot examples are committed, add fast
      validation per `TOOLING-STANDARDS.md`.

**Verification:**
- Run all targeted Rust crate tests touched by the slice
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run typecheck` and targeted frontend checks if UI diagnostics readers
  change
- Documentation review against `DOCUMENTATION-STANDARDS.md`
- Artifact validation review against `TOOLING-STANDARDS.md`

**Status:** Not started

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

- Plan updated with standards corrections and Milestone 1 scope
- First Milestone 1 slice implemented in `pantograph-workflow-service`
- Second Milestone 1 slice implemented in `pantograph-workflow-service`
- Third Milestone 1 slice implemented in `src-tauri/src/workflow`
- Fourth Milestone 1 slice implemented across `src-tauri` and service contract
  mirrors
- Fifth Milestone 1 slice implemented across `pantograph-workflow-service` and
  `src-tauri/src/workflow`
- Sixth Milestone 1 slice implemented across `crates/inference`,
  `pantograph-workflow-service`, and `src-tauri`
- Seventh Milestone 1 slice implemented across
  `pantograph-workflow-service` and `src-tauri`
- Eighth Milestone 1 slice implemented across
  `pantograph-workflow-service`, `src-tauri`, and TypeScript contract mirrors
- Ninth Milestone 1 slice implemented in `pantograph-workflow-service`
- Tenth implementation slice added in `src-tauri/src/workflow`
- Eleventh implementation slice added in `src-tauri/src/workflow`
- Twelfth implementation slice added across `pantograph-workflow-service`,
  `src-tauri`, and TypeScript workflow contract mirrors
- Thirteenth implementation slice added in `pantograph-workflow-service`
- Fourteenth implementation slice added in `src-tauri/src/workflow`
- Fifteenth implementation slice added in `src-tauri/src/workflow`
- Sixteenth implementation slice added in touched README ownership notes
- Seventeenth implementation slice added in `src-tauri/src/workflow`
- Eighteenth implementation slice added across `src-tauri` diagnostics and
  read-only GUI consumers
- Nineteenth implementation slice added in `src-tauri/src/llm` so the
  dedicated embedding sidecar now records backend-owned lifecycle snapshots and
  reuse decisions in Rust
- Twentieth implementation slice added in `crates/inference` so backend start
  paths now report reuse outcomes to the gateway, preserving reused-runtime
  attribution for adapter-specific cases such as Ollama daemon attachment
- Twenty-first implementation slice added in
  `crates/inference/src/backend/pytorch.rs` so identical model/device start
  requests reuse the already-loaded PyTorch runtime instead of forcing a reload
- Twenty-second implementation slice added across `crates/inference`,
  `src-tauri/src/llm`, and `src-tauri/src/workflow` so runtime lifecycle
  snapshots now preserve backend-owned structured decision reasons end to end
- Twenty-third implementation slice added in `src-tauri/src/llm/commands` so
  the embedding runtime lifecycle command is exercised directly against a
  backend-owned snapshot instead of relying only on lower-level gateway tests
- Twenty-fourth implementation slice added in
  `src-tauri/src/workflow/headless_workflow_commands.rs` so the read-only
  scheduler and trace snapshot command paths are exercised directly against
  backend-owned service and diagnostics stores
- Twenty-fifth implementation slice added in
  `src-tauri/src/workflow/headless_workflow_commands.rs` so diagnostics
  projection refresh logic is exercised directly against backend-owned
  scheduler snapshots, capability responses, and runtime lifecycle metrics
- Twenty-sixth implementation slice added in
  `src-tauri/src/workflow/event_adapter.rs` so node-engine event transport is
  covered through a pure Rust translation helper that preserves producer
  execution identity and emits backend-owned diagnostics snapshot events
- Twenty-seventh implementation slice added in
  `crates/pantograph-workflow-service/src/trace.rs` so a restarted
  `RunStarted` on the same execution id resets prior attempt-specific node,
  queue, and runtime state after terminal failure without erasing in-flight
  duplicate-start state
- Twenty-eighth implementation slice added across
  `crates/pantograph-workflow-service`, `src-tauri/src/workflow`, and matching
  frontend diagnostics types so cancellation-shaped workflow failures now flow
  through an explicit cancelled run contract instead of collapsing into generic
  failed state
- Twenty-ninth implementation slice added across
  `crates/pantograph-workflow-service`, `src-tauri/src/workflow`, and matching
  frontend diagnostics types so canonical trace summaries now retain
  backend-owned `session_id` attribution and session-scoped trace snapshot
  filters work when execution ids differ from session ids
- Thirtieth implementation slice added in
  `src-tauri/src/workflow/headless_workflow_commands.rs` so the headless trace
  snapshot helper and diagnostics projection path are covered for backend-owned
  `session_id` attribution at the adapter boundary
- Thirty-first implementation slice added in
  `src-tauri/src/workflow/headless_workflow_commands.rs` so the headless
  clear-history reader path is exercised directly and keeps scheduler/runtime
  snapshots while dropping retained run history

### Deviations

- Current runtime producers do not yet emit true warmup/reuse/instance-lifetime
  facts for every runtime path; the inference gateway and dedicated embedding
  sidecar now record authoritative lifecycle state, but remaining adapter-
  specific runtime paths still need to converge on the same producer contract.

### Follow-Ups

- Extend runtime lifecycle producers beyond the current inference-gateway,
  backend-start-outcome, embedding-sidecar, and PyTorch loaded-model reuse
  paths so every runtime host populates the same authoritative
  `WorkflowTraceRuntimeMetrics` fields.
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
