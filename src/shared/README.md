# src/shared

Shared frontend export boundary.

## Purpose
This directory owns lightweight shared frontend barrels for components, stores,
and utilities used across app modules.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Shared root export surface. |
| `components/` | Shared component exports. |
| `stores/` | Shared store exports. |
| `utils/` | Shared utility exports. |

## Problem
Common frontend exports need a predictable location without turning `src/shared`
into an unbounded dumping ground.

## Constraints
- Shared modules should remain broadly reusable inside the frontend app.
- Feature-specific behavior belongs in feature/service directories.
- Export changes require consumer migration.

## Decision
Keep only small shared barrels here and move domain-specific logic into feature
or service owners.

## Alternatives Rejected
- Put all common code directly under `src/shared`: rejected because ownership
  becomes unclear.
- Import shared internals by deep path everywhere: rejected because callers
  should use stable shared barrels.

## Invariants
- Shared exports remain low-level and domain-light.
- Feature-specific code should not migrate here just for convenience.
- Barrel exports should stay additive where possible.

## Revisit Triggers
- Shared modules become a separate package.
- Shared exports grow enough to need domain-specific subdirectories.
- A plugin API consumes shared frontend modules.

## Dependencies
**Internal:** shared component/store/utility subdirectories and frontend app
consumers.

**External:** TypeScript and Svelte where exported components require it.

## Related ADRs
- Reason: shared frontend barrels are local source organization.
- Revisit trigger: shared modules become package or plugin API.

## Usage Examples
```ts
import { SafeComponent, Logger } from './index';
```

## API Consumer Contract
- Inputs: frontend imports from app modules.
- Outputs: shared exported symbols.
- Lifecycle: static frontend module exports.
- Errors: missing exports should fail typecheck.
- Versioning: exported names require consumer migration when changed.

## Structured Producer Contract
- Stable fields: exported symbol names and submodule paths are consumed by
  TypeScript imports.
- Defaults: shared barrels should not apply runtime defaults.
- Enums and labels: export names carry organization semantics.
- Ordering: export order is not semantic.
- Compatibility: app modules depend on stable shared export names.
- Regeneration/migration: update imports and tests with export changes.

## Testing
```bash
npm run typecheck
```

## Notes
- Keep this boundary intentionally small.
