# Headless Embedding Implementation Notes

## Scope Completed

The headless embedding plan is implemented for v1:

- Contract freeze and service boundary ADR.
- Host-agnostic embedding service (`pantograph-workflow-service`).
- Tauri, UniFFI, and Rustler adapters delegating to shared service contracts.
- Model signature hardening with deterministic hash selection when model metadata is available.
- Contract tests plus CI contract gate.
- Rust host example and migration guide.

## Key Design Outcomes

1. Service logic is centralized in `pantograph-workflow-service`.
2. Adapter layers are transport/runtime wrappers, not business-rule owners.
3. `embed_objects_v1` preserves object order and supports per-object partial failures.
4. `model_signature` is required on successful responses and fails explicitly when deterministic signature requirements cannot be met.
5. Workflow validation enforces existence + logical graph validity, but not business-intent inference.

## Verification Commands

- `cargo test -p pantograph-workflow-service --test contract_v1`
- `cargo check -p pantograph-uniffi`
- `cargo check -p pantograph_rustler`
- `cargo test -p pantograph-uniffi --no-run`
- `cargo test -p pantograph-uniffi test_get_embedding_workflow_capabilities_v1_contract_success -- --nocapture`
- `cargo test -p pantograph-uniffi test_parse_embedding_payload_rejects_non_numeric -- --nocapture`

## Test Environment Notes

- Some adapter runtime tests require local TCP bind and/or NIF runtime link targets.
- In restricted sandbox environments, those tests may be ignored or compile-only validated.
