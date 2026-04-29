# Wave 07 Host Report: Managed Executable Fixture And Race Coverage

## Scope

Added focused coverage for Stage `13` converter execution and managed
dependency race behavior in the Tauri host adapter.

## Assigned Write Set

- `src-tauri/src/workflow/managed_media_conversion.rs`
- Stage `13` plan, coordination ledger, and report index updates

## Changes

- Added a Unix-only executable fixture test that installs a managed `ffmpeg`
  test double through the same staging/install/activate path as managed media
  dependencies, then converts through the production `StdProcessRunner`.
- Added a removal-race test proving `remove_managed_redistributable_version`
  refuses to remove an active dependency while a conversion lease is held and
  succeeds after the conversion future is aborted and the lease guard releases
  the lease.

## Verification

Passed:

```bash
cargo test -p pantograph managed_media_conversion -- --nocapture
```

## Residual Risk

The executable fixture covers the managed executable path and process-runner
boundary, not real bundled `ffmpeg`, `ocioconvert`, or `oiiotool` archives.
Real converter fixture coverage still needs managed test archives or explicit
test fixtures that are safe to redistribute with the repository.
