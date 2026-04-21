# src/components/orchestration

Frontend orchestration graph components.

## Purpose
This directory owns Svelte components that render high-level orchestration
graphs and orchestration node types in the app frontend.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Orchestration component exports. |
| `OrchestrationGraph.svelte` | Orchestration graph container and interaction surface. |
| `OrchestrationBaseNode.svelte` | Shared node layout for orchestration node renderers. |
| `StartNode.svelte` | Start node renderer. |
| `EndNode.svelte` | End node renderer. |
| `ConditionNode.svelte` | Conditional branch node renderer. |
| `LoopNode.svelte` | Loop node renderer. |
| `DataGraphNode.svelte` | Data graph invocation node renderer. |
| `MergeNode.svelte` | Merge node renderer. |

## Problem
Orchestration graphs are distinct from workflow data graphs. The frontend needs
dedicated renderers that display orchestration semantics while backend Rust owns
execution behavior.

## Constraints
- Components render orchestration state and must not own execution policy.
- Node labels and ports must align with node-engine orchestration DTOs.
- Orchestration UI should remain separate from workflow-node runtime UI.

## Decision
Keep orchestration graph rendering here and consume backend/node-engine
orchestration contracts. Shared node visuals live in `OrchestrationBaseNode`.

## Alternatives Rejected
- Render orchestration nodes with workflow node components: rejected because
  orchestration nodes carry control-flow semantics, not task execution UI.
- Let frontend define orchestration execution behavior: rejected because
  backend orchestration is the source of truth.

## Invariants
- Orchestration components are presentation and interaction surfaces only.
- Backend orchestration DTOs define node/edge semantics.
- Component exports should remain stable for graph composition.

## Revisit Triggers
- Orchestration graph editing moves fully into backend mutation responses.
- Orchestration components move into the reusable graph package.
- Orchestration DTOs become generated schemas.

## Dependencies
**Internal:** node-engine orchestration DTOs, frontend graph components, and
orchestration services.

**External:** Svelte 5 and graph rendering dependencies.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```ts
import { OrchestrationGraph } from './index';
```

## API Consumer Contract
- Inputs: orchestration graph data, node metadata, and UI callbacks.
- Outputs: rendered orchestration graph UI and interaction events.
- Lifecycle: components mount with the orchestration view and own only UI-local
  state.
- Errors: invalid graph state should be reported by services/backends.
- Versioning: prop and export changes require orchestration view consumers to
  migrate together.

## Structured Producer Contract
- Stable fields: component exports, node renderer names, and expected
  orchestration metadata keys are consumed by the app graph composition.
- Defaults: visual defaults should live in shared base components.
- Enums and labels: orchestration node type labels carry behavior.
- Ordering: rendered node/edge ordering follows graph data ordering.
- Compatibility: saved orchestration examples depend on matching node type
  labels and frontend renderers.
- Regeneration/migration: update node-engine DTOs, orchestration examples,
  frontend components, and tests together when contracts change.

## Testing
```bash
npm run test:frontend
```

## Notes
- Execution policy remains in backend orchestration modules.
