# packages/svelte-graph/src

## Purpose
This directory contains the reusable source for Pantograph's Svelte graph
editor package. It exists to keep graph interaction rules, workflow store
factories, and package-facing contracts in one place so the app shell does not
fork core canvas behavior.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `components/` | Reusable graph UI, including the main canvas, reconnect affordances, and horseshoe selector. |
| `connectionDragState.ts` | Shared state machine helpers that distinguish normal connect drags from explicit reconnect drags and gate reconnect cleanup. |
| `workflowConnectionInteraction.ts` | Shared connection drag cleanup and connect-end preservation decisions across package and app graphs. |
| `workflowConnectionInteraction.test.ts` | Unit coverage for connection interaction reset and connect-end preservation policy. |
| `workflowDragCursor.ts` | Shared drag-cursor decision helper for horseshoe anchor movement and open-menu selection. |
| `workflowDragCursor.test.ts` | Unit coverage for drag-cursor horseshoe decisions. |
| `horseshoeDragSession.ts` | Shared visibility, close, and anchor lifecycle for the horseshoe insert UI during active drags. |
| `horseshoeDragSession.test.ts` | Unit coverage for horseshoe request, sync, close, and clear state transitions. |
| `horseshoeInsertFeedback.ts` | Shared pending/rejection feedback state and session status-label resolution for horseshoe insert outcomes. |
| `horseshoeInsertFeedback.test.ts` | Unit coverage for feedback state transitions and session status-label projection. |
| `horseshoeInvocation.ts` | Shared `Space` open/confirm decisions, pointer-anchor freeze rules, and user-facing blocked-reason strings for horseshoe invocation. |
| `workflowHorseshoeOpenContext.ts` | Shared projection from drag/session/intent state into the horseshoe open-request context. |
| `workflowHorseshoeOpenContext.test.ts` | Unit coverage for connect, reconnect, and idle horseshoe open-context projection. |
| `workflowHorseshoeOpenRequest.ts` | Shared projection from open context into open-request trace and drag-session state. |
| `workflowHorseshoeOpenRequest.test.ts` | Unit coverage for connect and reconnect open-request projection. |
| `workflowHorseshoeSelection.ts` | Shared projection from horseshoe state and current items into keyboard context plus selected candidate, selected-index normalization, rotation, and query matching. |
| `workflowHorseshoeSelection.test.ts` | Unit coverage for selected, out-of-range, missing, clamped, rotated, and query-matched horseshoe candidate snapshots. |
| `workflowHorseshoeSessionUpdate.ts` | Shared projection for applying drag-session changes to selector feedback, query, selection, and trace state. |
| `workflowHorseshoeSessionUpdate.test.ts` | Unit coverage for unchanged, opened, and hidden horseshoe session update projection. |
| `stores/` | Package store factories for workflow, session, and view state. |
| `types/` | Stable TypeScript contracts for graph, backend, and group APIs. |
| `workflowEventOwnership.ts` | Projection helper for backend-provided workflow execution ids, active run identity, and stale-event relevance. |
| `workflowConnections.ts` | Shared connection validation, graph-edge normalization, candidate-to-intent projection, commit-anchor projection, revision selection, and rejected-intent preservation helpers. |
| `workflowConnections.test.ts` | Unit coverage for reusable connection helper behavior. |
| `workflowGraphSync.ts` | Reference-based store-to-SvelteFlow synchronization decision helper. |
| `workflowGraphSync.test.ts` | Unit coverage for SvelteFlow node and edge sync decisions. |
| `workflowHorseshoeTrace.ts` | Shared trace-label formatting for horseshoe session and open-request state. |
| `workflowHorseshoeTrace.test.ts` | Unit coverage for horseshoe trace formatting. |
| `workflowInsertPosition.ts` | Shared horseshoe insert anchor-to-graph position projection helper. |
| `workflowInsertPosition.test.ts` | Unit coverage for insert position projection. |
| `workflowMiniMap.ts` | Shared minimap color projection for backend node categories and graph group nodes. |
| `workflowMiniMap.test.ts` | Unit coverage for reusable minimap color projection. |
| `workflowPaletteDrag.ts` | Shared palette drag payload parsing and drop-position projection helpers. |
| `workflowPaletteDrag.test.ts` | Unit coverage for palette drag parsing and graph-space drop positioning. |
| `workflowPointerPosition.ts` | Shared mouse/touch client and container-relative pointer projection helpers. |
| `workflowPointerPosition.test.ts` | Unit coverage for pointer client and relative coordinate projection. |
| `index.ts` | Package export surface consumed by the app shell and any external package users. |

