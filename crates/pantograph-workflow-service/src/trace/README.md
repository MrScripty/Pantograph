# crates/pantograph-workflow-service/src/trace

## Purpose
This directory contains the backend-owned workflow trace contracts and in-memory
trace store used by Pantograph workflow execution. The boundary exists so
runtime, scheduler, replay, and execution-history semantics stay in the
host-agnostic workflow service instead of leaking into Tauri, bindings, or
frontend code.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | Module entrypoint that exposes trace contracts and the trace store facade. |
| `query.rs` | Owns backend trace snapshot filtering and unique-match runtime selection helpers. |
| `state.rs` | Owns trace run-state creation, restart resets, and event-application helpers. |
| `types.rs` | Canonical workflow trace DTOs, event enums, and request/response contracts. |
| `store.rs` | Owns retained trace state, replay behavior, request filtering, and snapshot generation. |
| `timing.rs` | Projects durable timing expectations from prior completed observations and creates idempotent timing observations from terminal trace state. |
| `runtime.rs` | Applies runtime snapshot events to canonical workflow trace state. |
| `scheduler.rs` | Applies scheduler snapshot events to canonical workflow trace state. |
| `tests.rs` | Trace DTO serialization, runtime inference, lifecycle reason, snapshot filtering, replay, scheduler attribution, waiting/resume, and dirty-task tests extracted from the module entrypoint. |
| `tests/` | Behavior-focused trace test submodules for lifecycle/restart and scheduler/runtime metric attribution coverage. |

## Problem
Pantograph needs machine-consumable workflow execution history that survives
replay, restart, and adapter transport without each host reconstructing its own
version of run state. Without a dedicated backend trace boundary, diagnostics
and runtime views would drift across hosts and recovery flows.

## Constraints
- The trace store must remain host-agnostic and free of Tauri/binding
  framework dependencies.
- Replay and recovery updates must reconcile into one canonical execution
  record rather than materializing duplicate runs.
- Request filtering and validation belong here so adapters do not reinterpret
  trace contract semantics.
- Snapshot fields and enum labels are machine-consumed and must remain
  deterministic.
- Snapshot filter normalization and blank-filter rejection must be enforced here
  so every host reads traces through the same trimmed, backend-owned contract.

## Decision
Keep canonical workflow trace ownership in `pantograph-workflow-service`.
`types.rs` freezes the transport-safe trace vocabulary, `store.rs` owns the
in-memory retained trace state and facade, `query.rs` owns backend trace
filtering and unique-match runtime selection, `state.rs` owns run-state
creation and event application, `timing.rs` owns timing-observation projection
and duration expectation enrichment for retained traces and opened workflow
graphs, and `runtime.rs` plus `scheduler.rs` apply backend-owned
runtime/scheduler facts into the canonical run state. The
canonical trace snapshot filter model is
`execution_id`, `session_id`, `workflow_id`, `workflow_name`, plus
`include_completed`, with whitespace trimming and blank-filter rejection applied
inside this boundary. Adapters may project or transport these contracts, but
they do not own trace lifecycle rules.

## Alternatives Rejected
- Keep workflow trace logic in a single oversized `trace.rs` file.
  Rejected because the trace boundary already carries multiple responsibilities
  and needed decomposition to stay reviewable.
- Let adapters hold their own retained run histories.
  Rejected because replay, restart, and scheduler attribution would drift
  between hosts.

## Invariants
- `WorkflowTraceStore` is the canonical owner of retained workflow trace state.
- Request validation for snapshot filters stays here, not in adapters.
- Trace snapshot filter semantics, including `workflow_name`, stay
  backend-owned and must not drift between transport surfaces.
- When a run is paused by `WaitingForInput`, a later
  `IncrementalExecutionStarted` or `NodeStarted` event for the same execution
  must resume the canonical trace back to `Running` rather than leaving the
  run stuck in a stale waiting state.
- Runtime and scheduler snapshot application must preserve backend-owned
  execution and session identity when those facts are available.
- `queue_wait_ms` is only emitted from measured queue timestamps. Snapshot
  observation time must not be repurposed as enqueue, dequeue, or queue-wait
  data.
- Queue-item attribution is execution-first. Session identifiers may describe
  session scope, but they are not a fallback key for attaching queue-item
  metrics to another execution's trace.
