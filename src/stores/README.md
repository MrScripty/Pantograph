# src/stores

## Purpose
This directory contains Pantograph’s app-level store surface. It wraps the
reusable graph package store factories with singleton instances and re-exports
them alongside Pantograph-specific stores so legacy components and newer package
components observe the same graph state.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `storeInstances.ts` | Creates the shared backend, registry, and package-derived store singletons used across the app. |
| `workflowStore.ts` | Thin compatibility layer that re-exports workflow store instances and actions for app components. |
| `diagnosticsStore.ts` | Single app-level owner for diagnostics subscriptions, trace snapshots, and diagnostics panel state. |
| `diagnosticsProjection.ts` | Pure helper module that normalizes diagnostics projections and builds immutable UI snapshots without subscribing to workflow events itself. |
| `graphSessionStore.ts` | Tracks the active graph/session identity at the app layer. |
| `viewStore.ts` | App navigation and zoom wrappers built around the package view stores. |
| `architectureStore.ts` | Converts architecture data into workflow-like graph structures for the shared canvas. |

## Problem
Pantograph has older app components that import global stores directly, but the
reusable graph package expects per-instance stores provided through context. The
app needs one place where those models meet so connection-intent state,
graph revisions, execution overlays, and diagnostics subscriptions do not
split.

## Constraints
- Legacy imports from `workflowStore.ts` must keep working during migration.
- Package components and app components must point at the same workflow store
  instances.
- Graph revision metadata and connection-intent state must stay synchronized
  regardless of whether the caller uses package context or legacy re-exports.
- Diagnostics subscriptions and retained trace history need one owner so
  workflow debugging state does not duplicate across components.

## Decision
Create package store singletons once in `storeInstances.ts`, then re-export the
relevant workflow store handles through `workflowStore.ts`. The new
`connectionIntent`, `setConnectionIntent`, and `clearConnectionIntent` exports
follow that pattern so app nodes and the app graph consume the same transient
eligibility state as package components. `diagnosticsStore.ts` follows the same
pattern for workflow diagnostics: it subscribes once to workflow events,
workflow graph snapshots, graph-session metadata, and the current workflow
session id, then hydrates immutable diagnostics snapshots with both event
history and workflow-service-backed runtime or scheduler views. When the active
session id is a graph edit session instead of a workflow-service session,
`diagnosticsStore.ts` now keeps a synthetic scheduler session summary in sync
with execution lifecycle events and suppresses expected `session_not_found`
noise from the scheduler panel. `diagnosticsProjection.ts` now owns the pure
projection-normalization step so additive fields like backend-owned
`currentSessionState` can be preserved across mixed producer paths without
pushing merge policy into Svelte components.

## Alternatives Rejected
- Keep separate app-only and package-only workflow stores.
  Rejected because graph revisions and intent highlighting would drift.
- Remove the legacy store facade immediately.
  Rejected because too many existing app components still import it directly.

## Invariants
- `workflowStore.ts` remains a facade over the singleton package workflow stores,
  not a second source of truth.
- Store singletons must be created once per app runtime.
- Connection-intent state exported here must reflect the same object seen by
  package components through context.
- `diagnosticsStore.ts` owns diagnostics subscriptions and retained trace
  history for the app runtime; components must not create parallel diagnostics
  listeners lightly.
- Workflow-service refreshes for runtime capabilities and session queue state
  should be triggered from this store boundary, not from diagnostics
  components.
- Projection merge policy for additive diagnostics fields belongs in the pure
  helper module at this boundary rather than in view code or workflow-service
  adapters.
- Expected edit-session scheduler misses must degrade to synthetic scheduler
  state here instead of surfacing as persistent user-facing errors.

## Revisit Triggers
- The legacy store facade is no longer imported anywhere.
- Multiple concurrent graph editors need isolated singleton sets.
- App-specific state diverges enough from package stores that a new boundary is
  required.

## Dependencies
**Internal:** `packages/svelte-graph`, `src/backends`, `src/registry`,
`src/services/workflow`.

**External:** Svelte stores.

## Related ADRs
- None.
- Reason: the singleton/facade pattern remains transitional.
- Revisit trigger: the migration off the facade reaches a stable end state.

## Usage Examples
```ts
import { connectionIntent, clearConnectionIntent } from '../stores/workflowStore';

connectionIntent.subscribe((intent) => {
  console.log(intent?.compatibleTargetKeys ?? []);
});

clearConnectionIntent();
```

```ts
import { diagnosticsSnapshot, startDiagnosticsStore } from '../stores/diagnosticsStore';

startDiagnosticsStore();
diagnosticsSnapshot.subscribe(({ selectedRun }) => {
  console.log(selectedRun?.status ?? 'no-run-selected');
});
```

## API Consumer Contract (Host-Facing Modules)
- App components should import from `workflowStore.ts` when they need the legacy
  global store facade.
- New graph features added to package workflow stores should be re-exported here
  only when app components still need direct access.
- The re-export surface is compatibility-oriented; breaking removals should wait
  until app callers have migrated.
- Diagnostics consumers should read `diagnosticsSnapshot` and use exported
  commands for selection or visibility changes instead of mutating trace data
  directly.
- Runtime and scheduler diagnostics are refreshed here in response to workflow
  id, session id, and execution lifecycle changes rather than by polling loops.
- Workflow-event-driven scheduler fallback and backend-sourced scheduler
  snapshots share this store boundary; components should not attempt to
  distinguish the producer path themselves.

## Structured Producer Contract (Machine-Consumed Modules)
- `workflowGraph` remains the graph projection consumed by backend/service
  layers.
- `connectionIntent` is transient and may be `null`; consumers must not persist
  it.
- `workflowGraph.derived_graph.graph_fingerprint` is regenerated metadata and is
  the revision token used for connection-intent commits.
- Diagnostics snapshots are in-memory, session-scoped views over workflow
  events; they are not durable artifacts in v1.
- `currentSessionState` is an additive backend-owned inspection snapshot and
  may be absent from event-driven projections even when a direct diagnostics
  fetch has already populated it.
