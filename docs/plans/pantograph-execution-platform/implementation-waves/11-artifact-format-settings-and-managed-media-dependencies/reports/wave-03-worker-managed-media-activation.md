# Wave 03 Worker: Managed Media Activation State

## Owned Write Set

- `crates/inference/src/managed_redistributables.rs`
- `crates/inference/src/managed_redistributables/**`
- `crates/inference/src/lib.rs`
- `crates/inference/tests/managed_redistributables.rs`
- `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/reports/wave-03-worker-managed-media-activation.md`

## Changes

- Added schema-versioned JSON state at `managed-dependencies/state.json` for selected, default, active version, and active leases.
- Added atomic state writes through temp-file write plus rename under the app-owned managed dependency root.
- Added fail-closed selection, default selection, and activation operations that only accept cataloged versions with all expected files present.
- Added a local install-from-staging scaffold that copies an already-prepared dependency directory into the versioned managed root and validates expected files before finalizing.
- Added explicit typed lease acquisition/release and removal behavior that blocks active-version removal while leases exist.
- Kept status/catalog DTO names under the managed redistributables vocabulary and made status projection read persisted selection without consulting host `PATH`.
- Re-exported the new public operations and persisted-state/lease DTOs from `crates/inference/src/lib.rs`.

## Verification

- Passed: `cargo test -p inference --test managed_redistributables`
- Passed: `cargo clippy -p inference --all-targets -- -D warnings`
- Passed: `cargo fmt --all -- --check`
- Host integration split the oversized worker draft into `contracts`,
  `catalog`, `state`, `operations`, and `paths` modules before commit; the
  largest source file in this boundary is now below the project file-size
  target.

## Residual Risks

- This slice intentionally does not download, verify checksums, or accept arbitrary URLs; staging input must already be prepared by a future trusted acquisition path.
- The catalog still has one version per dependency, so operations reject uncataloged versions until multi-version catalog data exists.
- Staging install replacement removes an existing version before final rename; a future production installer may want backup/rollback semantics around replacement.
- Lease holders are durable but not process-liveness-aware yet, so stale lease cleanup remains a later policy decision.
