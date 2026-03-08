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
| `NodePalette.svelte` | App palette for inserting workflow nodes into the active graph. |
| `NodeGroupEditor.svelte` | App wrapper for group editing and exposed-port management. |
| `NavigationBreadcrumb.svelte` | Breadcrumb UI for group/orchestration navigation. |
| `WorkflowToolbar.svelte` | Toolbar actions for workflow graph editing. |
| `nodes/` | Pantograph-specific node renderers and the shared app node shell. |

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

## Decision
Keep the app `WorkflowGraph.svelte` as a composition layer over package store
instances and `workflowService`. The component now follows the same intent flow
as the reusable graph: load candidates on connect start, use shared store state
for validation/highlighting, and route horseshoe invocation through the same
shared drag-session controller and drag-scoped window input used by the package
graph before committing through revision-aware anchor and insert APIs. Pending
and blocked horseshoe states remain visible instead of failing silently. The
app graph also consumes the shared connection-drag helper contract so occupied
output handles stay in normal fan-out mode and reconnect cleanup cannot bleed
into ordinary insert flows. Once the horseshoe is open, repeated `Space`
confirms the highlighted insert candidate, clears drag ownership immediately,
and turns pointer movement into menu selection input instead of menu
repositioning. Insert confirmation now waits for the backend outcome before the
UI tears the interaction down; rejected inserts keep the horseshoe visible,
show the rejection message in-context, and refresh candidates against the
backend-returned graph revision.

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
- Connection-intent cleanup must happen on cancel, pane click, escape, and graph
  mutation paths.
- Occupied output handles must remain available for multiple outgoing edges even
  when an edge is already present at that output.
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
- The app graph follows the package contract that only explicit target-end edge
  drags are reconnect flows; starting from an output handle is always a
  connection/fan-out gesture.
- The app graph follows the package horseshoe contract: first `Space` opens,
  open-state `Space` or `Enter` confirms, and menu-open pointer motion selects
  candidates without moving the menu anchor.
- The app graph must preserve horseshoe state through pending/rejected inserts
  so release-mode users can see and retry backend rejections.
- Rejection handling for failed connection and insert commits is currently
  store/console based, not event-emitter based.
- Horseshoe invocation is drag-session-scoped and uses shared package helpers;
  app code should not reintroduce container-focus-only keyboard assumptions.

## Structured Producer Contract (Machine-Consumed Modules)
- None.
- Reason: this directory renders UI and does not publish persisted machine
  artifacts directly.
- Revisit trigger: components begin generating saved manifests, templates, or
  other structured outputs.
