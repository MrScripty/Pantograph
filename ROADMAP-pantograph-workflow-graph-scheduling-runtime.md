# Roadmap: Pantograph Workflow Execution, Graph, Scheduling, and Runtime

## Status
In progress

Last updated: 2026-04-18

## Current Implementation Snapshot

This roadmap is no longer purely proposed work. Pantograph already has active
implementation progress across the runtime, diagnostics, scheduler, and trace
layers.

1. Metrics/trace spine: Complete
2. Parallel demand execution: In progress
3. KV cache implementation: Not started
4. Scheduler V2: Complete
5. Real workflow event contract: In progress
6. Incremental graph execution: Not started
7. Runtime adapter unification: Complete

## Current Source-of-Truth Summary

Metrics/trace spine now has a reconciled dedicated implementation plan in
`IMPLEMENTATION-PLAN-pantograph-metrics-trace-spine.md`, and that plan now
marks Milestones 1 through 6 complete while keeping the remaining follow-up
hardening work explicit. Runtime adapter unification and workflow-adapter
hardening now have a dedicated Milestone 5 implementation plan in
`IMPLEMENTATION-PLAN-pantograph-milestone-5-workflow-adapter-integration.md`,
and the diagnostics/documentation/rollout-safety close-out now has a dedicated
Milestone 6 implementation plan in
`IMPLEMENTATION-PLAN-pantograph-milestone-6-diagnostics-documentation-rollout-safety.md`.
Scheduler V2 planning now also has a dedicated implementation plan in
`IMPLEMENTATION-PLAN-pantograph-scheduler-v2.md`.
Parallel demand execution now has a dedicated standards-reviewed plan in
`IMPLEMENTATION-PLAN-pantograph-phase-2-parallel-demand-execution.md`, and
real workflow event contract completion now has a dedicated
standards-reviewed plan in
`IMPLEMENTATION-PLAN-pantograph-phase-5-real-workflow-event-contract.md`.
Binding platform planning now also has a dedicated standards-reviewed plan in
`IMPLEMENTATION-PLAN-pantograph-binding-platform.md`, covering curated
client-facing surface policy, shared backend-owned binding contract ownership,
and the C#, Python, and BEAM language lanes.
Phase 5 Milestone 1 decomposition is now complete across `node-engine`, the
Tauri workflow adapter, and the shared Svelte graph package, so the remaining
Phase 5 work can land against focused backend, adapter, and read-only GUI
boundaries instead of the previous oversized insertion points.
The roadmap remains the cross-target summary, while milestone-level
metrics/trace follow-up work, runtime-adapter sequencing, Scheduler V2
execution constraints, workflow-event completion sequencing, and parallel
execution refactor details are tracked in those dedicated plans.
The dedicated metrics/trace hardening plan now also reflects current
implementation reality: Milestone 1 is complete, Milestone 2 is complete, the
backend trace store has already been decomposed into `query.rs` and `state.rs`
behind the existing `WorkflowTraceStore` facade, Tauri diagnostics now split
overlay and trace-attempt ownership into `overlay.rs` and `attempts.rs`,
runtime-debug registry commands now split request/debug/test boundaries into
`request.rs`, `debug.rs`, and `tests.rs`, backend queue timing is
authoritative-only, queue attribution is execution-first, session/workflow-
scoped runtime metric reuse now requires a unique backend trace match instead
of collapsing to the first trace, runtime-debug trace reads now resolve to one
execution or return explicit ambiguity metadata instead of silently merging
multi-run scope, and the touched synchronous stores now use
`parking_lot::Mutex` instead of poison-based `std::sync::Mutex` locking.

Milestone 5 transport hardening, binding review, recovery/idempotency
verification, source-of-truth close-out, and the Milestone 6 diagnostics,
documentation, and rollout-safety reconciliation are complete. Scheduler V2
milestone 5 close-out is now also complete: transport projection, cleanup
recovery, restore/reclaim recovery, and source-of-truth reconciliation are
landed, and canonical scheduler snapshots now expose backend-owned
earliest-known admission ETA bounds plus fairness-driven queue-head bypass
visibility. Phase 7 runtime-adapter unification is now also complete: backend
Rust owns producer health, capability, reconciliation, and workflow-execution
diagnostics sequencing across gateway, dedicated-embedding, and
execution-observed runtime paths, and the roadmap/plan/README set is
reconciled as the final source of truth.

### Completed groundwork already in the repo

- Shared Rust-side backend and runtime identity normalization now exists and is
  used across gateway, workflow service, diagnostics, embedded runtime, and
  host adapters.
- Workflow runtime capability contracts now publish canonical runtime ids and
  canonical `required_backends` values rather than drifting alias forms.
- Runtime diagnostics preserve concrete producer/runtime observations,
  including lifecycle snapshots, observed runtime ids, and Python-backed
  producer traces.
- Dedicated embedding and Python-backed runtimes are now surfaced through the
  backend-owned workflow capability contracts.
- External/managed runtime capability reporting is materially more consistent
  than it was at roadmap creation time.
- RAG embedding-mode indexing transitions now also refresh the shared runtime
  registry on prepare and restore paths, reducing drift between real producer
  state and registry-backed diagnostics or eviction policy.
- Direct embedded-runtime shutdown now also reconciles the shared registry back
  to `stopped`, so embedded hosts do not leave runtime-residency state stale
  after process stop.
- Edit-session embedding execution now also reconciles the shared registry
  after restoring inference mode, so registry snapshots track the restored
  runtime instance instead of the pre-restore producer state.
- Edit-session embedding execution now also reconciles the shared registry
  immediately after switching into embedding mode, so workflow-start
  diagnostics and runtime policy observe the prepared producer instead of the
  pre-switch inference runtime.
