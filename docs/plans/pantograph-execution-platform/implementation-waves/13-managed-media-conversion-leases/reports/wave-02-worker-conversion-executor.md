# Wave 02 Worker Conversion Executor

## Scope

Implemented managed process execution scaffolding inside
`pantograph-media-conversion` without requiring real `ffmpeg`, `oiiotool`, or
`ocioconvert` binaries in tests.

## Files Changed

- `crates/pantograph-media-conversion/src/lib.rs`
- `docs/plans/pantograph-execution-platform/implementation-waves/13-managed-media-conversion-leases/reports/wave-02-worker-conversion-executor.md`

## Implemented

- Added `ManagedExecutablePath` validation for host-supplied executable paths.
  Empty paths, relative paths, paths with control characters, and path strings
  with shell metacharacters are rejected before process launch. Whitespace is
  allowed because managed dependency roots may live under user directories with
  spaces.
- Added `ProcessRunner`, `ProcessRunRequest`, and `ProcessRunOutput` contracts.
  Arguments are passed as a separate vector to `Command`, so the executor does
  not shell through user-supplied strings.
- Added `StdProcessRunner` scaffolding that writes source bytes to stdin with
  Tokio async process I/O,
  captures stdout/stderr, bounds stderr summaries, and maps process spawn,
  stdin, wait, timeout, and status failures onto the existing conversion error
  surface.
- Added `ManagedProcessConversionExecutor` to bridge the frozen
  `MediaConversionExecutor` trait to a managed converter process definition.
  The current scaffold treats process stdout as the converted artifact body and
  records the configured dependency attribution.
- Added deterministic unit tests using a fake process runner for executable
  path validation, argv separation, dependency attribution, bounded stderr
  failure summaries, timeout propagation, and cancellation propagation.

## Verification

- `cargo test -p pantograph-media-conversion`
- `cargo fmt --all -- --check`

Both passed.

## Deferred Work

- Real converter-specific argument planning for `ffmpeg`, `oiiotool`, and
  `ocioconvert`.
- Private temporary input/output file handling for tools that cannot operate as
  stdin/stdout filters.
- Host-owned active-version lease acquisition and release around executor use.
- Richer cancellation integration from the workflow/run cancellation boundary
  into process termination.
- Structured diagnostics projection beyond bounded stderr summaries.

## Cross-Boundary Needs

- The host boundary must supply already-resolved absolute executable paths from
  managed dependency leases; this crate intentionally does not discover or
  resolve binaries.
- Future host integration should decide which conversions are stdin/stdout
  compatible and which require private temp files owned outside
  workflow-service.