## Problem
Pantograph needs one reusable graph interaction core across both the package
editor and the app-specific editor. Without a shared source tree, connect,
reconnect, horseshoe insertion, and workflow-store behavior drift quickly and
produce mismatched user-visible bugs.

## Constraints
- Package code must stay reusable across backend implementations exposed through
  the package context and backend interfaces.
- Backend-owned graph mutations must not be reimplemented locally in the UI.
- Drag interaction helpers must stay small and testable because the host graph
  components are already large.

## Decision
Keep reusable interaction policy in small top-level helpers such as
`connectionDragState.ts`, `horseshoeDragSession.ts`, and
`horseshoeInvocation.ts`, then compose them from the package graph component
and the app graph wrapper. Export those helpers through `index.ts` so the app
layer can consume the same connect/reconnect rules instead of copying them.
Keep close-selector state transitions in `horseshoeDragSession.ts` so package
and app graph components share when hidden horseshoe sessions are no-ops and
when open, pending, or blocked state should be cleared.
Keep connection interaction reset and connect-end preservation in
`workflowConnectionInteraction.ts` so package and app graph components share
when drag state should clear and when active horseshoe work must remain visible.
Keep workflow execution event identity in `workflowEventOwnership.ts` as an
explicit projection object. Backend-authored event `ownership` payloads from
Tauri are authoritative for event identity, active run identity, and relevance;
the helper only falls back to legacy `execution_id` fields for events that do
not yet carry that backend projection.
Keep reusable minimap color projection in `workflowMiniMap.ts` so the graph
component does not own backend category presentation policy inline.
Keep store-to-SvelteFlow synchronization policy in `workflowGraphSync.ts` so
the graph component can preserve xyflow internal metadata without owning
reference-comparison rules inline.
Keep connection-intent projection and synchronous connection validation in
`workflowConnections.ts` so the graph component does not duplicate backend
candidate projection, fallback rejection state, or port-compatibility rules.
Keep connection and reconnect commit anchor projection plus active-intent
revision selection in `workflowConnections.ts` so graph components only perform
backend side effects after resolving a tested connection commit contract.
Keep palette drag parsing and drop-position projection in
`workflowPaletteDrag.ts` so the graph component owns browser events and store
mutations without owning payload or coordinate policy inline.
Keep horseshoe insert position projection in `workflowInsertPosition.ts` so
package and app graph components share the same anchor-to-graph coordinate
contract before calling backend insert APIs.
Keep drag-cursor horseshoe decisions in `workflowDragCursor.ts` so anchor
updates and open-menu item selection stay aligned across package and app graph
components.
Keep horseshoe open-context projection in `workflowHorseshoeOpenContext.ts` so
package and app graph components share the same editability, intent, anchor,
and connect-vs-reconnect insert gating before invoking the session controller.
Keep horseshoe open-request projection in `workflowHorseshoeOpenRequest.ts` so
package and app graph components pair diagnostic trace context with the same
drag-session request transition.
Keep session status-label projection in `horseshoeInsertFeedback.ts` so package
and app graph components share how feedback and drag-session state become
selector status text.
Keep horseshoe session update projection in `workflowHorseshoeSessionUpdate.ts`
so package and app graph components share how drag-session transitions reset
selector query, selected index, feedback, trace text, and timer cleanup intent.
Keep horseshoe selection snapshots and index normalization in
`workflowHorseshoeSelection.ts` so package and app keyboard handlers share how
pending state, selected candidates, rotation, and query matching are projected
before confirming an insert.
Keep horseshoe trace formatting in `workflowHorseshoeTrace.ts` so package and
app graph diagnostics use the same state labels while components own when to
record them.
Keep horseshoe diagnostic overlay rendering in exported
`components/HorseshoeDebugOverlay.svelte` so package and app graph components
share trace display markup while each graph owns overlay visibility and trace
state.
Keep pointer event coordinate projection in `workflowPointerPosition.ts` so
mouse/touch fallback behavior and container-relative math stay aligned across
package and app graph components.
Keep shared workflow graph edge defaults in `workflowGraphEdgeOptions.ts` so
package and app graph components use the same reconnectable edge type, styling,
and interaction width.
Keep shared workflow graph interaction gating in `workflowGraphInteraction.ts`
so package and app graph components suspend editing, selection, reconnecting, and
pane panning under the same conditions.
Keep shared workflow graph viewport constants in `workflowGraphViewport.ts` so
package and app graph components use the same fit, zoom, pan-activation, and
minimap mask defaults.
Keep shared workflow graph window listener lifecycle in
`workflowGraphWindowListeners.ts` so package and app graph components register
keyboard, palette drag, and blur cleanup events through one tested boundary.
Keep selected-node id projection in `workflowSelection.ts` so package and app
graph components update selection stores from the same SvelteFlow selection
snapshot semantics.
Keep horseshoe diagnostic trace and blocked-reason log projection in
`workflowHorseshoeTrace.ts` so package and app graph components share the same
duplicate-suppression and message formatting rules.
Keep the workflow graph horseshoe selector and diagnostic overlay composition in
`components/WorkflowGraphHorseshoeLayer.svelte` so package and app graph
components share rendering and selector status projection while retaining their
own state and callbacks.
Keep horseshoe keyboard action dispatch in `workflowHorseshoeKeyboard.ts` so
package and app graph components share trace, selection confirmation, rotation,
query editing, and close/open routing.