- Producer-aware workflow diagnostics runtime assembly now also lives in
  `crates/pantograph-embedded-runtime`, so Tauri workflow adapters consume a
  backend-owned fallback contract for active/embedding snapshots, model
  targets, and trace runtime metrics instead of rebuilding that logic locally.
- Capability-based diagnostics lifecycle fallback now also lives in
  `crates/pantograph-embedded-runtime`, so Tauri diagnostics no longer own
  runtime-capability alias matching or selected-vs-required fallback
  selection when no live lifecycle snapshot is present.
- Recovery stop-all paths now also reconcile the shared registry after tearing
  down producers, so failed clean-restart attempts do not leave stale
  embedding-runtime observations behind.
- Recovery restart now reuses the backend-owned active runtime config,
  rehydrates the dedicated embedding sidecar when parallel embedding remains
  configured, and refreshes both RAG vectorizer state and registry snapshots
  after restart. Alternate-port restart now also flows through the
  backend-owned restart contract instead of remaining a host-only limitation.
- Python-backed workflow producers are now treated as ephemeral execution
  runtimes in the convergence path, so completed task-adapter processes no
  longer leave stale ready-state registry observations or false reuse hints
  after the run has already exited.
- Window-close teardown now also uses the shared stop-and-sync registry
  adapter, closing the remaining obvious host-owned shutdown path that had
  still stopped producers without reconciling the backend-owned registry.
- Health-monitor crash detection now also triggers the shared recovery
  manager from Tauri-owned composition state, so automatic recovery no longer
  depends on a separate manual recovery command to enter the existing restart
  flow.
- Orchestration data-graph execution now delegates to the backend-owned
  `EmbeddedRuntime`, so Tauri no longer owns composite task-execution logic
  for that path and Python-sidecar producer observations still reconcile into
  the shared runtime registry during orchestration runs.
- Edit-session embedding restore now reconciles the shared runtime registry
  even when the restore attempt fails, so restore failures no longer leave
  the registry pinned to the pre-failure execution snapshot instead of the
  gateway's real post-failure state.
- RAG indexing restore paths now also synchronize the shared runtime registry
  after restore attempts, so restore failures in the Tauri indexing adapter
  no longer return early with stale registry state.
- Workflow runtime snapshot events now also preserve the backend-computed
  producer model target when execution runs through Python-backed sidecars or
  other runtime overrides, so diagnostics no longer fall back to the gateway's
  active-model target for non-gateway producers.
- Headless diagnostics snapshot reads now also preserve stored producer model
  targets for runtime overlays instead of rebuilding them from the current
  gateway mode, so post-run diagnostics remain consistent with the producer
  facts captured during execution.
- Embedded workflow execution now preserves multiple Python-sidecar producer
  observations from a single run instead of keeping only the last one, so
  mixed Python-runtime graphs reconcile every observed producer back into the
  shared runtime registry.
- Edit-session runtime trace metrics now also preserve every observed
  Python-sidecar runtime id from a run, so producer-aware workflow traces no
  longer collapse mixed Python-runtime executions down to only the final
  producer.
- Execution-time scheduler/runtime diagnostics snapshot assembly now also
  lives in `crates/pantograph-embedded-runtime::workflow_runtime`, so the
  Tauri workflow execution adapter no longer derives trace execution ids,
  runtime workflow ids, or registry-aware runtime snapshot projections when
  emitting execution diagnostics events.
- Tauri now also exposes an aggregate runtime debug snapshot command that
  synchronizes the shared runtime registry before returning runtime mode,
  health, recovery, and latest workflow diagnostics facts, so the GUI can
  inspect current runtime posture without rebuilding that view from multiple
  transport calls. That debug surface now also accepts optional workflow and
  session filters, and both the workflow diagnostics and runtime debug command
  paths now reuse the same shared workflow diagnostics projection helper
  instead of maintaining separate local projection paths. The same debug
  surface now also supports opt-in workflow trace reads with execution,
  session, and workflow filters plus completed-run selection while reusing the
  shared backend workflow trace snapshot helper.
- Tauri now has a shared targeted reclaim adapter that maps backend-owned
  runtime ids onto the correct host stop path for active-runtime and
  dedicated-embedding producers before re-synchronizing the shared registry,
  and the host now exposes synchronized runtime-registry snapshot and targeted
  reclaim commands through that shared path.
- Host-runtime producer matching for targeted reclaim now also lives in
  `crates/pantograph-embedded-runtime`, leaving the Tauri runtime-registry
  wrapper to consume backend-owned active-vs-embedding matching instead of
  keeping a separate host-local matcher.
- Targeted reclaim sequencing now also lives in
  `crates/pantograph-embedded-runtime`, with Tauri implementing only the host
  stop primitives needed by the backend-owned reclaim coordinator.
- Embedded-runtime reservation-release eviction now also routes through the
  same backend-owned reclaim coordinator, so current host reclaim paths no
  longer rely on ad hoc producer teardown.
- Workflow-session stale cleanup for idle, unloaded, non-keep-alive sessions
  now lives in `crates/pantograph-workflow-service`, and a bounded backend-
  owned cleanup worker now invokes that contract on a timer while Tauri only
  starts and stops the worker at the composition root.
- Headless scheduler diagnostics transport now preserves additive backend-owned
  `runtime_registry` facts instead of rebuilding or collapsing runtime
  admission posture in Tauri.
- Scheduler recovery coverage now spans direct stale cleanup, the background
  cleanup worker, restore-after-embedding transitions, and keep-alive-driven
  reclaim/unload transitions through the backend-owned diagnostics-provider
  boundary.

### Active implementation stream

- Metrics/trace follow-up hardening, with decomposition and lock-alignment
  complete and the remaining work concentrated in residual producer and
  acceptance coverage
- Workflow event contract completion planning and backend-owned transport
  parity preparation
- Binding platform planning and verification expansion for C#, Python, and
  BEAM host lanes
