# src/components/nodes

## Purpose
This directory contains Pantograph-specific node renderers layered on top of the
shared node shell. The boundary exists so app node types can customize form
content and labels while inheriting the same execution-state and
connection-intent affordances.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `BaseNode.svelte` | Shared app node shell that renders handles, execution status, and connection-intent highlighting. |
| `workflow/` | Workflow-node components such as inference, input, output, and grouped node variants. |
| `architecture/` | Architecture-mode node renderers for services, stores, components, and commands. |

## Problem
Pantograph has many app-specific node components, but users still need
consistent compatibility guidance while dragging edges. Without a shared node
shell, each node family would reimplement handle styling and intent-driven
eligibility cues.

## Constraints
- Node components must remain compatible with `@xyflow/svelte` handle behavior.
- App nodes rely on the legacy workflow store facade rather than package context
  directly.
- Intent styling has to coexist with existing execution-state visuals.

## Decision
Keep `BaseNode.svelte` as the shared shell for app nodes and teach it to read
the shared `connectionIntent` store from `workflowStore.ts`. Node-specific
components continue to focus on their own controls while the shell owns
highlight/dim behavior.

## Alternatives Rejected
- Copy intent styling into each workflow and architecture node.
  Rejected because it would drift quickly across many node types.
- Use only edge/handle styling without changing the card appearance.
  Rejected because scanning eligible nodes is slower when unrelated cards remain
  visually identical.

## Invariants
- `BaseNode.svelte` remains the only place that app node components should
  define shared handle styling.
- Connection-intent state does not change handle ids or node ids.
- App node components keep using composition rather than forking the shell for
  small visual changes.

## Revisit Triggers
- Workflow and architecture nodes need incompatible shells.
- Accessibility work requires richer non-visual eligibility cues.
- The app stops using the legacy workflow store facade.

## Dependencies
**Internal:** `src/stores/workflowStore.ts`, `src/components/README.md`,
`src/services/workflow/types.ts`.

**External:** Svelte and `@xyflow/svelte`.

## Related ADRs
- None.
- Reason: node-shell composition remains an internal UI detail.
- Revisit trigger: node renderer extension becomes a formal plugin API.

## Usage Examples
```svelte
<BaseNode {id} {data}>
  {#snippet children()}
    <p>{data.label}</p>
  {/snippet}
</BaseNode>
```

## API Consumer Contract (Host-Facing Modules)
- App node components expect `workflowStore.ts` to expose the shared workflow
  store singletons before they render.
- The shell derives intent styling from stores; callers should not set it via
  props.
- New node components should compose `BaseNode.svelte` instead of recreating
  shared handle markup.

## Structured Producer Contract (Machine-Consumed Modules)
- None.
- Reason: this directory renders UI only.
- Revisit trigger: node components begin emitting saved templates or other
  machine-consumed artifacts.
