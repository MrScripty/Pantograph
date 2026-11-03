# Graph editor components

## Purpose

Reusable Svelte components of the `@pantograph/svelte-graph` package: the
SvelteFlow-based workflow editor (`WorkflowGraph.svelte`), its toolbar,
palette, breadcrumb, group editor, cut tool, horseshoe insert selector and
the package's default node and edge components.

## Contents

- `WorkflowGraph.svelte` and `WorkflowGraph.css`: the editor surface and its
  base stylesheet. The stylesheet is bundled into any app that imports the
  editor, so its colour rules read the host's `--pg-*` tokens with the
  package's original values as fallbacks.
- `WorkflowToolbar.svelte`, `NodePalette.svelte`, `NavigationBreadcrumb.svelte`,
  `NodeGroupEditor.svelte`, `GroupPortMapper.svelte`, `ContainerBorder.svelte`,
  `ZoomTransition.svelte`, `CutTool.svelte`, `WorkflowGraphHorseshoeLayer.svelte`,
  `HorseshoeInsertSelector.svelte`, `WorkflowRunButton.svelte`.
- `nodes/`, `edges/`: the package's default node and edge components. The
  Pantograph app mounts its own copies under `src/components/nodes` and
  `src/components/edges`.
- `workflowGraphBackendActions.ts`: backend-facing actions the toolbar calls.

## Problem

The app's workbench redesign themes the editor through CSS custom properties,
but this package's stylesheet carried literal colours that painted over the
app's tokens (minimap ground, connecting-handle glow).

## Constraints

- The package stays host-agnostic: it must render without the host's tokens.
- Layout-affecting rules (node geometry, handle positions) do not change.

## Decision

Colour rules in `WorkflowGraph.css` use `var(--pg-…, <previous literal>)` so
a themed host overrides them and an unthemed host keeps the old appearance.
The heading set of this file is required by
`scripts/check-decision-traceability.sh` (plan issue WR-001).

## Alternatives Rejected

- Scoping the stylesheet under a host class: would require every consumer to
  add the class and still leaves literal colours in the package.
- Removing the stylesheet from the package: breaks unthemed consumers.

## Invariants

- `WorkflowGraph.css` sets no node geometry that the app's node components
  depend on; geometry lives in the node components.
- No rule in this directory uses the app's accent for anything but
  connecting or live state.

## Revisit Triggers

- The host stops mounting its own node and edge copies (then the package's
  `nodes/` and `edges/` need the token treatment too; plan issue WR-064).
- The package publishes independently and needs a documented theming API.

## Dependencies

- `@xyflow/svelte` for the canvas, minimap and controls.
- The host's `--pg-*` tokens are optional inputs.

## Related ADRs

- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md` (run-mode
  graphs render in an isolated read-only surface, not through this editor).

## Usage Examples

```svelte
<script>
  import { WorkflowGraphEditor } from '@pantograph/svelte-graph';
</script>
<WorkflowGraphEditor />
```
