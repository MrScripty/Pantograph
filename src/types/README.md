# src/types

Frontend domain type boundary.

## Purpose
This directory owns frontend TypeScript domain types that are shared by
components and services.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `nodes.ts` | Frontend node-related type declarations. |

## Problem
Shared frontend types need a stable source so components and services do not
define incompatible local versions of node contracts.

## Constraints
- Types should reflect backend DTOs where they cross command boundaries.
- UI-only types must be clearly frontend-owned.
- Type changes require consumer migration.

## Decision
Keep shared frontend domain types here and import them from components/services
instead of repeating structural aliases.

## Alternatives Rejected
- Define node types inside individual components: rejected because component
  copies drift.
- Treat frontend UI types as backend source of truth: rejected because backend
  DTOs own serialized workflow contracts.

## Invariants
- Cross-boundary types stay aligned with backend/service DTOs.
- UI-only types must not be mistaken for serialized backend contracts.
- Exported type names should remain stable for frontend consumers.

## Revisit Triggers
- Backend DTOs generate TypeScript types.
- Node type definitions move fully into the graph package.
- Plugin/extension types become public contracts.

## Dependencies
**Internal:** frontend components, stores, services, and backend adapter types.

**External:** TypeScript only.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```ts
import type { NodeData } from './nodes';
```

## API Consumer Contract
- Inputs: TypeScript imports from components and services.
- Outputs: shared compile-time type contracts.
- Lifecycle: types are erased at runtime but shape frontend source code.
- Errors: type drift should fail `tsc`.
- Versioning: exported types must migrate with all consumers.

## Structured Producer Contract
- Stable fields: exported type names and property names are machine-consumed by
  TypeScript compilation.
- Defaults: runtime defaults belong in services/stores, not type declarations.
- Enums and labels: union labels and node type names carry behavior.
- Ordering: property order is not semantic.
- Compatibility: serialized DTO types must stay aligned with backend contracts.
- Regeneration/migration: update generated/boundary types, services, stores,
  and tests together when DTO shapes change.

## Testing
```bash
npm run typecheck
```

## Notes
- Prefer generated types if backend DTO generation is introduced.
