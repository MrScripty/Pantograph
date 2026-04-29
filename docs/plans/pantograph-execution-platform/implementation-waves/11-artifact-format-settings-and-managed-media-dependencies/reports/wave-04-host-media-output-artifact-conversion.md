# Wave 04 Host Integration: Media Output Artifact Conversion

## Scope

First execution-cutover slice for workflow-service output validation. This
slice converts image/audio output bindings into ArtifactStore descriptors before
the workflow run response is checked against `max_value_bytes`.

## Changes

- Added workflow-service media output conversion for `image` and `audio` output
  ports.
- Decodes base64 or base64 data URLs into ArtifactStore bodies and replaces the
  workflow output value with a serialized `ArtifactDescriptor`.
- Preserves the JSON value cap by keeping large media bodies out of the
  response binding payload.
- Adds attribution metadata for workflow id, workflow version, run id, node id,
  and port id.
- Fails closed when a media output is present but the service has no configured
  ArtifactStore.

## Verification

Passed:

- `cargo test -p pantograph-workflow-service artifact_output_conversion`
- `cargo test -p pantograph-workflow-service --test artifact_store`
- `cargo test -p pantograph-workflow-service --test artifact_contract`
- `cargo test -p pantograph-workflow-service --test contract`
- `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`
- `cargo fmt --all -- --check`

## Residual Risk

- Python bridge stream chunks and non-image/audio binary producers still need
  later Wave `04` migrations.
- Diagnostics projections still need to link the emitted descriptors to run I/O
  projections.
