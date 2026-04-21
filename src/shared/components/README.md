# src/shared/components

Shared frontend component export boundary.

## Purpose
This directory owns shared component exports available to multiple frontend
domains.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Shared component barrel exports. |

## Problem
Cross-domain component exports need a stable import path without requiring
callers to depend on component implementation layout.

## Constraints
- Components exported here should be broadly reusable.
- Feature-specific components should stay in feature/component directories.
- Export changes must migrate consumers.

## Decision
Use this directory as the shared component barrel and keep implementation
ownership in the component modules themselves.

## Alternatives Rejected
- Export every app component as shared: rejected because domain ownership would
  blur.
- Deep import shared components everywhere: rejected because import paths become
  brittle.

## Invariants
- Shared component exports stay additive where practical.
- Shared components should not own feature-specific service calls.
- Components must remain compatible with app design-system expectations.

## Revisit Triggers
- Shared components become a package.
- A component needs feature-specific behavior.
- Design-system component ownership changes.

## Dependencies
**Internal:** shared/app components and design-system helpers.

**External:** Svelte 5.

## Related ADRs
- Reason: shared component exports are frontend-local.
- Revisit trigger: shared components become package API.

## Usage Examples
```ts
import * as sharedComponents from './index';
```

## API Consumer Contract
- Inputs: frontend component imports.
- Outputs: shared component symbols.
- Lifecycle: static module export surface.
- Errors: missing exports should fail typecheck.
- Versioning: export changes require consumer migration.

## Structured Producer Contract
- Stable fields: exported component names are consumed by TypeScript imports.
- Defaults: component defaults live with implementations.
- Enums and labels: component names carry organization semantics.
- Ordering: export order is not semantic.
- Compatibility: consumers depend on stable exported names.
- Regeneration/migration: update imports and tests with export changes.

## Testing
```bash
npm run typecheck
```

## Notes
- Keep component implementation docs near each component domain.
