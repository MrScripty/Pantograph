# packages/svelte-graph/src/constants

Shared graph package constants.

## Purpose
This directory owns constants used by the reusable graph package. The current
boundary centralizes port color mapping so port-type presentation remains
consistent across graph components.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `portColors.ts` | Port data type to color mapping and lookup helper. |

## Problem
Port color mapping appears in many visual graph surfaces. Duplicated color
rules would make node handles, edges, legends, and tests drift.

## Constraints
- Constants should remain framework-light and easy to import from package code.
- Port type labels must match package workflow type contracts.
- Visual constants should avoid becoming a second source of backend semantics.

## Decision
Keep reusable graph constants here and export supported constants through the
package root. Treat them as presentation contracts, not workflow runtime truth.

## Alternatives Rejected
- Inline port color maps in each component: rejected because visual semantics
  would drift.
- Store colors in backend descriptors: rejected because these are package
  presentation defaults, not execution contracts.

## Invariants
- Port color lookup remains deterministic for a given port data type.
- Unknown types must receive a safe default from `getPortColor`.
- Exported constants should be additive where possible.

## Revisit Triggers
- The design system owns all graph color tokens.
- Backend descriptors add explicit presentation metadata.
- Port type names change.

## Dependencies
**Internal:** package workflow types and graph components.

**External:** TypeScript only.

## Related ADRs
- Reason: port colors are a package presentation decision, not an ADR-level
  architecture decision.
- Revisit trigger: graph visual semantics become a cross-package design-system
  contract.

## Usage Examples
```ts
import { getPortColor } from '@pantograph/svelte-graph';
```

## API Consumer Contract
- Inputs: port data type labels.
- Outputs: CSS color values for package graph presentation.
- Lifecycle: constants are static module exports.
- Errors: unknown labels return a fallback color rather than throwing.
- Versioning: exported names should stay stable for package consumers.

## Structured Producer Contract
- Stable fields: exported constant names and port type keys are
  machine-consumed by graph components and tests.
- Defaults: unknown port types use the helper fallback.
- Enums and labels: port data type labels carry presentation semantics.
- Ordering: object key order is not semantic.
- Compatibility: visual changes should be coordinated with component snapshots
  or visual expectations.
- Regeneration/migration: update consumers and tests with port type or token
  changes.

## Testing
```bash
npm run test:frontend
```

## Notes
- Runtime data type semantics remain backend-owned.
