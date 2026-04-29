# Wave 02 Report: ArtifactStore Core

## Scope

Owned write set:

- `crates/pantograph-workflow-service/src/workflow/artifact_store.rs`
- `crates/pantograph-workflow-service/src/workflow/artifact_api.rs`
- workflow-service facade wiring and public re-exports
- `crates/pantograph-workflow-service/tests/artifact_store.rs`
- workflow-service README/test README updates

## Changes

- Added `ArtifactStore` as the backend owner for physical artifact bodies.
- Persisted bodies under private backend-owned paths and exposed only
  descriptors/read handles through public metadata.
- Added manifest persistence and restart reconciliation that preserves metadata
  when a body is missing.
- Added id validation, single-artifact size enforcement, byte-range reads,
  consume acknowledgement, delete-on-consume behavior, TTL cleanup, and store
  stats.
- Added `WorkflowService` facade methods for write, descriptor lookup, binary
  body read, consume acknowledgement, policy update, cleanup, and stats.

## Verification

Passed:

- `cargo test -p pantograph-workflow-service --test artifact_store`
- `cargo test -p pantograph-workflow-service --test artifact_contract`
- `cargo test -p pantograph-workflow-service --test contract`
- `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`
- `cargo fmt --all -- --check`

## Residual Risk

- Memory-cache policy fields are present in DTOs but not enforced yet.
- Stream body persistence and finalize lifecycle transitions are not
  implemented yet.
- Execution output conversion still needs to write media bodies into the store
  before workflow value-size validation.
- Diagnostics events/projections still need to reference these descriptors and
  preserve queryable metadata after body deletion.
