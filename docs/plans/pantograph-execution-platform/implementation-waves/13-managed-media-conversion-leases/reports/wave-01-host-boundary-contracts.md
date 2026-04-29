# Wave 01 Host Boundary Contracts

## Scope

Host-owned boundary design and shared contract freeze for Stage `13`.

## Files Changed

- `Cargo.toml`
- `crates/pantograph-media-conversion/Cargo.toml`
- `crates/pantograph-media-conversion/src/lib.rs`
- `crates/pantograph-media-conversion/src/README.md`
- `docs/plans/pantograph-execution-platform/13-managed-media-conversion-leases.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/13-managed-media-conversion-leases/coordination-ledger.md`

## Boundary Decision

Stage `13` uses a neutral `pantograph-media-conversion` crate for conversion
contracts and executor traits. The crate does not depend on
`pantograph-workflow-service`, `inference`, Tauri, or bindings. This preserves
the host-agnostic workflow-service boundary while giving later host-owned
conversion code a stable request/result/error and attribution contract.

## Implemented Contracts

- Typed conversion, artifact, workflow-run, node, port, dependency-version, and
  lease ids.
- `MediaConversionTarget` with bounded format, codec, quality, bitrate, CRF,
  bit depth, color profile, and color-managed fields.
- Internal source/result body carriers for backend-owned bytes.
- `MediaConversionDependencyAttribution` for per-conversion dependency version
  and lease id recording.
- `MediaConversionExecutor` trait for later managed converter implementations.
- Typed `MediaConversionError` variants for invalid requests, unsupported
  conversions, dependency unavailability, process failure, timeout,
  cancellation, and I/O failures.

## Verification

- `cargo test -p pantograph-media-conversion`
- `cargo fmt --all -- --check`
- `npm run traceability`

## Deferred

- Real process invocation for managed `ffmpeg`, `oiiotool`, and
  `ocioconvert`.
- Active-version lease acquisition and release around a concrete conversion.
- ArtifactStore private temporary input/output handling.
- Diagnostics and GUI projection of conversion status/failures.
