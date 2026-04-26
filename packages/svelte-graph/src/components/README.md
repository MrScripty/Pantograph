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
| `WorkflowGraph.css` | Package graph canvas and SvelteFlow chrome styling imported by `WorkflowGraph.svelte`. |
| `WorkflowToolbar.svelte` | Graph toolbar shell for workflow save/new/clear controls and status badges. |
| `WorkflowRunButton.svelte` | Scheduler-backed run button, workflow-event subscription, active-run ownership, and run cleanup lifecycle. |
| `workflowGraphBackendActions.ts` | Owns package-local backend mutation calls, connection-intent loading, reconnect rollback, and accepted graph-sync projection used by `WorkflowGraph.svelte`. |
| `../workflowGraphBackendActionCore.ts` | Provides dependency-injected backend action primitives shared by package and app graph action adapters. |
| `../workflowGraphBackendActionCore.test.ts` | Unit coverage for shared mutation projection, connection rejection, edge removal, and reconnect rollback behavior. |
| `../workflowGraphDeletion.ts` | Projects package graph delete-selection and session-scoped edge-removal requests from SvelteFlow events. |
| `../workflowGraphDeletion.test.ts` | Unit coverage for package graph delete and edge-removal request projection. |
| `../workflowGraphReconnect.ts` | Projects reconnect-start and reconnect-result decisions for package graph event handlers. |
| `../workflowGraphReconnect.test.ts` | Unit coverage for package graph reconnect start and result decision projection. |
| `../workflowConnectionInteraction.ts` | Owns connection drag reset and connect-end preservation decisions. |
| `../workflowConnectionInteraction.test.ts` | Unit coverage for connection interaction reset and connect-end preservation. |
| `../workflowConnections.ts` | Computes reusable connection validation, graph-edge normalization, candidate-to-intent projection, commit anchors, revision selection, and rejected-intent preservation. |
| `../workflowConnections.test.ts` | Unit coverage for package connection helper behavior. |
| `../horseshoeDragSession.ts` | Owns horseshoe drag visibility, close, request, sync, and anchor state transitions. |
| `../horseshoeDragSession.test.ts` | Unit coverage for horseshoe drag-session transitions. |
| `../horseshoeInsertFeedback.ts` | Owns horseshoe insert pending/rejection state and status-label projection. |
| `../horseshoeInsertFeedback.test.ts` | Unit coverage for feedback state and status-label projection. |
| `../workflowHorseshoeSessionUpdate.ts` | Owns horseshoe session-to-selector state update projection. |
| `../workflowHorseshoeSessionUpdate.test.ts` | Unit coverage for selector reset and timer cleanup update decisions. |
| `../workflowGraphSync.ts` | Computes reference-based store-to-SvelteFlow node and edge synchronization decisions. |
| `../workflowGraphSync.test.ts` | Unit coverage for package graph sync decisions. |
| `../workflowDragCursor.ts` | Resolves drag-cursor movement into horseshoe anchor or selection updates. |
| `../workflowDragCursor.test.ts` | Unit coverage for drag-cursor horseshoe decisions. |
| `../workflowHorseshoeKeyboard.ts` | Resolves drag-time horseshoe keyboard events into component actions. |
| `../workflowHorseshoeKeyboard.test.ts` | Unit coverage for horseshoe keyboard policy decisions. |
| `../workflowHorseshoeOpenContext.ts` | Projects editability, drag-session, connection-drag, and intent state into horseshoe open context. |
| `../workflowHorseshoeOpenContext.test.ts` | Unit coverage for horseshoe open-context projection. |
| `../workflowHorseshoeOpenRequest.ts` | Projects horseshoe open context into diagnostic trace and requested session state. |
| `../workflowHorseshoeOpenRequest.test.ts` | Unit coverage for open-request projection. |
| `../workflowHorseshoeSelection.ts` | Projects horseshoe item selection into keyboard context and the candidate to confirm, normalizes selected indices, rotates selection, and resolves query matches. |
| `../workflowHorseshoeSelection.test.ts` | Unit coverage for horseshoe selection snapshots, index normalization, rotation, and query matching. |
| `../workflowHorseshoeTrace.ts` | Formats horseshoe diagnostic trace labels for session and open-request state. |
| `../workflowHorseshoeTrace.test.ts` | Unit coverage for horseshoe trace formatting. |
| `../workflowInsertPosition.ts` | Projects horseshoe insert anchors into backend insert position hints. |
| `../workflowInsertPosition.test.ts` | Unit coverage for insert position projection. |
| `../workflowMiniMap.ts` | Maps package workflow node groups and backend categories to minimap colors. |
| `../workflowMiniMap.test.ts` | Unit coverage for package workflow minimap color projection. |
| `../workflowNodeActivation.ts` | Resolves node double-click activation and group zoom targets. |
| `../workflowNodeActivation.test.ts` | Unit coverage for node activation and group zoom-target decisions. |
| `../workflowPaletteDrag.ts` | Parses package palette drag payloads and projects drop coordinates. |
| `../workflowPaletteDrag.test.ts` | Unit coverage for palette drag parsing and drop-position projection. |
| `../workflowPointerPosition.ts` | Resolves mouse/touch client positions and container-relative pointer coordinates. |
| `../workflowPointerPosition.test.ts` | Unit coverage for pointer coordinate projection. |
| `NodePalette.svelte` | Palette for adding node definitions into the active graph. |
| `CutTool.svelte` | Edge-cut interaction used for Ctrl-drag deletion. |
| `ContainerBorder.svelte` | Orchestration/group boundary overlay used during zoom transitions. |
| `HorseshoeInsertSelector.svelte` | Cursor-anchored horseshoe selector used to browse compatible insertable node types during an active connection intent. |
| `HorseshoeDebugOverlay.svelte` | Renders drag-time horseshoe trace, display state, and blocked-reason diagnostics. |
| `nodes/` | Shared node shells and reusable package node components, including connection-intent highlighting. |
| `edges/` | Edge renderers and reconnect affordances used by `WorkflowGraph.svelte`; reconnect can start from either occupied edge endpoint so drag-off disconnect works directly from connected ports. |

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
renders pending/blocked selector states instead of silently doing nothing, and
closes through the same shared hidden-state transition.
`HorseshoeInsertSelector.svelte` opens once the session is ready. Node shells
then read the same store to dim incompatible targets and highlight eligible
anchors. The canvas now tracks explicit drag mode as part of that shared
interaction contract: normal output-handle drags are connect/fan-out flows,
while explicit reconnect drags can start from either rendered edge endpoint.
Reconnect anchors now sit directly on occupied edge endpoints so drag-off
disconnect works from the same inputs/outputs users see as connected. Once the
horseshoe is open, the first `Space` has already been consumed; pressing `Space` again confirms the current
highlighted insert candidate, clears drag state immediately, and leaves pointer
motion free to change the highlighted item against a fixed menu anchor. Insert
confirmation now keeps the horseshoe state alive until the backend responds; if
the insert is rejected, the selector stays visible, shows the rejection
message, and refreshes candidates from the returned graph revision for retry.
Palette-driven HTML drag sessions now emit explicit start/end signals so the
graph can disable pan, drag, selection, and reconnect behavior until the
external drag completes. Selection persistence also moved into the shared store
contract so backend graph snapshots reapply the current selected-node ids
instead of dropping selection metadata on every sync. Toolbar run controls now
claim execution ownership from the first execution-scoped workflow event instead
of pre-pinning to the edit-session id, so session-backed runs keep accepting
valid scheduler/runtime or incremental events until the backend publishes the
real run id and stale events from older runs still stop at the execution-id
boundary. `WorkflowRunButton.svelte` delegates backend workflow-event reduction
to the focused store helper in `stores/workflowExecutionEvents.ts`, keeping
`WorkflowToolbar.svelte` responsible for toolbar composition rather than
run-lifecycle orchestration.
Minimap color projection lives in `workflowMiniMap.ts` so category-to-color
mapping stays testable outside the SvelteFlow component.
Store-to-SvelteFlow synchronization decisions live in `workflowGraphSync.ts`,
keeping reference comparisons out of `WorkflowGraph.svelte`.
Connection validation and backend candidate projection live in
`workflowConnections.ts`, while `workflowGraphBackendActions.ts` owns backend
calls and accepted/rejected graph-sync projection for `WorkflowGraph.svelte`.
Shared backend action primitives now live in
`workflowGraphBackendActionCore.ts` so accepted graph projection, connection
rejection preservation, edge removal, insert-and-connect, and reconnect
rollback stay aligned with the app graph without coupling package code to the
Pantograph `WorkflowService` singleton.
Connection and reconnect commit anchor projection plus active-intent revision
selection also live in `workflowConnections.ts`, so `WorkflowGraph.svelte`
resolves a tested commit contract before invoking backend graph mutations.
Rejected connection-intent fallback state also lives in
`workflowConnections.ts`, so preserved compatible targets and insertable node
types stay aligned while `WorkflowGraph.svelte` attaches backend rejection
metadata.
Package graph edge deletion, edge cutting, and reconnect-end cleanup now require
an active session id before calling backend edge-removal APIs, avoiding empty
session-id fallbacks while still allowing local node removal to proceed.
Those backend edge-removal calls now live in
`workflowGraphBackendActions.ts`, keeping `WorkflowGraph.svelte` focused on
selection cleanup and event wiring.
Delete-selection and edge-removal request projection now live in
`workflowGraphDeletion.ts`, while `WorkflowGraph.svelte` only clears interaction
state and invokes the accepted backend action request.
Reconnect start and reconnect result branching now live in
`workflowGraphReconnect.ts`, while `WorkflowGraph.svelte` owns applying the
resulting intent, cleanup, and logging side effects.
Connection drag reset and connect-end preservation live in
`workflowConnectionInteraction.ts`, while `WorkflowGraph.svelte` owns clearing
the backing connection-intent store and host-specific preview state.
Horseshoe keyboard policy lives in `workflowHorseshoeKeyboard.ts`, while
`WorkflowGraph.svelte` maps resolved actions to graph-specific state changes.
Node double-click and group zoom-target decisions live in
`workflowNodeActivation.ts`, while `WorkflowGraph.svelte` invokes the view store
side effects.
Palette drag payload parsing and graph-space drop positioning live in
`workflowPaletteDrag.ts`, while `WorkflowGraph.svelte` owns the browser event
and store mutation side effects.
Horseshoe insert position projection lives in `workflowInsertPosition.ts`,
while `workflowGraphBackendActions.ts` owns the backend insert call and
`WorkflowGraph.svelte` owns the interaction feedback lifecycle.
Drag-cursor horseshoe decisions live in `workflowDragCursor.ts`, while
`WorkflowGraph.svelte` applies the selected-index or session-state update.
Horseshoe open-context projection lives in `workflowHorseshoeOpenContext.ts`,
while `WorkflowGraph.svelte` supplies the current stores and interaction state
before invoking the drag-session controller.
Horseshoe open-request projection lives in `workflowHorseshoeOpenRequest.ts`,
while `WorkflowGraph.svelte` applies the returned trace and session state.
Horseshoe status-label projection lives in `horseshoeInsertFeedback.ts`, while
`WorkflowGraph.svelte` supplies the current feedback and session state.
Horseshoe session update projection lives in
`workflowHorseshoeSessionUpdate.ts`, while `WorkflowGraph.svelte` assigns local
state and clears its active query-reset timer.
Horseshoe selection snapshots and selected-index normalization live in
`workflowHorseshoeSelection.ts`, while `WorkflowGraph.svelte` maps the resolved
keyboard action to confirmation, rotation, or query side effects.
Horseshoe trace labels live in `workflowHorseshoeTrace.ts`, while
`WorkflowGraph.svelte` owns when interaction state changes should record a new
trace string.
Horseshoe diagnostic overlay rendering lives in `HorseshoeDebugOverlay.svelte`
and is exported for app graph reuse, while `WorkflowGraph.svelte` owns when the
overlay is visible and which trace state to supply.
Mouse/touch pointer extraction and container-relative coordinate projection live
in `workflowPointerPosition.ts`, while `WorkflowGraph.svelte` owns the concrete
container element.
Workflow graph default edge options live in `workflowGraphEdgeOptions.ts`, while
`WorkflowGraph.svelte` passes them through to SvelteFlow.
Workflow graph interaction gating lives in `workflowGraphInteraction.ts`, while
`WorkflowGraph.svelte` passes the projected editability state into SvelteFlow.
Workflow graph viewport and minimap defaults live in `workflowGraphViewport.ts`,
while `WorkflowGraph.svelte` passes the constants through to SvelteFlow.
Workflow graph window listener setup lives in `workflowGraphWindowListeners.ts`,
while `WorkflowGraph.svelte` supplies only the concrete keyboard and palette
callbacks.
Workflow graph selected-node id projection lives in `workflowSelection.ts`,
while `WorkflowGraph.svelte` only writes the projected ids into the store.
Workflow graph horseshoe blocked-reason log decisions live in
`workflowHorseshoeTrace.ts`, while `WorkflowGraph.svelte` only emits the warning.
`WorkflowGraphHorseshoeLayer.svelte` owns the shared horseshoe selector and
debug overlay composition plus selector status labels; graph components provide
state, candidates, and callbacks.
Horseshoe keyboard action dispatch lives in `workflowHorseshoeKeyboard.ts`,
while `WorkflowGraph.svelte` provides callbacks that mutate local graph state.
Package graph canvas and SvelteFlow chrome styling lives in
`WorkflowGraph.css` so `WorkflowGraph.svelte` stays below the decomposition
threshold while preserving the same package-owned visual contract.

