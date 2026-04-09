# ADR-001: Headless Workflow Service Boundary

## Status
Accepted

## Context
Pantograph currently executes workflow use-cases primarily in Tauri command modules. This couples application orchestration to one host framework and blocks a stable headless framework API for external Rust consumers.

Current issues:
- Tauri command layer mixes transport, host state wiring, and use-case orchestration.
- Event adapter completion payloads are not a reliable API contract.
- External consumers need generic workflow input/output semantics with deterministic run contracts.

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

Graph editing is part of the core application-service boundary, not a Tauri-only
desktop concern. The core service crate owns graph document CRUD, edit-session
lifecycle, canonical mutation validation, revision-aware connection intent, and
undo/redo semantics. Host adapters may expose those operations over transport,
but they must not implement or fork graph-edit business logic.

## Consequences

### Positive
- Standards-compliant service independence and layering.
- One canonical contract reused across adapters.
- Easier contract testing and versioning.
- Enables Rust-first framework embedding API without desktop coupling.

### Negative
- Initial refactor cost to extract orchestration.
- More explicit trait interfaces to inject host resources.
- Frontend HTTP workflow exports in bindings now require explicit opt-in
  features to avoid accidental misuse as the primary headless API.

## Implementation Notes
- Freeze `workflow_run` and `workflow_get_capabilities` contracts before implementation.
- Freeze graph-edit contracts before moving editing code out of Tauri.
- Extend capabilities with model inventory (`models[]`) derived from graph usage.
- Keep workflow sessions scheduler-managed in service layer (`create/run/close`).
- Keep graph edit sessions distinct from scheduler-managed run sessions.
- Migrate Tauri workflow commands to thin delegation wrappers.
- Isolate frontend HTTP host behavior in a dedicated adapter crate and keep it
  out of default binding surfaces.
- Add contract tests in service layer and parity checks in adapters.

## Compliance Mapping
- Layered separation of concerns: Coding Standards and Architecture Patterns.
- Service independence: framework-agnostic orchestration.
- Immutable contracts: freeze API contract before implementation.

## Implementation Status

Partially implemented.

The service-boundary extraction described in this ADR is implemented, but the
canonical backend-owned embedded runtime is not yet fully extracted from the
optional GUI layer. Direct execution support still exists in Tauri-owned
modules, which means Pantograph is not yet exposing a backend-owned native
runtime facade for UniFFI/C# embedding.

Follow-up plan:

- `docs/embedded-runtime-extraction-plan.md`

Delivered artifacts:

- Service layer contracts and orchestration: `crates/pantograph-workflow-service`
- Frontend HTTP host adapter crate:
  `crates/pantograph-frontend-http-adapter`
- Shared capability core (workflow validation + runtime requirement computation):
  `crates/pantograph-workflow-service/src/capabilities.rs`
- Session lifecycle and scheduler admission in service layer:
  `crates/pantograph-workflow-service/src/workflow.rs`
- Tauri thin adapter commands: `src-tauri/src/workflow/headless_workflow_commands.rs`
- UniFFI adapter exports: `crates/pantograph-uniffi/src/lib.rs`
- Rustler adapter NIFs: `crates/pantograph-rustler/src/lib.rs`
- UniFFI/Rustler default mode excludes frontend HTTP workflow exports; optional
  feature flag gates frontend HTTP (`frontend-http`).
- Contract tests: `crates/pantograph-workflow-service/tests/contract.rs`
- CI guardrail: `.github/workflows/headless-embedding-contract.yml`

Remaining work to satisfy the full architectural intent:

- Extract direct workflow host/runtime wiring from `src-tauri` into a backend
  crate owned by Pantograph.
- Make Tauri consume that backend runtime instead of owning direct execution
  logic.
- Add a direct UniFFI runtime facade so native clients use Pantograph directly
  instead of going through the optional frontend HTTP adapter.
