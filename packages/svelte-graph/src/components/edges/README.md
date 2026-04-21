# packages/svelte-graph/src/components/edges

Reusable edge rendering components for the graph package.

## Purpose
This directory owns edge components that render package-level graph connection
UI. The current boundary focuses on reconnectable workflow edges that coordinate
with shared connection-drag policy.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `ReconnectableEdge.svelte` | Edge renderer with reconnect affordances and drag lifecycle integration. |

## Problem
Reconnect behavior must be consistent between the package graph editor and the
app wrapper. If edge components implement their own drag semantics, reconnect
cleanup and insertion behavior can drift from shared helpers.

## Constraints
- Edge components must work inside `@xyflow/svelte` rendering.
- Components may emit UI events but must not perform backend graph mutations
  directly.
- Reconnect behavior must use package shared state helpers.
- Visual affordances should not alter graph data by themselves.

## Decision
Keep reusable edge renderers here and compose them with shared connection
helpers from the package source. Backend mutation remains owned by callers that
receive graph interaction events.

## Alternatives Rejected
- Keep reconnect handling inline in the main graph component: rejected because
  edge UI and graph container behavior are easier to test separately.
- Let app-specific components own package edge behavior: rejected because the
  package graph must remain reusable.

## Invariants
- Edge UI must not mutate durable graph state directly.
- Reconnect lifecycle must preserve connection-vs-reconnect mode ownership.
- Exported edge component names are package API surface through `index.ts`.

## Revisit Triggers
- Additional edge renderers need shared policy.
- Reconnect logic moves into an engine-backed response contract.
- Package edge components become externally supported SDK components.

## Dependencies
**Internal:** package graph components, connection drag helpers, and workflow
types.

**External:** Svelte 5 and `@xyflow/svelte`.

## Related ADRs
- Reason: package edge behavior is currently documented in package READMEs
  rather than an ADR.
- Revisit trigger: reconnect policy becomes an external package contract.

## Usage Examples
```ts
import { ReconnectableEdge } from '@pantograph/svelte-graph';
```

## API Consumer Contract
- Inputs: edge props and interaction callbacks supplied by the graph renderer.
- Outputs: rendered edge UI and reconnect interaction events.
- Lifecycle: Svelte mounts/unmounts the component with graph edge visibility.
- Errors: component-level errors should not be used for expected backend
  mutation rejection.
- Versioning: exported component props should migrate with package consumers.

## Structured Producer Contract
- Stable fields: exported component name and event/callback payload shape are
  consumed by the package graph.
- Defaults: visual defaults should match the graph editor interaction model.
- Enums and labels: reconnect mode labels come from shared package helpers.
- Ordering: edge event ordering follows pointer/keyboard event delivery.
- Compatibility: package exports must remain additive unless a migration is
  documented.
- Regeneration/migration: update package exports, app imports, tests, and this
  README together when edge contracts change.

## Testing
```bash
npm run test:frontend
```

## Notes
- Backend graph mutations stay outside this component.
