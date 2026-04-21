# src/components/nodes/architecture

Architecture-diagram node components.

## Purpose
This directory owns node components used by Pantograph's architecture graph
views, including service, store, backend, command, and component nodes.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Architecture node component exports. |
| `ArchBaseNode.svelte` | Shared architecture node layout and visual treatment. |
| `ArchBackendNode.svelte` | Backend architecture node renderer. |
| `ArchCommandNode.svelte` | Command/transport architecture node renderer. |
| `ArchComponentNode.svelte` | Frontend component architecture node renderer. |
| `ArchServiceNode.svelte` | Service architecture node renderer. |
| `ArchStoreNode.svelte` | Store architecture node renderer. |

## Problem
Architecture views need domain-specific node renderers that explain codebase
boundaries without mixing those visuals into workflow execution node components.

## Constraints
- Architecture nodes are presentation components, not workflow task nodes.
- Labels and colors should stay consistent with architecture service metadata.
- Components should not own architecture graph data generation.

## Decision
Keep architecture node renderers in a dedicated directory and export them
through `index.ts`. Architecture data remains owned by services/config modules.

## Alternatives Rejected
- Reuse workflow node components for architecture diagrams: rejected because
  architecture nodes represent code ownership, not executable workflow tasks.
- Generate architecture visuals directly in services: rejected because visual
  composition belongs in Svelte components.

## Invariants
- Architecture node components remain presentation-only.
- Shared visual structure should live in `ArchBaseNode.svelte`.
- Node type-specific components should avoid duplicating layout logic.

## Revisit Triggers
- Architecture graph schema becomes generated.
- Architecture node types move into the reusable graph package.
- Visual style becomes part of a formal design-system contract.

## Dependencies
**Internal:** architecture services/config and shared graph/node components.

**External:** Svelte 5.

## Related ADRs
- Reason: architecture node presentation does not currently require an ADR.
- Revisit trigger: architecture graph data becomes a product contract.

## Usage Examples
```ts
import { ArchServiceNode } from './index';
```

## API Consumer Contract
- Inputs: architecture node props and metadata from graph services.
- Outputs: rendered architecture node UI.
- Lifecycle: components mount with graph nodes and do not own durable state.
- Errors: invalid architecture metadata should be handled by graph services.
- Versioning: component prop changes require architecture graph consumers to
  migrate.

## Structured Producer Contract
- Stable fields: exported component names and expected node metadata keys are
  consumed by architecture graph wiring.
- Defaults: visual defaults should live in shared base components.
- Enums and labels: architecture node type labels carry presentation semantics.
- Ordering: not applicable to individual node renderers.
- Compatibility: architecture views depend on consistent component exports.
- Regeneration/migration: update architecture services, node exports, and tests
  together when metadata contracts change.

## Testing
```bash
npm run lint:full
```

## Notes
- Keep workflow execution concerns out of architecture diagram components.
