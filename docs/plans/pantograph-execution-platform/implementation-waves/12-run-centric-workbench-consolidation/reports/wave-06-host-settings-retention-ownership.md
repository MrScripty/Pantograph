# Wave 06 Host Settings Retention Ownership

## Scope

Resolved the Stage `12` Settings ownership ambiguity found during source audit.

## Findings

- `SettingsPage.svelte` already owned ArtifactStore policy, format defaults,
  managed media dependencies, and embedded runtime/app settings.
- `IoInspectorPage.svelte` still edited the global diagnostics retention policy,
  which made persistent retention ownership ambiguous.

## Changes

- Moved diagnostics retention policy editing to the workbench Settings page.
- Kept I/O Inspector as an inspection surface for artifact metadata, read-only
  retention policy/state, cleanup results, and cleanup operations.
- Updated workbench docs and the Stage `12` plan to state that persistent
  retention policy edits live on Settings.

## Verification

```bash
node --experimental-strip-types --test src/components/workbench/settingsPagePresenters.test.ts src/components/workbench/ioInspectorPresenters.test.ts src/services/workflow/WorkflowService.commands.test.ts
npm run typecheck -- --pretty false
npm run lint:full
npm run lint:a11y
npm run build
npm run traceability
```
