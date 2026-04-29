# Wave 06 Host Report: Tauri Media Conversion Adapter

## Scope

Implemented the desktop host adapter that connects
`pantograph-workflow-service` neutral media conversion requests to Pantograph
managed media dependencies.

## Assigned Write Set

- `src-tauri/Cargo.toml`
- `src-tauri/src/app_setup.rs`
- `src-tauri/src/workflow/managed_media_conversion.rs`
- `src-tauri/src/workflow/mod.rs`
- `src-tauri/src/README.md`
- `src-tauri/src/workflow/README.md`
- Stage `13` plan, coordination ledger, and report index updates

## Changes

- Added `pantograph-media-conversion` to the Tauri crate so the desktop app can
  implement the neutral conversion executor trait.
- Added `TauriManagedMediaConversionExecutor`, which:
  - builds managed dependency lease holders from workflow run/node/port
    attribution,
  - acquires active managed dependency plans from `inference`,
  - resolves executable paths through the typed resolver,
  - runs planned stdin/stdout converter steps with `ProcessRunner`,
  - returns typed conversion result/dependency attribution, and
  - releases acquired leases after success or explicit failure.
- Injected the adapter into the shared `WorkflowService` during app startup.
- Added focused fake-runner tests for ffmpeg audio conversion and
  color-managed image conversion through `ocioconvert`, `oiiotool`, and
  OpenColorIO attribution.

## Verification

Passed:

```bash
cargo test -p pantograph managed_media_conversion -- --nocapture
cargo check -p pantograph-embedded-runtime -p pantograph
```

`cargo check` still reports existing Tauri dead-code warnings in workflow
diagnostics/event/runtime modules; this slice did not introduce new warning
classes after startup executor injection began handling its result.

## Follow-Up

- Add private temp-file fallback and cleanup for converter formats that cannot
  operate over stdin/stdout.
- Add cancellation and dropped-future lease cleanup coverage.
- Add fixture-based real-binary tests once managed test fixtures are available.
- Surface conversion failures in GUI/API projections during the remaining
  Stage `13` rollout work.
