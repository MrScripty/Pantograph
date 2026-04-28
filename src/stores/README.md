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
| `workbenchStore.ts` | Transient workbench navigation and active-run context shared by Scheduler, Diagnostics, Graph, I/O Inspector, Library, Network, and Node Editor pages. |
| `schedulerRunListStore.ts` | Transient Scheduler run-table filter, sort, and column-visibility state shared by the Scheduler page and presenter tests. |
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
pushing merge policy into Svelte components. Diagnostics relevance decisions
come from the backend projection context; this store only drops snapshots marked
irrelevant by that context. `workbenchStore.ts` owns the frontend-only page
selection and active-run context used by the run-centric shell. That state is
deliberately transient: it coordinates pages during the current GUI session, but
does not persist active-run selection or become a backend source of run truth.
`schedulerRunListStore.ts` owns dense Scheduler table filter, sort, and
column-visibility state so the page does not duplicate run-list UI state in
component-local variables while backend projection services remain the only
source of run data. Scope and accepted-date filters stay in the same transient
store boundary as status and policy filters.

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
- Workbench page selection and active-run context are frontend navigation state
  only. Components must fetch run details, timelines, I/O metadata, and Library
  usage through backend projection services rather than enriching the selected
  active-run context with durable data.
- Scheduler run-list filters, sort order, and column visibility are transient
  UI preferences. They must not mutate backend queue state or be treated as
  scheduler policy.
- Scheduler scope and accepted-date filters are presentation filters over
  backend projection fields. They must not become client authority checks or
  durable scheduler policy.

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
- `diagnosticsStore.ts` must treat `WorkflowDiagnosticsProjection.context` as
  the owner of diagnostics snapshot relevance and execution attribution.

## Structured Producer Contract (Machine-Consumed Modules)
- `workflowGraph` remains the graph projection consumed by backend/service
  layers.
- `connectionIntent` is transient and may be `null`; consumers must not persist
  it.
- `workflowGraph.derived_graph.graph_fingerprint` is backend-owned metadata
  when a backend graph snapshot provides it, and it is the revision token used
  for connection-intent commits. Frontend package stores only rebuild it for
  local/default graph construction or loaded graphs that lack derived metadata.
- Diagnostics snapshots are in-memory, session-scoped views over workflow
  events; they are not durable artifacts in v1.
- Workbench active-run context is in-memory GUI state and may be `null` at
  startup even when runs exist in scheduler projections.
- Scheduler run-list filter state is in-memory GUI state, including status,
  policy, scope, accepted-date, search, and sort controls. It may be reset
  without changing queued runs, selected active run, or backend projections.
- `currentSessionState` is an additive backend-owned inspection snapshot and
  may be absent from event-driven projections even when a direct diagnostics
  fetch has already populated it.
- `WorkflowDiagnosticsProjection.context` is backend-authored; missing context
  may be normalized only for compatibility with older producer paths.
