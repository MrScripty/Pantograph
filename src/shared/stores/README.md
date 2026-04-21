# src/shared/stores

Shared frontend store export boundary.

## Purpose
This directory owns shared store exports for app-wide or cross-feature frontend
state.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Shared store barrel exports. |

## Problem
Shared store exports need a stable import path while keeping store
implementation ownership clear.

## Constraints
- Feature-specific stores should remain in feature/store owners.
- Shared stores should not duplicate backend state truth.
- Export changes must migrate consumers.

## Decision
Use this directory as the shared store barrel and keep store implementations
focused in their owning modules.

## Alternatives Rejected
- Put all stores under shared: rejected because feature ownership would blur.
- Deep import shared stores everywhere: rejected because import paths become
  brittle.

## Invariants
- Shared store exports remain low-level and broadly reusable.
- Backend-owned workflow/runtime facts should not be redefined in shared stores.
- Store exports should stay additive where practical.

## Revisit Triggers
- Shared stores become package exports.
- Store ownership moves into generated clients.
- Cross-feature state needs a dedicated domain boundary.

## Dependencies
**Internal:** frontend stores and app consumers.

**External:** Svelte store/runtime APIs where used.

## Related ADRs
- Reason: shared store exports are frontend-local.
- Revisit trigger: store exports become package/plugin API.

## Usage Examples
```ts
import * as sharedStores from './index';
```

## API Consumer Contract
- Inputs: frontend store imports.
- Outputs: shared store symbols.
- Lifecycle: store lifetimes are owned by implementations/consumers.
- Errors: missing exports should fail typecheck.
- Versioning: export changes require consumer migration.

## Structured Producer Contract
- Stable fields: exported store names and store value shapes are
  machine-consumed by frontend code.
- Defaults: store defaults live with implementations.
- Enums and labels: store keys and state labels carry behavior.
- Ordering: export order is not semantic.
- Compatibility: app modules depend on stable store exports.
- Regeneration/migration: update imports, store tests, and docs with store
  shape changes.

## Testing
```bash
npm run typecheck
```

## Notes
- Keep backend durable truth in backend/service stores, not generic shared
  stores.