## Alternatives Rejected
- Ask the backend on every pointer move.
  Rejected because drag performance would depend on round-trip latency.
- Keep compatibility highlighting local to node definitions only.
  Rejected because target occupancy, cycles, and stale revisions depend on live
  session state.

## Invariants
- `WorkflowGraph.svelte` must never create an edge locally that bypasses the
  backend-owned `connectAnchors` commit path.
- `workflowConnectionInteraction.ts` must keep drag reset and connect-end
  preservation decisions shared with the app graph.
- `workflowGraphBackendActions.ts` must keep package graph backend mutation
  calls and response projection aligned with the reusable `WorkflowBackend`
  contract instead of reintroducing app-local service coupling.
- `workflowGraphBackendActionCore.ts` must stay dependency-injected and must not
  import app services or singleton stores.
- `workflowGraphDeletion.ts` must keep delete and edge-removal request
  projection side-effect free.
- `workflowGraphReconnect.ts` must keep reconnect branching side-effect free;
  components own store writes and logging.
- `WorkflowRunButton.svelte` must keep execution state rendered in its template
  on Svelte runes-backed reactive state so workflow events can switch between
  running and waiting-for-input labels without a manual refresh.
- `WorkflowToolbar.svelte` must remain a composition shell for graph-level
  toolbar controls and status badges.
