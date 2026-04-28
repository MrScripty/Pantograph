# src/components

## Purpose
This directory contains Pantograph’s application-facing Svelte components. It
wraps the reusable graph package with app-specific node registries, orchestration
navigation, and shell UI so the product can layer its own workflows and
architecture views on top of the shared editor.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `WorkflowGraph.svelte` | Pantograph graph state coordinator that wires orchestration navigation, revision-aware connection intent, palette insert, reconnect, and horseshoe flows. |
| `WorkflowGraphCanvas.svelte` | Renders the app SvelteFlow canvas, controls, minimap, container boundary, edge-insert marker, horseshoe layer, and cut tool. |
| `WorkflowGraph.css` | App graph canvas and SvelteFlow chrome styling imported by `WorkflowGraphCanvas.svelte`. |
| `WorkflowContainerBoundary.svelte` | Renders the orchestration boundary overlay, clickable border hit zones, and boundary anchors for the app workflow graph. |
| `WorkflowEdgeInsertPreviewMarker.svelte` | Renders the cursor-anchored palette edge-insert preview marker. |
| `workflowContainerBoundary.ts` | Computes orchestration boundary extents and viewport visibility for graph zoom-out transitions. |
| `workflowContainerBoundary.test.ts` | Unit coverage for orchestration boundary bounds and visibility projection. |
| `workflowContainerSelection.ts` | Resolves app orchestration boundary keyboard actions and container selection transitions. |
| `workflowContainerSelection.test.ts` | Unit coverage for app orchestration boundary keyboard action mapping and selection transitions. |
| `workflowConnections.ts` | Computes app graph connection validation, graph-edge normalization, backend candidate projection, commit anchors, and revision selection. |
| `workflowConnections.test.ts` | Unit coverage for app graph connection helper behavior. |
| `edgeInsertInteraction.ts` | Computes palette edge-insert hover state, preview refresh/staleness/cleanup decisions, and rendered-edge hit testing. |
| `workflowGraphBackendActions.ts` | Owns app graph `WorkflowService` session lookup, edge insertion, backend graph refresh, and adapters into shared package backend action primitives. |
| `workflowGraphEdgeInsertPreview.ts` | Coordinates palette edge-insert preview refresh requests and stale-response guards around the edge-insert interaction state helpers. |
| `workflowGraphKeyboardActions.ts` | Coordinates app graph container keyboard commands and horseshoe window keyboard dispatch. |
| `workflowGraphPaletteHandlers.ts` | Coordinates app palette drop and drag-over events before delegating to node insertion or edge-insert preview handlers. |
| `workflowGraphSource.ts` | Resolves whether the app graph should render workflow store data or the architecture graph. |
| `workflowGraphSource.test.ts` | Unit coverage for app graph source selection. |
| `workflowMiniMap.ts` | Maps workflow node groups and backend categories to minimap colors. |
| `workflowMiniMap.test.ts` | Unit coverage for app workflow minimap color projection. |
| `workflowPaletteDrag.ts` | Computes app palette drag eligibility, drag payload parsing, and graph-space drop positions. |
| `workflowPaletteDrag.test.ts` | Unit coverage for app palette drag parsing and drop-position projection. |
| `workflowGraphTypes.ts` | Defines the app workflow graph node and edge component registry used by SvelteFlow. |
| `NodePalette.svelte` | App palette for inserting workflow nodes into the active graph. |
| `NodeGroupEditor.svelte` | App wrapper for group editing and exposed-port management. |
| `NavigationBreadcrumb.svelte` | Breadcrumb UI for group/orchestration navigation. |
| `WorkflowToolbar.svelte` | Toolbar actions for workflow graph persistence and scheduler-backed workflow submission. |
| `workbench/` | Scheduler-first app shell, page wrappers, and active-run workbench views. |
| `diagnostics/` | Legacy workflow diagnostics panel, tab views, and presentation helpers retained outside the active workbench shell. |
| `nodes/` | Pantograph-specific node renderers and the shared app node shell. |
| `runtime-manager/` | Mounted Settings runtime-manager cards and panel for backend-owned redistributable inspection and version policy. |
| `server-status/` | Focused presentation subcomponents used by the `ServerStatus.svelte` Settings shell. |

