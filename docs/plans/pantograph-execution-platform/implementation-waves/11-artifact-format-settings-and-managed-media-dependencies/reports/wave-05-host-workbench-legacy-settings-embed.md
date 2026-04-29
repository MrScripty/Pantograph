# Wave 05 Host Workbench Legacy Settings Embed

## Scope

Embedded the remaining legacy global settings controls into the workbench
Settings page so persistent app configuration has one canonical page.

## Changes

- Added server connection/status controls to `SettingsPage.svelte`.
- Added model path, device policy, RAG, and sandbox configuration controls to
  `SettingsPage.svelte`.
- Updated workbench documentation to include these settings in the canonical
  Settings page ownership boundary.

## Verification

```bash
npm run typecheck -- --pretty false
npm run build
```
