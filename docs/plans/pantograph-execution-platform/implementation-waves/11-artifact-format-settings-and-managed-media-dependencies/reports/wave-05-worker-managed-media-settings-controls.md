# Wave 05 Worker Managed Media Settings Controls

## Scope

Added workbench Settings controls for managed media dependencies using the
existing backend Tauri command surface.

## Changed Files

- `src/services/workflow/types.ts`
- `src/services/workflow/WorkflowCommandService.ts`
- `src/services/workflow/WorkflowService.commands.test.ts`
- `src/components/workbench/SettingsPage.svelte`
- `src/components/workbench/settingsPagePresenters.ts`
- `src/components/workbench/settingsPagePresenters.test.ts`
- `src/components/workbench/README.md`

## Implemented

- Added TypeScript DTOs mirroring backend managed redistributable status,
  catalog, selection, version, and action payloads.
- Added workflow service methods for list, status, install-from-staging,
  select, set default, activate, and remove managed media dependency actions.
- Added command-forwarding tests proving frontend services return backend
  status responses directly.
- Added Settings page status rows and operational controls for `ffmpeg`,
  `ocioconvert`, `oiiotool`, and OpenColorIO.
- Kept dependency status backend-owned: after every action the page replaces
  display state from the returned backend status instead of applying
  optimistic persistent state locally.
- Documented the Settings page ownership boundary in the workbench README.

## Verification

- `node --experimental-strip-types --test src/services/workflow/WorkflowService.commands.test.ts`
- `node --experimental-strip-types --test src/components/workbench/settingsPagePresenters.test.ts`
- `npm run typecheck -- --pretty false`
- `npm run build`

## Residual Risks

- Install-from-staging currently requires a user-provided staging directory
  path. A later Library/downloader slice still needs to provide a richer
  managed dependency acquisition flow once source artifacts and checksums are
  pinned.
- Legacy side-panel/server/runtime settings surfaces still exist and need a
  separate consolidation slice so the workbench Settings page is the only
  persistent settings owner.
