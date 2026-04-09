# Headless Workflow Implementation Notes

## Scope Completed

The headless workflow plan is implemented with generic workflow I/O:

- Contract freeze and service boundary ADR.
- Host-agnostic workflow service (`pantograph-workflow-service`).
- Tauri, UniFFI, and Rustler adapters delegating to shared service contracts.
- Dedicated frontend HTTP adapter crate (`pantograph-frontend-http-adapter`).
- Feature-gated frontend HTTP binding exports (`frontend-http`) so default
  bindings do not imply HTTP/Tauri-based headless integration.
- Contract tests plus CI contract gate.
- Rust host example and migration guide.

## Pending Extraction

The backend/service boundary is in place, but the backend-owned embedded
runtime is still pending extraction. Direct workflow execution used by the
optional GUI currently lives in `src-tauri`, so UniFFI does not yet expose a
canonical native runtime facade for C# embedding.

Tracked plan:

- `docs/embedded-runtime-extraction-plan.md`

## Key Design Outcomes

1. Service logic is centralized in `pantograph-workflow-service`.
2. Adapter layers are transport/runtime wrappers, not business-rule owners.
3. `workflow_run` now operates on generic `inputs[]` and `outputs[]` bindings.
4. Workflow execution semantics are output-node/target based, not embedding-type based.
5. Workflow validation enforces existence + logical graph validity.
6. Headless API usage is explicit: core service first, optional frontend HTTP adapter only for modular GUI composition.

## Verification Commands

- `cargo test -p pantograph-workflow-service --test contract`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check -p pantograph-frontend-http-adapter`
- `cargo test -p pantograph-frontend-http-adapter`
- `cargo check -p pantograph-uniffi --no-default-features`
- `cargo check -p pantograph-uniffi --features frontend-http`
- `cargo check -p pantograph_rustler --no-default-features`
- `cargo check -p pantograph_rustler --features frontend-http`

## Test Environment Notes

- Some adapter runtime tests require local TCP bind and/or NIF runtime link targets.
- In restricted sandbox environments, those tests may be ignored or compile-only validated.