## Alternatives Rejected
- Keep connect/reconnect state management inline in both graph components.
  Rejected because the occupied-output bug showed that duplicated interaction
  rules drifted and became harder to reason about.
- Move all graph interaction policy into the app layer only.
  Rejected because the package graph would lose the behavior contract the app is
  supposed to share.

## Invariants
- Shared graph interaction helpers in this directory remain the source of truth
  for package/app horseshoe gating and connect-vs-reconnect ownership.
- `workflowConnectionInteraction.ts` must keep connection drag reset and
  connect-end preservation decisions shared across package and app graph
  components.
- Horseshoe keyboard policy stays shared: first `Space` requests open, second
  `Space` confirms the highlighted insert only after the menu is open and has a
  valid selection. `workflowHorseshoeKeyboard.ts` owns keyboard-to-action
  projection so graph components only perform the resolved side effects.
- `horseshoeDragSession.ts` owns closing a horseshoe display; components must
  not reconstruct hidden/open-request/blocked cleanup inline.
- Open horseshoe sessions freeze anchor updates; pointer motion is interpreted
  as menu selection input instead of menu repositioning.
- `workflowDragCursor.ts` owns that anchor-versus-selection decision and returns
  explicit side-effect decisions for graph components to apply.
- `workflowHorseshoeOpenContext.ts` must derive `supportsInsert` from
  `connectionDragState.ts` so reconnect drags cannot silently diverge from
  package connect-drag semantics.
- `workflowHorseshoeOpenRequest.ts` must keep open-request trace data and the
  requested session transition derived from the same context snapshot.
- `horseshoeInsertFeedback.ts` must keep selector status labels derived from
  one feedback/session snapshot.
- `workflowHorseshoeSessionUpdate.ts` must keep selector query, selected index,
  feedback, trace text, and query-reset timer cleanup decisions derived from one
  old/new session snapshot.
- `workflowHorseshoeSelection.ts` must keep keyboard `hasSelection`, selected
  index clamping, rotation, query matching, and the confirmed candidate derived
  from package horseshoe selection rules.
- `workflowHorseshoeTrace.ts` must remain formatting-only and must not decide
  whether a horseshoe session should open or close.
- `HorseshoeDebugOverlay.svelte` must remain display-only and receive projected
  trace state from graph components through the package export surface.
- `workflowGraphEdgeOptions.ts` must keep default workflow edge options
  transport-free and reusable by both graph components.
- `workflowGraphInteraction.ts` must keep SvelteFlow interaction gating pure and
  reusable so graph components only supply current UI state.
- `workflowGraphViewport.ts` must keep static SvelteFlow viewport and minimap
  defaults reusable by package and app graph canvases.
- `workflowGraphWindowListeners.ts` must own shared graph window listener
  registration and cleanup ordering for package and app graph canvases.
- `workflowSelection.ts` must own selected-node id projection for package and app
  graph selection handlers.
