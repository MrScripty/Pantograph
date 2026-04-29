# Wave 05 Worker Artifact Retention Audit

## Scope

- Worker: A
- Slice: backend ArtifactStore retention audit behavior.
- Write set used:
  - `crates/pantograph-workflow-service/tests/artifact_store.rs`
  - `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/reports/wave-05-worker-artifact-retention-audit.md`

## Result

Added focused proof tests for the Stage 11 retention audit requirement:

- `artifact_store_retention_deletion_preserves_audit_descriptor_and_fails_body_reads`
  - Proves consume-triggered physical body deletion removes the body file.
  - Proves descriptor metadata remains queryable after deletion and across store reopen.
  - Proves retained audit fields survive deletion:
    - artifact id
    - payload kind
    - byte length
    - content hash
    - format metadata
    - attribution
  - Proves deleted artifacts clear access modes/read handles and fail binary body reads closed.
- `workflow_service_retention_cleanup_keeps_descriptor_queryable_while_body_is_unavailable`
  - Proves TTL cleanup through `WorkflowService` preserves descriptor queryability.
  - Proves service-level binary body reads fail closed with an invalid-request error after body deletion.

No implementation changes were required. The existing ArtifactStore delete path already preserves descriptor metadata while clearing body access.

## Verification

- `cargo test -p pantograph-workflow-service --test artifact_store artifact_store_retention_deletion_preserves_audit_descriptor_and_fails_body_reads`
- `cargo test -p pantograph-workflow-service --test artifact_store workflow_service_retention_cleanup_keeps_descriptor_queryable_while_body_is_unavailable`
- `cargo test -p pantograph-workflow-service --test artifact_store`
- `cargo fmt --all -- --check`
- `cargo check -p pantograph-workflow-service`

## Remaining Risks

- This slice proves ArtifactStore and WorkflowService facade behavior only. Diagnostics ledger retention projections must still preserve and expose artifact metadata independently when projection rows outlive physical artifact bodies.
- Stream body retention deletion was not changed in this slice. Existing stream tests cover finalized stream readability, but stream-specific retention audit projection behavior remains part of the broader Stage 11 implementation.