- Parallel demand execution planning and `node-engine` decomposition prep
- Future incremental execution work that will build on the completed
  metrics/trace, scheduler, and runtime-registry boundaries

### Next gate before more implementation breadth

- Keep roadmap/plan status aligned with implementation reality
- Finish the dedicated Phase 5 and Phase 2 implementation plans before
  widening execution or graph-surface changes
- Build later workflow, scheduler, and incremental-execution work on the
  now-frozen backend-owned runtime-registry and metrics/trace boundaries
  instead of reopening completed adapter-boundary refactors

## Objective

Improve Pantograph's core workflow engine so local-first execution becomes
faster, more incremental, more observable, and more deterministic without
pulling the project toward web-platform breadth that is better served by Dify
or Flowise.

This roadmap targets four tightly coupled areas:

- workflow execution performance and correctness
- graph editing and incremental recomputation
- session scheduling and runtime residency policy
- inference runtime reuse, cacheability, and diagnostics

## Why This Roadmap

Pantograph already has strong architectural boundaries:

- `crates/node-engine` owns execution and graph contracts
- `crates/pantograph-workflow-service` owns host-agnostic application service
  orchestration
- `crates/pantograph-embedded-runtime` owns Pantograph-specific runtime
  integration
- `packages/svelte-graph` owns reusable graph-editor interaction policy

The opportunity is still depth rather than surface-area expansion:

- `demand_multiple` in `crates/node-engine/src/engine.rs` is still sequential
  and explicitly marked as a future optimization
- queueing and keep-alive policy in
  `crates/pantograph-workflow-service/src/workflow.rs` is intentionally simple
- first-class workflow event semantics are not fully preserved end-to-end yet
- KV cache workflow nodes are scaffolded but not backed by a real cache store
- runtime preflight and backend-owned technical-fit selection are stronger than
  before, but runtime residency pressure, recovery verification, and selector
  hardening are still conservative

## Strategic Principles

- Prefer measurable execution improvements over new surface-area features.
- Keep business logic in Rust service and runtime crates, not adapters.
- Preserve backend-owned graph mutation and graph revision semantics.
- Treat runtime readiness, cacheability, and scheduling as first-class workflow
  concerns.
- Add observability before introducing more policy complexity.
- Optimize for deterministic local execution first; leave distributed execution
  as a later architecture option.

## Scope

### In Scope

- execution metrics and workflow traces
- bounded parallelism for independent workflow branches
- session scheduler improvements and runtime residency policy
- real workflow event contracts for incremental and interactive runs
- graph invalidation and partial recomputation
- KV cache persistence and reuse for compatible models
- runtime adapter unification for managed and external runtimes
- stronger contract, performance, and acceptance testing

### Out of Scope

- SaaS or multi-tenant platform features
- cloud deployment and orchestration work
- large node-catalog expansion for its own sake
- distributed multi-host scheduler work
- replacing existing architecture boundaries without a separate ADR

## Success Criteria

Pantograph should be meaningfully stronger when all roadmap phases land:

- repeated workflow runs complete faster because runtimes and caches are reused
- graphs with independent branches execute concurrently where safe
- graph edits trigger narrower invalidation and cheaper reruns
- session queues expose predictable admission, ordering, and capacity behavior
- frontend consumers receive stable events for waiting, incremental execution,
  graph mutation, and runtime state
- runtime readiness and runtime-selection failures are machine-consumable and
  easy to diagnose
- performance regressions are visible in tests or benchmark gates

## Core Roadmap Points

This roadmap is organized around seven primary development targets:

1. Metrics/trace spine
2. Parallel demand execution
3. KV cache implementation
4. Scheduler V2
5. Real workflow event contract
6. Incremental graph execution
7. Runtime adapter unification

Hardening, benchmarks, documentation, and acceptance coverage remain required
cross-cutting delivery work rather than an eighth roadmap point.

## Roadmap Overview

### Phase 1: Metrics/Trace Spine

**Status:** Complete

**Goal:** Make execution, queueing, and runtime behavior measurable before
adding more scheduling or graph complexity.

**Detailed source of truth:**

- `IMPLEMENTATION-PLAN-pantograph-metrics-trace-spine.md`

**Progress to date:**

- Backend-owned diagnostics projection exists in Rust rather than TypeScript.
- Runtime lifecycle and runtime trace contracts now preserve observed runtime
  ids and producer/runtime snapshots.
- Workflow-service and Tauri diagnostics now derive and normalize runtime data
  through shared identity helpers instead of drifting local alias rules.
- The host now exposes an aggregate runtime debug snapshot command that returns
  synced registry, lifecycle, health, recovery, and latest workflow
  diagnostics state for internal GUI debugging, including targeted
  workflow/session-scoped reads through the shared diagnostics projection
  helper reused by both diagnostics command surfaces. That same debug surface
  can now include filtered workflow trace snapshots on demand through the
  shared backend trace helper rather than building trace payloads in Tauri.
- Runtime and capability contracts are materially more machine-consumable than
  at roadmap creation time.

**Follow-up hardening still open:**

- Converge the remaining runtime hosts on the same authoritative
  `WorkflowTraceRuntimeMetrics` producer contract.
- Extend adapter-boundary diagnostics and snapshot-path acceptance coverage for
  the remaining command and transport surfaces beyond the now-hardened
  runtime-debug path.
- Extend acceptance coverage around the new backend-owned cancellation contract
  across the remaining producer and transport surfaces.
- Decide whether deeper metrics inspection should stay in diagnostics or move
  into a dedicated trace/metrics module.

### Phase 2: Parallel Demand Execution

**Status:** In progress

**Detailed source of truth:**

- `IMPLEMENTATION-PLAN-pantograph-phase-2-parallel-demand-execution.md`

**Goal:** Improve workflow execution latency for independent branches and
prepare the engine for metric-informed scheduling.

**Progress to date:**

