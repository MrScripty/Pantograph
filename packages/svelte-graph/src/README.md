# @pantograph/svelte-graph source

## Purpose

Source of the reusable graph editor package (see `../README.md` for the
package and dependency ownership). It holds the editor components, the
graph-context stores and the pure interaction helpers that both this package's
`WorkflowGraph` and the app's `src/components/WorkflowGraphCanvas.svelte` use.

## Contents

| Path | Responsibility |
| --- | --- |
| `index.ts` | Public exports of the package. |
| `components/` | Package editor components (`WorkflowGraph`, palette, toolbar, group editor, horseshoe selector, default node and edge components). |
| `stores/` | Session, view and workflow store factories and their pure graph queries. |
| `context/` | Graph context creation and lookup. |
| `backends/` | `MockWorkflowBackend` for tests and demos. |
| `types/`, `constants/`, `utils/` | Shared types, port colours, registry building and geometry. |
| `workflowGraphViewport.ts`, `workflowGraphEdgeOptions.ts` | SvelteFlow canvas constants shared by every editor mount (zoom limits, fit options, minimap mask colour, default edge options). |
| `workflow*.ts`, `horseshoe*.ts`, `connectionDragState.ts`, `cutInteraction.ts`, `reconnectInteraction.ts`, `portTypeCompatibility.ts`, `graphRevision.ts`, `paletteDragState.ts` + tests | Pure interaction and synchronisation helpers. |

## Problem

The Pantograph app and the package each mount SvelteFlow; canvas defaults
(zoom, fit, minimap, default edge options) must agree or the two editors
behave differently, and literal colours in those defaults override the app's
token styling.

## Constraints

- Constants are plain values passed as SvelteFlow props; the app styles the
  canvas through the `--xy-*` CSS variables in
  `src/components/WorkflowGraph.css`.
- A colour passed as a prop (for example `maskColor`) wins over the CSS
  variable, so it must match the workbench token it stands for.
- Package tests run under the root `npm run test:frontend`.

## Decision

`WORKFLOW_GRAPH_MINIMAP_MASK_COLOR` is the workbench sunken panel at 60%
(`rgba(35, 39, 45, 0.6)`, `--pg-panel-sunken`). `WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS`
carries no inline `style`, so edge colour follows `--xy-edge-stroke` and the
edge component's own styles.

## Alternatives Rejected

- Keeping a literal edge stroke in the default options: it bypassed the
  canvas variables and the app's edge component ignored it anyway.
- Moving the constants into the app: the package's own `WorkflowGraph` mounts
  SvelteFlow with the same values.

## Invariants

- Viewport and edge-option constants are shared, not copied; their tests pin
  the values.
- No literal colour in a shared constant unless SvelteFlow requires a prop
  value, and then it names the token it mirrors.

## Revisit Triggers

- The app stops passing `maskColor`, so the CSS variable can own the mask.
- The package gains its own theme tokens.

## Dependencies

**Internal:** consumed by `src/components/WorkflowGraphCanvas.svelte`,
`src/registry/pantographNodeTypes.ts` and the app stores.

**External:** `svelte`, `@xyflow/svelte` (peer dependencies).

## Related ADRs

- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`

## Usage Examples

```svelte
<script lang="ts">
  import { SvelteFlow, MiniMap } from '@xyflow/svelte';
  import {
    WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS,
    WORKFLOW_GRAPH_MINIMAP_MASK_COLOR,
  } from '@pantograph/svelte-graph';
</script>

<SvelteFlow defaultEdgeOptions={WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS}>
  <MiniMap maskColor={WORKFLOW_GRAPH_MINIMAP_MASK_COLOR} />
</SvelteFlow>
```
