# Wave 05 Host Report: Tauri Artifact Commands

## Scope

Expose the backend ArtifactStore facade through the Tauri workflow command
surface so the workbench can look up descriptors, read bodies, acknowledge
consumption, inspect policy, update policy, and view store stats.

## Files Changed

- `crates/pantograph-workflow-service/src/workflow/artifact_store.rs`
- `src-tauri/src/workflow/headless_workflow_commands.rs`
- `src-tauri/src/workflow/commands.rs`
- `src-tauri/src/app_setup.rs`
- `docs/plans/pantograph-execution-platform/11-artifact-format-settings-and-managed-media-dependencies.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/coordination-ledger.md`

## Result

- `ArtifactBodyRead` is serializable for command responses.
- The Tauri workflow API now exposes:
  - `workflow_artifact_descriptor`
  - `workflow_read_artifact_body`
  - `workflow_acknowledge_artifact_consumed`
  - `workflow_artifact_policy`
  - `workflow_update_artifact_policy`
  - `workflow_artifact_store_stats`
- Commands delegate to the canonical backend `WorkflowService` ArtifactStore
  owner and preserve backend error envelopes.

## Deferred Work

- Frontend binary display/download must avoid storing media bodies in JSON view
  state.
- Stream subscription/read APIs and host-binding parity remain open.
- The workbench Settings page still needs to become the persistent policy owner.

## Verification

- `cargo check -p pantograph`
- `cargo test -p pantograph-workflow-service --test artifact_store`
- `cargo fmt --all -- --check`