- Phase 2 decomposition is underway in `crates/node-engine`: workflow-event
  helpers, graph-event helpers, the executor-facing `demand` and
  `demand_multiple` choreography, dependency-input assembly, node
  preparation, output-cache lifecycle handling, demand event emission, and
  in-flight bookkeeping now live behind focused internal modules, creating a
  compliant insertion boundary for the later bounded parallel coordinator.
- The remaining recursive demand orchestration now also lives behind a private
  `node-engine` execution-core owner, completing the Milestone 1 hot-path
  decomposition boundary ahead of the concurrency-contract and coordinator
  slices.
- Phase 2 now also has a private multi-demand request-plan boundary that keeps
  caller-visible requested-target order separate from the current sequential
  execution-target list before bounded parallel scheduling changes land.
- Phase 2 now also has a private multi-demand result collector that makes the
  current deterministic result-map merge semantics explicit before concurrent
  completion paths are introduced.
- Phase 2 now also executes the sequential multi-demand path through a private
  coordinator owner so bounded scheduling can land by changing coordinator
  internals instead of reopening facade orchestration.
- Phase 2 now also has a private execution-budget contract with a current
  default of one in-flight target, giving later bounded scheduling an explicit
  budget owner before additive runtime controls land.
- Phase 2 planning now also separates caller-visible requested targets from
  minimal root execution targets so redundant top-level drives are pruned
  before coordinator execution begins.
- Phase 2 now also represents root work as an explicit private execution-batch
  schedule, even though the current coordinator still runs a single sequential
  batch.

**Milestones:**

- Replace sequential `demand_multiple` behavior with bounded parallel demand
  execution for independent nodes
- Preserve deterministic output and event semantics under parallel execution
- Add concurrency-safe invalidation boundaries for downstream recomputation
- Add optional execution-budget controls such as max parallel tasks per run
- Add throughput benchmarks for representative workflows

### Phase 3: KV Cache Implementation

**Status:** Not started

**Goal:** Convert the existing KV cache scaffolding into a real workflow
primitive that improves reruns, prompt-prefix reuse, and iterative local work.

**Milestones:**

- Implement a real KV cache store with memory and disk policies
- Validate cache compatibility against model fingerprints
- Support markers and truncation for partial reuse
- Add cache metadata and eviction policy
- Surface cache hits, misses, and invalidation reasons in diagnostics
- Integrate cacheability with runtime selection and preflight where useful

### Phase 4: Scheduler V2

**Status:** Complete

**Goal:** Move from simple queue ordering plus keep-alive toward a runtime-aware
session scheduler that makes better admission and reuse decisions.

**Detailed source of truth:**

- `IMPLEMENTATION-PLAN-pantograph-scheduler-v2.md`

**Milestones:**

- Backend-owned scheduler module extraction is now landed in
  `crates/pantograph-workflow-service/src/scheduler/`, with `workflow.rs`
  reduced to facade/orchestration ownership for the existing queue surface
- The current priority/FIFO queue behavior is now represented by an explicit
  backend-owned scheduler policy object, and queue items now expose additive
  machine-consumable scheduler decision reasons plus canonical
  `queue_position` ordering diagnostics
- Queue items and trace queue metrics now also expose additive backend-owned
  `scheduler_admission_outcome` values, so queued versus admitted visibility
  no longer depends on adapter-local inference from item status
- Headless workflow diagnostics transport now also preserves additive
  backend-owned scheduler `runtime_registry` diagnostics instead of
  re-deriving runtime admission posture in Tauri
- Workflow error envelopes now also preserve additive backend-owned
  `details.scheduler` payloads for scheduler-capacity failures, so transport
  adapters can forward stable reason codes and counts without reclassifying
  free-form error text
- Scheduler recovery coverage now explicitly exercises stale cleanup, cleanup
  worker execution, restore-after-embedding runtime reconciliation, and
  reclaim-driven warmup posture at the backend diagnostics-provider boundary
- The backend-owned scheduler policy now also applies the first starvation-
  protection promotion rule, so long-waiting queued runs can surface a stable
  `starvation_protection` reason when they legitimately overtake newer higher-
  priority work
- Runtime-pressure unload selection now also carries backend-owned target
  workflow and `usage_profile` affinity inputs, so default rebalance behavior
  preserves more-reusable idle runtimes before falling back to generic LRU
- Scheduler runtime-affinity selection now also carries backend-owned
  `required_backends` and `required_models` refreshed from workflow
  capabilities and preflight cache updates, so rebalance preserves
  shared-backend and shared-model idle runtimes before unrelated sessions even
  across different workflows
- Add scheduler policy objects instead of encoding all queue behavior directly
  in one service module
- Introduce explicit runtime affinity and warm-session reuse decisions by
  workflow id, model dependency, and `usage_profile`
- Add starvation protection and fair ordering across queued runs
- Queue ETA and admission diagnostics now expose backend-owned earliest-known
  admission timing instead of transport-side estimates
- Scheduler snapshots now also identify when fairness-driven warm reuse will
  bypass the current queue head, keeping the remaining admission policy
  observable without adapter-local queue reconstruction
- Runtime rebalance behavior now keeps runs queued when loaded-session
  capacity is exhausted by active work and no reclaim path exists
- Scheduler error codes and decision reasons now flow as stable
  machine-consumable payloads

**Dependency note:**

- Keep the remaining Scheduler V2 work layered on the completed backend-owned
  runtime-registry boundary and current metrics/trace surfaces; do not reopen
  adapter-boundary ownership or move scheduler truth into Tauri while the
  later fairness/error-surface work is still pending.

### Phase 5: Real Workflow Event Contract

**Status:** In progress

**Detailed source of truth:**

- `IMPLEMENTATION-PLAN-pantograph-phase-5-real-workflow-event-contract.md`
- `IMPLEMENTATION-PLAN-pantograph-binding-platform.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-5-rustler-nif-testability-and-beam-verification.md`

