# Wave 03 Worker: OpenColorIO Conversion Leases

## Write Set

- `crates/inference/src/managed_media_dependencies.rs`
- `crates/inference/src/lib.rs`
- `crates/inference/tests/managed_media_dependencies.rs`
- `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/reports/wave-03-worker-ocio-conversion-leases.md`

## Implemented

- Added a backend scaffold for media conversion dependency planning with non-runtime-specific DTO names.
- Added fail-closed OpenColorIO activation validation on top of the public managed redistributables API. The boundary requires an active managed OpenColorIO version whose expected files are present under app data.
- Added conversion dependency planning for image, audio, video, and 3D jobs:
  - image and 3D jobs lease `oiiotool`
  - color-managed jobs additionally lease `ocioconvert` and OpenColorIO
  - audio and video jobs lease `ffmpeg`
- Added explicit plan release that releases all acquired managed dependency leases.
- Added rollback for partial acquisition failures so failed plans do not leave newly acquired leases behind.
- Re-exported public media dependency planning and OpenColorIO validation types/functions from `crates/inference/src/lib.rs`.
- Added integration tests for dependency planning, lease-backed removal blocking, release behavior, missing/inactive fail-closed paths, and OpenColorIO validation states.

## Residual Risk

- OpenColorIO ABI validation is intentionally not real in this scaffold because this slice does not add unsafe FFI or dynamic library loading. Successful OpenColorIO activation returns `not_validated` with an explicit reason after managed expected-file validation passes.
- The safe activation boundary validates only the managed redistributable catalog, active selection, and expected file presence. It does not prove that the native library is loadable by the eventual conversion process.

## Verification

- Passed: `cargo test -p inference --test managed_media_dependencies`
- Passed: `cargo clippy -p inference --all-targets -- -D warnings`
- Passed after host integration: `cargo fmt --all -- --check`
- Worker-local formatting before host integration passed for this worker's Rust
  write set with `rustfmt --edition 2021`.
