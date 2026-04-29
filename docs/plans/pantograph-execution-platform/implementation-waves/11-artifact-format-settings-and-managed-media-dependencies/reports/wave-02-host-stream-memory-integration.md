# Wave 02 Host Integration: ArtifactStore Stream, Cache, And Disk Policy

## Scope

Integrated worker output for the ArtifactStore stream and memory-cache slice,
then completed the host-side standards pass before commit.

## Changes

- Split the oversized ArtifactStore implementation into focused helper modules:
  manifest/recovery, memory-cache accounting, and stream lifecycle.
- Preserved the public `ArtifactStore` and `WorkflowService` facade while
  adding stream open, chunk append, finalize, and binary-safe read behavior.
- Added global `max_disk_bytes` enforcement for retained bodies and stream
  growth, keeping the policy contract meaningful for both immediate writes and
  streaming artifacts.
- Added focused disk-budget tests in a separate integration test file so the
  existing ArtifactStore test file remains below the file-size standard.

## Verification

Passed during integration:

- `cargo test -p pantograph-workflow-service --test artifact_store`
- `cargo test -p pantograph-workflow-service --test artifact_store_policy`
- `cargo test -p pantograph-workflow-service --test artifact_contract`
- `cargo fmt --all -- --check`

## Residual Risk

- Execution still needs the later Wave `04` cutover so media-producing nodes
  write descriptors through ArtifactStore before workflow-output JSON
  validation.
- Diagnostics projections still need to reference ArtifactStore descriptors and
  lifecycle metadata rather than payload bodies.
