# src-tauri/src/workflow/event_adapter

## Purpose
This directory contains the focused helper modules behind the stable
`workflow::event_adapter` facade. It translates backend-owned `node-engine`
workflow events into Tauri transport events and refreshes the backend-owned
diagnostics projection without turning the Tauri adapter into the owner of
workflow semantics.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `translation.rs` | Pure translation from `node_engine::WorkflowEvent` into Tauri workflow-event DTOs. |
| `diagnostics_bridge.rs` | Diagnostics-store update bridge that pairs translated workflow events with backend-owned diagnostics snapshots. |
| `tests.rs` | Adapter regression coverage for translation and diagnostics projection behavior. |

## Problem
`src-tauri/src/workflow/event_adapter.rs` had become a shared insertion point
for event translation, diagnostics projection, and adapter tests. That made it
too easy for future Phase 5 work to mix transport details with backend-owned
workflow semantics in one oversized file.

## Constraints
- Tauri remains an adapter boundary and must not become the owner of workflow
  lifecycle meaning.
- Diagnostics projection stays backend-owned and the adapter may only record
  or forward that state.
- Event translation must preserve backend-owned execution identity and
  additive timing semantics.

## Decision
Keep `workflow::event_adapter` as the stable facade while moving translation
and diagnostics-bridge helpers into focused internal modules. This preserves
existing imports and adapter behavior while creating standards-compliant
insertion points for later event-contract completion work.

## Alternatives Rejected
- Leaving translation and diagnostics-bridge logic in one file.
  Rejected because the adapter file already exceeded decomposition thresholds.
- Moving diagnostics update logic into TypeScript.
  Rejected because diagnostics state is backend-owned and must remain in Rust.

## Invariants
- `TauriEventAdapter` remains a transport adapter over backend-owned event
  semantics.
- Event translation preserves backend execution ids rather than synthesizing
  adapter-local ownership.
- Translation of `WaitingForInput`, `GraphModified`, and
  `IncrementalExecutionStarted` must preserve backend-owned prompt/task,
  dirty-task, and resumed-task semantics rather than collapsing them into
  adapter-local diagnostics-only state.
- Diagnostics snapshots are derived from backend-owned trace and workflow-event
  projections, not from frontend reconstruction.

## Revisit Triggers
- Tauri workflow transport gains another distinct event family that needs a
  separate helper module.
- Diagnostics projection ownership moves again and the bridge no longer belongs
  at this boundary.
- The adapter begins to expose enough transport-specific behavior to require a
  higher-level internal module split.

## Dependencies
**Internal:** `workflow::diagnostics`, `workflow::events`, and
`node_engine::WorkflowEvent`.

**External:** Tauri IPC channel types and serde-backed event payloads.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- Reason: This adapter remains part of the Tauri transport boundary rather than
  a new backend owner.
- Revisit trigger: Workflow execution transport leaves the Tauri app boundary
  or diagnostics projection ownership changes materially.

## Usage Examples
```rust
use crate::workflow::event_adapter::TauriEventAdapter;
```

## API Consumer Contract
- External callers instantiate `TauriEventAdapter` through the stable
  `workflow::event_adapter` facade.
- The helper modules in this directory are internal implementation details and
  should not be imported directly by downstream callers.
- Adapter sends continue to emit the translated primary workflow event followed
  by a diagnostics snapshot event for the same execution id.

## Structured Producer Contract
- Translation preserves the canonical Tauri workflow-event DTO shapes already
  published by `workflow/events.rs`.
- Backend-owned `WorkflowCancelled` events pass through as explicit cancelled
  workflow events at this boundary; the adapter must not infer cancellation by
  classifying free-form failure strings.
- Backend-owned `WaitingForInput`, `GraphModified`, and
  `IncrementalExecutionStarted` events must preserve execution ownership and
  their additive payload fields when translated into the app-facing workflow
  event DTOs and diagnostics snapshots.
- Diagnostics snapshots emitted here must preserve backend-owned execution ids
  and backend trace timing when present.
