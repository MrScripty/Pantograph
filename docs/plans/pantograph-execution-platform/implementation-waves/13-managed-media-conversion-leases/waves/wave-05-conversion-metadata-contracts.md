# Wave 05: Conversion Metadata Contracts

## Purpose

Freeze the shared descriptor and diagnostics metadata fields that later
host-owned conversion execution will populate.

## Scope

- Add typed conversion status to ArtifactStore format metadata and durable I/O
  diagnostics format metadata.
- Add conversion id, conversion command id, and dependency lease attribution
  fields to the same contracts.
- Preserve pass-through behavior by defaulting all conversion fields to empty
  unless real conversion occurs.
- Map workflow-service artifact metadata into diagnostics-ledger metadata
  without stringly typed conversion status.
- Update contract tests and README traceability for the modified contract
  surfaces.

## Write Set

- `crates/pantograph-workflow-service/src/workflow/artifact_contracts.rs`
- `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`
- `crates/pantograph-workflow-service/src/workflow/artifact_output_conversion.rs`
- `crates/pantograph-workflow-service/tests/artifact_contract.rs`
- `crates/pantograph-workflow-service/tests/artifact_store.rs`
- `crates/pantograph-diagnostics-ledger/src/event.rs`
- `crates/pantograph-diagnostics-ledger/src/lib.rs`
- `crates/pantograph-diagnostics-ledger/src/tests.rs`
- pass-through metadata constructors in Tauri and embedded-runtime adapters
- matching README and Stage `13` progress docs

## Non-Goals

- Real process execution.
- Active-version lease acquisition or release.
- Temporary file handling for non-streaming converter tools.
- Frontend rendering of conversion state.

## Verification

- `cargo test -p pantograph-diagnostics-ledger -p pantograph-workflow-service artifact --tests`
- `cargo check -p pantograph-embedded-runtime -p pantograph`
- `cargo fmt --all -- --check`
- `npm run traceability`
