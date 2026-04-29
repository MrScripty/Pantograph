# Wave 02 Report: ArtifactStore Streaming and Memory Cache

## Scope

Owned write set:

- `crates/pantograph-workflow-service/src/workflow/artifact_store.rs`
- `crates/pantograph-workflow-service/src/workflow/artifact_api.rs`
- `crates/pantograph-workflow-service/src/workflow.rs`
- `crates/pantograph-workflow-service/src/lib.rs`
- `crates/pantograph-workflow-service/tests/artifact_store.rs`

## Changes

- Added ArtifactStore stream lifecycle support:
  - open stream descriptors with `artifact-stream://` handles
  - append ordered stream chunks
  - return descriptor-only `ArtifactStreamChunkRecord` metadata
  - finalize stream bodies into retained read/download artifacts
- Kept stream chunk bodies out of serialized DTOs and manifest metadata.
- Added private stream body files under the existing backend-owned bodies
  directory, with no public raw path exposure.
- Added bounded in-memory body cache accounting:
  - caches only retained bodies at or below `spill_threshold_bytes`
  - only caches when `max_memory_bytes` has room
  - rebuilds cache accounting on reopen/policy update without exceeding budget
  - evicts cache entries on consume/delete and retention cleanup
- Extended ArtifactStore stats with memory-cache and streaming byte counters.
- Added WorkflowService facade methods and public re-exports for stream open,
  chunk append, and finalize requests.

## Tests

Added focused coverage for:

- stream chunk append/finalize and descriptor-only serialized chunk metadata
- memory cache accounting and max-memory budget enforcement
- cleanup/read behavior after cached bodies are deleted
- WorkflowService stream facade integration

## Verification

Passed:

- `cargo test -p pantograph-workflow-service --test artifact_store`
- `cargo fmt --all -- --check`

## Residual Risk

- Execution output cutover to write media through ArtifactStore remains out of
  scope for this worker slice.
- Diagnostics projections still need to link finalized artifact descriptors in
  the later execution/diagnostics wave.
