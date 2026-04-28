# src/config

Frontend configuration metadata boundary.

## Purpose
This directory owns frontend configuration modules that describe app-level
metadata consumed by UI and services.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `architecture.ts` | Architecture graph/config metadata used by architecture views. |

## Problem
Configuration metadata needs a stable location so UI views can consume it
without embedding hard-coded structures in components.

## Constraints
- Config modules should stay declarative and side-effect light.
- Metadata keys must align with consuming views and services.
- Runtime policy should remain in backend/service layers, not static frontend
  config.

## Decision
Keep frontend metadata config here and import it from feature/view modules.

## Alternatives Rejected
- Inline architecture metadata in components: rejected because data and
  presentation would drift.
- Move frontend presentation config into backend crates: rejected because this
  config is app UI metadata.

## Invariants
- Config exports should be deterministic.
- Consumers should import config through stable module paths.
- Metadata changes must update dependent views/tests together.

## Revisit Triggers
- Architecture config becomes generated from source analysis.
- Config metadata becomes user-editable.
- A formal schema is introduced for frontend config.

## Dependencies
**Internal:** architecture views and frontend services.

**External:** TypeScript only.

## Related ADRs
- Reason: frontend config metadata is documented locally.
- Revisit trigger: config becomes generated or persisted.

## Usage Examples
```ts
import { PANTOGRAPH_ARCHITECTURE } from './architecture';
```

## API Consumer Contract
- Inputs: static config data authored in TypeScript.
- Outputs: typed metadata consumed by frontend views.
- Lifecycle: config is loaded as module state by the frontend bundle.
- Errors: malformed config should fail during typecheck or consuming tests.
- Versioning: exported config shape changes require view/service migrations.

## Structured Producer Contract
- Stable fields: architecture ids, labels, groups, and relationships are
  consumed by frontend views.
- Defaults: missing optional metadata should be normalized by consumers.
- Enums and labels: architecture type labels carry presentation semantics.
- Ordering: arrays should remain deterministic for stable rendering.
- Compatibility: views may depend on config ids across releases.
- Regeneration/migration: update consumers and tests with config shape changes.
  Architecture edges should describe active app-shell ownership; retired
  components may remain listed only when they are labeled as legacy or pending
  cleanup.

## Testing
```bash
npm run typecheck
```

## Notes
- Keep executable behavior out of config modules.
