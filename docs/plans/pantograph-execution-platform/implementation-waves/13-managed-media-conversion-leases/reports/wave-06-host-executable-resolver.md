# Wave 06 Host Report: Managed Executable Resolver

## Status

Complete.

## Files Changed

- `crates/inference/src/managed_media_dependencies.rs`
- `crates/inference/src/lib.rs`
- `crates/inference/src/README.md`
- `crates/inference/tests/managed_media_dependencies.rs`
- `crates/inference/tests/README.md`

## Implementation Notes

- Added `resolve_media_conversion_dependency_executable_path` as the typed
  resolver for host adapters that need to launch managed media tool binaries.
- The resolver accepts only tool dependencies: `ffmpeg`, `ocioconvert`, and
  `oiiotool`.
- The resolver rejects `OpenColorIO` because it is a managed native library
  artifact, not a process executable.
- Tests now cover successful tool executable resolution from lease metadata
  and fail-closed native-library rejection.

## Verification

- Passed: `cargo test -p inference --test managed_media_dependencies`

## Follow-Up

- The Tauri host adapter should consume this resolver instead of inferring
  executable paths from `expected_files` directly.
