# src/components/side-panel

Side-panel UI components for the app shell.

## Purpose
This directory owns focused Svelte components used inside the app side panel,
including activity display, follow-up input, and settings.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Side-panel component exports. |
| `ActivityLog.svelte` | Activity/event list renderer for side-panel context. |
| `FollowUpInput.svelte` | Follow-up prompt/input component. |
| `SettingsTab.svelte` | Side-panel settings UI. |

## Problem
The side panel mixes user input, activity context, and settings controls. Those
concerns need small components so the app shell does not accumulate unrelated
UI state.

## Constraints
- Components should own UI-local state only.
- Agent/workflow activity facts come from services/stores.
- Settings changes must flow through configured service/store owners.

## Decision
Keep side-panel subcomponents here and export them through `index.ts`.
Application containers compose these components with service-backed state.

## Alternatives Rejected
- Keep all side-panel UI inside one large component: rejected because input,
  activity, and settings have different state/lifecycle concerns.
- Let side-panel controls mutate backend state directly: rejected because
  services/stores own command invocation and persistence.

## Invariants
- Activity display preserves source ordering supplied by stores.
- Follow-up input emits user intent rather than calling backend commands
  directly.
- Settings UI delegates persistence through configured owners.

## Revisit Triggers
- Side panel becomes plugin-extensible.
- Activity events become backend-generated structured contracts.
- Settings move to a generated schema-driven UI.

## Dependencies
**Internal:** app stores, agent services, workflow services, and shared UI
components.

**External:** Svelte 5.

## Related ADRs
- Reason: side-panel component composition is a frontend-local decision.
- Revisit trigger: side-panel APIs become extension/plugin contracts.

## Usage Examples
```ts
import { ActivityLog, FollowUpInput, SettingsTab } from './index';
```

## API Consumer Contract
- Inputs: side-panel props, activity records, input callbacks, and settings
  values.
- Outputs: rendered UI and user interaction events.
- Lifecycle: components mount within the side-panel container and should clean
  up UI resources on teardown.
- Errors: backend/service errors should be passed through presenters rather than
  thrown by leaf components.
- Versioning: exported component props must migrate with the side-panel
  container.

## Structured Producer Contract
- Stable fields: exported component names and event callback payloads are
  consumed by app UI composition.
- Defaults: input/settings defaults should come from stores/services.
- Enums and labels: activity type labels and settings keys carry behavior.
- Ordering: activity rows preserve store-provided order.
- Compatibility: side-panel containers depend on stable exports.
- Regeneration/migration: update containers, stores, and tests with prop
  contract changes.

## Testing
```bash
npm run lint:full
```

## Notes
- Keep service calls in containers/stores rather than leaf components.