## Problem
Pantograph needs app-specific graph composition on top of the reusable package:
extra node types, orchestration transitions, and compatibility with legacy app
stores/services. The app still needs to honor the same connection-intent rules
as the package graph so GUI behavior and backend validation stay aligned.

## Constraints
- Components must coexist with package-provided store factories and node types.
- The app graph supports both workflow and architecture modes.
- Legacy service/store callers still exist, so migration to package-native APIs
  is incremental.
- Built-in node UI must stay aligned with backend-owned node contracts and
  starter workflow templates.
- Diagnostics rendering should be owned by the workbench Diagnostics page. Graph
  execution controls should not subscribe to diagnostics state.
- Graph editor workflow submission must use the scheduler execution session
  boundary and must not directly execute the edit-session graph.
- Settings-side runtime management must remain a presentation surface over the
  backend-owned managed-runtime contract rather than introducing GUI-owned
  runtime policy.

## Decision
Keep the app `WorkflowGraph.svelte` as a composition layer over package store
instances and backend action helpers. The component now follows the same intent
flow as the reusable graph: load candidates on connect start, use shared store
state for validation/highlighting, and route horseshoe invocation through the
same shared drag-session controller and drag-scoped window input used by the
package graph before committing through revision-aware anchor and insert APIs.
Pending and blocked horseshoe states remain visible instead of failing
silently. The app graph also consumes the shared drag-session close helper and
connection-drag helper contract so reconnect cleanup cannot bleed into ordinary
insert flows. Reconnect affordances are
available directly on occupied edge endpoints so dragging off connected inputs
and outputs starts a reconnect/disconnect gesture instead of a fresh edge
creation. Once the horseshoe is open, repeated `Space`
confirms the highlighted insert candidate, clears drag ownership immediately,
and turns pointer movement into menu selection input instead of menu
repositioning. Insert confirmation now waits for the backend outcome before the
UI tears the interaction down; rejected inserts keep the horseshoe visible,
show the rejection message in-context, and refresh candidates against the
backend-returned graph revision. Palette-originated HTML drag sessions now
broadcast explicit start/end events so the graph can disable xyflow pan, drag,
selection, and reconnect affordances until the external drag completes. The app
also treats `selectedNodeIds` as the stable selection source across backend
graph snapshots so node placement and later graph syncs do not silently wipe
selection state. While a palette drag is active, the app graph also treats the
cursor as the only edge-insert hit point and previews replacement of an
existing workflow edge only after backend validation confirms the dragged node
type can bridge both endpoints. Settings-side server/runtime status rendering
is now also split into focused `server-status/` and `runtime-manager/`
subdirectories so the mounted shell can expose the dedicated version-aware
runtime-manager view without continuing to grow one large component file.
Toolbar execution-event handling now delegates execution-id claiming and stale
event filtering to the shared package workflow execution projector so the app
toolbar does not maintain a second local relevance gate.
The root app shell now lives in `workbench/`, which gives Scheduler,
Diagnostics, Graph, I/O Inspector, Library, Network, and Node Editor a shared
navigation surface and selected-run context. The graph is mounted through a page
wrapper instead of being selected by the old root canvas/workflow mode switch,
and diagnostics rendering now comes from the projection-backed workbench
Diagnostics page.
`WorkflowToolbar.svelte` labels the graph action as Submit and routes saved,
clean workflows through scheduler execution session create/run/close commands.
That keeps the Graph page on the same canonical run path as Scheduler,
Diagnostics, and I/O Inspector instead of executing the edit-session graph
directly.
The app workflow graph delegates orchestration boundary overlay rendering to
`WorkflowContainerBoundary.svelte` and boundary math to
`workflowContainerBoundary.ts`, while the parent keeps viewport tracking,
selection state, and orchestration transition ownership.
Zoom-out transition decisions also live in `workflowContainerBoundary.ts`; the
parent only applies the returned transition action.
Orchestration boundary keyboard action mapping lives in
`workflowContainerSelection.ts`, while the graph applies the selected-state and
view-transition side effects.
Container selection transitions for node selection, pane clicks, palette drags,
and boundary toggles also live in `workflowContainerSelection.ts`; the graph
only assigns the resolved selection state.
Minimap color projection lives in `workflowMiniMap.ts` so category-to-color
mapping remains testable outside the graph component.
The app SvelteFlow node and edge registry lives in `workflowGraphTypes.ts` so
`WorkflowGraph.svelte` remains focused on graph state and interaction handling.
Cut gesture state, line sampling, and overlay rendering come from the package
`CutTool`; the app graph only owns the backend edge-deletion callback.
Palette edge-insert hover projection, commit eligibility, preview edge flagging,
rendered-edge hit testing, async preview request staleness checks, and preview
cleanup decisions live in `edgeInsertInteraction.ts`.
Palette edge-insert marker rendering lives in
`WorkflowEdgeInsertPreviewMarker.svelte`.
Palette drag payload parsing, graph-mode eligibility, and graph-space drop
position projection live in `workflowPaletteDrag.ts`; the parent graph only
maps browser events into those helpers and owns backend preview/commit effects.
Connection validation and backend candidate projection live in
`workflowConnections.ts`, while `WorkflowGraph.svelte` owns backend calls and
interaction cleanup.
Accepted graph mutation projection, insert-and-connect, connection rejection
preservation, edge removal, and reconnect rollback now use the package
`workflowGraphBackendActionCore.ts` helpers through app-local adapters. The app
graph keeps `WorkflowService` session lookup and rejected-connect refresh
side effects local so shared package code does not depend on Pantograph
singletons.
App graph delete-selection projection, reconnect-start branching, and reconnect
result handling now use the package graph interaction helpers. The app graph
still owns architecture mode, palette edge-insert preview, and
`WorkflowService` orchestration because those are Pantograph-specific concerns.
Connection and reconnect commit anchor projection plus active-intent revision
selection also live in `workflowConnections.ts`, so the app graph resolves a
tested commit contract before invoking backend graph mutations.
Rejected connection-intent fallback state now uses the package
`preserveConnectionIntentState()` helper, while the app graph owns backend
refresh and warning side effects.
Connection drag reset and connect-end preservation now use the package
`workflowConnectionInteraction.ts` helper; the app graph only clears
app-specific connection-intent and preview state around that shared decision.
Workflow-versus-architecture graph source selection lives in
`workflowGraphSource.ts`, keeping graph mode policy outside the store-sync
effect.
Horseshoe keyboard event interpretation now comes from the package
`workflowHorseshoeKeyboard.ts` helper; the app graph only maps resolved actions
to app-owned state and backend side effects.
Node double-click and group zoom-target decisions now come from the package
`workflowNodeActivation.ts` helper; the app graph only invokes app view-store
side effects.
Horseshoe insert position projection now comes from the package
`workflowInsertPosition.ts` helper so package and app graph insert coordinates
stay aligned.
Drag-cursor horseshoe anchor and selection decisions now come from the package
`workflowDragCursor.ts` helper; the app graph only applies the resolved state
change.
Horseshoe open-context projection now comes from the package
`workflowHorseshoeOpenContext.ts` helper; the app graph only supplies app store
state before requesting or syncing the shared horseshoe drag session.
Horseshoe open-request projection now comes from the package
`workflowHorseshoeOpenRequest.ts` helper; the app graph only applies returned
trace and session state.
Horseshoe status-label projection now comes from the package
`horseshoeInsertFeedback.ts` helper; the app graph only supplies current
feedback and session state.
Horseshoe session update projection now comes from the package
`workflowHorseshoeSessionUpdate.ts` helper; the app graph only assigns local
state and clears the active query-reset timer.
Horseshoe selection snapshots, selected-index normalization, rotation, and
query matching now come from the package `workflowHorseshoeSelection.ts` helper;
the app graph only performs app-owned confirmation and timer side effects after
keyboard resolution.
Horseshoe diagnostic trace labels now come from the package
`workflowHorseshoeTrace.ts` helper so package and app graph traces stay aligned.
Horseshoe diagnostic overlay rendering now uses the package
`HorseshoeDebugOverlay` component so package and app graph trace display stays
aligned while the app graph owns overlay visibility and trace state.
Workflow graph default edge options now come from the package
`WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS` export so app and package canvases share
the same reconnectable edge defaults.
Workflow graph interaction gating now comes from the package
`resolveWorkflowGraphInteractionState` helper so app and package canvases share
the same edit, drag, selection, reconnect, and pane-panning conditions.
Workflow graph viewport and minimap defaults now come from package
`workflowGraphViewport.ts` constants so app and package canvases share the same
fit, zoom, pan-activation, and minimap mask defaults.
Workflow graph window listener lifecycle now comes from the package
`registerWorkflowGraphWindowListeners` helper so app and package canvases share
the same keyboard, palette drag, and blur cleanup registration.
Workflow graph selected-node id projection now comes from the package
`collectSelectedNodeIds` helper so app and package canvases share the same
SvelteFlow selection snapshot semantics.
Workflow graph horseshoe blocked-reason log decisions now come from the package
`resolveWorkflowHorseshoeBlockedReasonLog` helper so app and package canvases
share the same duplicate-suppression and message formatting rules.
Workflow graph horseshoe selector and debug overlay rendering now comes from the
package `WorkflowGraphHorseshoeLayer` component so app and package canvases
share the same horseshoe UI composition and selector status labels.
Workflow graph horseshoe keyboard action dispatch now comes from the package
`dispatchWorkflowHorseshoeKeyboardAction` helper so app and package canvases
share the same trace, query, rotation, close/open, and confirm behavior.
Mouse/touch pointer projection now comes from the package
`workflowPointerPosition.ts` helper; the app graph only supplies the active
container bounds.
App graph canvas and SvelteFlow chrome styling lives in `WorkflowGraph.css`, so
visual policy is separated from the interaction-heavy graph composition
component.