- Scheduler snapshot observation time is not currently a trace queue-state
  input; scheduler projection consumes measured queue item/session facts only.
- Recovery or replay updates for the same execution id update one canonical run
  record in place.
- Timing expectations are projected from durable prior completed observations.
  The active execution is not recorded until after its terminal snapshot is
  enriched, so completion diagnostics compare against previous history rather
  than including themselves in their baseline.
- Opened-graph timing expectations are ledger projections keyed by workflow id
  and graph fingerprint. They must not require an active execution id.
- Trace DTO serialization, runtime inference, lifecycle reason, snapshot
  filtering, replay, baseline scheduler attribution, waiting/resume, and
  dirty-task tests stay indexed by `tests.rs`, while larger lifecycle/restart
  and scheduler/runtime metric coverage lives under `tests/` so both the module
  entrypoint and behavior slices remain reviewable.

## Revisit Triggers
- A durable trace store replaces the in-memory retained-history implementation.
- Another backend-owned service needs to consume the same trace contracts
  outside this crate boundary.
- Trace retention or filtering semantics become policy-heavy enough to require
  a separate sub-crate.

## Dependencies
**Internal:** `crate::workflow`, especially workflow-service errors,
capabilities, session summaries, and queue items.

**External:** `serde` for transport serialization.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- Reason: workflow trace is part of the headless workflow-service contract
  rather than a Tauri-only feature.
- Revisit trigger: workflow trace ownership moves out of
  `pantograph-workflow-service`.

## Usage Examples
```rust
let trace_store = WorkflowTraceStore::new(200);
let response = trace_store.snapshot(&WorkflowTraceSnapshotRequest::default())?;
```

## API Consumer Contract
- Hosts and adapters consume `WorkflowTraceStore` through the public facade
  exported by `mod.rs`.
- `snapshot()` validates request filters and returns
  `WorkflowTraceSnapshotResponse` with canonical `WorkflowTraceSummary` values.
- `select_runtime_metrics()` reuses the same backend-owned request semantics
  but only returns runtime metrics when the filter resolves to exactly one
  execution. Multi-run matches stay explicit through
  `matched_execution_ids` instead of silently picking the first trace.
- `snapshot()` trims surrounding whitespace from optional filters and rejects
  blank filter values instead of silently inventing adapter-local fallback
  semantics.
- `snapshot()` may include optional timing expectations when a diagnostics
  ledger is configured. Missing timing expectations mean history is unavailable
  or insufficient, not that a node failed to run.
- `snapshot_all()` and `clear_history()` operate on the retained in-memory
  trace set owned by this directory.
- Callers may set execution metadata and graph context additively before or
  after trace events arrive; the store reconciles those facts into the same
  execution record.

## Structured Producer Contract
- Public trace DTOs use snake_case field names and enum labels.
- `WorkflowTraceSummary.runtime.observed_runtime_ids` preserves every
  backend-observed producer runtime id retained for the run.
- `WorkflowTraceSnapshotResponse.retained_trace_limit` communicates the current
  in-memory retention bound to consumers.
- `WorkflowTraceSnapshotRequest` supports additive filtering by
  `execution_id`, `session_id`, `workflow_id`, and `workflow_name`.
- When a filter field is omitted, snapshot semantics fall back to the backend
  default rather than adapter-specific filtering behavior.
- Queue timing fields are authoritative-only: missing queue timestamps remain
  absent instead of being synthesized from scheduler snapshot capture time.
- `waiting_for_input` and the derived run status are backend-owned lifecycle
  facts: interactive pause transitions and incremental resume transitions must
  reconcile here before any adapter or diagnostics projection consumes them.
- The latest dirty-task set, incremental rerun task ids, and graph-memory
  impact summary are backend-owned trace facts. Adapters may cache or replay
  them for live presentation, but they must not become the canonical owner of
  graph-reconciliation history.
- `timing_expectation` fields are optional duration-comparison projections.
  They are not progress percentages and consumers must render insufficient
  history explicitly.
- Terminal workflow events persist timing observations before the returned
  snapshot is enriched, so a completed run can contribute to the history shown
  after completion. In-progress elapsed durations must not be classified as
  faster than usual.
