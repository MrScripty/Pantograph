# Workflow node cards

## Purpose

Typed node cards for the Graph tab's editor canvas. Each card renders one
engine node type (inputs, outputs, inference, model selection, dependency
environment, groups) on top of `../BaseNode.svelte`, plus the pure helpers
that compute what the cards show.

## Contents

| Path | Responsibility |
| --- | --- |
| `GenericNode.svelte` | Fallback card for any node type without a dedicated component: `BaseNode` alone. |
| `NodeHeaderTitle.svelte` | The title every typed card puts in `BaseNode`'s `header` snippet (token type, one 20px line). |
| `*InputNode.svelte` | Editable input values (text, number, boolean, selection, vector, masked text, audio, linked element). |
| `*OutputNode.svelte` | Read-only previews of produced values (text, vector, image, audio, point cloud). |
| `LLMInferenceNode.svelte`, `EmbeddingNode.svelte`, `ModelProviderNode.svelte`, `PumaLibNode.svelte`, `AgentToolsNode.svelte` | Inference, model and tool configuration cards. |
| `DependencyEnvironment*.svelte` | Dependency environment card and its panels (status, bindings, mode, activity log). |
| `NodeGroupNode.svelte` | Collapsed node group with its exposed ports; keeps the same handle geometry as `BaseNode`. |
| `*State.ts`, `*Display.ts`, `*Options.ts`, `primitiveInputMetadata.ts` + tests | Pure state and display helpers the cards call. |

## Problem

The cards grew one colour scheme each (category borders, coloured header
tiles, pulsing status dots, per-type focus colours), which overrode
`BaseNode`'s selection treatment and fought the redesign's token palette and
status glyph vocabulary.

## Constraints

- Presentation only: handle ids, classes used by logic (`nodrag`, `nopan`,
  `nowheel`, `copyable-output`), event handlers and stored node data keys stay
  as they are, so saved graphs load and connect unchanged.
- `BaseNode`'s header is 37px tall; a `header` snippet must be one 20px line.
- Token colours only (`pg-*` Tailwind colours or `var(--pg-*)`); the accent is
  for live state (streaming, running); no pulsing animation, no cyan.
- Focus uses the global `:focus-visible` accent ring; cards do not suppress it
  with a Tailwind outline reset on focus.

## Decision

Cards render their title with `NodeHeaderTitle` and leave the status glyph,
border, selection and error line to `BaseNode`. Fields are sunken panels
(`bg-pg-panel-sunken border-pg-border`), buttons are raised panels, errors use
`text-pg-danger`, and active toggles use the selected panel with
`aria-pressed`.

## Alternatives Rejected

- Keeping a category colour on the card border: it overrode the selection
  border (a scoped wrapper rule outranks `.base-node.selected`).
- Restyling through `WorkflowGraph.css` with `:global` overrides: the cards
  carried inline colours CSS could not reach.

## Invariants

- No card overrides `.base-node` or `.node-header` styles.
- Status is shown once, by `BaseNode`'s glyph with screen-reader text; cards
  may add a status sentence but never a coloured dot.
- Literal colours remain only where a non-CSS API needs one: the three.js scene
  background in `PointCloudOutputNode.svelte` (the `--pg-panel-sunken` value)
  and the image preview backdrop (`--pg-ink` at 82%).

## Revisit Triggers

- A node type needs a visual marker the glyph vocabulary does not cover.
- `BaseNode` header height or port offsets change.

## Dependencies

**Internal:** `../BaseNode.svelte`, `src/stores/workflowStore.ts`,
`src/stores/linkStore.ts`, `src/services/workflow/*`,
`src/components/workbench/ui/StatusGlyph.svelte`.

**External:** `svelte`, `@xyflow/svelte`, `@tauri-apps/api` (Puma-Lib
hydration), `three` (point cloud preview).

## Related ADRs

- `docs/adr/ADR-006-canonical-node-contract-ownership.md`
- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`

## Usage Examples

```svelte
<BaseNode {id} {data} {selected}>
  {#snippet header()}
    <NodeHeaderTitle label={data.label || 'Number Input'} />
  {/snippet}
  <input
    class="nodrag nopan nowheel w-full rounded border border-pg-border bg-pg-panel-sunken px-2 py-1 text-sm text-pg-text"
    type="number"
  />
</BaseNode>
```
