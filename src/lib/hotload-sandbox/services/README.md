# src/lib/hotload-sandbox/services

Frontend hot-load sandbox service boundary.

## Purpose
This directory owns frontend services for registering, importing, validating,
caching, and reporting errors for runtime-generated components.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `ComponentRegistry.ts` | Runtime component registry service. |
| `GlobRegistry.ts` | Static/dynamic glob import registry helper. |
| `ImportManager.ts` | Runtime component import coordination. |
| `ValidationCache.ts` | Validation result cache. |
| `ErrorReporter.ts` | Sandbox error reporting helper. |

## Problem
Runtime component loading combines module discovery, validation state, import
caching, and user-facing diagnostics. Keeping that in components would make
error handling inconsistent.

## Constraints
- Services must preserve component ids and validation diagnostics.
- Imports must not bypass backend validation decisions.
- Error reporting should be structured enough for UI display.

## Decision
Keep hotload sandbox service classes here and let UI components consume their
registry/import/error state.

## Alternatives Rejected
- Perform dynamic imports directly in every preview component: rejected because
  cache and error behavior would drift.
- Store validation state only in backend modules: rejected because frontend
  rendering needs session-local cache and diagnostics.

## Invariants
- Component ids and module paths remain stable within a frontend session.
- Validation cache entries must correspond to the generated component version.
- Error reports preserve original failure context.

## Revisit Triggers
- Generated component state moves outside `src`.
- Validation/import state becomes backend-synchronized.
- Sandbox service contracts become plugin-facing.

## Dependencies
**Internal:** hotload sandbox types, components, and Tauri validation commands.

**External:** browser dynamic import behavior and TypeScript.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```ts
import { ComponentRegistry } from './ComponentRegistry';
```

## API Consumer Contract
- Inputs: component ids, glob modules, validation results, and import requests.
- Outputs: registry entries, imported modules, cached validation, and error
  records.
- Lifecycle: services live for a frontend session or preview lifecycle.
- Errors: import and validation failures are reported through structured error
  records.
- Versioning: service method/record changes require component consumers to
  migrate.

## Structured Producer Contract
- Stable fields: registry entry keys, validation cache keys, error fields, and
  import result shapes are machine-consumed.
- Defaults: missing validation should be treated as untrusted until refreshed.
- Enums and labels: validation/import status labels carry behavior.
- Ordering: registry/listing output should be deterministic when displayed.
- Compatibility: generated component previews depend on service record shapes.
- Regeneration/migration: update components, backend validators, tests, and
  docs with service contract changes.

## Testing
```bash
npm run lint:full
```

## Notes
- This directory owns frontend sandbox state, not backend validation policy.
