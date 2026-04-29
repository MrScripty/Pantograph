# Wave 05 Host Frontend Artifact Stream Read

## Scope

Added frontend DTOs, workflow-service command forwarding, and I/O Inspector
controls for binary-safe reads from in-progress ArtifactStore streams.

## Changed Files

- `src/services/workflow/types.ts`
- `src/services/workflow/WorkflowCommandService.ts`
- `src/services/workflow/WorkflowService.commands.test.ts`
- `src/components/workbench/IoInspectorPage.svelte`

## Implemented

- Added TypeScript contracts for `WorkflowArtifactStreamReadRequest`,
  `WorkflowArtifactStreamReadResponse`, and `WorkflowArtifactStreamBodyRead`.
- Added `WorkflowCommandService.readArtifactStream`, forwarding to
  `workflow_read_artifact_stream`.
- Added mock stream-read behavior and command-forwarding coverage.
- Added an I/O Inspector stream read action for artifacts with stream handles,
  reusing transient Blob URL previews without storing bodies in projection
  state.

## Verification

- `node --experimental-strip-types --test src/services/workflow/WorkflowService.commands.test.ts`
- `npm run typecheck -- --pretty false`
- `npm run build`
