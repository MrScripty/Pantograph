# packages/svelte-graph/src/utils

Reusable graph package utility helpers.

## Purpose
This directory owns small pure helpers used by the graph package, including
registry construction and geometry calculations.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `buildRegistry.ts` | Builds package node type registries from component mappings. |
| `geometry.ts` | Geometry helpers for line/edge intersection and graph interaction calculations. |

## Problem
Graph package components need shared low-level helpers that are easier to test
outside Svelte components. Without this boundary, registry and geometry logic
would be repeated in UI files.

## Constraints
- Utilities should remain pure and framework-light where possible.
- Registry helpers must preserve package node type contracts.
- Geometry helpers must be deterministic for tests and pointer interactions.

## Decision
Keep reusable utility helpers here and export stable helpers through the package
root when app consumers need them.

## Alternatives Rejected
- Keep geometry inline in graph components: rejected because interaction tests
  need pure helper coverage.
- Build registries ad hoc in every consumer: rejected because node type mapping
  rules should stay consistent.

## Invariants
- Utility helpers do not perform backend mutations.
- Registry output must match package `NodeTypeRegistry` contracts.
- Geometry helpers must avoid DOM reads unless explicitly documented.

## Revisit Triggers
- Utility modules grow into domain-specific packages.
- Registry construction starts consuming backend descriptor schemas directly.
- Geometry helpers need browser layout measurement.

## Dependencies
**Internal:** package registry types and graph components.

**External:** TypeScript only.

## Related ADRs
- Reason: these helpers are package-internal utility boundaries, not ADR-level
  decisions.
- Revisit trigger: utility behavior becomes a documented external SDK surface.

## Usage Examples
```ts
import { buildRegistry, linesIntersect } from '@pantograph/svelte-graph';
```

## API Consumer Contract
- Inputs: component maps, registry metadata, and geometry coordinates.
- Outputs: registry objects and deterministic geometry results.
- Lifecycle: helpers are pure module functions.
- Errors: invalid utility inputs should return explicit results or throw
  developer-facing errors documented by the helper.
- Versioning: exported utility signatures must migrate with package consumers.

## Structured Producer Contract
- Stable fields: registry keys, component mapping values, and geometry result
  shapes are machine-consumed by package components and tests.
- Defaults: registry helpers must document any fallback component selection.
- Enums and labels: node type ids are semantic package keys.
- Ordering: registry construction should preserve deterministic key iteration
  where displayed or tested.
- Compatibility: package consumers may import exported utilities from `index.ts`.
- Regeneration/migration: update exports, consumers, and tests together when
  utility contracts change.

## Testing
```bash
npm run test:frontend
```

## Notes
- Keep heavyweight workflow policy out of package utilities.
