# Wave 02 Report: ArtifactStore Stream and Memory Slice

## Scope

Owned write set:

- `crates/pantograph-workflow-service/src/workflow/artifact_store.rs`
- `crates/pantograph-workflow-service/tests/artifact_store.rs`
- `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/reports/wave-02-worker-artifact-store-stream-memory.md`

## Changes

- Added in-memory body cache accounting bounded by `ArtifactPolicy.max_memory_bytes`
  and `spill_threshold_bytes`; disk body files remain the durable source.
- Added stream lifecycle persistence: open stream descriptors, ordered chunk
  appends to backend-owned stream files, finalize to retained body files, and
  finalized-body hash/length/read-handle transitions.
- Kept stream chunk bodies out of manifest metadata and tightened read behavior
  so unfinished/non-retained artifacts return `BodyUnavailable`.
- Added focused tests for cache budget/spill behavior, stream append/finalize,
  finalized reads, unfinished-stream read rejection, cleanup cache eviction, and
  JSON DTO safety for descriptors/read metadata.

## Verification

Passed:

- `cargo test -p pantograph-workflow-service --test artifact_store`
- `cargo test -p pantograph-workflow-service --test artifact_contract`
- `cargo fmt --all -- --check`

## Residual Risk

- Host integration split stream handling, manifest persistence/reconciliation,
  and memory-cache policy into separate modules before commit. The oversized
  file-size risk from the worker draft is resolved in
  `wave-02-host-stream-memory-integration.md`.
- The memory cache is opportunistic and bounded, not an eviction cache; when the
  budget is full, additional otherwise-cacheable bodies spill to disk-only until
  policy update/reopen/cleanup changes the cache set.
