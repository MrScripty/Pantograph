# packages/svelte-graph/src/components

## Purpose
This directory contains the reusable Svelte graph editor surface for package
consumers: canvas orchestration, node rendering, edge rendering, navigation,
and editing tools. The boundary exists so connection UX, graph navigation, and
shared node presentation rules live outside the Pantograph app shell.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `WorkflowGraph.svelte` | Main graph canvas that owns connect/reconnect flows, candidate loading, revision-aware edge commits, and the drag-time horseshoe insert flow. |
| `NodePalette.svelte` | Palette for adding node definitions into the active graph. |
| `CutTool.svelte` | Edge-cut interaction used for Ctrl-drag deletion. |
| `ContainerBorder.svelte` | Orchestration/group boundary overlay used during zoom transitions. |
| `HorseshoeInsertSelector.svelte` | Cursor-anchored horseshoe selector used to browse compatible insertable node types during an active connection intent. |
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
or commit. When the user presses `Space` during an active drag, the same
component opens `HorseshoeInsertSelector.svelte` at the cursor, browses the
ranked `insertableNodeTypes` list locally with wheel/typeahead, and commits the
selection through backend-owned `insertNodeAndConnect`. Node shells then read
the same store to dim incompatible targets and highlight eligible anchors.

## Alternatives Rejected
- Ask the backend on every pointer move.
  Rejected because drag performance would depend on round-trip latency.
- Keep compatibility highlighting local to node definitions only.
  Rejected because target occupancy, cycles, and stale revisions depend on live
  session state.

## Invariants
- `WorkflowGraph.svelte` must never create an edge locally that bypasses the
  backend-owned `connectAnchors` commit path.
- Insert-from-drag must commit through backend-owned `insertNodeAndConnect`; the
  UI must not compose local `addNode` plus `connectAnchors`.
- Connection-intent highlighting must clear when the graph changes or the drag
  interaction ends.
- Reconnect flows that temporarily remove an edge must restore the original edge
  if the replacement commit is rejected.

## Revisit Triggers
- Backend candidate queries become too slow for one-shot drag-start loading.
- Package consumers need custom candidate ranking or filtering hooks.
- Horseshoe result counts routinely exceed the visible window and require
  category grouping or searchable overflow.

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
- `WorkflowGraph.svelte` binds `Space` to the horseshoe selector only while a
  connection intent with compatible insertable node types is active.
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
