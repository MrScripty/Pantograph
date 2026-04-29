# Wave 05 Host Legacy Settings Handoff

## Scope

Retire the side-panel Settings tab as a persistent settings owner so the
workbench Settings page remains the canonical surface for global Pantograph
configuration.

## Changes

- Replaced the side-panel Settings tab content with a focused handoff to the
  workbench Settings page.
- Removed side-panel mounting of server status, model, device, RAG, and
  sandbox settings controls from the Settings tab.
- Updated side-panel component documentation to state that persistent settings
  changes must happen in the workbench Settings page.

## Compatibility Notes

- The `SettingsTab.svelte` export remains stable for the side-panel container.
- The tab now emits navigation intent through existing workbench and panel
  stores instead of owning global settings state.
- Remaining global settings surfaces outside the side panel still need a
  follow-up audit before the parent milestone can close.

## Verification

```bash
npm run typecheck -- --pretty false
npm run build
```
