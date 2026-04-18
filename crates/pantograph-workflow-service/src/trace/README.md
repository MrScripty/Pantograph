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
| `types.rs` | Canonical workflow trace DTOs, event enums, and request/response contracts. |
| `store.rs` | Owns retained trace state, replay behavior, request filtering, and snapshot generation. |
| `runtime.rs` | Applies runtime snapshot events to canonical workflow trace state. |
| `scheduler.rs` | Applies scheduler snapshot events to canonical workflow trace state. |

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
in-memory retained trace state and request validation path, and `runtime.rs`
plus `scheduler.rs` apply backend-owned runtime/scheduler facts into the
canonical run state. The canonical trace snapshot filter model is
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
- Runtime and scheduler snapshot application must preserve backend-owned
  execution and session identity when those facts are available.
- Recovery or replay updates for the same execution id update one canonical run
  record in place.

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
- `snapshot()` trims surrounding whitespace from optional filters and rejects
  blank filter values instead of silently inventing adapter-local fallback
  semantics.
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
