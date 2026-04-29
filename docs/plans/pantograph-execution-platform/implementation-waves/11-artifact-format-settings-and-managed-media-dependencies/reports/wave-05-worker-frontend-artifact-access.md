# Wave 05 Worker Report: Frontend Artifact Access

## Scope

Added TypeScript workflow ArtifactStore DTOs, Tauri command bindings, and I/O
Inspector controls for retained artifact body access and consume
acknowledgement.

## Files Changed

- `src/services/workflow/types.ts`
- `src/services/workflow/WorkflowCommandService.ts`
- `src/services/workflow/WorkflowService.commands.test.ts`
- `src/components/workbench/ioInspectorPresenters.ts`
- `src/components/workbench/ioInspectorPresenters.test.ts`
- `src/components/workbench/IoInspectorPage.svelte`

## Result

- Mirrored frontend DTOs for artifact descriptors, read responses, consume
  acknowledgement, ArtifactStore policy, and store stats.
- Added `WorkflowCommandService` methods for:
  - `workflow_artifact_descriptor`
  - `workflow_read_artifact_body`
  - `workflow_acknowledge_artifact_consumed`
  - `workflow_artifact_policy`
  - `workflow_update_artifact_policy`
  - `workflow_artifact_store_stats`
- Added mock behavior for the new command methods.
- Added I/O Inspector per-artifact Read, Download, and Acknowledge controls.
- Read/download paths verify the current backend descriptor before body access.
- Body bytes are converted directly into transient `Blob` object URLs inside
  event handlers. Projection records and long-lived artifact rows still carry
  descriptor metadata only.
- Stale preview object URLs are revoked when replaced, when artifacts disappear
  after refresh, and on component teardown. Download-only URLs are revoked after
  a short delay.
- Diagnostic JSON rendering was not expanded and does not receive artifact body
  bytes.

## Verification

- Passed: `node --experimental-strip-types --test src/components/workbench/ioInspectorPresenters.test.ts`
- Passed: `node --experimental-strip-types --test src/services/workflow/WorkflowService.commands.test.ts`
- Passed: `npm run typecheck -- --pretty false`
- Passed: `npm run build`

## Deferred Work

- No backend command or contract changes were required.
- Persistent ArtifactStore policy ownership remains deferred to the Settings
  page worker; this worker only added service bindings and I/O Inspector access.
- Stream-handle playback/subscription remains outside this worker.
