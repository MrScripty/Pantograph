# Graph node components

## Purpose

Svelte components that SvelteFlow renders as node cards on the Graph tab's
editor canvas (and on the architecture view). `BaseNode.svelte` is the shared
card every workflow node builds on; the subdirectories hold the typed cards.

## Contents

| Path | Responsibility |
| --- | --- |
| `BaseNode.svelte` | Shared workflow node card: status glyph header, port labels, absolutely positioned handles, error line, optional content slot, connection-intent and inference-drift handle classes. |
| `workflow/` | Typed workflow node cards built on `BaseNode` plus their pure state helpers. See `workflow/README.md`. |
| `architecture/` | Cards for the read-only App Architecture system graph (`ArchBaseNode` and its category wrappers). |

## Problem

Saved graphs store node positions, handle ids and edge endpoints. The cards
must keep handle geometry stable across restyles, while the workbench redesign
(`docs/design/workbench-redesign-spec.md`) asks for one token palette, a glyph
header and no per-category colour, glow or pulsing on the canvas.

## Constraints

- Handle ids equal the node definition's port ids; edges reference them.
- Handle geometry is a contract with saved graphs and with edge routing: the
  header is 37px tall and the ports section padding places handle `i` at
  `52 + 20·i` px from the card top.
- Colours come from the `--pg-*` tokens in `src/styles.css`; no literal
  colours, cyan, glow or pulsing animation.
- Selection stays visible without relying on hue alone (border plus outline
  ring).

## Decision

`BaseNode` owns the card chrome, the status glyph (from
`editNodeExecutionGlyph` in `workbench/graphInspectionPresenters.ts`) and the
handles. Typed cards pass a `header` snippet (a title only) and their custom
content as children; they never restyle the card border or header, so
selection, running and error treatments look the same on every card.

## Alternatives Rejected

- Category-coloured headers and borders per node type: they competed with the
  status glyph and the accent reserved for live state, and overrode the
  selection border.
- Type-coloured handles: the data type stays in the port label's `title`; the
  handle shows only connected or not.

## Invariants

- Header height 37px and handle offsets `52 + 20·i` px never change without a
  migration of saved layouts and edge anchors.
- Handle ids, types (`target` left, `source` right) and the classes
  `port-connected`, `intent-*` and `inference-drift-affected` keep their names.
- The execution state is shown as a glyph with screen-reader text, never by
  colour alone.

## Revisit Triggers

- Ports gain per-port rows of variable height (the fixed offset breaks).
- The spec adds a per-category marker to the node card.

## Dependencies

**Internal:** `src/stores/workflowStore.ts` (edges, execution states,
connection intent), `src/components/workflowInferenceDriftEdgeOverlays.ts`,
`src/components/workbench/ui/StatusGlyph.svelte`,
`src/components/workbench/graphInspectionPresenters.ts`.

**External:** `svelte`, `@xyflow/svelte` (`Handle`, `Position`).

## Related ADRs

- `docs/adr/ADR-006-canonical-node-contract-ownership.md` (port definitions the
  handles render)
- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md` (the Graph
  tab's edit mode that mounts these cards)

## Usage Examples

```svelte
<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';
  let { id, data, selected = false } = $props();
</script>

<BaseNode {id} {data} {selected}>
  {#snippet header()}
    <NodeHeaderTitle label={data.label || 'Text Output'} />
  {/snippet}
  <div class="text-xs text-pg-text-muted">No output yet</div>
</BaseNode>
```
