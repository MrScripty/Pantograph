# packages/svelte-graph/src/components

## Purpose
This directory contains the reusable Svelte graph editor surface for package
consumers: canvas orchestration, node rendering, edge rendering, navigation,
and editing tools. The boundary exists so connection UX, graph navigation, and
shared node presentation rules live outside the Pantograph app shell.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `WorkflowGraph.svelte` | Main graph canvas that owns connect/reconnect flows, candidate loading, and revision-aware edge commits. |
| `NodePalette.svelte` | Palette for adding node definitions into the active graph. |
| `CutTool.svelte` | Edge-cut interaction used for Ctrl-drag deletion. |
| `ContainerBorder.svelte` | Orchestration/group boundary overlay used during zoom transitions. |
| `nodes/` | Shared node shells and reusable package node components, including connection-intent highlighting. |
| `edges/` | Edge renderers and reconnect affordances used by `WorkflowGraph.svelte`. |

## Problem
Package consumers need a graph editor that can enforce backend-owned connection
eligibility while still feeling interactive locally. If canvas behavior lives in
app code, each consumer would drift on snapping, rejection handling, and drag
state cleanup.

## Constraints
- Components must work with any `WorkflowBackend` implementation provided
  through graph context.
- Drag-time validation must be synchronous from the component perspective even
  though candidate discovery is async.
- Node and edge components must preserve `@xyflow/svelte` expectations for
  reconnect, selection, and internal measurement metadata.

## Decision
Keep connection intent client-owned in `WorkflowGraph.svelte`: on connect start,
it requests candidates from the backend, caches them in workflow stores, uses
that cache for `isValidConnection`, and clears the intent on cancel, pane click,
or commit. Node shells then read the same store to dim incompatible targets and
highlight eligible anchors.

## Alternatives Rejected
- Ask the backend on every pointer move.
  Rejected because drag performance would depend on round-trip latency.
- Keep compatibility highlighting local to node definitions only.
  Rejected because target occupancy, cycles, and stale revisions depend on live
  session state.

## Invariants
- `WorkflowGraph.svelte` must never create an edge locally that bypasses the
  backend-owned `connectAnchors` commit path.
- Connection-intent highlighting must clear when the graph changes or the drag
  interaction ends.
- Reconnect flows that temporarily remove an edge must restore the original edge
  if the replacement commit is rejected.

## Revisit Triggers
- Insert-and-connect becomes a committed graph operation with its own UI flow.
- Backend candidate queries become too slow for one-shot drag-start loading.
- Package consumers need custom candidate ranking or filtering hooks.

## Dependencies
**Internal:** `packages/svelte-graph/src/stores`, `packages/svelte-graph/src/context`,
`packages/svelte-graph/src/types`, `packages/svelte-graph/src/constants`.

**External:** Svelte 5 and `@xyflow/svelte`.

## Related ADRs
- None.
- Reason: the component architecture remains internal to the graph package.
- Revisit trigger: component lifecycle or context requirements become part of a
  documented public SDK.

## Usage Examples
```svelte
<script lang="ts">
  import { WorkflowGraphEditor } from '@pantograph/svelte-graph';
</script>

<WorkflowGraphEditor showContainerBorder={true} />
```

## API Consumer Contract (Host-Facing Modules)
- Components in this directory expect a graph context created with the package
  context helpers; they do not manage backend/session setup themselves.
- `WorkflowGraph.svelte` consumes workflow, view, and session stores from that
  context and assumes `workflowGraph.derived_graph.graph_fingerprint` is kept
  current.
- Connection rejection is surfaced through console logging and shared store
  state today; consumers should not expect custom DOM events for rejection yet.
- Compatibility policy is additive: new graph behaviors should layer on the
  existing context contract instead of replacing it silently.

## Structured Producer Contract (Machine-Consumed Modules)
- None.
- Reason: components render UI and do not publish persisted machine-consumed
  artifacts directly.
- Revisit trigger: a component begins generating saved templates, schemas, or
  serialized graph metadata.