**Goal:** Stop degrading engine semantics at the adapter boundary and give the
frontend and headless hosts a stable event language for interactive and
incremental runs.

**Progress to date:**

- Diagnostics and runtime-state reporting are more structured than before.
- Runtime-state reconstruction from workflow capabilities is more consistent.
- `WaitingForInput`, `IncrementalExecutionStarted`, and `GraphModified`
  transport vocabulary now exist across the engine, Tauri adapter, and
  diagnostics/trace projection path, and `GraphModified` now preserves a real
  execution id instead of borrowing the workflow id at the adapter boundary.
- `node-engine::WorkflowExecutor` now emits backend-owned `GraphModified`
  events for graph-mutation invalidation paths and
  `IncrementalExecutionStarted` for multi-demand execution entry, so those
  contracts no longer exist only as adapter/diagnostics placeholders.
- Edit-session execution now also preserves real `WaitingForInput` pause
  semantics end-to-end: unresolved `human-input` nodes emit the backend-owned
  wait event from `node-engine`, and the embedded runtime/Tauri transport stop
  treating that interactive pause as `WorkflowFailed`.
- Non-streaming workflow-run callers now also receive a backend-owned
  `InvalidRequest` envelope when a workflow requires interactive input, so
  direct adapters do not flatten that host-contract mismatch into an internal
  runtime error.
- Frontend execution-ownership helpers now claim the active run from the first
  execution-scoped workflow event instead of pre-pinning session-backed runs
  to the edit-session id, so valid scheduler/runtime and incremental events are
  not dropped before the backend emits `Started`.
- The shared graph package now rejects execution-scoped events that omit or
  mismatch the pinned run id once execution ownership is claimed, and the
  existing GUI now surfaces `WaitingForInput` as a real node/wait state
  instead of silently treating it as an unhandled event.
- The existing GUI now also maps backend-owned `IncrementalExecutionStarted`
  and `GraphModified` events onto node execution state, so incremental reruns
  and dirty-subgraph invalidation no longer stop at transport-only parity.
- The shared graph package now also mirrors the backend-owned `Cancelled`
  workflow event and the existing GUI closes out cancelled runs through that
  contract instead of depending on failure-only handling.
- UniFFI buffered workflow events now expose canonical backend event-type names
  for the newer event vocabulary instead of deriving transport labels from
  unstable debug output.
- The app diagnostics consumer now also tracks run execution identity
  separately from editable-session ownership, so session-backed diagnostics
  filtering no longer rejects backend-owned run events before the session UI
  sees a `Started` event.
- `node-engine::WorkflowEvent` now includes a canonical `WorkflowCancelled`
  variant, backend Rust emitters can publish explicit cancellation outcomes,
  and Tauri diagnostics/transport no longer classify `WorkflowFailed` strings
  to infer cancellation for node-engine events.
- Non-streaming workflow-service surfaces now also expose a backend-owned
  `cancelled` error code/envelope, and the embedded runtime plus frontend HTTP
  adapter preserve that contract instead of flattening user-driven
  cancellation into `runtime_timeout`.
- `node-engine` orchestration execution now emits a backend-owned terminal
  workflow event for true executor error exits instead of returning those
  failures without a final lifecycle event.
- Binding-side acceptance coverage now also checks the backend-owned
  `cancelled` envelope through the UniFFI frontend-HTTP workflow-run surface,
  and Rustler has matching serializer-parity coverage for the same envelope.
- Orchestration subgraph execution now preserves backend-owned interactive and
  cancellation outcomes instead of flattening `WaitingForInput` or `Cancelled`
  into the generic data-node error-handle path.
- Embedded data-graph execution now also preserves backend-owned
  `WaitingForInput` outcomes instead of converting them into synthetic
  terminal-node error outputs before orchestration sees them, and the Tauri
  orchestration adapter now forwards that backend result directly.
- The headless workflow transport helper is now also pinned for the
  backend-owned interactive `invalid_request` envelope used by non-streaming
  workflow runs.
- UniFFI frontend-HTTP workflow-run coverage and the Rustler workflow-error
  serializer helper now also pin the same interactive `invalid_request`
  envelope, matching the binding-side `cancelled` coverage already in place.
- Edit-session graph mutation responses in `pantograph-workflow-service` now
  also carry an additive backend-owned canonical `GraphModified` event with
  deterministic dirty-task ordering, so binding-facing graph edit responses no
  longer need to infer mutation semantics from the returned graph snapshot
  alone.
- Tauri edit-session mutation commands now also forward that additive backend
  response contract, and the shared graph store consumes the backend-owned
  `GraphModified` payload read-only instead of inventing graph-mutation
  semantics locally for backend round-trip edits.
- Phase 5 acceptance coverage now also includes the GUI workflow-event reducer
  path for `IncrementalExecutionStarted` resume and `GraphModified` replay,
  and Tauri diagnostics restart tests now pin that stale dirty-task and
  incremental-task overlays are cleared when an execution id is reused for a
  fresh attempt.
- The app-owned workflow toolbar now also consumes backend workflow events
  through a focused helper built on the shared reducer path instead of keeping
  a stale partial switch in the Svelte component, and the app-local workflow
  execution-state type now includes the backend-owned `waiting` state needed
  for interactive runs.
- The focused toolbar helper coverage now also explicitly pins
  `WaitingForInput` transitions and cancelled-run cleanup at the app boundary,
  so the GUI contract is not only inferred through resume behavior.
- Rustler now also has focused serializer-parity coverage for backend-owned
  `GraphModified` and `WaitingForInput` event JSON at the BEAM boundary, so
  the NIF event channel no longer relies on inference from downstream
  consumers for those transport labels and additive fields.
