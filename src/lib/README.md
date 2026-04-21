# src/lib

Frontend library helper boundary.

## Purpose
This directory owns internal frontend library helpers that are shared across
application features but are not feature entrypoints themselves.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `design-system/` | Frontend design tokens, icons, validation, and agent context helpers. |
| `hotload-sandbox/` | Frontend runtime component sandbox types, components, and services. |
| `tauriConnectionIntentWire.ts` | Tauri connection-intent wire contract helpers. |
| `workflowGraphMutationResponse.ts` | Workflow graph mutation response application helpers. |

## Problem
Shared frontend logic needs a home that is not tied to a single feature or
component tree. Without this boundary, common helpers are duplicated or hidden
inside feature code.

## Constraints
- Library helpers should remain frontend-only unless explicitly generated from
  backend contracts.
- Cross-boundary wire helpers must stay aligned with backend/Tauri DTOs.
- Runtime component sandbox helpers must preserve generated-state ownership
  rules.

## Decision
Keep reusable frontend helper modules under `src/lib` and document subdomains
with focused READMEs.

## Alternatives Rejected
- Put shared helpers in arbitrary feature directories: rejected because import
  ownership becomes unclear.
- Move browser-only helpers into backend crates: rejected because these helpers
  are frontend composition code.

## Invariants
- Helpers should be deterministic and testable where possible.
- Backend DTO helpers must preserve backend error/category semantics.
- Library modules should avoid owning app-wide state directly.

## Revisit Triggers
- Helpers become package-level exports.
- Wire contracts are generated from backend schemas.
- Hotload sandbox ownership moves outside `src`.

## Dependencies
**Internal:** frontend services, stores, components, and backend wire types.

**External:** TypeScript, Svelte where used, and browser APIs.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```ts
import { applyWorkflowGraphMutationResponse } from './workflowGraphMutationResponse';
```

## API Consumer Contract
- Inputs: frontend helper inputs, backend/Tauri response DTOs, and sandbox
  metadata.
- Outputs: normalized helper results consumed by frontend code.
- Lifecycle: helpers are module functions/classes loaded by the frontend
  bundle.
- Errors: helper failures should preserve backend categories when handling wire
  responses.
- Versioning: exported helper signatures must migrate with all consumers.

## Structured Producer Contract
- Stable fields: helper result shapes and wire-normalized keys are consumed by
  services/components/tests.
- Defaults: helper defaults should be explicit in helper source and tests.
- Enums and labels: backend/wire labels retain source semantics.
- Ordering: helper output ordering should be deterministic where displayed.
- Compatibility: helpers may be imported broadly inside the frontend app.
- Regeneration/migration: update helper tests and consumers with contract
  changes.

## Testing
```bash
npm run test:frontend
```

## Notes
- Prefer small helpers over feature-wide utility dumping.
