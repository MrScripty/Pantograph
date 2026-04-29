# Wave 06 Worker A Report: Workflow-Service Conversion Hook

## Status

Complete.

## Files Changed

- `crates/pantograph-workflow-service/Cargo.toml`
- `crates/pantograph-workflow-service/src/workflow.rs`
- `crates/pantograph-workflow-service/src/workflow/service_config.rs`
- `crates/pantograph-workflow-service/src/workflow/artifact_output_conversion.rs`
- `crates/pantograph-workflow-service/src/workflow/workflow_run_api.rs`
- `crates/pantograph-workflow-service/src/workflow/README.md`
- `crates/pantograph-workflow-service/src/README.md`
- `crates/pantograph-workflow-service/tests/README.md`
- `Cargo.lock`
- `docs/plans/pantograph-execution-platform/implementation-waves/13-managed-media-conversion-leases/reports/wave-06-worker-workflow-service-conversion-hook.md`

## Implementation Notes

- Added an optional injected
  `Arc<dyn pantograph_media_conversion::MediaConversionExecutor>` to
  `WorkflowService`, with builder and setter methods.
- Made workflow output artifactization async only at the media conversion
  boundary and updated workflow run execution to await it.
- Preserved pass-through artifactization when no override media-type mismatch
  exists.
- When an override requests a different media type, workflow-service now builds
  a host-neutral `MediaConversionRequest` from the resolved output format,
  binding ids, run id, source media type, and source bytes.
- Without an injected executor, override mismatches fail closed with a service
  capability error.
- On executor success, converted bytes are written to the artifact and
  descriptor format metadata records conversion id/status/command identity and
  dependency attribution from the result. Workflow-service does not acquire
  leases or spawn host processes.
- Added a fake-executor unit test covering invocation and descriptor
  conversion attribution.
- Integration review moved conversion command identity and lease holder
  attribution into the neutral conversion result contract so workflow-service
  records host-supplied lease facts instead of deriving them locally.
- Integration review preserved pass-through stream chunk ordering. Converted
  streams become a single converted output chunk, while non-converted streams
  keep their original chunk sequence.

## Verification

- Passed: `cargo test -p pantograph-workflow-service artifact_output_conversion`
  - 22 artifact output conversion tests passed.
- Passed after integration review: `cargo test -p pantograph-media-conversion`
- Pending for commit gate: `cargo fmt --all -- --check`
- Pending for commit gate: `TRACEABILITY_STAGED_ONLY=1 npm run traceability`

## Blockers

- Host adapter implementation remains pending. It needs to acquire/release
  managed dependency plans, resolve managed executable paths, run command-plan
  steps, and inject the neutral executor at app startup.