- Tauri event-adapter coverage now also pins the backend-owned
  `WaitingForInput` translation path and its waiting-state diagnostics
  projection, so that backend task/prompt semantics are explicitly preserved
  through the app-facing node/message contract.
- That waiting-path parity is now also anchored by a focused `src-tauri`
  adapter test rather than being inferred only from downstream consumer
  behavior.
- Backend-owned workflow trace state now also resumes waiting runs back to
  `Running` when `IncrementalExecutionStarted` arrives, and the focused Tauri
  adapter boundary pins that corrected diagnostics parity together with
  backend-owned `GraphModified` dirty-task overlays and incremental-resume
  task ids. Those transport semantics are therefore covered directly at
  translation time instead of only in downstream diagnostics and GUI reducer
  tests.
- UniFFI buffered-event coverage now also parses serialized workflow-event JSON
  and pins additive payload parity for `WaitingForInput`, `GraphModified`,
  `WorkflowCancelled`, and `IncrementalExecutionStarted`, tightening the
  binding-side acceptance path beyond canonical event-label checks alone.
- The non-streaming frontend-HTTP workflow host now also pins a backend-owned
  `cancelled` envelope through its real `workflow_run` entrypoint, reducing
  the remaining acceptance-gap surface for cancellation parity outside Tauri
  and UniFFI.
- The same frontend-HTTP adapter now also pins backend-owned interactive
  `invalid_request` envelopes through the real `workflow_run` entrypoint, so
  that non-streaming interactive mismatch behavior is covered directly at the
  transport boundary rather than only through helper-level mapping tests.
- The embedded runtime session-backed non-streaming run path now also has
  focused interactive-mismatch coverage, so `run_workflow_session` preserves
  the same backend-owned `invalid_request` contract already pinned on the
  direct embedded `workflow_run` surface.
- The raw embedded data-graph and edit-session execution paths now also
  explicitly pin that a `WaitingForInput` pause does not drift into terminal
  failed/completed/cancelled events, tightening the current backend
  human-input producer paths beyond positive-event-only assertions.
- The backend `node-engine` orchestration wait/cancel producer tests now also
  explicitly pin that subgraph pauses and cancellations do not drift into
  completed or mismatched terminal workflow events at the orchestration layer.
- The streamed Tauri edit-session execution runtime now also has focused
  coverage for its waiting-versus-error result mapping, so backend
  `waiting_for_input` outcomes remain non-error transport states instead of
  being tested only indirectly through the broader execution flow.
- UniFFI frontend-HTTP session-run coverage now also pins backend-owned
  `cancelled` and interactive `invalid_request` envelopes at the real binding
  boundary, so session-backed non-streaming binding parity is no longer
  inferred only from direct workflow-run coverage.
- UniFFI direct embedded-runtime coverage now also pins the same backend-owned
  interactive `invalid_request` envelope on `FfiPantographRuntime::workflow_run`
  and `FfiPantographRuntime::workflow_run_session`, so the real direct runtime
  binding path no longer relies on frontend-HTTP parity to prove the
  non-streaming interactive mismatch contract.
- Rustler frontend-HTTP session-host coverage now also pins the same
  backend-owned `cancelled` and interactive `invalid_request` contracts for
  session-backed runs, reducing the remaining BEAM-side acceptance gap to the
  opaque NIF wrapper itself rather than the Rustler-owned workflow host path.
- The concrete embedded-runtime workflow host now also has a focused
  pre-cancelled `WorkflowRunHandle` test at the real `WorkflowHost::run_workflow`
  boundary, so non-streaming cancellation parity is no longer inferred only
  through workflow-service or outer adapter wrappers.
- The touched backend trace and Tauri workflow READMEs are now reconciled with
  the landed contract: backend trace ownership explicitly documents
  resume-from-waiting semantics, and the Tauri adapter/workflow boundary docs
  now state that waiting/input prompts, dirty-task overlays, and incremental
  resume task ids are preserved as backend-owned transport facts.
- Phase 5 Milestone 1 decomposition is now complete: `node-engine` event
  contract/helpers and graph-event helpers live behind focused internal
  modules, the Tauri event adapter is split into translation and diagnostics
  bridge modules, and `WorkflowToolbar.svelte` now delegates backend workflow
  event reduction to a focused read-only store helper with targeted tests.
- Phase 5 Milestone 2 completion-matrix freeze is now also complete: current
  producer/consumer coverage for `WaitingForInput`, `GraphModified`,
  `IncrementalExecutionStarted`, and `Cancelled` is recorded in the dedicated
  plan, and the cancellation row now reflects the canonical backend-owned
  event path instead of adapter-side failure-message inference.

**Still missing:**

- Backend-owned emission coverage for the remaining interactive paths that
  still do not produce the event vocabulary consistently beyond the current
  human-input pause path, orchestration subgraph pause/cancel path, and the
  now-canonical graph-mutation/incremental paths
- Broader backend emission coverage for any additional cancellable producer
  paths that still terminate without publishing the canonical cancellation
  event, beyond the now-covered orchestration error exits
- Extend acceptance coverage for the new `cancelled` envelope only where a
  remaining non-streaming/headless command or runtime-hosted binding surface is
  still not directly pinned beyond the now-covered embedded-runtime host and
  session-run surfaces, streamed Tauri execution helper, UniFFI frontend-HTTP
  workflow-run/session-run bindings, UniFFI direct embedded-runtime
  workflow-run/session-run bindings, Rustler workflow host and session-host
  paths, and frontend-HTTP workflow-run transport
- Binding-platform follow-on: freeze the curated client-facing surface for the
  Pantograph headless binding platform instead of treating wrapper exports as
  the product contract by default
- C# lane follow-on: harden the existing generated/package/runtime path into a
  documented first-class supported binding with stronger host-language contract
  assertions
- Python lane follow-on: add a distinct host-consumer binding path and testing
  story that is separate from Pantograph's out-of-process Python worker/runtime
  concerns
