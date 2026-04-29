# Wave 03 Worker: Managed Media Redistributables

## Owned Write Set

- `crates/inference/src/managed_redistributables.rs`
- `crates/inference/src/managed_redistributables/**`
- `crates/inference/src/lib.rs`
- `crates/inference/tests/managed_redistributables.rs`
- `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/reports/wave-03-worker-managed-media-redistributables.md`

## Changes

- Added a backend-owned managed redistributables boundary for FFmpeg, ocioconvert, oiiotool, and OpenColorIO.
- Added typed dependency IDs, tool-binary/native-library-artifact categories, install state, readiness, package/archive metadata, source metadata, catalog entries, selection state, and version/status projection DTOs.
- Added static catalog metadata with display name, source owner/project, license/redistribution identifier, platform key, version, package/archive kind, expected files, and checksum/signature `Option` placeholders.
- Added filesystem-only status projection rooted at Pantograph's app data `managed-dependencies` directory. Readiness is fail-closed and based only on expected files under the managed dependency root.
- Exported the public module types and functions from `crates/inference/src/lib.rs`.
- Added focused integration tests for required IDs, categories, unmanaged PATH exclusion, expected-file readiness, and metadata fields.

## Verification

- Passed: `cargo test -p inference --test managed_redistributables`
- Passed during host integration: `cargo clippy -p inference --all-targets -- -D warnings`
- Passed: `cargo fmt --all -- --check`

## Residual Risks

- Archive names, download URLs, checksums, and signatures remain explicit placeholders until source artifacts are selected and pinned.
- OpenColorIO native library expected file naming may need refinement once the package layout is finalized for each platform.
- Install/download/select/activate actions and active-version leases remain for
  a later Wave `03` slice.
