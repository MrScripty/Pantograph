# src/lib/hotload-sandbox

Frontend hot-load sandbox boundary.

## Purpose
This directory owns frontend types, components, and services for rendering and
managing runtime-generated Svelte components.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Hotload sandbox export surface. |
| `types.ts` | Sandbox type contracts. |
| `components/` | Safe rendering and error display components. |
| `services/` | Component registry, import, validation cache, and error reporting services. |

## Problem
Runtime-generated components need a controlled frontend rendering and import
path. Without a sandbox boundary, generated component state can leak into
ordinary app component ownership.

## Constraints
- Generated component files under `src/generated` are ignored runtime state,
  while history metadata lives in `.pantograph/generated-components.git/`.
- Frontend sandbox components must not bypass backend validation.
- Runtime imports need explicit error reporting and cache behavior.

## Decision
Keep frontend hotload sandbox support here while backend validation remains in
Tauri hotload modules.

## Alternatives Rejected
- Render generated components directly from arbitrary imports: rejected because
  validation/error isolation is required.
- Move all sandbox UI into Tauri: rejected because rendering and Svelte
  component lifecycle belong in the frontend.

## Invariants
- Safe rendering components isolate errors from the rest of the app.
- Import/validation services preserve component identity and diagnostics.
- Generated-state marker docs and externalized history metadata stay aligned
  with Tauri versioning commands.

## Revisit Triggers
- Generated component history moves away from the repo-local `.pantograph`
  storage path.
- Sandbox import/validation contracts become generated.
- Hotload sandbox becomes a plugin system.

## Dependencies
**Internal:** generated component workspace, Tauri validation commands, and
frontend component services.

**External:** Svelte 5 and browser dynamic import behavior.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```ts
import { ComponentRegistry } from './services/ComponentRegistry';
```

## API Consumer Contract
- Inputs: component ids, module paths, validation results, and import requests.
- Outputs: rendered safe components, registry entries, and diagnostics.
- Lifecycle: services manage runtime component registration/import caching for
  the frontend session.
- Errors: load/render/validation errors are reported through sandbox
  diagnostics rather than crashing the app.
- Versioning: sandbox type changes require generated component consumers to
  migrate.

## Structured Producer Contract
- Stable fields: component ids, registry entries, validation cache keys, and
  error payloads are machine-consumed.
- Defaults: fallback rendering uses safe/error components.
- Enums and labels: validation and load status labels carry behavior.
- Ordering: component registry iteration should remain deterministic where
  displayed.
- Compatibility: generated component state may outlive one frontend session.
- Regeneration/migration: update backend validators, frontend services,
  generated-state docs, and tests together.

## Testing
```bash
npm run lint:full
```

## Notes
- Do not treat runtime-authored generated component files as normal tracked
  source.
