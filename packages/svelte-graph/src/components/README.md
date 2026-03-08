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
| `edges/` | Edge renderers and reconnect affordances used by `WorkflowGraph.svelte`; reconnect is intentionally target-side only so occupied output handles stay available for fan-out drags. |

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
or commit. Horseshoe invocation now goes through a shared drag-session
controller: the graph starts a shared drag session on connect/reconnect start,
captures `Space` and pointer movement through drag-scoped window listeners,
preserves queued opens while candidates load, records explicit blocked reasons,
and renders pending/blocked selector states instead of silently doing nothing.
`HorseshoeInsertSelector.svelte` opens once the session is ready. Node shells
then read the same store to dim incompatible targets and highlight eligible
anchors. The canvas now tracks explicit drag mode as part of that shared
interaction contract: normal output-handle drags are connect/fan-out flows,
while explicit target-end reconnect drags are the only reconnect path. Source
reconnect anchors are not rendered because they overlap occupied output handles
and conflict with multi-edge fan-out. Once the horseshoe is open, the first
`Space` has already been consumed; pressing `Space` again confirms the current
highlighted insert candidate, clears drag state immediately, and leaves pointer
motion free to change the highlighted item against a fixed menu anchor. Insert
confirmation now keeps the horseshoe state alive until the backend responds; if
the insert is rejected, the selector stays visible, shows the rejection
message, and refreshes candidates from the returned graph revision for retry.

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
- Horseshoe open failures must resolve to an explicit blocked reason instead of
  silently doing nothing.
- Successful horseshoe confirmation must end the drag before later drag-end
  cleanup runs so the pointer is no longer treated as dragging an edge.
- While the horseshoe is open, pointer movement must not reposition the menu;
  it can only affect item selection inside the existing anchored layout.
- Rejected horseshoe inserts must remain visible in-context and refresh
  connection-intent candidates against the backend-returned revision instead of
  silently clearing the interaction.
- Reconnect cleanup must only remove the original edge for unfinished reconnect
  drags; normal connect/horseshoe flows must never inherit reconnect cleanup.
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
- `WorkflowGraph.svelte` binds `Space` to the horseshoe selector through
  drag-scoped window listeners and the shared drag-session controller; failed
  opens remain visible through pending/blocked selector states plus internal
  blocked-reason diagnostics.
- When the horseshoe is open, `Space` and `Enter` both confirm the highlighted
  insert candidate; successful confirmation ends drag ownership immediately.
- Rejected insert confirmations keep the horseshoe visible with a status
  message so release users are not dependent on browser-console warnings.
- Horseshoe insertion is only supported for normal connect drags. Explicit
  reconnect drags surface a blocked reason instructing the user to start a new
  drag from the output handle instead of pretending to support splice semantics.
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