## Alternatives Rejected
- Replace the app graph entirely with the package component immediately.
  Rejected because Pantograph still needs app-specific node sets and
  orchestration integration.
- Keep legacy `addEdge`/`validateConnection` behavior only in the app graph.
  Rejected because it would diverge from the backend-owned eligibility model.

## Invariants
- `WorkflowGraph.svelte` must stay compatible with Pantograph’s architecture and
  workflow graph modes.
- App components should consume shared workflow store instances instead of
  creating parallel graph state.
- Diagnostics components render store snapshots and must not subscribe directly
  to workflow events.
- Connection-intent cleanup must happen on cancel, pane click, escape, and graph
  mutation paths.
- Connection drag reset and connect-end cleanup must use the package
  interaction helper so active horseshoe insert work is not cleared differently
  from the package graph.
- Dragging from an occupied edge endpoint must start reconnect/disconnect rather
  than silently creating a duplicate edge.
- Horseshoe selection must route through `workflowService.insertNodeAndConnect`
  so the app never creates orphan nodes on stale or incompatible inserts.
- Horseshoe open failures should be diagnosable through the shared blocked
  reason flow rather than app-only heuristics.
- Closing app graph horseshoe display state must use the package
  `closeHorseshoeDisplay()` helper instead of rebuilding hidden cleanup inline.
