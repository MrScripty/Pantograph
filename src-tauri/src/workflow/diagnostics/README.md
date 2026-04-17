# src-tauri/src/workflow/diagnostics

## Purpose
This directory contains the Tauri-side workflow diagnostics projection layer.
It exists so the desktop host can retain additive UI/debug overlays while
reusing backend-owned workflow trace, runtime snapshot, and scheduler snapshot
contracts from Rust service crates rather than rebuilding those semantics in
TypeScript or command handlers.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | Module entrypoint that exposes the diagnostics store and transport-facing DTOs. |
| `store.rs` | Owns the retained overlay state and merges it with backend-owned trace snapshots. |
| `trace.rs` | Converts workflow events and backend trace summaries into diagnostics-friendly run projections. |
| `types.rs` | Defines the Tauri-facing diagnostics DTOs and snapshot request/response shapes. |
| `tests.rs` | Regression coverage for projection, replay, and clear-history behavior. |

## Problem
The GUI needs a stable diagnostics view that includes retained event history,
last progress text, and current runtime/scheduler overlays, but the canonical
workflow execution truth already lives in backend Rust crates. Without a narrow
projection layer here, Tauri command code would either duplicate trace logic or
the frontend would become a second owner of execution state.

## Constraints
- Canonical run, node, runtime, and scheduler semantics come from
  `pantograph-workflow-service` and `pantograph-embedded-runtime`.
- Tauri may retain additive UI/debug overlays, but it must not become the
  owner of workflow execution truth.
- Diagnostics payloads cross a host/UI boundary and must keep deterministic
  field names and omission semantics.
- Clear-history and replay flows must rebuild from backend-owned trace state
  rather than preserving stale adapter-local runs.

## Decision
Keep Tauri diagnostics as a projection-only boundary. `store.rs` owns only the
retained overlays that do not exist in canonical workflow trace state, while
`trace.rs` and `types.rs` adapt backend-owned `WorkflowTraceStore`,
runtime-lifecycle snapshots, and scheduler snapshots into the GUI diagnostics
shape. This preserves one backend source of truth while still supporting
desktop debug views and retained event history.

## Alternatives Rejected
- Rebuild diagnostics state in TypeScript.
  Rejected because business logic and execution truth are backend-owned in
  Pantograph.
- Fold retained UI overlays into `pantograph-workflow-service::WorkflowTraceStore`.
  Rejected because progress text, retained event truncation, and other GUI-only
  overlays are transport concerns, not canonical workflow-service state.

## Invariants
- `WorkflowTraceStore` remains the canonical owner of workflow trace history.
- This directory may merge overlays onto backend traces, but it must not invent
  new canonical run records from overlay-only runtime or scheduler state.
- Runtime lifecycle fallback and producer-aware runtime shaping remain
  backend-owned in `pantograph-embedded-runtime`.
- Diagnostics payload casing and enum labels stay stable for the GUI consumer.

## Revisit Triggers
- A non-Tauri host needs the same diagnostics projection boundary.
- The GUI stops needing retained overlay-only event history.
- Canonical workflow trace contracts expand enough that this directory can
  shrink to pure transport serialization.

## Dependencies
**Internal:** `pantograph-workflow-service::WorkflowTraceStore`,
`pantograph-embedded-runtime::workflow_runtime`, and
`src-tauri/src/workflow` command/event modules.

**External:** `serde` for transport serialization and the Tauri host runtime
that consumes these DTOs.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- Reason: diagnostics projection is a host adapter boundary over backend-owned
  workflow and runtime state.
- Revisit trigger: diagnostics ownership moves out of the Tauri host or gains a
  second app host.

## Usage Examples
```rust
let diagnostics = WorkflowDiagnosticsStore::default();
let projection = diagnostics.snapshot();
let trace = diagnostics.trace_snapshot(Default::default())?;
```

## API Consumer Contract
- Tauri workflow commands and runtime debug commands may request snapshots from
  this directory, but they must treat the returned run/runtime/scheduler facts
  as projections over backend-owned trace and runtime data.
- `WorkflowDiagnosticsProjection` returns `runs_by_id`, `run_order`, `runtime`,
  `scheduler`, and `retained_event_limit` with stable field names.
- `trace_snapshot()` validates backend-owned trace filters through
  `WorkflowTraceSnapshotRequest` instead of accepting arbitrary adapter-local
  filtering rules.
- `clear_history()` clears retained overlays and backend trace history together
  so the GUI does not keep stale local diagnostics after a reset.

## Structured Producer Contract
- `DiagnosticsRunTrace`, `DiagnosticsNodeTrace`, and related DTOs are
  machine-consumed transport shapes with camelCase fields.
- Runtime and scheduler overlays are additive. When canonical backend trace
  identity is absent, this directory may update overlay-only state but must not
  synthesize a canonical execution id locally.
- `DiagnosticsTraceRuntimeMetrics.observed_runtime_ids` preserves every
  backend-observed producer runtime id from the underlying trace.
- Omitted optional fields mean the backend did not provide a value; consumers
  must not infer stronger semantics from absence.
