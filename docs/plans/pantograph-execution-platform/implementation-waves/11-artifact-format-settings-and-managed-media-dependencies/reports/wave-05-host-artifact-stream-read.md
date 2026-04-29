# Wave 05 Host Artifact Stream Read

## Scope

Added a binary-safe stream read primitive for in-progress ArtifactStore streams.
This gives GUI/API callers a polling read path for currently available stream
bytes without serializing chunk bodies into diagnostic events, descriptors, or
projection JSON.

## Changed Files

- `crates/pantograph-workflow-service/src/workflow/artifact_contracts.rs`
- `crates/pantograph-workflow-service/src/workflow/artifact_api.rs`
- `crates/pantograph-workflow-service/src/workflow/artifact_store/stream.rs`
- `crates/pantograph-workflow-service/src/lib.rs`
- `crates/pantograph-workflow-service/tests/artifact_contract.rs`
- `crates/pantograph-workflow-service/tests/artifact_store.rs`
- `crates/pantograph-embedded-runtime/src/embedded_workflow_service_api.rs`
- `src-tauri/src/workflow/headless_workflow_commands.rs`
- `src-tauri/src/workflow/commands.rs`
- `src-tauri/src/app_setup.rs`

## Implemented

- Added `ArtifactStreamReadRequest`, `ArtifactStreamReadResponse`, and
  `ArtifactStreamBodyRead`.
- Added `ArtifactStore::read_stream_body` for range reads against the currently
  written stream file while the artifact lifecycle remains `streaming`.
- Added `WorkflowService::read_artifact_stream_body`.
- Exposed the stream read path through the embedded runtime and Tauri
  `workflow_read_artifact_stream` command.

## Verification

- `cargo test -p pantograph-workflow-service --test artifact_contract artifact_access_contracts_are_handle_based`
- `cargo test -p pantograph-workflow-service --test artifact_store artifact_store_streams_chunks_and_finalizes_descriptor_without_serialized_bodies`
- `cargo check -p pantograph`
- `rustfmt --edition 2021 --check` for the changed Rust files in this slice.

## Deferred

This slice does not add a push subscription bus or frontend stream UI. It
establishes the binary-safe read contract needed by those follow-up controls.
