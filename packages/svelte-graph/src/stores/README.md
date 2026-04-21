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
| `canonicalizeWorkflowGraph.ts` | Canonicalizes loaded graphs by reconciling legacy node types, stale inference-setting overlays, and missing expand-setting passthrough edges before a session starts. |
| `definitionOverlay.ts` | Rehydrates backend-supplied additive `node.data.definition` port overlays on top of static registry metadata during graph materialization. |
| `inferenceSettingsPorts.ts` | Builds additive inference-setting port definitions, merges upstream schema with promoted inference-node defaults, and de-duplicates settings that should flow only through `inference_settings`. |
| `runtimeData.ts` | Removes transient execution data from nodes without touching persisted configuration fields. |
| `workflowExecutionEvents.ts` | Reduces backend-owned workflow execution events into read-only execution overlays and downstream runtime-data mirrors for GUI consumers. |

## Problem
The graph editor needs a shared state boundary that can serve both UI rendering
and transport payload generation. Interactive connection guidance adds another
cross-cutting concern: the UI needs backend-derived target eligibility while the
stores still need a current graph revision snapshot to validate a commit.
Model-derived inference settings add a second contract problem: the stores must
shape additive per-node ports consistently so `expand-settings`, primitive
inputs, and inference nodes all see the same override handles.

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
Loaded graphs are canonicalized before the edit session is created and again
before the local node array is materialized, so legacy saved workflows are
reconciled into the current dynamic-port contract instead of remaining as stale
snapshots until a manual reconnect happens.
The same store now owns selected-node persistence: `selectedNodeIds` is updated
from graph selection events, applied back onto freshly materialized node
snapshots, and reset when workflows or sessions are cleared so backend graph
replacements do not silently drop the current selection.
Inference-setting port shaping now lives in `inferenceSettingsPorts.ts` so the
same additive port contract is reused when syncing expand-setting passthrough
nodes and downstream inference consumers. That helper now merges upstream
schema with promoted inference-node defaults, strips duplicate direct inference
ports from the node-visible definition, and keeps expand-setting schemas stable
when multiple inference consumers are attached. `definitionOverlay.ts` ensures
those dynamic ports survive graph rehydration when backend snapshots already
include per-node `definition.inputs` and `definition.outputs` overlays.
Workflow execution event reduction now lives in
`workflowExecutionEvents.ts` instead of `WorkflowToolbar.svelte`, keeping the
component focused on subscription and run-lifecycle ownership while the store
boundary owns the read-only event-to-overlay reduction shared by GUI
consumers. That reducer now consumes the explicit execution ownership projection
from `workflowEventOwnership.ts` so active-run identity and stale-event
relevance are evaluated once before node overlays or runtime-data mirrors are
updated.
Collapsed node group create, ungroup, and port-mapping edits now follow the
same backend-session rule as other structural graph mutations:
`createWorkflowStores.ts` applies the returned graph mutation snapshot and
derives `nodeGroups` from group node data rather than rewriting group nodes or
boundary edges locally.

## Alternatives Rejected
- Store connection intent only inside `WorkflowGraph.svelte`.
  Rejected because nodes also need the same state to render eligibility cues.
- Recompute graph revision only when saving or syncing from the backend.
  Rejected because drag-time commits need a current fingerprint for stale-intent
  protection.

## Invariants
- Structural graph edits must originate from a backend session and update local
  stores only from the returned graph snapshot.
- Node group stores are derived from backend graph snapshots; group create,
  ungroup, and port edits must not synthesize nodes or boundary edges locally.
- `workflowGraph` must reflect the latest nodes, edges, and derived graph
  fingerprint after every applied graph snapshot.
- `connectionIntent` is not persisted; it must reset on graph mutation,
  workflow load, workflow clear, and default-graph load.
- `selectedNodeIds` is transient UI state; graph rematerialization must project
  it back onto node snapshots until the user or consumer clears the selection.
- Runtime cleanup helpers must continue to touch only explicitly requested
  transient keys.
- Dynamic inference-setting ports must be derived from backend-owned schema and
  written back into `node.data.definition`; ad hoc component-local copies are
  not authoritative.
- Backend workflow events may update execution overlays and additive runtime
  output mirrors in store-managed state, but they must not become a second
  source of truth for persisted graph structure.
- Execution event reducers must consume the shared workflow event ownership
  projection instead of composing execution-id claiming and relevance checks
  locally.

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
- Store consumers that own graph selection must update `selectedNodeIds`
  whenever the rendered graph selection changes, or backend snapshot refreshes
  will intentionally reapply the last known ids.
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
- Dynamic inference-setting ports are additive overlays on `node.data.definition`;
  saved graphs may persist them, but the authoritative shape is regenerated from
  schema sync when model metadata changes.
- When an inference node is synchronized from an `inference_settings` source,
  settings promoted into that shared schema surface must not remain duplicated
  as direct static inputs in the node-visible definition.
- Graph load canonicalization must be idempotent: reloading an already current
  graph must not keep appending edges or reshaping definitions beyond the
  current contract.
