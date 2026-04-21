# src/lib/hotload-sandbox/components

Safe rendering components for the hot-load sandbox.

## Purpose
This directory owns Svelte components that render runtime-loaded components and
their error states without destabilizing the rest of the app.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `ComponentContainer.svelte` | Container for runtime-loaded component display. |
| `SafeComponent.svelte` | Guarded component renderer. |
| `ErrorPlaceholder.svelte` | Error-state renderer for failed runtime components. |

## Problem
Runtime-loaded components can fail validation, import, or render. The app needs
dedicated safe renderers so failures remain localized and diagnosable.

## Constraints
- Components must display diagnostics without hiding validation failures.
- Runtime component UI should not mutate workflow/runtime state directly.
- Browser component resources must be cleaned up on unmount.

## Decision
Keep hotload rendering components here and let sandbox services provide
registry/import/validation data.

## Alternatives Rejected
- Render runtime components directly in app containers: rejected because error
  isolation would be inconsistent.
- Put error placeholders in unrelated shared components: rejected because
  sandbox diagnostics have specific context.

## Invariants
- Runtime render failures stay localized.
- Error placeholders show actionable diagnostics.
- Components consume sandbox service state rather than loading modules directly
  when possible.

## Revisit Triggers
- Sandbox rendering becomes plugin-facing.
- Svelte error-boundary behavior changes.
- Validation diagnostics gain a formal schema.

## Dependencies
**Internal:** hotload sandbox services and types.

**External:** Svelte 5.

## Related ADRs
- Reason: sandbox rendering is documented locally.
- Revisit trigger: hotload components become plugin API.

## Usage Examples
```ts
import SafeComponent from './SafeComponent.svelte';
```

## API Consumer Contract
- Inputs: component references, props, error payloads, and diagnostics.
- Outputs: rendered runtime component or error UI.
- Lifecycle: components mount/unmount with runtime preview surfaces.
- Errors: rendering/import failures should be captured and displayed.
- Versioning: props must migrate with sandbox service consumers.

## Structured Producer Contract
- Stable fields: prop names and diagnostic payload keys are consumed by
  sandbox containers.
- Defaults: failed render path uses `ErrorPlaceholder`.
- Enums and labels: status/error labels carry UI behavior.
- Ordering: diagnostic lists preserve service-provided order.
- Compatibility: generated component preview surfaces depend on stable props.
- Regeneration/migration: update services, types, and tests with prop changes.

## Testing
```bash
npm run lint:full
```

## Notes
- Keep runtime import policy in sandbox services.
