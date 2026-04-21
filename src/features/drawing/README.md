# src/features/drawing

Drawing feature export boundary.

## Purpose
This directory owns the frontend entrypoint for drawing/canvas-related feature
exports.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Drawing feature exports consumed by app modules. |

## Problem
Canvas and drawing helpers need a stable feature import path without coupling
callers to component internals.

## Constraints
- Drawing feature exports should stay frontend-only.
- Durable workflow graph mutation remains backend/store-owned.
- Export shape changes must migrate app consumers.

## Decision
Keep drawing feature exports in `index.ts` and delegate behavior to graph
components, stores, and services.

## Alternatives Rejected
- Deep import drawing internals everywhere: rejected because graph UI structure
  is still evolving.
- Treat drawing feature exports as backend graph contracts: rejected because
  backend graph truth comes from workflow services.

## Invariants
- Drawing exports are app frontend contracts.
- Entry module should not perform backend mutations.
- Exports should remain additive where practical.

## Revisit Triggers
- Drawing features move into the reusable graph package.
- Canvas tools become plugin-facing.
- Backend graph mutation APIs replace local drawing helpers.

## Dependencies
**Internal:** graph components, stores, and package helpers.

**External:** TypeScript only.

## Related ADRs
- Reason: drawing feature exports are frontend-local.
- Revisit trigger: drawing tools become plugin or package API.

## Usage Examples
```ts
import * as drawingFeature from './index';
```

## API Consumer Contract
- Inputs: frontend module imports.
- Outputs: exported drawing feature symbols.
- Lifecycle: static module export surface.
- Errors: missing exports should fail typecheck.
- Versioning: export changes require consumer migration.

## Structured Producer Contract
- Stable fields: exported symbol names are machine-consumed by TypeScript.
- Defaults: entrypoint does not apply interaction defaults.
- Enums and labels: drawing export names carry organizational meaning.
- Ordering: export order is not semantic.
- Compatibility: app imports depend on stable names.
- Regeneration/migration: update consumers and tests with export changes.

## Testing
```bash
npm run typecheck
```

## Notes
- Keep durable graph policy outside this entrypoint.
