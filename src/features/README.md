# src/features

Frontend feature entrypoint boundary.

## Purpose
This directory groups frontend feature modules and re-export points so app code
can import feature-level functionality without deep paths.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Feature barrel exports. |
| `agent/` | Agent feature entrypoint exports. |
| `config/` | Configuration feature entrypoint exports. |
| `drawing/` | Drawing/canvas feature entrypoint exports. |
| `llm/` | LLM feature entrypoint exports. |
| `rag/` | RAG feature entrypoint exports. |

## Problem
Frontend features span services, stores, components, and types. Feature
entrypoints provide stable import surfaces without making components depend on
internal paths.

## Constraints
- Feature modules should re-export stable frontend APIs only.
- Backend-owned workflow/runtime semantics must not be redefined here.
- Deep imports should be avoided when a feature export exists.

## Decision
Use `src/features` as a frontend import boundary and keep feature-specific
exports small.

## Alternatives Rejected
- Import every feature implementation by deep path: rejected because call sites
  would couple to directory layout.
- Put feature exports in root `src/index.ts`: rejected because feature grouping
  improves ownership and review.

## Invariants
- Feature exports should be additive where possible.
- Feature entrypoints should avoid side effects.
- Feature modules should delegate behavior to services/stores/components.

## Revisit Triggers
- Feature modules become independently packaged.
- A feature export becomes a plugin/extension contract.
- Feature ownership moves into route-level modules.

## Dependencies
**Internal:** frontend services, stores, components, and type modules.

**External:** TypeScript only.

## Related ADRs
- Reason: feature entrypoints are frontend-local organization.
- Revisit trigger: feature exports become external extension contracts.

## Usage Examples
```ts
import { agentFeature } from './agent';
```

## API Consumer Contract
- Inputs: feature-level imports from app modules.
- Outputs: stable frontend exports.
- Lifecycle: modules are loaded by the frontend bundle.
- Errors: export drift should be caught by typecheck.
- Versioning: exported names should migrate with all importers.

## Structured Producer Contract
- Stable fields: exported symbol names and feature namespace paths are consumed
  by TypeScript imports.
- Defaults: feature entrypoints should not apply runtime defaults.
- Enums and labels: feature names are organizational labels.
- Ordering: barrel export ordering is not semantic.
- Compatibility: deep import removal requires consumers to migrate.
- Regeneration/migration: update imports and tests with export changes.

## Testing
```bash
npm run typecheck
```

## Notes
- Keep feature entrypoints thin.