- Successful horseshoe confirmation must drop drag state immediately so later
  drag-end events do not keep the pointer in a dragging interaction.
- While the horseshoe is open, pointer movement must update highlighted menu
  selection rather than the anchored menu position.
- Rejected insert attempts must remain visible to the user and preserve retry
  context instead of failing only through hidden console output.
- External palette drags must temporarily disable graph gestures so native drag
  and xyflow pointer ownership never overlap.
- Backend-driven graph resync must preserve the authoritative selected-node id
  set instead of clearing node selection as a side effect.
- Palette edge insertion must use cursor hit-testing plus backend preview and
  commit APIs before replacing an existing edge; node overlap alone must not
  trigger insertion.
- Workflow palette drop coordinates must be projected from viewport state by
  `workflowPaletteDrag.ts`, not recomputed ad hoc inside graph event handlers.
- Node registration must stay consistent with bundled templates so shipped
  starter workflows render without fallback-node surprises.
- Icon-only app-shell buttons must expose an accessible name with `aria-label`
  or `aria-labelledby`; `title` alone is not treated as an accessible control
  contract.
- Palette node items must remain keyboard-activatable with Enter and Space when
  they use generic `role="button"` semantics for drag-and-double-click behavior.
- Svelte a11y suppressions on graph-canvas hosts require an adjacent
  `a11y-reviewed:` comment explaining the ownership boundary.
