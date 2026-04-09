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

Implemented for the Rust backend boundary and UniFFI-native JSON contract
facade.

The service-boundary extraction described in this ADR is implemented, and the
direct runtime host is now owned by `pantograph-embedded-runtime` instead of
`src-tauri`. The UniFFI wrapper now exposes a native embedded runtime object
for workflow/session methods. Generated C# packaging/smoke coverage and
full-path image workflow acceptance remain follow-up verification items.

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
- Backend-owned embedded runtime: `crates/pantograph-embedded-runtime/src/lib.rs`
- UniFFI adapter exports:
  - `crates/pantograph-uniffi/src/runtime.rs`
  - `crates/pantograph-uniffi/src/lib.rs`
- Rustler adapter NIFs: `crates/pantograph-rustler/src/lib.rs`
- UniFFI/Rustler default mode excludes frontend HTTP workflow exports; optional
  feature flag gates frontend HTTP (`frontend-http`).
- Contract tests: `crates/pantograph-workflow-service/tests/contract.rs`
- CI guardrail: `.github/workflows/headless-embedding-contract.yml`

Remaining work to satisfy packaging/acceptance closure:

- Generate and compile C# bindings for the direct `FfiPantographRuntime`
  facade.
- Add one full-path image-generation acceptance check through the backend-owned
  embedded runtime and UniFFI-native configuration path.
