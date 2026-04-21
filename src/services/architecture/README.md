# src/services/architecture

Frontend architecture graph service boundary.

## Purpose
This directory owns architecture graph types, layout helpers, and service
exports used by architecture visualization components.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Architecture service exports. |
| `types.ts` | Architecture graph TypeScript contracts. |
| `layout.ts` | Architecture graph layout helpers. |

## Problem
Architecture views need graph-specific types and layout logic that should not
live inside node components or static config alone.

## Constraints
- Architecture graph data is presentation/inspection metadata, not workflow
  execution data.
- Layout helpers should be deterministic.
- Type changes must migrate config and node components together.

## Decision
Keep architecture graph service types and layout helpers here and let
components consume service-level contracts.

## Alternatives Rejected
- Compute architecture layout inside Svelte components: rejected because layout
  should be testable and reusable.
- Treat architecture graph types as workflow graph DTOs: rejected because the
  domains differ.

## Invariants
- Architecture types align with `src/config/architecture.ts`.
- Layout helpers avoid backend side effects.
- Component metadata keys should remain stable for architecture node renderers.

## Revisit Triggers
- Architecture data becomes generated from source analysis.
- Architecture graph layout moves into the graph package.
- Architecture visualization becomes a supported export.

## Dependencies
**Internal:** architecture config and architecture node components.

**External:** TypeScript only.

## Related ADRs
- Reason: architecture graph visualization is frontend-local.
- Revisit trigger: architecture data generation becomes a build/runtime
  contract.

## Usage Examples
```ts
import { layoutArchitecture } from './layout';
```

## API Consumer Contract
- Inputs: architecture graph config and layout options.
- Outputs: typed architecture graph records and layout positions.
- Lifecycle: helpers run during frontend rendering/service setup.
- Errors: invalid config should fail typecheck or service validation.
- Versioning: type/layout changes require config and components to migrate.

## Structured Producer Contract
- Stable fields: architecture graph node ids, categories, labels, and layout
  coordinates are machine-consumed by components.
- Defaults: layout defaults should be explicit in `layout.ts`.
- Enums and labels: architecture categories carry presentation semantics.
- Ordering: graph node/edge order should remain deterministic for rendering.
- Compatibility: architecture node components depend on service type shapes.
- Regeneration/migration: update config, services, components, and tests with
  contract changes.

## Testing
```bash
npm run typecheck
```

## Notes
- Keep workflow graph execution concerns out of architecture services.