- `WorkflowGraph.svelte` may keep the canvas container focusable for keyboard
  graph commands, but the reviewed Svelte a11y suppression set must explicitly
  cover the noninteractive-tabindex warning that comes with that container.
- Insert-from-drag must commit through backend-owned `insertNodeAndConnect`; the
  UI must not compose local `addNode` plus `connectAnchors`.
- Horseshoe open failures must resolve to an explicit blocked reason instead of
  silently doing nothing.
- Closing a horseshoe display must use `closeHorseshoeDisplay()` so hidden,
  blocked, pending, and open cleanup semantics stay shared with the app graph.
- Successful horseshoe confirmation must end the drag before later drag-end
  cleanup runs so the pointer is no longer treated as dragging an edge.
- While the horseshoe is open, pointer movement must not reposition the menu;
  it can only affect item selection inside the existing anchored layout.
- `workflowDragCursor.ts` must preserve that behavior by updating anchors only
  while the horseshoe display state allows pointer anchoring.
- `workflowHorseshoeOpenContext.ts` must keep open-request context projection
  aligned with connection-drag insert support before the session controller is
  invoked.
- `workflowHorseshoeOpenRequest.ts` must derive request traces and requested
  session transitions from one context snapshot.
- `HorseshoeDebugOverlay.svelte` must stay display-only and receive already
  projected horseshoe trace state from graph components.
