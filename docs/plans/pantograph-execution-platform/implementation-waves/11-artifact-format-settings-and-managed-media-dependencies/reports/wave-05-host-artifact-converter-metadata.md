# Wave 05 Host Artifact Converter Metadata

## Scope

Captured backend converter identity and active managed dependency version
metadata in artifact descriptors produced by workflow output conversion.

## Changes

- Image descriptors now record the selected image capability provider as the
  converter id.
- Audio and video descriptors now record the selected ffmpeg-backed capability
  provider as the converter id.
- 3D descriptors now record the selected 3D capability provider as the
  converter id.
- Workflow-service now owns `ArtifactFormatDependencyVersions`, a narrow
  host-supplied snapshot of active managed dependency versions. This keeps
  `pantograph-workflow-service` independent of the `inference` crate while
  allowing descriptors to record active converter/library versions.
- Tauri startup and managed media dependency commands synchronize active managed
  versions into workflow-service capabilities after list/status/install/select/
  default/activate/remove operations.
- The UniFFI embedded runtime performs the same synchronization at startup and
  after managed media dependency mutation commands.
- Image artifact metadata records the active `oiiotool` converter version and
  active OpenColorIO library version when those managed dependencies are active.
  Audio/video metadata records active `ffmpeg` versions through the existing
  capability provider fields.
- Focused artifact conversion tests now assert converter identity for backend
  defaults and explicit output-node overrides.

## Verification

```bash
cargo test -p pantograph-workflow-service artifact_output_conversion
cargo test -p pantograph-workflow-service --test artifact_format_settings
cargo test --manifest-path src-tauri/Cargo.toml managed_media_dependency_helpers_project_status_and_actions
cargo test -p pantograph-uniffi managed_media_dependency -- --nocapture
cargo fmt --all -- --check
cargo check -p pantograph-workflow-service
rustfmt --edition 2021 --check crates/pantograph-workflow-service/src/workflow/artifact_output_conversion.rs
```

`cargo check -p pantograph-workflow-service` and the file-scoped `rustfmt`
command are retained from the earlier converter-id slice. The 2026-04-29 active
version metadata update used the targeted tests and `cargo fmt --all -- --check`
listed above.
