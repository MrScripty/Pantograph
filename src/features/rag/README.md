# src/features/rag

RAG feature export boundary.

## Purpose
This directory owns the frontend entrypoint for retrieval-augmented generation
feature exports.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | RAG feature exports consumed by app modules. |

## Problem
RAG UI/service code needs a stable frontend import boundary without coupling
callers to storage or command internals.

## Constraints
- Backend indexing and retrieval remain owned by services/Tauri modules.
- Feature exports should stay side-effect light.
- Export changes must migrate consumers.

## Decision
Keep RAG feature exports here and delegate behavior to services/components.

## Alternatives Rejected
- Deep import RAG internals from app modules: rejected because storage and UI
  structure may change.
- Implement retrieval behavior in this entrypoint: rejected because services
  and backend modules own retrieval.

## Invariants
- Exported names are frontend contracts.
- Retrieval ranking and storage behavior are not defined here.
- Entry modules should be safe to import in frontend code.

## Revisit Triggers
- RAG becomes plugin-facing.
- RAG service contracts are generated.
- Retrieval UI moves to a route-level feature module.

## Dependencies
**Internal:** RAG services, stores, and components.

**External:** TypeScript only.

## Related ADRs
- Reason: RAG feature exports are frontend-local.
- Revisit trigger: RAG feature exports become extension API.

## Usage Examples
```ts
import * as ragFeature from './index';
```

## API Consumer Contract
- Inputs: frontend module imports.
- Outputs: exported RAG feature symbols.
- Lifecycle: static module export surface.
- Errors: missing exports should fail typecheck.
- Versioning: export changes require consumer migration.

## Structured Producer Contract
- Stable fields: exported symbol names are machine-consumed by TypeScript.
- Defaults: entrypoint does not apply retrieval defaults.
- Enums and labels: RAG export names carry organizational meaning.
- Ordering: export order is not semantic.
- Compatibility: app imports depend on stable names.
- Regeneration/migration: update consumers and tests with export changes.

## Testing
```bash
npm run typecheck
```

## Notes
- Keep retrieval behavior in service/backend owners.