- `workflowGraphEdgeOptions.ts` must own shared SvelteFlow default edge options
  for package and app workflow graph canvases.
- `workflowGraphInteraction.ts` must own shared SvelteFlow interaction gating for
  package and app workflow graph canvases.
- `workflowGraphViewport.ts` must own shared SvelteFlow viewport and minimap
  defaults for package and app workflow graph canvases.
- `workflowGraphWindowListeners.ts` must own shared window listener registration
  and cleanup ordering for package and app workflow graph canvases.
- `workflowSelection.ts` must own selected-node id projection for package and app
  workflow graph selection handlers.
- `workflowHorseshoeTrace.ts` must own horseshoe blocked-reason log projection
  for package and app workflow graph components.
- `WorkflowGraphHorseshoeLayer.svelte` must remain rendering-only and must not
  call backend graph mutation APIs directly.
- `workflowHorseshoeKeyboard.ts` must own keyboard action dispatch so package
  and app graph components cannot drift on trace, query, and selection handling.
- `horseshoeInsertFeedback.ts` must derive selector status labels from one
  feedback/session snapshot.
- `workflowHorseshoeSessionUpdate.ts` must derive selector reset, feedback,
  trace, and timer cleanup decisions from one session transition snapshot.
- `workflowHorseshoeSelection.ts` must derive keyboard selection availability,
  normalized selected indices, rotated indices, query matches, and confirmation
  candidates from the same package selection rules.
