# src/registry

Frontend node type registry boundary.

## Purpose
This directory owns frontend registry mappings that connect Pantograph node
types to Svelte node components.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `pantographNodeTypes.ts` | App node type to component registry mapping. |

## Problem
Workflow graph rendering needs a deterministic mapping from backend node type
ids to frontend components. Duplicated mappings would cause templates, saved
workflows, and UI renderers to drift.

## Constraints
- Node type ids must match backend descriptors and saved workflows.
- The registry maps presentation components; backend descriptors own execution
  semantics.
- Missing mappings should fall back intentionally through graph UI behavior.

## Decision
Keep app-specific node type mapping in this registry directory and consume
backend-owned node ids from graph data.

## Alternatives Rejected
- Let each graph view construct its own node registry: rejected because mapping
  consistency matters across app views.
- Define backend execution semantics in frontend registry code: rejected
  because backend descriptors own behavior.

## Invariants
- Registry keys are backend node type ids.
- Registry values are Svelte components or package-compatible node renderers.
- Mapping changes must coordinate with workflow templates and saved workflows.

## Revisit Triggers
- Node registry becomes generated from backend descriptors.
- Plugin/extension nodes are loaded dynamically.
- App consumes package graph registry directly.

## Dependencies
**Internal:** workflow node components and package graph registry utilities.

**External:** Svelte component types.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```ts
import { pantographNodeTypes } from './pantographNodeTypes';
```

## API Consumer Contract
- Inputs: backend node type ids and frontend component imports.
- Outputs: node type registry consumed by graph renderers.
- Lifecycle: registry is loaded as frontend module state.
- Errors: missing node types should be handled by graph fallback components.
- Versioning: registry changes require template and saved workflow checks.

## Structured Producer Contract
- Stable fields: registry keys and component mapping values are
  machine-consumed by graph renderers.
- Defaults: fallback node behavior is owned by graph components.
- Enums and labels: node type ids carry backend execution semantics.
- Ordering: registry key order is not semantic.
- Compatibility: saved workflows/templates may reference registered node ids.
- Regeneration/migration: update backend descriptors, templates, saved
  workflows, and registry mappings together when node ids change.

## Testing
```bash
npm run test:frontend
```

## Notes
- Backend descriptors remain the source of execution truth.
