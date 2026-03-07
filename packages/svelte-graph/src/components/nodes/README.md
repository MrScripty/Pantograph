# packages/svelte-graph/src/components/nodes

## Purpose
This directory contains reusable node shells and package node components used by
the graph editor. The boundary exists so anchor layout, execution-state
presentation, and connection-intent highlighting are applied consistently across
node types.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `BaseNode.svelte` | Shared node chrome and handle rendering, including compatible-target highlighting and incompatible-node dimming. |
| `GenericNode.svelte` | Fallback renderer for node definitions without a specialized package component. |
| `TextInputNode.svelte` | Input-focused package node built on `BaseNode.svelte`. |
| `TextOutputNode.svelte` | Output-focused package node built on `BaseNode.svelte`. |
| `PumaLibNode.svelte` | Package node for Puma library-backed workflows. |

## Problem
Connection-intent guidance is only useful if nodes render it consistently.
Without a shared node shell, each node type would need to reimplement eligible
anchor styling, execution badges, and edge-connected affordances.

## Constraints
- Handle markup must stay compatible with `@xyflow/svelte`.
- Node components need access to workflow stores through graph context rather
  than bespoke prop drilling.
- Intent styling must not hide execution-state signals or connected-port cues.

## Decision
Centralize port rendering and intent-driven styling in `BaseNode.svelte`, then
layer node-specific content on top. Nodes read the shared `connectionIntent`
store to mark eligible target handles, emphasize the active source handle, and
dim unrelated nodes.

## Alternatives Rejected
- Put compatibility styling in each node component.
  Rejected because every new node would need to duplicate the same logic.
- Style only the handles and leave node cards unchanged.
  Rejected because eligible-node scanning is much faster when the whole card is
  visibly promoted or dimmed.

## Invariants
- `BaseNode.svelte` remains the source of truth for handle positioning and
  shared node execution styling.
- Intent highlighting must not change handle ids or node ids.
- Output-handle emphasis applies only to the active source anchor during an
  intent.

## Revisit Triggers
- Different node families need incompatible layout strategies for handles.
- Accessibility review requires non-visual eligibility affordances.
- Package consumers request theming hooks for intent-state styling.

## Dependencies
**Internal:** `packages/svelte-graph/src/components`, `packages/svelte-graph/src/context`,
`packages/svelte-graph/src/constants`.

**External:** Svelte and `@xyflow/svelte`.

## Related ADRs
- None.
- Reason: node-shell composition is still an internal package implementation
  detail.
- Revisit trigger: node renderers become an externally supported plugin API.

## Usage Examples
```svelte
<BaseNode {id} {data}>
  {#snippet children()}
    <p>{data.label}</p>
  {/snippet}
</BaseNode>
```

## API Consumer Contract (Host-Facing Modules)
- Package node components expect to render inside the package graph context so
  they can read workflow stores.
- Consumers should treat `BaseNode.svelte` as the shared shell and supply custom
  content through composition instead of forking handle logic lightly.
- Intent styling is derived from store state; callers do not set it through
  props directly.

## Structured Producer Contract (Machine-Consumed Modules)
- None.
- Reason: these components render UI only.
- Revisit trigger: node components begin emitting saved manifests or generated
  templates.
