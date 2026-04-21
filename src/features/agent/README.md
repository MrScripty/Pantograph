# src/features/agent

Agent feature export boundary.

## Purpose
This directory owns the frontend entrypoint for agent-related feature exports.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Agent feature exports consumed by app modules. |

## Problem
Agent UI and service code needs a stable import path without coupling callers to
implementation directories.

## Constraints
- The feature entrypoint should stay side-effect light.
- Agent backend behavior remains in services and Tauri/backend modules.
- Exported symbols must stay aligned with consumers.

## Decision
Keep agent feature exports in `index.ts` and delegate behavior to agent
services/components.

## Alternatives Rejected
- Import agent internals directly everywhere: rejected because feature imports
  become brittle.
- Put agent exports in unrelated feature roots: rejected because ownership is
  less clear.

## Invariants
- Exports are frontend contracts for app code.
- Agent command/runtime behavior is not implemented in this entrypoint.
- Export changes require importer migration.

## Revisit Triggers
- Agent feature becomes plugin-extensible.
- Agent exports are split by route or capability.
- Agent backend contracts move to generated types.

## Dependencies
**Internal:** agent services, stores, components, and types.

**External:** TypeScript only.

## Related ADRs
- Reason: agent feature export shape is frontend-local.
- Revisit trigger: agent feature exports become extension API.

## Usage Examples
```ts
import * as agentFeature from './index';
```

## API Consumer Contract
- Inputs: frontend module imports.
- Outputs: exported agent feature symbols.
- Lifecycle: static module export surface.
- Errors: missing exports should fail typecheck.
- Versioning: export changes require consumer migration.

## Structured Producer Contract
- Stable fields: exported symbol names are machine-consumed by TypeScript.
- Defaults: entrypoint does not apply behavior defaults.
- Enums and labels: feature export names carry organizational meaning.
- Ordering: export order is not semantic.
- Compatibility: app imports depend on stable names.
- Regeneration/migration: update all consumers and tests with export changes.

## Testing
```bash
npm run typecheck
```

## Notes
- Keep agent behavior in services/stores, not this barrel.
