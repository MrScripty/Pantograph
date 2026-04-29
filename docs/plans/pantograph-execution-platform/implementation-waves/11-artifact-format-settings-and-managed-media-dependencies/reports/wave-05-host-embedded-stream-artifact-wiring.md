# Wave 05 Host Embedded Stream Artifact Wiring

## Scope

Wired the embedded runtime's `WorkflowService` into normal executor extension
snapshots so direct Python media stream artifactization is active in production
execution paths.

## Changes

- `RuntimeExtensionsSnapshot` now carries an optional `WorkflowService`
  reference.
- Workflow, session, edit-session, and data-graph executor setup now build
  runtime snapshots with the embedded runtime's canonical workflow service.
- Runtime extension application installs the service under
  `runtime_extension_keys::WORKFLOW_SERVICE`, enabling executor-side stream
  artifactization by default.
- Added a focused regression test proving the workflow service extension is
  applied to a node-engine executor.

## Verification

```bash
cargo test -p pantograph-embedded-runtime runtime_extensions_apply_workflow_service_for_stream_artifacts
cargo test -p pantograph-embedded-runtime recorder_stream
cargo check -p pantograph-embedded-runtime
rustfmt --edition 2021 --check crates/pantograph-embedded-runtime/src/runtime_extensions.rs crates/pantograph-embedded-runtime/src/lib.rs crates/pantograph-embedded-runtime/src/embedded_runtime_lifecycle.rs crates/pantograph-embedded-runtime/src/embedded_data_graph_execution.rs crates/pantograph-embedded-runtime/src/embedded_edit_session_execution.rs crates/pantograph-embedded-runtime/src/embedded_workflow_host.rs crates/pantograph-embedded-runtime/src/workflow_execution_session_execution.rs crates/pantograph-embedded-runtime/src/lib_tests.rs
```
