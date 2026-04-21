# packages/svelte-graph/src/context

Svelte context boundary for graph package stores and services.

## Purpose
This directory owns graph package context creation, keys, types, and accessors.
It lets reusable components consume workflow/view/session stores without deep
prop drilling or app-specific imports.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `createGraphContext.ts` | Creates graph context values from package store factories or provided options. |
| `keys.ts` | Svelte context keys for package graph context values. |
| `types.ts` | TypeScript contract for graph context shape. |
| `useGraphContext.ts` | Runtime accessor for components that need graph context. |

## Problem
Package graph components need shared stores and backend abstractions while
remaining reusable outside the app shell. Without a context boundary, components
would import app stores directly or require noisy prop chains.

## Constraints
- Context values must be package-level contracts, not app-specific singletons.
- Components should fail clearly when required context is missing.
- Store factories remain the source of context value construction.
- Context shape changes affect every package component consumer.

## Decision
Keep context keys, creation helpers, and accessors together. Package components
consume context through this boundary, while app code supplies backend/store
implementations through documented options.

## Alternatives Rejected
- Import app stores directly from package components: rejected because the
  package must remain reusable.
- Pass every graph dependency as individual props: rejected because package
  component composition would become brittle.

## Invariants
- Context keys are stable module-level values.
- Context construction uses package store factories.
- Deep app imports are not allowed in package context code.
- Missing context should be treated as a developer wiring error.

## Revisit Triggers
- Package consumers require multiple graph contexts on one page.
- Context shape becomes too broad and needs smaller capability contexts.
- The graph package becomes an external SDK with semver guarantees.

## Dependencies
**Internal:** package store factories, backend types, and component consumers.

**External:** Svelte context APIs and TypeScript.

## Related ADRs
- Reason: graph package context is documented locally and has not required an
  ADR.
- Revisit trigger: context behavior becomes a public SDK compatibility promise.

## Usage Examples
```ts
import { createGraphContext, useGraphContext } from '@pantograph/svelte-graph';
```

## API Consumer Contract
- Inputs: graph context options, backend adapters, and store instances.
- Outputs: Svelte context values consumed by graph package components.
- Lifecycle: context is created by a parent component and read by children for
  the lifetime of that component tree.
- Errors: missing context should fail fast in `useGraphContext`.
- Versioning: context type changes require package component and app wrapper
  updates together.

## Structured Producer Contract
- Stable fields: `GraphContext` property names and context keys are
  machine-consumed by package components.
- Defaults: creation helpers use package store defaults when callers do not
  provide explicit stores.
- Enums and labels: context key labels are semantic within the Svelte runtime.
- Ordering: property order is not semantic.
- Compatibility: changing context shape affects every graph component.
- Regeneration/migration: update types, creation/accessor helpers, package
  components, and tests together.

## Testing
```bash
npm run test:frontend
```

## Notes
- Keep app-specific dependency injection at the package boundary.
