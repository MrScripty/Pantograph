# crates/pantograph-workflow-service/src

## Purpose
Host-agnostic application service contracts and orchestration entrypoints for Pantograph workflow APIs.

## Boundaries
- No transport framework dependencies (Tauri/UniFFI/Rustler).
- No UI concerns.
- Host/runtime dependencies exposed via traits.

## Contents
- `embedding.rs`: v1 headless embedding contracts, host traits, and orchestration logic.

## Headless Embedding v1

Primary operations:

- `embed_objects_v1`
- `get_embedding_workflow_capabilities_v1`

Primary contract types:

- `EmbedObjectsV1Request`
- `EmbedObjectsV1Response`
- `GetEmbeddingWorkflowCapabilitiesV1Request`
- `GetEmbeddingWorkflowCapabilitiesV1Response`
- `ModelSignature`

## Verification

- Contract tests: `crates/pantograph-workflow-service/tests/contract_v1.rs`
- CI gate: `.github/workflows/headless-embedding-contract.yml`