- `workflowHorseshoeTrace.ts` must keep package/app trace labels aligned for
  blocked, pending, open, and idle horseshoe states.
- Rejected horseshoe inserts must remain visible in-context and refresh
  connection-intent candidates against the backend-returned revision instead of
  silently clearing the interaction.
- Reconnect cleanup must only remove the original edge for unfinished reconnect
  drags; normal connect/horseshoe flows must never inherit reconnect cleanup.
- Dragging from an occupied edge endpoint must start reconnect/disconnect rather
  than silently spawning a duplicate edge.
- Connection-intent highlighting must clear when the graph changes or the drag
  interaction ends.
- Reconnect flows that temporarily remove an edge must restore the original edge
  if the replacement commit is rejected.
- External palette drags must not overlap with xyflow-owned pan, node drag,
  selection, or reconnect gestures.
- Store-backed graph rematerialization must preserve the selected node ids that
  the consumer last acknowledged through selection change events.
- Session-owned execution UI must claim the active run from backend-authored
  event ownership or the first workflow event that carries a non-empty
  `workflow_run_id`, then ignore later events that no longer match that run.
- Session-owned execution UI must treat backend-owned `WaitingForInput`,
  `IncrementalExecutionStarted`, `GraphModified`, and `Cancelled` events as
  execution-state updates rather than leaving those contracts unobserved in the
  existing GUI.
- Palette node items must remain keyboard-activatable with Enter and Space when
  they use generic `role="button"` semantics for drag-and-double-click behavior.
- Svelte a11y suppressions on graph-canvas hosts require an adjacent
  `a11y-reviewed:` comment explaining the ownership boundary.
- `workflowMiniMap.ts` must preserve backend category color semantics and keep
  group-node coloring ahead of category coloring.
- `workflowGraphSync.ts` must not reassign nodes when a caller intentionally
  suppresses the next node sync after a local drag operation.
- `workflowConnections.ts` must keep candidate-derived target validation aligned
  with backend source anchors and package port compatibility.
- `workflowConnections.ts` must reject incomplete connection or reconnect
  commits before backend calls and must use the active intent revision only for
  matching source anchors.
- `workflowConnections.ts` must preserve prior compatible targets and
  insertable candidates when backend rejection state is attached to a still-open
  connection intent.
- Package graph edge-removal calls must not synthesize an empty session id;
  callers skip backend edge deletion when no active session is available.
- `workflowHorseshoeKeyboard.ts` must keep `Space`, `Enter`, arrows, `Escape`,
  and query-editing keys aligned with the documented horseshoe interaction
  contract before graph keyboard behavior changes.
- `workflowNodeActivation.ts` must keep double-click timing and group zoom-target
  projection aligned before group navigation behavior changes.
- `workflowPaletteDrag.ts` must return `null` for missing or malformed drag
  payloads so native drop events cannot throw before graph interaction cleanup
  has completed.
- `workflowInsertPosition.ts` must stay side-effect-free so app and package
  graphs cannot drift on anchor-to-viewport math.
- `workflowPointerPosition.ts` must keep mouse and touch coordinate fallback
  behavior aligned before connect, reconnect, and horseshoe pointer handling
  changes.

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
- `NodePalette.svelte` must emit palette drag lifecycle events before and after
  native HTML drag sessions so `WorkflowGraph.svelte` can suppress conflicting
  graph gestures during external drops.
- `WorkflowGraph.svelte` consumes `workflowPaletteDrag.ts` for package palette
  drops; consumers should treat helper exports from `index.ts` as the supported
  way to reuse that payload and coordinate policy.
- `WorkflowGraph.svelte` binds `Space` to the horseshoe selector through
  drag-scoped window listeners and the shared drag-session controller; failed
  opens remain visible through pending/blocked selector states plus internal
  blocked-reason diagnostics.
- When the horseshoe is open, `Space` and `Enter` both confirm the highlighted
  insert candidate; successful confirmation ends drag ownership immediately.
- Rejected insert confirmations keep the horseshoe visible with a status
  message so release users are not dependent on browser-console warnings.
- Horseshoe insertion is only supported for normal connect drags. Explicit
  reconnect drags from either endpoint surface a blocked reason instructing the
  user to start a new drag from the output handle instead of pretending to
  support splice semantics.
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
