# Wave 05 Host Report: UniFFI Artifact Surface

## Scope

Bring the native UniFFI embedded runtime onto the same ArtifactStore path as the
GUI service for descriptor-backed media outputs and expose the first artifact
access methods through JSON DTOs.

## Files Changed

- `crates/pantograph-embedded-runtime/src/embedded_workflow_service_api.rs`
- `crates/pantograph-uniffi/src/runtime.rs`
- `crates/pantograph-uniffi/src/runtime_tests.rs`
- `docs/plans/pantograph-execution-platform/11-artifact-format-settings-and-managed-media-dependencies.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/coordination-ledger.md`

## Result

- The UniFFI runtime constructor now opens an ArtifactStore under
  `<app_data_dir>/artifacts` and attaches it to the canonical workflow service.
- Embedded runtime service methods now expose descriptor lookup, body read,
  consume acknowledgement, policy read/update, and store stats.
- UniFFI JSON methods expose those same operations to host bindings.
- Focused runtime coverage verifies the policy/stat surface and missing
  descriptor error envelope.

## Deferred Work

- Persistent settings, managed redistributable status/actions, media capability
  DTOs, and final binary-safe body transport semantics still need binding
  parity.

## Verification

- `cargo test -p pantograph-uniffi direct_runtime_exposes_artifact_store_contract_surface`
- `cargo check -p pantograph-uniffi`
- `cargo fmt --all -- --check`
