# Wave 06 Host Report: Artifact Metadata Retention Proof

## Scope

Added focused regression coverage for the Stage `13` requirement that
conversion metadata remain queryable after retention or consume policy removes
the physical artifact body.

## Assigned Write Set

- `crates/pantograph-workflow-service/src/workflow/artifact_store.rs`
- Stage `13` plan, coordination ledger, and report index updates

## Changes

- Added an ArtifactStore unit test that writes a converted image artifact with
  conversion id, status, command id, and dependency lease attribution.
- The test consumes the artifact with `delete_on_consume` enabled, then asserts
  the descriptor is still queryable, the body/read handle is gone, and all
  conversion metadata remains on the descriptor.

## Verification

Passed:

```bash
cargo test -p pantograph-workflow-service descriptor_keeps_conversion_metadata_after_delete_on_consume_removes_body -- --nocapture
```

## Follow-Up

This proof covers ArtifactStore descriptor retention. Diagnostics projection
retention behavior is covered by existing ledger/workflow retention tests and
should be extended if conversion-specific projection filters are added later.
