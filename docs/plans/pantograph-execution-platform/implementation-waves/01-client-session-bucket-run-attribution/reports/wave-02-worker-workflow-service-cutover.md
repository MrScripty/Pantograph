# Wave 02 Worker Report: workflow-service-cutover

## Status

Partial.

## Write Set

- `Cargo.lock`
- `crates/pantograph-workflow-service/Cargo.toml`
- `crates/pantograph-workflow-service/src/lib.rs`
- `crates/pantograph-workflow-service/src/workflow.rs`
- `crates/pantograph-workflow-service/src/workflow/attribution_api.rs`
- `crates/pantograph-workflow-service/src/workflow/contracts.rs`
- `crates/pantograph-workflow-service/src/workflow/service_config.rs`
- `crates/pantograph-workflow-service/src/workflow/tests.rs`
- `crates/pantograph-workflow-service/src/workflow/tests/attribution.rs`
- `docs/plans/pantograph-execution-platform/01-client-session-bucket-run-attribution.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/01-client-session-bucket-run-attribution/coordination-ledger.md`

## Implemented

- Added a direct workflow-service dependency on
  `pantograph-runtime-attribution`.
- Added optional attribution-store configuration to `WorkflowService`.
- Added native Rust service methods for attribution client registration,
  durable client-session open/resume, and attributed workflow execution.
- Added attributed workflow-run contracts that reject caller-supplied `run_id`
  and use the backend-generated `WorkflowRunId` from the durable attribution
  store before host execution starts.
- Added targeted tests for backend-owned run-id propagation and caller-run-id
  rejection.
- Hardened the public generic `WorkflowService::workflow_run` boundary so
  caller-supplied `run_id` values are rejected instead of accepted as trusted
  attribution. The internal execution helper still receives backend-generated
  ids from the service-owned and attributed execution paths.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p pantograph-workflow-service attribution`
- `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`
- `cargo test -p pantograph-workflow-service`
- `cargo check --workspace --all-features`
- `cargo test -p pantograph-workflow-service workflow_run`

All commands passed.

## Deviations

- This slice does not yet remove or internalize legacy workflow-session public
  APIs. It introduces the durable-attribution workflow-run path first so the
  service has a tested native Rust target before adapter and binding cutover.

## Follow-Ups

- Replace, remove, or make internal the existing public workflow-session entry
  points in workflow-service, UniFFI, and Rustler.
- Decide whether non-attributed `WorkflowService::new` remains a temporary
  compatibility path or should require configured attribution before Stage `01`
  completion.
