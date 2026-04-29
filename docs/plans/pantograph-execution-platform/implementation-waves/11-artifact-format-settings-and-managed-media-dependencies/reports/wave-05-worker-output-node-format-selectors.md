# Wave 05 Worker Output Node Format Selectors

## Scope

Added graph-local artifact format selectors to the image and audio output node
UIs. The selectors read backend-owned defaults and capabilities through the
workflow service and persist only explicit per-node overrides in node data.

## Changed Files

- `src/components/nodes/workflow/ImageOutputNode.svelte`
- `src/components/nodes/workflow/AudioOutputNode.svelte`
- `src/components/nodes/workflow/README.md`

## Implemented

- Added image output format controls for format, quality, and color profile.
- Added audio output format controls for container, codec, and bitrate.
- Added `artifact_format_override` node data for explicit overrides.
- Kept missing or `null` `artifact_format_override` as the "use backend
  Settings default" state.
- Loaded selectable options from
  `workflowService.artifactFormatCapabilities`.
- Loaded default values from `workflowService.artifactFormatSettings`.
- Documented the frontend ownership boundary: Settings owns persistent defaults;
  output nodes own only transient UI state and explicit graph-data overrides.

## Verification

- `npm run typecheck -- --pretty false`
- `npm run build`

## Remaining Risks

- The selectors persist graph data for run snapshots, but backend execution must
  still consume `artifact_format_override` during artifact conversion before the
  selected formats affect produced artifact bytes.
- Each component owns small local option-projection helpers because this worker
  could not edit shared TypeScript helper files.
