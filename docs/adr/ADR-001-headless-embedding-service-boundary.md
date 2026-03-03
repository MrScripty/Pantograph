# ADR-001: Headless Workflow Service Boundary

## Status
Accepted

## Context
Pantograph currently executes workflow use-cases primarily in Tauri command modules. This couples application orchestration to one host framework and blocks a stable headless framework API for external Rust consumers.

Current issues:
- Tauri command layer mixes transport, host state wiring, and use-case orchestration.
- Event adapter completion payloads are not a reliable API contract.
- External consumers need object-in/object-out embedding semantics with deterministic metadata.

## Decision
Adopt a three-layer boundary for headless workflow features:

1. Domain/Framework Layer
- `node-engine` and `workflow-nodes`
- No dependency on Tauri, UniFFI, Rustler, or transport details.

2. Application Service Layer
- New host-agnostic service module/crate for workflow run use-cases.
- Owns request/response DTOs and business orchestration semantics.
- Depends on domain/framework and trait-based host resources.

3. Host Adapter Layer
- Tauri commands, UniFFI wrappers, Rustler NIF wrappers.
- Transport-only mapping and dependency injection.
- No duplicated business logic.

## Consequences

### Positive
- Standards-compliant service independence and layering.
- One canonical contract reused across adapters.
- Easier contract testing and versioning.
- Enables Rust-first framework embedding API without desktop coupling.

### Negative
- Initial refactor cost to extract orchestration.
- Temporary compatibility wrappers required during migration.
- More explicit trait interfaces to inject host resources.

## Implementation Notes
- Freeze `workflow_run` and `workflow_get_capabilities` contracts before implementation.
- Migrate Tauri workflow commands to thin delegation wrappers.
- Add contract tests in service layer and parity checks in adapters.

## Compliance Mapping
- Layered separation of concerns: Coding Standards and Architecture Patterns.
- Service independence: framework-agnostic orchestration.
- Immutable contracts: freeze API contract before implementation.

## Implementation Status

Implemented.

Delivered artifacts:

- Service layer contracts and orchestration: `crates/pantograph-workflow-service`
- Shared capability core (workflow validation + runtime requirement computation):
  `crates/pantograph-workflow-service/src/capabilities.rs`
- Tauri thin adapter commands: `src-tauri/src/workflow/headless_workflow_commands.rs`
- UniFFI adapter exports: `crates/pantograph-uniffi/src/lib.rs`
- Rustler adapter NIFs: `crates/pantograph-rustler/src/lib.rs`
- Contract tests: `crates/pantograph-workflow-service/tests/contract.rs`
- CI guardrail: `.github/workflows/headless-embedding-contract.yml`
