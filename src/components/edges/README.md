# src/components/edges

App-specific workflow edge components.

## Purpose
This directory owns edge renderers used by the Pantograph app shell when it
wraps or specializes reusable graph package behavior.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `ReconnectableEdge.svelte` | App edge renderer for reconnect affordances and package graph integration. |

## Problem
The app needs workflow-specific edge rendering while still sharing interaction
semantics with the reusable graph package. Without a focused boundary, app edge
behavior can drift from package reconnect logic.

## Constraints
- Edge components must not own backend graph mutations.
- Reconnect behavior must stay aligned with shared package helpers.
- UI affordances must not hide backend rejection reasons.

## Decision
Keep app-specific edge renderers here and treat backend mutation responses as
authoritative. Shared interaction policy remains in the graph package.

## Alternatives Rejected
- Inline app edge behavior in the main workflow graph: rejected because
  reconnect UI should remain reviewable and focused.
- Fork package edge policy in app code: rejected because package/app behavior
  must stay consistent.

## Invariants
- Edge components emit interactions; stores/backends own durable mutation.
- Reconnect UI should preserve package-level connection ownership state.
- Backend rejection information should remain visible to app presenters.

## Revisit Triggers
- App and package edge components converge fully.
- Backend-owned edge insertion responses replace app-specific wiring.
- Additional app-only edge renderers are introduced.

## Dependencies
**Internal:** workflow graph components, package graph helpers, and workflow
stores.

**External:** Svelte 5 and `@xyflow/svelte`.

## Related ADRs
- Reason: edge rendering is documented locally and does not yet have an ADR.
- Revisit trigger: reconnect policy becomes an app/package compatibility
  contract.

## Usage Examples
```ts
import ReconnectableEdge from './ReconnectableEdge.svelte';
```

## API Consumer Contract
- Inputs: graph edge props and interaction callbacks.
- Outputs: rendered edge UI and reconnect events.
- Lifecycle: Svelte mounts and unmounts edge components with visible graph
  edges.
- Errors: expected backend mutation rejection is not represented by component
  exceptions.
- Versioning: props and events must migrate with graph component consumers.

## Structured Producer Contract
- Stable fields: event/callback payload keys and exported component name are
  consumed by app graph wiring.
- Defaults: visual defaults should match package graph behavior.
- Enums and labels: reconnect/connection labels come from shared graph helpers.
- Ordering: pointer event ordering follows browser delivery.
- Compatibility: app and package edge behavior should stay aligned.
- Regeneration/migration: update package/app graph tests and docs with edge
  contract changes.

## Testing
```bash
npm run test:frontend
```

## Notes
- Keep canonical mutation policy in backend graph responses.
