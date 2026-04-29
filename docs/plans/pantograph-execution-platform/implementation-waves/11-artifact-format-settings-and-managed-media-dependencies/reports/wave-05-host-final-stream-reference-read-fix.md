# Wave 05 Host Final Stream Reference Read Fix

## Scope

Fixed an integration issue between backend ArtifactStore stream finalization and
frontend audio preview reads.

## Changes

- `AudioOutputNode.svelte` now reads finalized or final stream-reference chunks
  through `workflowService.readArtifactBody`.
- In-progress stream-reference chunks still use
  `workflowService.readArtifactStream`.

## Reason

ArtifactStore finalization moves an artifact from streaming to retained state.
After that transition, `readArtifactStream` correctly fails closed because the
pending stream body is gone; retained bytes must be read through the artifact
body API.

## Verification

```bash
node --experimental-strip-types --test src/components/workflowToolbarEvents.test.ts
npm run typecheck -- --pretty false
npm run build
```
