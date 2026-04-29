# Wave 05 Host Artifact Converter Metadata

## Scope

Captured backend converter identity metadata in artifact descriptors produced by
workflow output conversion.

## Changes

- Image descriptors now record the selected image capability provider as the
  converter id.
- Audio and video descriptors now record the selected ffmpeg-backed capability
  provider as the converter id.
- 3D descriptors now record the selected 3D capability provider as the
  converter id.
- Focused artifact conversion tests now assert converter identity for backend
  defaults and explicit output-node overrides.

## Verification

```bash
cargo test -p pantograph-workflow-service artifact_output_conversion
cargo check -p pantograph-workflow-service
rustfmt --edition 2021 --check crates/pantograph-workflow-service/src/workflow/artifact_output_conversion.rs
```
