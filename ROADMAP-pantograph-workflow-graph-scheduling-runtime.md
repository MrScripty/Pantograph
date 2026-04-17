# Roadmap: Pantograph Workflow Execution, Graph, Scheduling, and Runtime

## Status
In progress

Last updated: 2026-04-16

## Current Implementation Snapshot

This roadmap is no longer purely proposed work. Pantograph already has active
implementation progress in the runtime and diagnostics layers.

1. Metrics/trace spine: In progress
2. Parallel demand execution: Not started
3. KV cache implementation: Not started
4. Scheduler V2: In progress
5. Real workflow event contract: Partial prerequisite groundwork only
6. Incremental graph execution: Not started
7. Runtime adapter unification: In progress

## Current Source-of-Truth Summary

Runtime adapter unification and workflow-adapter hardening now have a dedicated
Milestone 5 implementation plan in
`IMPLEMENTATION-PLAN-pantograph-milestone-5-workflow-adapter-integration.md`,
and the diagnostics/documentation/rollout-safety close-out now has a dedicated
Milestone 6 implementation plan in
`IMPLEMENTATION-PLAN-pantograph-milestone-6-diagnostics-documentation-rollout-safety.md`.
Scheduler V2 planning now also has a dedicated implementation plan in
`IMPLEMENTATION-PLAN-pantograph-scheduler-v2.md`. The roadmap remains the
cross-target summary, while milestone-level runtime-adapter sequencing,
Scheduler V2 execution constraints, and close-out details are tracked in those
dedicated plans. Milestone 5 transport hardening, binding review,
recovery/idempotency verification, source-of-truth close-out, and the
Milestone 6 diagnostics, documentation, and rollout-safety reconciliation are
complete.

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

### Active implementation stream

- Runtime adapter unification and runtime producer convergence
- Metrics/trace hardening for runtime lifecycle visibility
- Contract normalization across workflow service, Tauri diagnostics, embedded
  runtime wiring, and gateway lifecycle reporting
- Scheduler and later runtime-policy work that builds on the completed
  runtime-registry and Milestone 6 close-out boundaries

### Next gate before more implementation breadth

- Keep roadmap/plan status aligned with implementation reality
- Build scheduler-v2 and later runtime-policy work on the now-frozen
  backend-owned runtime-registry boundary instead of reopening completed
  adapter-boundary refactors

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

**Status:** In progress

**Goal:** Make execution, queueing, and runtime behavior measurable before
adding more scheduling or graph complexity.

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

**Still missing:**

- Full run-level and per-node metric coverage across all execution paths
- Complete queue-state, residency, and eviction inspection surfaces
- Full event-contract parity for waiting, graph-modified, and incremental-run
  semantics

### Phase 2: Parallel Demand Execution

**Status:** Not started

**Goal:** Improve workflow execution latency for independent branches and
prepare the engine for metric-informed scheduling.

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

**Status:** In progress

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
- The backend-owned scheduler policy now also applies the first starvation-
  protection promotion rule, so long-waiting queued runs can surface a stable
  `starvation_protection` reason when they legitimately overtake newer higher-
  priority work
- Runtime-pressure unload selection now also carries backend-owned target
  workflow and `usage_profile` affinity inputs, so default rebalance behavior
  preserves more-reusable idle runtimes before falling back to generic LRU
- Add scheduler policy objects instead of encoding all queue behavior directly
  in one service module
- Introduce explicit runtime affinity and warm-session reuse decisions by
  workflow id, model dependency, and `usage_profile`
- Add starvation protection and fair ordering across queued runs
- Add queue ETA and admission diagnostics
- Improve runtime rebalance behavior when loaded-session capacity is exhausted
- Define scheduler error codes and decision reasons as stable machine-consumable
  payloads

**Dependency note:**

- Do not start Scheduler V2 implementation until the remaining runtime-producer
  convergence work is closed and the metrics/trace spine is strong enough to
  make scheduler policy observable.

### Phase 5: Real Workflow Event Contract

**Status:** Partial prerequisite groundwork only

**Goal:** Stop degrading engine semantics at the adapter boundary and give the
frontend and headless hosts a stable event language for interactive and
incremental runs.

**Progress to date:**

- Diagnostics and runtime-state reporting are more structured than before.
- Runtime-state reconstruction from workflow capabilities is more consistent.

**Still missing:**

- First-class `WaitingForInput` and `GraphModified` workflow events end-to-end
- Explicit incremental-execution-started and stale-event rejection semantics
- Full adapter parity between backend-owned events and frontend consumers

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

**Status:** In progress

**Goal:** Make managed runtimes and external runtimes look consistent to the
workflow scheduler, preflight layer, and diagnostics surfaces.

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
- Embedded-runtime hosted shutdown, live gateway sync, and edit-session
  restore paths now also route through the shared backend registry lifecycle
  helpers, reducing one more internal host-specific sequencing fork.
- Orchestration data-graph execution now also routes through
  `crates/pantograph-embedded-runtime`, leaving the Tauri orchestration module
  to inject state and forward events instead of owning composite task
  execution or registry-aware Python runtime observation handling.

**Still missing:**

- Remaining health-check, reconnect, and degraded-state hardening for all
  runtime producers beyond the current gateway plus dedicated-embedding path
- Full convergence of all runtime producers on one registry-ready capability
  contract family
- Broader producer coverage over the backend-owned runtime-registry boundary
  beyond the current gateway plus dedicated-embedding observation path

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

1. Finish runtime adapter unification and remaining producer-convergence work
2. Complete metrics/trace spine hardening so scheduler and registry policy are
   observable
3. Begin scheduler-v2 foundation on top of the completed runtime-registry
   boundary
4. Implement parallel demand execution in `node-engine`
5. Finish the real workflow event contract end-to-end
6. Implement incremental graph execution
7. Implement KV cache as a real reusable runtime primitive

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
