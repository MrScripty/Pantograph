# Wave 05 Worker 3D Output Node Format Selector

## Scope

Added a graph-local artifact format selector to the point-cloud/3D output node.
The selector reads backend-owned defaults and capabilities through the workflow
service and persists only explicit per-node overrides in node data.

## Changed Files

- `src/components/nodes/workflow/PointCloudOutputNode.svelte`
- `src/components/nodes/workflow/README.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/reports/wave-05-worker-3d-output-node-format-selector.md`

## Implemented

- Added a 3D output format selector to `PointCloudOutputNode.svelte`.
- Loaded selectable 3D formats from
  `workflowService.artifactFormatCapabilities().three_d_formats`.
- Loaded the default 3D format from
  `workflowService.artifactFormatSettings().settings.three_d`.
- Kept missing or `null` `artifact_format_override` as the "use backend
  Settings default" state.
- Persisted `{ format_id }` to `artifact_format_override` only when the user
  chooses a non-default 3D format.
- Mirrored the image/audio selector behavior for unsupported current override
  values, graph gesture isolation, load errors, default labels, and override
  badges.
- Documented point-cloud output artifact format ownership in the workflow node
  README.

## Verification

- `npm run typecheck -- --pretty false`
- `npm run build`

## Remaining Risks

- No focused component/presenter test was added because this node layer does not
  currently expose a direct focused test harness for Svelte output-node selector
  interactions.
- Backend execution must still consume `artifact_format_override` for 3D
  artifact conversion before the selected format changes produced artifact
  bytes.
