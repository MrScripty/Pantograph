# Graph edge components

## Purpose

Custom SvelteFlow edge renderers for the Graph tab's editor canvas.

## Contents

| Path | Responsibility |
| --- | --- |
| `ReconnectableEdge.svelte` | The default workflow edge (`type: 'reconnectable'`): bezier wire, insert-preview and inference-drift overlays, a wide invisible interaction path and the two reconnect anchors. |

## Problem

Edges used a per-data-type gradient with a glow filter, a light-blue insert
preview and a saturated red drift overlay. The redesign board
(`docs/design/mockups/boards/shell.css`, `.wire`) draws wires neutral grey and
keeps the accent for live state, so the canvas read as busier than the rest of
the workbench.

## Constraints

- The edge id, the `react-flow__edge-path` and `react-flow__edge-interaction`
  classes, the interaction width and the reconnect anchor positions
  (`insetReconnectAnchorPosition`) are behaviour, not styling.
- Token colours only; SVG presentation attributes cannot read CSS variables,
  so colours live in the component's scoped style.

## Decision

The main path is `--pg-border-strong` at 1.5px and `--pg-text` at 2px when
selected; the insert preview (a live drag interaction) uses the accent at 45%
opacity; inference drift is a dashed `--pg-danger` overlay with its title.
The edge no longer reads the node store to colour itself.

## Alternatives Rejected

- Keeping type colours at the wire ends: the data type is already in the port
  label's `title`, and the hues competed with the status glyphs.
- Styling only from `WorkflowGraph.css`: other mounts of this edge (the node
  group editor) would keep the old colours.

## Invariants

- Edges are neutral at rest; the accent appears only during a live
  interaction.
- Drift is dashed as well as red, so it is not shown by colour alone.

## Revisit Triggers

- The edit canvas gets live run state per edge (the run-mode snapshot already
  draws live wires in accent).

## Dependencies

**Internal:** `src/components/reconnectInteraction.ts`,
`src/components/workflowInferenceDriftEdgeOverlays.ts`; registered in
`src/components/workflowGraphTypes.ts` and `src/components/NodeGroupEditor.svelte`.

**External:** `svelte`, `@xyflow/svelte` (`getBezierPath`, `EdgeReconnectAnchor`).

## Related ADRs

- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`

## Usage Examples

```ts
import ReconnectableEdge from './edges/ReconnectableEdge.svelte';

export const workflowEdgeTypes = { reconnectable: ReconnectableEdge };
```
