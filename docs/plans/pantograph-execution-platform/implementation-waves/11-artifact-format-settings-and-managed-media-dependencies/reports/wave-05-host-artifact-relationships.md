# Wave 05 Host Artifact Relationships

## Scope

Added first-class ArtifactStore relationship metadata for preview/revision
artifacts and wired existing stream artifactization paths to preserve it.

## Changes

- `ArtifactDescriptor` now carries optional `artifact_role`,
  `parent_artifact_id`, and `revision_index` fields.
- Artifact write and stream-open requests accept the same relationship metadata,
  validate parent artifact ids, and preserve relationships through stream
  finalization.
- Workflow output artifact conversion marks final workflow outputs with
  `artifact_role: "workflow_output"`.
- Embedded Python stream artifactization and the Tauri event adapter preserve
  bounded `artifact_role`, `preview_role`, `parent_artifact_id`, and
  `revision_index` metadata when replacing inline media stream bodies with
  ArtifactStore references.
- Focused tests cover descriptor contract shape, ArtifactStore child/revision
  preservation, embedded stream metadata propagation, and Tauri stream metadata
  propagation.

## Verification

```bash
cargo test -p pantograph-workflow-service --test artifact_store
cargo test -p pantograph-workflow-service --test artifact_contract
cargo test -p pantograph-embedded-runtime recorder_stream
cargo test -p pantograph event_adapter
cargo fmt --all -- --check
```