- BEAM lane follow-on: extract Rustler-boundary logic into testable Rust where
  appropriate and add a BEAM-hosted harness that proves the real NIF contract
  with the runtime-provided `enif_*` symbols present

### Phase 6: Incremental Graph Execution

**Status:** Not started

**Goal:** Make graph editing and reruns cheaper by tightening invalidation and
sync boundaries between the package graph and backend-owned graph state.

**Milestones:**

- Add partial graph invalidation instead of broad graph refreshes after every
  structural edit
- Reduce full graph fingerprint recomputation where incremental derivation is
  safe
- Tighten stale-intent and stale-event handling using session and execution ids
- Extend graph-session tests for insert-and-connect, edge insertion, undo/redo,
  and selection persistence across backend snapshots
- Add explicit graph mutation diagnostics to the frontend package

### Phase 7: Runtime Adapter Unification

**Status:** Complete

**Goal:** Make managed runtimes and external runtimes look consistent to the
workflow scheduler, preflight layer, and diagnostics surfaces.

**Detailed source of truth:**

- `IMPLEMENTATION-PLAN-pantograph-phase-7-runtime-adapter-unification.md`

**Progress to date:**

- Shared backend/runtime identity normalization is in place in Rust.
- Frontend HTTP host, embedded runtime, workflow service, diagnostics, and
  gateway lifecycle reporting now converge on shared runtime/backend identity
  helpers.
- Workflow capability payloads now publish canonical runtime ids and canonical
  backend requirements more consistently across managed, external, embedding,
  and Python-backed runtimes.
- Diagnostics preserve concrete runtime producers and lifecycle snapshots.
- External capability handling is materially less placeholder-driven than at the
  start of this roadmap.
- Runtime-registry Milestone 3 closeout is complete: backend Rust owns
  admission, warmup, retention, and eviction behavior, Tauri remains a
  composition/transport boundary for those paths, and the current cleanup and
  warmup timing loops are explicitly bounded and documented.
- Execution-specific Python-sidecar runtime snapshots can now be reconciled
  into the shared runtime registry without replacing gateway-observed runtimes.
- Direct embedded/headless workflow runs now reconcile Python-sidecar runtime
  snapshots into the shared registry instead of depending on Tauri-only
  diagnostics capture.
- Shared runtime display-name and backend-alias mapping now lives in
  `crates/pantograph-runtime-identity` so embedded and Tauri runtime producers
  no longer maintain separate host-local identity tables.
- Lifecycle-snapshot status classification now lives in
  `crates/pantograph-runtime-registry` so embedded and Tauri runtime producers
  no longer duplicate warmup/error-to-status mapping.
- Dedicated embedding-runtime reuse/start/stop orchestration now lives in
  `crates/inference`, leaving the Tauri gateway wrapper as a consumer of a
  backend-owned coordinator instead of the owner of that runtime lifecycle.
- Gateway and Python-sidecar runtime observation translation for the shared
  runtime registry now lives in `crates/pantograph-embedded-runtime`, leaving
  the Tauri registry module as a thin re-export of backend-owned producer
  mapping logic.
- Dedicated embedding runtime capability mapping now lives in
  `crates/pantograph-embedded-runtime`, leaving the Tauri headless workflow
  adapter as a consumer of backend-owned runtime capability helpers.
- Embedding workflow graph inspection and Puma-Lib model-id resolution for
  runtime mode preparation now lives in `crates/pantograph-embedded-runtime`,
  reducing Tauri workflow execution commands to consumers of backend-owned
  embedding workflow rules.
- Temporary embedding-mode prepare/restore orchestration for workflow execution
  and RAG indexing now lives in `crates/inference`, leaving Tauri callers to
  provide request inputs and consume backend-owned restore context.
- Workflow execution extension wiring and runtime trace/model-target shaping
  now live in `crates/pantograph-embedded-runtime`, leaving Tauri workflow
  commands to consume backend-owned execution metadata helpers instead of
  maintaining host-local copies.
- Embedding model-path resolution and workflow-specific embedding runtime
  preparation now live in `crates/pantograph-embedded-runtime`, leaving Tauri
  startup, RAG, and workflow execution adapters to consume one backend-owned
  embedding preparation rule set.
- Runtime diagnostics projection for workflow execution now lives in
  `crates/pantograph-embedded-runtime`, leaving Tauri workflow commands to
  emit transport events and update stores without owning runtime trace fallback
  semantics. Execution-path runtime snapshot override reconciliation now also
  lives there, so Tauri no longer decides when producer-specific execution
  facts should update shared registry state.
- Workflow-execution diagnostics now also synchronize the shared runtime
  registry through a backend-owned helper before projecting execution
  snapshots, so `workflow_execution_runtime.rs` no longer owns a separate
  sync-before-snapshot sequence.
- Registry admission and runtime-unavailable failures from workflow execution
  now cross the workflow boundary as deterministic `runtime_not_ready` or
  `invalid_request` envelopes instead of being collapsed into generic
  `internal_error` adapter failures.
- Diagnostics lifecycle snapshot normalization now also lives in
  `crates/pantograph-embedded-runtime`, leaving Tauri diagnostics to serialize
  backend-owned runtime lifecycle facts instead of canonicalizing runtime ids
  or inferring lifecycle reasons locally.
- Runtime-registry sync-before-snapshot and sync-before-reclaim behavior now
  also lives in `crates/pantograph-embedded-runtime`, leaving the Tauri
  registry command surface to serialize backend-owned reconciliation results
  instead of deciding when host state must be synchronized first.
- Runtime-registry stop-all and restore reconciliation behavior now also lives
  in `crates/pantograph-embedded-runtime`, leaving Tauri recovery and shutdown
  wrappers to invoke backend-owned lifecycle coordination instead of owning the
  post-transition registry-sync rule locally.
