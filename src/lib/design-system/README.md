# src/lib/design-system

Frontend design-system helper boundary.

## Purpose
This directory owns frontend design tokens, icon helpers, validation utilities,
and agent context values used by Pantograph UI components.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Design-system export surface. |
| `tokens.ts` | Shared design tokens. |
| `icons.ts` | Icon mapping and helper exports. |
| `validator.ts` | Design-system validation helpers. |
| `agentContext.ts` | Agent-specific UI context helpers. |

## Problem
UI components need consistent token and icon usage. Without a local design
system boundary, presentation constants drift across components.

## Constraints
- Design helpers are frontend presentation contracts.
- Tokens should not encode workflow/runtime policy.
- Validation helpers should remain deterministic and testable.

## Decision
Keep design-system helpers here and import them from components rather than
duplicating constants.

## Alternatives Rejected
- Inline tokens and icon maps in components: rejected because visual semantics
  drift.
- Store UI tokens in backend descriptors: rejected because these are frontend
  presentation defaults.

## Invariants
- Tokens and icon exports remain stable for component consumers.
- Validation helpers should report actionable design-system violations.
- Agent context helpers should not own agent backend behavior.

## Revisit Triggers
- A formal design-system package is introduced.
- Tokens become generated from a design source.
- Component validation moves into CI-specific tooling.

## Dependencies
**Internal:** frontend components and validation scripts.

**External:** TypeScript and icon libraries.

## Related ADRs
- Reason: design-system helpers are frontend-local.
- Revisit trigger: tokens become a cross-package contract.

## Usage Examples
```ts
import { tokens } from './tokens';
```

## API Consumer Contract
- Inputs: component style needs and validation targets.
- Outputs: token values, icon references, and validation results.
- Lifecycle: modules are static frontend imports.
- Errors: validation helpers return/report design issues for tooling.
- Versioning: exported token/icon names require consumer migration when
  changed.

## Structured Producer Contract
- Stable fields: token names, icon keys, validation result keys, and context
  keys are machine-consumed by components/tests.
- Defaults: visual defaults should be declared in token modules.
- Enums and labels: icon keys and token names carry presentation semantics.
- Ordering: token object ordering is not semantic.
- Compatibility: component imports depend on stable names.
- Regeneration/migration: update components, tests, and docs with token/icon
  changes.

## Testing
```bash
npm run lint:full
```

## Notes
- Keep backend domain behavior outside design-system helpers.
