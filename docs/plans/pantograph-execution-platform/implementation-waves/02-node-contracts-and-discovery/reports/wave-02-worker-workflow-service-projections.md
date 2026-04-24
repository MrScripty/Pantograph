# Wave 02 Worker Report: workflow-service-projections

## Status

Complete.

## Write Set

- `Cargo.lock`
- `crates/pantograph-workflow-service/Cargo.toml`
- `crates/pantograph-workflow-service/src/graph/registry.rs`
- `crates/pantograph-workflow-service/src/graph/types.rs`
- `crates/pantograph-workflow-service/src/graph/validation.rs`
- `docs/plans/pantograph-execution-platform/02-node-contracts-and-discovery.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/02-node-contracts-and-discovery/coordination-ledger.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/02-node-contracts-and-discovery/reports/wave-02-worker-workflow-service-projections.md`

## Implemented

- Added `pantograph-node-contracts` as a direct workflow-service dependency.
- Replaced direct workflow-service conversion from `node_engine::TaskMetadata`
  with projection from canonical `NodeTypeContract` records returned by
  `workflow_nodes::builtin_node_contracts`.
- Preserved canonical value types in the workflow-service graph DTO projection
  by adding explicit `PortDataType` variants for model handles, embedding
  handles, database handles, vectors, tensors, and audio samples.
- Routed workflow-service port compatibility through
  `pantograph_node_contracts::PortValueType` so graph validation no longer
  owns separate compatibility semantics.
- Kept the existing workflow-service DTO boundary stable for current callers
  while making it a projection over canonical contracts.
- Added a follow-up effective-contract resolution pass that stores canonical
  `NodeTypeContract` records in `NodeRegistry`, resolves per-node dynamic port
  overlays as `EffectiveNodeContract` values, and only then projects effective
  ports into existing workflow-service DTOs.
- Added canonical merge semantics that preserve unrelated static ports while
  allowing dynamic overlays to add or override ports by stable port id.
- Added direct compatibility diagnostic projection so incompatible direct
  connection rejections expose canonical source/target node ids, port ids,
  value types, rejection reason, and message under `contract_diagnostic`.
- Added workflow-service contract validation for binding graph JSON and routed
  Rustler/UniFFI workflow validation through that backend-owned projection
  instead of direct `node_engine::validation` policy.

## Verification

- `cargo test -p pantograph-workflow-service`
- `cargo check --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`
- `cargo test -p pantograph-node-contracts`
- `cargo clippy -p pantograph-node-contracts --all-targets -- -D warnings`
- `cargo test -p pantograph-workflow-service graph::contract_validation`
- `cargo test -p pantograph-uniffi test_validate_empty_workflow`
- `cargo check -p pantograph_rustler -p pantograph-uniffi`
- `cargo clippy -p pantograph-workflow-service -p pantograph-uniffi -p pantograph_rustler --all-targets -- -D warnings`

All commands passed.

## Deviations

- Connection mutation APIs still return existing workflow-service rejection DTOs
  where applicable. The compatibility decision now comes from canonical
  `PortValueType` rules, and direct incompatible type failures include a
  canonical `contract_diagnostic`.
- Aggregate no-compatible insert/candidate summaries still return coarse
  workflow-service reasons rather than a list of every suppressed compatibility
  diagnostic.
- `cargo test -p pantograph_rustler test_validation_empty_graph` still fails
  during test binary linking on missing Erlang NIF symbols such as
  `enif_release_resource`; Rustler type checking and clippy pass.

## Follow-Ups

- Decide whether aggregate no-compatible insert/candidate summaries should
  expose suppressed compatibility diagnostics or remain coarse graph-authoring
  summaries.
- Update binding-facing graph validation to consume backend-owned contract
  projections instead of direct `node_engine` semantics.