- Recovery restart-plan derivation now also lives in
  `crates/pantograph-embedded-runtime`, leaving Tauri recovery to drive retry
  orchestration and app wiring without owning backend port-override or
  dedicated-embedding restart policy.
- Recovery retry-strategy and exponential-backoff policy now also live in
  `crates/pantograph-embedded-runtime`, leaving Tauri recovery to orchestrate
  host effects without owning attempt sequencing or retry-delay math.
- Health-check probe assessment and degraded/unhealthy threshold policy now
  also live in `crates/pantograph-embedded-runtime`, leaving the Tauri health
  monitor to own polling cadence, HTTP transport, and event emission without
  re-encoding failure-count progression locally.
- Health-monitor-driven runtime-registry synchronization now also routes
  through `crates/pantograph-embedded-runtime`, so failed health checks can
  mark the active runtime `unhealthy` in the shared registry instead of
  leaving registry state at the last lifecycle-derived ready snapshot.
- Dedicated embedding runtime health checks now also route through
  `crates/pantograph-embedded-runtime`, so embedding-sidecar failures converge
  registry state to `unhealthy` through the same backend-owned producer
  contract instead of a Tauri-local special case.
- Ordinary runtime-registry synchronization now also consumes the latest
  gateway-stored backend health overlays, so later mode-info refreshes do not
  reset active or dedicated-embedding runtimes from assessed `unhealthy` back
  to lifecycle-derived ready.
- Tauri server command return paths now also route through one shared
  sync-and-return helper, so command-level early returns such as
  `start_sidecar_inference` embedding-path resolution warnings no longer leave
  registry state stale after the main runtime already started.
- Tauri host lifecycle commands, recovery, agent startup, and RAG
  status/indexing flows now also refresh the RAG manager's embedding endpoint
  from shared gateway facts, so failed or mode-switching embedding producer
  transitions no longer leave host-local vector-search consumers pointed at
  stale runtime URLs.
- Embedded-runtime hosted shutdown, live gateway sync, and edit-session
  restore paths now also route through the shared backend registry lifecycle
  helpers, reducing one more internal host-specific sequencing fork.
- Orchestration data-graph execution now also routes through
  `crates/pantograph-embedded-runtime`, leaving the Tauri orchestration module
  to inject state and forward events instead of owning composite task
  execution or registry-aware Python runtime observation handling.
- Execution-path runtime snapshot overrides now also preserve a matching
  producer's existing `unhealthy` registry state, so Python-sidecar or other
  non-gateway execution observations do not silently downgrade backend-owned
  health assessment failures back to lifecycle-ready snapshots.

**Close-out summary:**

- Producer health, reconnect, degraded-state, and health-overlay semantics now
  converge through backend-owned Rust helpers instead of adapter-local policy.
- Runtime capability publication now converges on one backend-owned contract
  family across gateway, dedicated-embedding, and execution-observed producer
  paths.
- Restore, recovery, reclaim, diagnostics, and workflow-execution registry
  reconciliation now route through backend-owned helpers, leaving Tauri as a
  composition and transport boundary.
- No new ADR or dependency was required to close Phase 7; the existing
  ownership boundary remained sufficient.

**Milestones:**

- Freeze the dedicated Phase 7 source of truth and bring the immediate runtime
  insertion areas back toward standards compliance before more producer logic
  lands
- Converge producer health, reconnect, and degraded-state semantics behind
  backend-owned Rust helpers instead of adapter-local policy
- Unify the registry-ready capability contract family across gateway,
  dedicated-embedding, and execution-observed runtime producers
- Broaden backend-owned runtime-registry boundary coverage for restore,
  recovery, reclaim, and execution-path producer reconciliation
- Reconcile README, roadmap, and acceptance coverage so the final Phase 7
  ownership boundary and transport contract remain auditable

## Cross-Cutting Delivery Work

These items apply to every phase above and are not separate roadmap points:

- contract and acceptance coverage for scheduler, runtime, and graph flows
- benchmark fixtures for representative local workflows
- documentation updates for new boundaries and diagnostics
- rollout order and fallback behavior for risky runtime or scheduler changes
- explicit non-goals for work postponed beyond this roadmap

## Recommended Execution Order From Current Repo State

This replaces the original purely hypothetical sequence. It reflects the code
that has already landed and the safest next dependency order from here.

1. Complete metrics/trace spine hardening so runtime, scheduler, and registry
   policy stay observable as later roadmap work lands
2. Finish the real workflow event contract end-to-end on top of the now-frozen
   scheduler and runtime-registry boundaries
3. Implement parallel demand execution in `node-engine`
4. Implement incremental graph execution
5. Implement KV cache as a real reusable runtime primitive

## Dependencies and Sequencing Notes

- Scheduler improvements should follow metrics and runtime-contract cleanup.
- Runtime-registry work should follow the current runtime-unification stream,
  not begin in parallel with unresolved producer contract drift.
- Graph incrementality should not outrun backend event-contract work.
- KV cache implementation should align with runtime fingerprint and model
  dependency contracts, not invent a parallel identity model.
- Any phase that expands cross-layer contracts should update the nearest README
  and ADR surfaces in the same change set.

## Candidate Follow-On Work

Not part of the main roadmap, but left intentionally compatible:

- distributed or multi-host execution
- runtime registry persistence
- plugin-facing node execution contracts
- richer human-in-the-loop workflow sessions
- benchmark corpus for model-family-specific scheduling policies

## Definition of Done

This roadmap is complete when Pantograph can credibly claim all of the
following:

- workflow execution is measurably faster on repeated local runs
- independent graph branches exploit safe concurrency
- graph edits and reruns invalidate less work
- scheduler behavior is explicit, debuggable, and runtime-aware
- managed and external runtime readiness are surfaced through coherent
  contracts
- cache reuse and runtime residency materially reduce local iteration cost
