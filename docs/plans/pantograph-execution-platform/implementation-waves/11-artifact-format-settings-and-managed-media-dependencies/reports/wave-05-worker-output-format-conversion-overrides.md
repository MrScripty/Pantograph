# Wave 05 Worker Output Format Conversion Overrides

## Scope

Integrated output-node artifact format selections into workflow-service output
conversion without changing the frozen DTO surface or increasing JSON payload
limits.

## Changed Files

- `crates/pantograph-workflow-service/src/workflow/artifact_output_conversion.rs`
- `crates/pantograph-workflow-service/src/workflow/workflow_run_api.rs`
- `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`
- `crates/pantograph-workflow-service/src/workflow/tests/workflow_run.rs`

## Implemented

- `convert_media_outputs_to_artifacts` now accepts optional run graph settings.
- Session runs pass graph settings decoded from the immutable run snapshot into
  output conversion.
- Image outputs use backend artifact format defaults unless the selected output
  node carries an explicit `artifact_format_override`.
- Audio outputs use backend artifact format defaults unless the selected output
  node carries an explicit `artifact_format_override`.
- Override objects are validated for shape, allowed format/container ids,
  quality, color profile, codec, and bitrate before the descriptor is written.
- Overrides that request a different media type than the actual payload are
  rejected with a typed capability violation until real transcoding is
  implemented.

## Verification

- `cargo test -p pantograph-workflow-service artifact_output_conversion`
- `rustfmt --edition 2021 --check crates/pantograph-workflow-service/src/workflow/artifact_output_conversion.rs crates/pantograph-workflow-service/src/workflow/workflow_run_api.rs crates/pantograph-workflow-service/src/workflow/session_execution_api.rs crates/pantograph-workflow-service/src/workflow/tests/workflow_run.rs`
- `cargo check -p pantograph-workflow-service`
- `cargo fmt --all -- --check`

## Residual Risks

- This slice validates and records requested output metadata, but does not
  transcode media bytes. A later conversion-job slice must use managed
  `ffmpeg`, `oiiotool`, `ocioconvert`, and OpenColorIO dependencies to produce
  the requested output bytes.
- Direct non-session workflow runs do not have graph settings, so they use
  backend defaults and any explicit media type detected from the output payload.
