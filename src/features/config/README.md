# src/features/config

Configuration feature export boundary.

## Purpose
This directory owns the frontend entrypoint for configuration-related feature
exports.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Configuration feature exports consumed by app modules. |

## Problem
Configuration views and services need a stable feature import path without
deep-coupling callers to implementation details.

## Constraints
- The entrypoint should re-export config feature APIs only.
- Runtime persistence remains in services/backend commands.
- Export shape changes must migrate consumers.

## Decision
Keep config feature exports in this directory and delegate behavior to
configuration services/components.

## Alternatives Rejected
- Deep import configuration internals everywhere: rejected because directory
  layout changes would break callers.
- Put config feature exports in static config metadata: rejected because
  feature APIs and static metadata have different ownership.

## Invariants
- Exported names are frontend contracts.
- Entry modules should stay side-effect light.
- Persistence behavior stays outside this entrypoint.

## Revisit Triggers
- Config UI becomes schema-generated.
- Config feature APIs become plugin-facing.
- Config persistence moves to generated client bindings.

## Dependencies
**Internal:** config modules, services, stores, and components.

**External:** TypeScript only.

## Related ADRs
- Reason: config feature export shape is frontend-local.
- Revisit trigger: config feature exports become extension API.

## Usage Examples
```ts
import * as configFeature from './index';
```

## API Consumer Contract
- Inputs: frontend module imports.
- Outputs: exported config feature symbols.
- Lifecycle: static module export surface.
- Errors: missing exports should fail typecheck.
- Versioning: export changes require consumer migration.

## Structured Producer Contract
- Stable fields: exported symbol names are machine-consumed by TypeScript.
- Defaults: entrypoint does not apply persistence defaults.
- Enums and labels: config export names carry organizational meaning.
- Ordering: export order is not semantic.
- Compatibility: app imports depend on stable names.
- Regeneration/migration: update consumers and tests with export changes.

## Testing
```bash
npm run typecheck
```

## Notes
- Keep config state mutation in services/stores.
