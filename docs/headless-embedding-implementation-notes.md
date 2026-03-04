# Headless Workflow Implementation Notes

## Scope Completed

The headless workflow plan is implemented:

- Contract freeze and service boundary ADR.
- Host-agnostic workflow service (`pantograph-workflow-service`).
- Tauri, UniFFI, and Rustler adapters delegating to shared service contracts.
- Dedicated frontend HTTP adapter crate (`pantograph-frontend-http-adapter`).
- Feature-gated frontend HTTP binding exports (`frontend-http`,
  `frontend-http-legacy`) so default bindings do not imply HTTP/Tauri-based
  headless integration.
- Model signature hardening with deterministic hash selection when model metadata is available.
- Contract tests plus CI contract gate.
- Rust host example and migration guide.

## Key Design Outcomes

1. Service logic is centralized in `pantograph-workflow-service`.
2. Adapter layers are transport/runtime wrappers, not business-rule owners.
3. `workflow_run` preserves object order and supports per-object partial failures.
4. `model_signature` is required on successful responses and fails explicitly when deterministic signature requirements cannot be met.
5. Workflow validation enforces existence + logical graph validity, but not business-intent inference.
6. Headless API usage is explicit: core service first, optional frontend HTTP
   adapter only for modular GUI composition.

## Verification Commands

- `cargo test -p pantograph-workflow-service --test contract`
- `cargo check -p pantograph-frontend-http-adapter`
- `cargo test -p pantograph-frontend-http-adapter`
- `cargo check -p pantograph-uniffi`
- `cargo check -p pantograph-uniffi --features frontend-http`
- `cargo check -p pantograph-uniffi --features frontend-http-legacy`
- `cargo check -p pantograph_rustler`
- `cargo check -p pantograph_rustler --features frontend-http`
- `cargo check -p pantograph_rustler --features frontend-http-legacy`
- `cargo test -p pantograph-uniffi --no-run`
- `cargo test -p pantograph-uniffi test_workflow_get_capabilities_contract_success -- --nocapture`
- `cargo test -p pantograph-uniffi test_parse_embedding_payload_rejects_non_numeric -- --nocapture`

## Test Environment Notes

- Some adapter runtime tests require local TCP bind and/or NIF runtime link targets.
- In restricted sandbox environments, those tests may be ignored or compile-only validated.
