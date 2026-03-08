# packages/svelte-graph/src/stores

## Purpose
This directory assembles the package’s reactive state model for workflow,
session, and view concerns. The boundary exists so reusable graph components and
Pantograph app wrappers share one definition of graph structure, derived graph
metadata, execution overlays, and transient connection-intent state.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `createWorkflowStores.ts` | Owns workflow nodes, edges, derived graph metadata, execution state, and connection intent for the active editor. |
| `createSessionStores.ts` | Manages session lifecycle, graph loading, and current graph selection. |
| `createViewStores.ts` | Holds viewport and navigation state such as group stacks and zoom targets. |
| `runtimeData.ts` | Removes transient execution data from nodes without touching persisted configuration fields. |

## Problem
The graph editor needs a shared state boundary that can serve both UI rendering
and transport payload generation. Interactive connection guidance adds another
cross-cutting concern: the UI needs backend-derived target eligibility while the
stores still need a current graph revision snapshot to validate a commit.

## Constraints
- Derived graph fingerprints must stay synchronized with node and edge edits or
  revision-aware connection commits become unreliable.
- Runtime execution data must remain separable from persisted graph state.
- Connection intent is transient UI state and must be cleared aggressively when
  the graph changes, a session changes, or the interaction is cancelled.

## Decision
Keep `connectionIntent` inside `createWorkflowStores.ts` alongside the graph and
derived graph state. That lets `WorkflowGraph.svelte` fetch candidates once,
store them centrally, reuse them for synchronous drag validation, and clear the
intent from one place whenever nodes, edges, or workflow loads change.
`createSessionStores.ts` now binds the active edit-session id into
`createWorkflowStores.ts`, and structural graph edits flow through the backend
session before the stores replace local graph state from the returned snapshot.

## Alternatives Rejected
- Store connection intent only inside `WorkflowGraph.svelte`.
  Rejected because nodes also need the same state to render eligibility cues.
- Recompute graph revision only when saving or syncing from the backend.
  Rejected because drag-time commits need a current fingerprint for stale-intent
  protection.

## Invariants
- Structural graph edits must originate from a backend session and update local
  stores only from the returned graph snapshot.
- `workflowGraph` must reflect the latest nodes, edges, and derived graph
  fingerprint after every applied graph snapshot.
- `connectionIntent` is not persisted; it must reset on graph mutation,
  workflow load, workflow clear, and default-graph load.
- Runtime cleanup helpers must continue to touch only explicitly requested
  transient keys.

## Revisit Triggers
- Multiple simultaneous connection intents need independent store partitions.
- Session-driven server state becomes authoritative for active drag intent.
- Derived graph computation becomes expensive enough to require incremental
  updates instead of full recomputation on every edit.

## Dependencies
**Internal:** `packages/svelte-graph/src/types`, `packages/svelte-graph/src/backends`,
`packages/svelte-graph/src/graphRevision.ts`.

**External:** Svelte stores and `@xyflow/svelte` node/edge types.

## Related ADRs
- None.
- Reason: the store ownership split is still local to the package and app.
- Revisit trigger: a future refactor separates persisted graph state and
  transient UI intent into distinct supported APIs.

## Usage Examples
```ts
import { createWorkflowStores } from '@pantograph/svelte-graph';

const stores = createWorkflowStores(backend);
stores.setConnectionIntent({
  sourceAnchor: { node_id: 'llm', port_id: 'response' },
  graphRevision: 'abc123',
  compatibleNodeIds: ['output'],
  compatibleTargetKeys: ['output:text'],
  insertableNodeTypes: [],
});
```

## API Consumer Contract (Host-Facing Modules)
- Store consumers should mutate graph structure through store actions or backend
  sync helpers, not by assigning directly to node/edge arrays.
- Session stores must set the active session id before loading a graph that can
  trigger backend-owned edits.
- `setConnectionIntent` accepts either a fully derived UI intent object or
  `null`; `clearConnectionIntent` is the preferred cancellation path.
- Session/view stores depend on workflow stores being created first and passed
  into `createSessionStores`.
- Compatibility policy is additive: store fields may grow, but existing graph
  and connection-intent semantics should not silently change.

## Structured Producer Contract (Machine-Consumed Modules)
- `workflowGraph` is the machine-consumed projection of the active node and edge
  stores.
- `derived_graph.graph_fingerprint` and `consumer_count_map` are regenerated
  metadata; consumers should not persist hand-authored values.
- `connectionIntent.graphRevision` records the candidate query snapshot and is
  invalid once `workflowGraph.derived_graph.graph_fingerprint` changes.
- Missing `connectionIntent` means “no active connect/reconnect interaction,”
  not “no compatible targets exist.”
