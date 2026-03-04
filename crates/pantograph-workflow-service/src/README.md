# crates/pantograph-workflow-service/src

## Purpose
Host-agnostic application service contracts and orchestration entrypoints for Pantograph workflow APIs.

## Boundaries
- No transport framework dependencies (Tauri/UniFFI/Rustler).
- No UI concerns.
- Host/runtime dependencies exposed via traits.

## Contents
- `workflow.rs`: headless workflow contracts, host traits, and orchestration logic.
- `capabilities.rs`: shared workflow capability/validation utilities used by all adapters.

## Headless Workflow API

Primary operations:

- `workflow_run`
- `workflow_get_capabilities`
- `create_workflow_session`
- `run_workflow_session`
- `close_workflow_session`

Primary contract types:

- `WorkflowRunRequest`
- `WorkflowRunResponse`
- `WorkflowCapabilitiesRequest`
- `WorkflowCapabilitiesResponse`
- `RuntimeSignature`
- `WorkflowSessionCreateRequest`
- `WorkflowSessionRunRequest`
- `WorkflowSessionCloseRequest`

## Capability Ownership

- Runtime requirement extraction/estimation is backend-owned in this crate.
- Adapters should provide host dependencies (workflow roots, backend identity,
  optional model metadata), not duplicate capability business logic.
- `workflow_get_capabilities` includes `models[]` inventory with `model_id`,
  optional `model_revision_or_hash`, optional `model_type`, `node_ids`, and
  `roles`.

## Verification

- Contract tests: `crates/pantograph-workflow-service/tests/contract.rs`
- CI gate: `.github/workflows/headless-embedding-contract.yml`