- `WorkflowContainerBoundary.svelte` owns boundary hit-zone markup and emits only
  selection toggles; it must not mutate graph stores directly.
- `WorkflowEdgeInsertPreviewMarker.svelte` owns only marker presentation and must
  not read graph state or call backend APIs directly.
- `workflowContainerBoundary.ts` must stay DOM-free so boundary projection
  and zoom-out transition decisions remain unit-testable.
- `workflowContainerSelection.ts` must stay side-effect-free so boundary
  keyboard and selection policy can be tested without SvelteFlow or app stores.
- `workflowMiniMap.ts` must preserve backend category color semantics and keep
  group-node coloring ahead of category coloring.
- `workflowGraphTypes.ts` must include every node type referenced by bundled
  templates and architecture graphs before falling back to generic renderers.
- App cut gestures must delegate to the package `CutTool` so the package and app
  canvases share modifier, line-sampling, and overlay behavior.
- `edgeInsertInteraction.ts` must keep rendered-edge hit testing, commit
  eligibility, preview request staleness, preview cleanup, and preview flag
  projection DOM-light and covered by unit tests before palette edge-insert
  behavior is changed.
- `workflowConnections.ts` must prefer active backend candidate intent when it
  matches the source anchor, then fall back to package port compatibility only
  when no active intent applies.
- `workflowConnections.ts` must reject incomplete connection or reconnect
  commits before backend calls and must use the active intent revision only for
  matching source anchors.
- App graph rejected connection or insert flows must preserve visible
  compatible targets through the package connection-intent preservation helper
  instead of rebuilding fallback state inline.
- `workflowGraphSource.ts` must preserve the architecture-pending state so the
  app graph does not flash workflow nodes while architecture data is loading.
- App graph store-to-SvelteFlow synchronization must key drag-time node-sync
  suppression to the rendered graph identity so a workflow or architecture
  source change applies nodes and edges as one coherent snapshot.
- App graph horseshoe keyboard behavior must use the package keyboard resolver
  so `Space`, `Enter`, arrow, `Escape`, and query-editing semantics stay aligned
  with the package graph.
- App graph node double-click behavior must use the package node activation
  resolver so group navigation timing and zoom bounds stay aligned with the
  package graph.
- App graph horseshoe insert coordinates must use the package insert-position
  resolver before calling backend insert APIs.
- App graph drag-cursor behavior must use the package drag-cursor resolver so
  open-menu selection and hidden-menu anchor movement stay aligned with the
  package graph.
- App graph horseshoe open-context projection must use the package helper so
  reconnect insert blocking and connect intent availability stay aligned with
  the package graph.
- App graph horseshoe open requests must use the package request helper so
  diagnostic trace and session transition context cannot drift.
- App graph horseshoe status labels must use the package feedback helper so
  pending, rejection, and blocked-state text cannot drift.
- App graph horseshoe session updates must use the package session-update
  helper so query, selection, feedback, trace, and timer cleanup behavior cannot
  drift.
- App graph horseshoe keyboard confirmation, selected-index clamping, rotation,
  and query matching must use the package selection helper so `hasSelection`
  and the confirmed candidate cannot drift.
- App graph horseshoe trace labels must use the package trace formatter instead
  of composing parallel string formats inline.
