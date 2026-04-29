# Wave 04 Worker: Expanded Output Conversion

## Scope

Extended workflow-service artifact output conversion in
`crates/pantograph-workflow-service/src/workflow/artifact_output_conversion.rs`
without changing frozen public contracts or JSON value caps.

## Changes

- Preserved existing image/audio base64 and data-url descriptor conversion.
- Expanded descriptor-eligible output detection to the frozen
  `ArtifactPayloadKind` families already present in contracts:
  `Video`, `ThreeD`, `LargeTable`, `GenericBinary`, and `Structured`.
- Added object-body recognition for common oversized/file-shaped payload keys
  such as `video_base64`, `table_base64`, `data_base64`, `file_base64`,
  `bytes_base64`, `blob_base64`, `data_url`, and `content_data_url`.
- Added media-type and format metadata inference for video, 3D, table,
  structured, and generic binary outputs using the existing
  `ArtifactFormatMetadata` shape.
- Replaced matched output values with ArtifactStore descriptors before
  workflow output value validation; inline base64 bodies are not retained in
  converted binding values.
- Added focused module tests for video, generic file, and table-shaped payload
  descriptor conversion with no-inline-base64 assertions.

## Verification

- Passed:
  `cargo test -p pantograph-workflow-service artifact_output_conversion`
- Passed:
  `cargo test -p pantograph-workflow-service --lib artifact_output_conversion`
- Passed for the touched file:
  `rustfmt --edition 2021 --check crates/pantograph-workflow-service/src/workflow/artifact_output_conversion.rs`
- Passed:
  `cargo fmt --all -- --check`

## Escalation

No escalation required. The implementation did not need contract changes,
ArtifactStore storage-internal changes, diagnostics schema changes, or JSON cap
increases.
