# packages/svelte-graph/src/stores

## Purpose
This directory contains the store factories that assemble graph session, view,
and workflow state for the reusable `@pantograph/svelte-graph` package. The
boundary exists so graph state mutations, derived graph snapshots, and runtime
execution updates are defined once and reused by both package consumers and the
Pantograph application wrapper.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `createWorkflowStores.ts` | Owns workflow node, edge, execution, and streaming state, including execution-local runtime data updates. |
| `createViewStores.ts` | Manages viewport, group navigation, and presentation-oriented graph view state. |
| `createSessionStores.ts` | Coordinates graph loading, selection, and session lifecycle around the workflow stores. |
| `runtimeData.ts` | Pure helpers for removing execution-local node data fields without mutating saved configuration. |

## Problem
The graph UI needs a single source of truth for both persisted graph structure
and transient execution state. Without a dedicated store layer, runtime updates
from workflow events would leak through components inconsistently and rerun
cleanup rules would be duplicated across the application.

## Constraints
- Store updates must remain reactive and cheap enough for frequent workflow
  execution events.
- Runtime state cleanup must not delete durable node configuration that is saved
  with the workflow graph.
- The package store API is shared by Pantograph app code and reusable graph
  components, so surface changes have broad frontend impact.

## Decision
Keep persisted graph edits and execution-local runtime data in the same workflow
store, but expose explicit actions for runtime-only mutations and cleanup. The
audio rerun fix follows that rule by clearing targeted runtime keys at run start
instead of rebuilding nodes or mutating saved graph data.

## Alternatives Rejected
- Store runtime execution data in ad hoc component state only.
  Rejected because upstream workflow events need a shared distribution path to
  downstream nodes.
- Recreate the whole workflow graph on every run to clear runtime state.
  Rejected because it would disrupt selection, viewport state, and local UI
  ownership for no benefit.

## Invariants
- `updateNodeData` is for durable graph/configuration edits; runtime-only cleanup
  must not rely on it.
- Execution-local cleanup helpers must remove only targeted keys and leave all
  other node data unchanged.
- Derived `workflowGraph` output must continue to reflect the current node and
  edge stores after runtime updates and user edits.

## Revisit Triggers
- Multiple runtime-data cleanup policies emerge and require per-node-type
  ownership rather than key-based clearing.
- Workflow execution begins supporting overlapping runs and needs execution-id
  partitioning inside the store layer.
- Package consumers need a formal separation between persisted graph state and
  runtime overlays.

## Dependencies
**Internal:** `packages/svelte-graph/src/types`, `packages/svelte-graph/src/backends`,
and view/session store factories in this directory.

**External:** Svelte stores and `@xyflow/svelte` node/edge types.

## Related ADRs
- None.
- Reason: no ADR currently covers store-level ownership of execution-local node
  data versus persisted workflow configuration.
- Revisit trigger: the store API changes in a way that affects external package
  consumers or serialized workflow compatibility.

## Usage Examples
```ts
import { createWorkflowStores } from '@pantograph/svelte-graph';

const stores = createWorkflowStores(backend);
stores.clearNodeRuntimeData(['audio', 'stream']);
```

## API Consumer Contract (Host-Facing Modules)
None.
Reason: this store API is consumed by frontend code within the package/app
boundary, not by external hosts or a cross-process API surface.
Revisit trigger: these stores become a supported plugin SDK surface.

## Structured Producer Contract (Machine-Consumed Modules)
- `workflowGraph` mirrors the current node and edge stores into the graph shape
  consumed by backend workflow execution commands.
- Runtime node data may contain transient execution fields such as stream chunks
  and terminal outputs; callers must treat those fields as volatile unless the
  node definition documents them as persisted configuration.
- Cleanup helpers such as `clearNodeRuntimeData` remove only the requested keys;
  absence of a key means “no runtime value is currently present,” not “use a
  persisted default.”
- If persisted workflow compatibility changes, update this README or add an ADR
  before changing the store contract.
