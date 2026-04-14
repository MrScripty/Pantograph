# Roadmap: Pantograph Workflow Execution, Graph, Scheduling, and Runtime

## Status
In progress

Last updated: 2026-04-13

## Current Implementation Snapshot

This roadmap is no longer purely proposed work. Pantograph already has active
implementation progress in the runtime and diagnostics layers.

1. Metrics/trace spine: In progress
2. Parallel demand execution: Not started
3. KV cache implementation: Not started
4. Scheduler V2: Not started
5. Real workflow event contract: Partial prerequisite groundwork only
6. Incremental graph execution: Not started
7. Runtime adapter unification: In progress

## Current Source-of-Truth Summary

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

### Active implementation stream

- Runtime adapter unification and runtime producer convergence
- Metrics/trace hardening for runtime lifecycle visibility
- Contract normalization across workflow service, Tauri diagnostics, embedded
  runtime wiring, and gateway lifecycle reporting

### Next gate before more implementation breadth

- Finish any remaining runtime producer convergence that still emits divergent
  contracts
- Keep roadmap/plan status aligned with implementation reality
- Build scheduler-v2 and later runtime-policy work on the now-frozen
  backend-owned runtime-registry boundary

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
- runtime preflight is stronger than before, but runtime residency and
  technical-fit policy are still conservative

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

**Status:** Not started

**Goal:** Move from simple queue ordering plus keep-alive toward a runtime-aware
session scheduler that makes better admission and reuse decisions.

**Milestones:**

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
- Execution-specific Python-sidecar runtime snapshots can now be reconciled
  into the shared runtime registry without replacing gateway-observed runtimes.
- Direct embedded/headless workflow runs now reconcile Python-sidecar runtime
  snapshots into the shared registry instead of depending on Tauri-only
  diagnostics capture.

**Still missing:**

- Remaining health-check, reconnect, and degraded-state hardening for all
  runtime producers beyond the current gateway-centric path
- Full convergence of all runtime producers on one registry-ready capability
  contract family
- Broader producer coverage over the backend-owned runtime-registry boundary
  beyond the current gateway-centric observation path

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
