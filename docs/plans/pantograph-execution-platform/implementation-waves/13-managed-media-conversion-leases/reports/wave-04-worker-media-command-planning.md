# Wave 04 Worker Media Command Planning Report

## Summary

Implemented deterministic managed media command planning in
`pantograph-media-conversion` without wiring workflow execution. Plans are
host-neutral and contain required managed dependency ids, stdin/stdout stream
markers, and separate argv vectors instead of shell command strings.

## Files Changed

- `crates/pantograph-media-conversion/src/lib.rs`
  - Added `UnsupportedCommandPlan` typed error.
  - Added `MediaCommandPlan`, `MediaCommandPlanStep`, and
    `MediaCommandPlanStream`.
  - Added image, audio, and video command-plan builders.
  - Added fail-closed 3D planning behavior.
  - Added unit coverage for default and explicit target metadata.
- `crates/pantograph-media-conversion/README.md`
  - Documented command-plan scope, dependency mapping, and host-neutral
    invariants.
- `crates/pantograph-media-conversion/src/README.md`
  - Documented source-level command-plan ownership and producer contract.
- `docs/plans/pantograph-execution-platform/implementation-waves/13-managed-media-conversion-leases/reports/wave-04-worker-media-command-planning.md`
  - This report.

## Verification

- `cargo test -p pantograph-media-conversion` passed.
- `cargo fmt --all -- --check` passed.
- `npm run traceability` passed.

## Deferred Work

- Workflow-service/Tauri wiring remains intentionally deferred.
- Host execution still needs to map plan steps to managed executable leases and
  process invocation.
- Actual converter-specific argv may need final adjustment when the managed
  `oiiotool`, `ocioconvert`, and `ffmpeg` runtime adapters are implemented.
- 3D conversion remains unsupported until a concrete managed 3D converter
  dependency is selected.

## Cross-Boundary Needs

- Host adapters need a lease-acquisition path from `required_dependency_ids` to
  concrete managed executable paths and supporting dependency leases.
- ArtifactStore adapters should continue to provide bytes privately; command
  plans must not grow ArtifactStore paths or host filesystem assumptions.
- Diagnostics/bindings may later project command-plan metadata, but should keep
  argv vectors structured and avoid shell command strings.
