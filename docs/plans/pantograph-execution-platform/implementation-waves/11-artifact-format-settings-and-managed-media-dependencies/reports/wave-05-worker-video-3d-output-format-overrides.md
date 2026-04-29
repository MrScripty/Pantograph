# Wave 05 Worker Video 3D Output Format Overrides

## Scope

Implemented workflow-service artifact output conversion handling for backend
video and 3D artifact format defaults plus graph-local
`artifact_format_override` metadata.

## Changed Files

- `crates/pantograph-workflow-service/src/workflow/artifact_output_conversion.rs`
- `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/reports/wave-05-worker-video-3d-output-format-overrides.md`

## Implemented

- Added video output format resolution using backend `ArtifactFormatSettings`
  and `ArtifactFormatCapabilities`.
- Validated video `container_id`, `codec_id`, `crf`, and `bit_depth` before
  writing descriptors.
- Added 3D output format resolution using backend defaults and capability
  validation for `format_id`.
- Rejected explicit video and 3D override media-type mismatches with the
  existing capability-violation transcode-required error path.
- Preserved actual emitted descriptor media type and format identity when a
  runtime output already declares a concrete video or 3D media type.
- Kept this slice metadata-only: no real transcoding and no JSON size cap
  changes.

## Verification

- `rustfmt --edition 2021 crates/pantograph-workflow-service/src/workflow/artifact_output_conversion.rs`
- `cargo test -p pantograph-workflow-service artifact_output_conversion`

## Residual Risks

- Video and 3D overrides still require later conversion/transcoding work to
  produce bytes in a requested format when the runtime emits a different media
  type.