- App graph horseshoe diagnostic rendering must use the package
  `HorseshoeDebugOverlay` component instead of duplicating trace display markup.
- App graph SvelteFlow default edge options must use the package
  `WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS` export so edge type and interaction
  defaults cannot drift from the package graph.
- App graph SvelteFlow interaction gating must use the package
  `resolveWorkflowGraphInteractionState` helper so editability and palette-drag
  behavior cannot drift from the package graph.
- App graph SvelteFlow viewport and minimap defaults must use package
  `workflowGraphViewport.ts` constants so zoom and minimap presentation cannot
  drift from the package graph.
- App graph window listener lifecycle must use the package
  `registerWorkflowGraphWindowListeners` helper so keyboard and palette-drag
  cleanup cannot drift from the package graph.
- App graph selected-node id projection must use the package
  `collectSelectedNodeIds` helper so selection snapshot handling cannot drift
  from the package graph.
- App graph horseshoe blocked-reason logging must use the package
  `resolveWorkflowHorseshoeBlockedReasonLog` helper so diagnostic duplicate
  suppression cannot drift from the package graph.
- App graph horseshoe selector and debug overlay rendering must use the package
  `WorkflowGraphHorseshoeLayer` component so horseshoe UI composition cannot
  drift from the package graph.
- App graph horseshoe keyboard action dispatch must use the package
  `dispatchWorkflowHorseshoeKeyboardAction` helper so trace, query, and
  selection behavior cannot drift from the package graph.
- App graph pointer coordinate handling must use the package pointer-position
  resolver so mouse/touch fallback behavior stays aligned with the package
  graph.

## Revisit Triggers
- The app graph fully converges with the package graph and can be deleted.
- Pantograph introduces a second graph canvas with different interaction rules.
- Orchestration transitions require server-owned connection intent or insert
  flows.

## Dependencies
**Internal:** `src/stores`, `src/services/workflow`, `src/registry`,
`packages/svelte-graph`.

**External:** Svelte 5 and `@xyflow/svelte`.

## Related ADRs
- None.
- Reason: the app composition layer is still evolving toward the reusable
  package shape.
- Revisit trigger: the app/package graph split becomes stable enough to warrant
  a formal ADR.

## Usage Examples
```svelte
<WorkflowGraph />
```

## API Consumer Contract (Host-Facing Modules)
- These components are app-internal and expect Pantograph store singletons to be
  initialized before rendering.
- `WorkflowGraph.svelte` relies on `workflowService` session state and the
  shared workflow store exports; callers should not instantiate it outside the
  app shell without recreating those dependencies.
- The app graph allows reconnect and disconnect drags from either rendered edge
  endpoint rather than treating occupied ports as fresh-connection starts.
- The app graph follows the package horseshoe contract: first `Space` opens,
  open-state `Space` or `Enter` confirms, and menu-open pointer motion selects
  candidates without moving the menu anchor.
- `NodePalette.svelte` must emit palette-drag lifecycle events and
  `WorkflowGraph.svelte` must honor them by suppressing graph interactions until
  drag end.
- Workflow palette drags may replace an existing workflow edge only when the
  cursor is within the edge hit threshold and `workflowService` confirms a
  valid bridge through the dragged node type.
- The app graph must preserve horseshoe state through pending/rejected inserts
  so release-mode users can see and retry backend rejections.
- Rejection handling for failed connection and insert commits is currently
  store/console based, not event-emitter based.
- Diagnostics panel tabs should treat Scheduler, Runtime, and Graph as
  explicitly planned surfaces until those roadmap items ship.
- Horseshoe invocation is drag-session-scoped and uses shared package helpers;
  app code should not reintroduce container-focus-only keyboard assumptions.
- `workflowToolbarEvents.ts` should consume shared workflow execution projection
  results rather than importing low-level execution-ownership helpers directly.

## Structured Producer Contract (Machine-Consumed Modules)
- None.
- Reason: this directory renders UI and does not publish persisted machine
  artifacts directly.
- Revisit trigger: components begin generating saved manifests, templates, or
  other structured outputs.
