# Wave 05 Worker Embedded Python Stream Artifacts

Date: 2026-04-29

## Scope

Implemented a contained embedded-runtime slice that artifactizes Python media stream chunks when a workflow service handle is present in `ExecutorExtensions`.

## Changes

- Added `runtime_extension_keys::WORKFLOW_SERVICE` for an `Arc<pantograph_workflow_service::WorkflowService>`.
- Added executor-local stream artifactization for `audio_base64` and `image_base64` chunks.
- Preserved text/token streams and media chunks when no workflow service extension is installed.
- Removed inline media bodies from artifactized stream events and emitted bounded artifact metadata:
  - `artifact_id`
  - `stream_handle`
  - `read_handle` after finalization
  - `media_type`
  - `payload_kind`
  - `sequence`
  - `byte_length`
  - `available_byte_length`
  - `byte_range_start`
  - `byte_range_end_exclusive`
  - `lifecycle_state`
  - `is_final`

## Wiring Status

The executor-side implementation is complete behind the new extension key. The default embedded runtime extension snapshot does not yet auto-propagate the embedded runtime's `WorkflowService` into every workflow execution. That broader production wiring was intentionally not changed in this slice because it would require edits outside the requested ownership set.

Required future write set for automatic embedded-runtime wiring:

- `crates/pantograph-embedded-runtime/src/runtime_extensions.rs`
- `crates/pantograph-embedded-runtime/src/embedded_runtime_lifecycle.rs`
- any call sites that construct `RuntimeExtensionsSnapshot` or apply runtime extensions if they need workflow-service propagation semantics

## Tests

Added focused coverage in `crates/pantograph-embedded-runtime/src/task_executor_tests/recorder_stream.rs` for direct Python producer chunks with a configured workflow service extension. Existing inline stream behavior remains covered by the no-service extension test.
