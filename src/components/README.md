# src/components

## Purpose
This directory contains Pantograph’s application-facing Svelte components. It
wraps the reusable graph package with app-specific node registries, orchestration
navigation, and shell UI so the product can layer its own workflows and
architecture views on top of the shared editor.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `WorkflowGraph.svelte` | Pantograph graph canvas that wires app node types, orchestration navigation, revision-aware connection-intent flows, and the `Space`-invoked horseshoe insert selector. |
| `WorkflowContainerBoundary.svelte` | Renders the orchestration boundary overlay, clickable border hit zones, and boundary anchors for the app workflow graph. |
| `NodePalette.svelte` | App palette for inserting workflow nodes into the active graph. |
| `NodeGroupEditor.svelte` | App wrapper for group editing and exposed-port management. |
| `NavigationBreadcrumb.svelte` | Breadcrumb UI for group/orchestration navigation. |
| `WorkflowToolbar.svelte` | Toolbar actions for workflow graph editing. |
| `diagnostics/` | Workflow diagnostics panel, tab views, and presentation helpers for retained execution traces. |
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
- Diagnostics rendering should stay inside the workflow workspace without
  becoming a parallel app shell.
- Settings-side runtime management must remain a presentation surface over the
  backend-owned managed-runtime contract rather than introducing GUI-owned
  runtime policy.

## Decision
Keep the app `WorkflowGraph.svelte` as a composition layer over package store
instances and `workflowService`. The component now follows the same intent flow
as the reusable graph: load candidates on connect start, use shared store state
for validation/highlighting, and route horseshoe invocation through the same
shared drag-session controller and drag-scoped window input used by the package
graph before committing through revision-aware anchor and insert APIs. Pending
and blocked horseshoe states remain visible instead of failing silently. The
app graph also consumes the shared connection-drag helper contract so reconnect
cleanup cannot bleed into ordinary insert flows. Reconnect affordances are
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
type can bridge both endpoints. The workflow workspace now also includes a
bottom diagnostics panel that renders retained execution traces from the
diagnostics store without moving transport or event-normalization logic into
Svelte components. Settings-side server/runtime status rendering is now also
split into focused `server-status/` and `runtime-manager/` subdirectories so
the mounted shell can expose the dedicated version-aware runtime-manager view
without continuing to grow one large component file.
Toolbar execution-event handling now delegates execution-id claiming and stale
event filtering to the shared package workflow execution projector so the app
toolbar does not maintain a second local relevance gate.
The app workflow graph delegates orchestration boundary overlay rendering to
`WorkflowContainerBoundary.svelte`, while the parent keeps viewport tracking,
selection state, and orchestration transition ownership.

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
- Dragging from an occupied edge endpoint must start reconnect/disconnect rather
  than silently creating a duplicate edge.
- Horseshoe selection must route through `workflowService.insertNodeAndConnect`
  so the app never creates orphan nodes on stale or incompatible inserts.
- Horseshoe open failures should be diagnosable through the shared blocked
  reason flow rather than app-only heuristics.
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