- `workflowHorseshoeTrace.ts` must own horseshoe diagnostic trace and
  blocked-reason log projection for package and app graph components.
- `WorkflowGraphHorseshoeLayer.svelte` must own selector plus debug overlay
  composition and selector status labels without owning graph state or backend
  mutation callbacks.
- `workflowHorseshoeKeyboard.ts` must own horseshoe keyboard action dispatch
  semantics while graph components provide concrete state callbacks.
- Horseshoe insert feedback stays visible until the backend accepts the insert
  or the interaction is explicitly cleared; rejected inserts must not collapse
  into a silent no-op.
- Package exports must remain additive and backward compatible unless a planned
  breaking change is documented separately.
- Helpers in this directory do not perform backend mutations directly; they
  compute state and decisions for callers that own side effects.
- `workflowNodeActivation.ts` owns node double-click timing and group zoom-target
  projection so package/app graph components do not duplicate activation policy.
- `workflowEventOwnership.ts` must only project backend-supplied execution ids
  and active-run relevance; when backend `ownership` is present, it must not
  override relevance with package-local current-run comparisons.
- Backend-authored workflow event `ownership` payloads take precedence over
  legacy raw `execution_id` fields.
- `workflowMiniMap.ts` must keep group-node coloring ahead of backend category
  coloring and provide a stable fallback color for unknown node types.
- `workflowGraphSync.ts` must clear one-shot node-sync suppression while still
  allowing edge updates through.
- `workflowConnections.ts` must prefer active backend candidate intent when it
  matches the source anchor, then fall back to static port compatibility only
  when no active intent applies.
- `workflowConnections.ts` must reject incomplete connection or reconnect
  commits before backend calls and must use the active intent revision only for
  matching source anchors.
- Rejected connection or insert flows that preserve the selector display must
  use `preserveConnectionIntentState()` so compatible targets and insertable
  nodes stay visible while backend rejection metadata is attached.
- `workflowPaletteDrag.ts` must keep malformed palette drag payloads from
  escaping as unhandled graph drop exceptions.
- `workflowInsertPosition.ts` must return `null` without an anchor and must not
  own backend insert side effects.
- `workflowPointerPosition.ts` must return `null` when no touch point or
  container bounds are available instead of forcing callers to synthesize
  coordinates.

## Revisit Triggers
- A second non-Pantograph consumer needs a different reconnect policy.
- Graph interaction logic outgrows helper-level extraction and needs a dedicated
  package submodule or ADR.
- The app stops wrapping package graph behavior and can consume the package
  component directly.

## Dependencies
**Internal:** `packages/svelte-graph/src/components`,
`packages/svelte-graph/src/stores`, `packages/svelte-graph/src/types`.

**External:** Svelte 5 and `@xyflow/svelte`.

## Related ADRs
- None identified as of 2026-03-07.
- Reason: the interaction-policy extraction stays inside the package boundary.
- Revisit trigger: package exports become a documented external SDK surface.

## Usage Examples
```ts
import {
  createConnectionDragState,
  startConnectionDrag,
  supportsInsertFromConnectionDrag,
} from '@pantograph/svelte-graph';

const dragState = startConnectionDrag();
const canInsert = supportsInsertFromConnectionDrag(dragState);
```

## API Consumer Contract
- Consumers may import helpers from `index.ts`; deep imports into internal
  files are not part of the stable package contract.
- Connection-drag helpers expose pure state transitions only. Callers own event
  wiring, backend calls, and UI cleanup.
- Horseshoe helpers return explicit blocked reasons instead of silently failing,
  and callers should surface or log those reasons consistently.
- New exports in this directory should preserve existing names and semantics for
  current app consumers unless a migration is documented.
- Consumers that need execution-event relevance should use
  `projectWorkflowEventOwnership()` instead of composing lower-level helper
  calls.

## Structured Producer Contract
- `index.ts` is the structured producer for package consumers. Export names and
  the shape of exported helper inputs/outputs are the stable contract.
- Top-level helper modules return plain objects with explicit boolean/string
  fields so callers can serialize, test, or inspect decisions without framework
  internals.
- `WorkflowEventOwnershipProjection` contains event execution id, active
  execution id, and relevance as the current package-level execution event
  projection contract.
- If a helper contract changes, package exports and downstream app imports must
  be updated in the same change set; do not leave mixed old/new interaction
  semantics in the repo.
