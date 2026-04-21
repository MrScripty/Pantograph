# src/shared/utils

Shared frontend utility export boundary.

## Purpose
This directory owns shared utility exports for frontend modules.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Shared utility barrel exports. |

## Problem
Utility exports need a stable location without becoming a dumping ground for
domain behavior.

## Constraints
- Utilities should be pure or clearly document side effects.
- Domain-specific helpers belong in their owning feature/service directories.
- Export changes must migrate consumers.

## Decision
Use this directory as a small shared utility barrel and keep domain helpers near
their owners.

## Alternatives Rejected
- Put all helpers under shared utilities: rejected because ownership and test
  targeting become unclear.
- Deep import common utilities everywhere: rejected because import paths become
  brittle.

## Invariants
- Shared utilities remain broadly reusable.
- Utilities should not perform backend mutations.
- Exported utility names should stay stable.

## Revisit Triggers
- Utilities become a separate package.
- A utility gains domain-specific behavior.
- Shared utility exports grow large enough to split by domain.

## Dependencies
**Internal:** frontend app consumers.

**External:** TypeScript only unless a specific utility documents otherwise.

## Related ADRs
- Reason: shared utility exports are frontend-local.
- Revisit trigger: utilities become package/plugin API.

## Usage Examples
```ts
import * as sharedUtils from './index';
```

## API Consumer Contract
- Inputs: frontend utility imports.
- Outputs: shared utility symbols.
- Lifecycle: utilities are static module exports.
- Errors: missing exports should fail typecheck.
- Versioning: utility signatures require consumer migration when changed.

## Structured Producer Contract
- Stable fields: exported utility names and return shapes are machine-consumed
  by frontend code and tests.
- Defaults: helper defaults should be explicit in implementation/tests.
- Enums and labels: utility option labels carry behavior.
- Ordering: output ordering should be deterministic where arrays are returned.
- Compatibility: app modules depend on stable utility exports.
- Regeneration/migration: update imports and tests with utility contract
  changes.

## Testing
```bash
npm run typecheck
```

## Notes
- Keep domain policy out of generic utilities.
