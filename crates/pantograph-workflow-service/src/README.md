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

Primary contract types:

- `WorkflowRunRequest`
- `WorkflowRunResponse`
- `WorkflowCapabilitiesRequest`
- `WorkflowCapabilitiesResponse`
- `RuntimeSignature`

## Capability Ownership

- Runtime requirement extraction/estimation is backend-owned in this crate.
- Adapters should provide host dependencies (workflow roots, backend identity,
  optional model metadata), not duplicate capability business logic.

## Verification

- Contract tests: `crates/pantograph-workflow-service/tests/contract.rs`
- CI gate: `.github/workflows/headless-embedding-contract.yml`
