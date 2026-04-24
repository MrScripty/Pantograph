# Wave 02 Worker Report: workflow-service-cutover

## Status

Partial.

## Write Set

- `Cargo.lock`
- `crates/pantograph-embedded-runtime/src/embedded_workflow_service_api.rs`
- `crates/pantograph-runtime-attribution/Cargo.toml`
- `crates/pantograph-runtime-attribution/src/records.rs`
- `crates/pantograph-runtime-attribution/src/tests.rs`
- `crates/pantograph-uniffi/src/frontend_http.rs`
- `crates/pantograph-uniffi/src/lib_tests.rs`
- `crates/pantograph-uniffi/src/runtime.rs`
- `crates/pantograph-uniffi/src/runtime_tests.rs`
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
- Exposed workflow-service native Rust client bucket create/delete methods and
  re-exported the bucket request/record types needed by native callers.
- Added a targeted attributed-run test for explicit backend-owned bucket
  selection.
- Added JSON boundary support for attribution requests/responses, including
  bounded credential-secret parsing and explicit/default bucket-selection
  serialization.
- Added embedded-runtime facade methods and UniFFI embedded/frontend-HTTP JSON
  methods for client registration, durable client-session open/resume, client
  bucket create/delete, and attributed workflow runs.
- Configured UniFFI-owned workflow services with ephemeral attribution stores
  so the new JSON boundary methods can execute without caller-managed service
  wiring.
- Added UniFFI tests covering direct embedded-runtime and frontend-HTTP
  attributed workflow runs through JSON contracts.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p pantograph-runtime-attribution`
- `cargo test -p pantograph-workflow-service attribution`
- `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`
- `cargo test -p pantograph-workflow-service`
- `cargo check --workspace --all-features`
- `cargo test -p pantograph-workflow-service workflow_run`
- `cargo test -p pantograph-uniffi --features frontend-http`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

All commands passed.

## Deviations

- This slice does not yet remove or internalize legacy workflow-session public
  APIs. It introduces the durable-attribution workflow-run path first so the
  service has a tested native Rust target before adapter and binding cutover.
- The host extended the Wave `02` report with boundary-projection edits because
  the durable attribution request types needed JSON-safe credential parsing
  before binding façades could call the service. Legacy public workflow-session
  removal remains a separate follow-up rather than being hidden behind
  compatibility wrappers.

## Follow-Ups

- Replace, remove, or make internal the existing public workflow-session entry
  points in workflow-service, UniFFI, and Rustler.
- Decide whether non-attributed `WorkflowService::new` remains a temporary
  compatibility path or should require configured attribution before Stage `01`
  completion.
