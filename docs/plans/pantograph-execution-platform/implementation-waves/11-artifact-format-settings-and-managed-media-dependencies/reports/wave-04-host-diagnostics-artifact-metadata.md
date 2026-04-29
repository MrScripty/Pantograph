# Wave 04 Host Report: Diagnostics Artifact Metadata

## Scope

Link ArtifactStore descriptors into the typed diagnostics ledger projection
without storing media bodies or raw descriptor JSON in the ledger.

## Files Changed

- `crates/pantograph-diagnostics-ledger/src/event.rs`
- `crates/pantograph-diagnostics-ledger/src/lib.rs`
- `crates/pantograph-diagnostics-ledger/src/schema.rs`
- `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`
- `crates/pantograph-diagnostics-ledger/src/tests.rs`
- `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`
- `crates/pantograph-workflow-service/src/workflow/tests/diagnostics.rs`
- `crates/pantograph-workflow-service/tests/contract.rs`

## Result

- `IoArtifactObserved` events now carry validated descriptor metadata fields:
  payload kind, lifecycle state, access modes, read/stream handles, and format
  metadata.
- `io_artifact_projection` stores those fields in durable typed projection
  columns with schema migration support and projection version `5`.
- Workflow-service I/O artifact diagnostics detect ArtifactStore descriptors,
  use the real artifact id and handle references, and classify retained
  descriptor outputs as payload references instead of synthetic metadata-only
  JSON values.
- Non-descriptor workflow values remain metadata-only with size/hash summaries.

## Deferred Work

- Python stream events still need producer-boundary conversion so chunk bodies
  never enter live event JSON.
- Frontend/binding artifact read/download/consume APIs remain assigned to Wave
  `05`.

## Verification

- `cargo test -p pantograph-diagnostics-ledger io_artifact`
- `cargo test -p pantograph-workflow-service workflow_io_artifact_query`
- `cargo test -p pantograph-workflow-service artifact_output_conversion`
- `cargo fmt --all -- --check`
