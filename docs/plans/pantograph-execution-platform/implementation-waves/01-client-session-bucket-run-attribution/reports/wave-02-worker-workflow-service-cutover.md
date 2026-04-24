# Wave 02 Worker Report: workflow-service-cutover

## Status

Integrated.

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
- `crates/pantograph-rustler/src/frontend_http_nifs.rs`
- `crates/pantograph-rustler/src/lib.rs`
- `crates/pantograph-rustler/src/workflow_host_contract.rs`
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
- Removed legacy workflow-session public wrappers from UniFFI embedded-runtime
  and frontend-HTTP bindings.
- Removed legacy workflow-session frontend-HTTP NIFs from Rustler and replaced
  them with durable attribution NIFs for client registration, durable
  client-session open/resume, bucket create/delete, and attributed workflow
  execution.
- Configured Rustler's frontend-HTTP workflow service with an ephemeral
  attribution store and removed the now-unused scheduler-request NIF helper.
- Renamed the remaining workflow-service scheduler contracts, embedded-runtime
  execution-session paths, Tauri diagnostics projections, and node-engine
  residency/checkpoint helpers from workflow-session terminology to
  execution-session terminology.
- Renamed serialized scheduler, graph-state, and workflow-run option fields
  from `workflow_session_*` to `workflow_execution_session_*`.
- Renamed private node-engine and embedded-runtime module paths that still
  carried legacy workflow-session terminology.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p pantograph-runtime-attribution`
- `cargo test -p pantograph-workflow-service attribution`
- `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`
- `cargo test -p pantograph-workflow-service`
- `cargo check --workspace --all-features`
- `cargo test -p pantograph-workflow-service workflow_run`
- `cargo test -p pantograph-uniffi --features frontend-http`
- `cargo check -p pantograph_rustler --features frontend-http`
- `cargo test -p pantograph-embedded-runtime workflow_runtime`
- `cargo test -p node-engine workflow_execution_session`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `rg -n "WorkflowSession|create_workflow_session|run_workflow_session|close_workflow_session|workflow_get_session_status|workflow_get_session_inspection|workflow_list_session_queue|workflow_cancel_session_queue_item|workflow_reprioritize_session_queue_item|workflow_set_session_keep_alive|workflow_cleanup_stale_sessions|spawn_workflow_session_stale_cleanup_worker" crates src-tauri -g '*.rs'`
- `rg -n "workflow_session|workflow-session|workflow session|Workflow session" crates src-tauri -g '*.rs'`

All cargo commands passed. Both source vocabulary checks returned no matches.

## Deviations

- This slice does not yet remove or internalize legacy workflow-session public
  APIs. It introduces the durable-attribution workflow-run path first so the
  service has a tested native Rust target before adapter and binding cutover.
- The host extended the Wave `02` report with boundary-projection edits because
  the durable attribution request types needed JSON-safe credential parsing
  before binding façades could call the service. Legacy public workflow-session
  removal remains a separate follow-up rather than being hidden behind
  compatibility wrappers.
- `cargo test -p pantograph_rustler --features frontend-http` was attempted
  and failed during test binary linking on unresolved Erlang NIF symbols such
  as `enif_send`. This is an existing Rustler test-link environment limitation;
  `cargo check -p pantograph_rustler --features frontend-http` verifies the
  Rustler cutover compile surface.
- The old workflow-session terminology cutover also renamed JSON field names
  for scheduler inspection and graph-state projections. This is intentional for
  Stage `01`; compatibility aliases were not kept because the stage requires
  removing residual workflow-session public vocabulary.

## Follow-Ups

- Apply the Stage `01` stage-end refactor gate and decide whether the renamed
  execution-session scheduler controls remain an internal native runtime
  management surface or need further restriction before Stage `02`.
- Decide whether non-attributed `WorkflowService::new` remains a temporary
  compatibility path or should require configured attribution before Stage `01`
  completion.
