# Wave 02 Worker Report: composition-contracts

## Status

Complete.

## Scope

- Primary write set: `crates/pantograph-node-contracts/`.
- Report path:
  `docs/plans/pantograph-execution-platform/implementation-waves/05-composition-factoring-and-migration/reports/wave-02-worker-composition-contracts.md`.
- Shared files touched: none.
- Forbidden files touched: none.

## Changes

- Added canonical composed-node contract DTOs:
  `ComposedNodeContract`, `ComposedInternalGraph`,
  `ComposedInternalNode`, `ComposedInternalEdge`,
  `ComposedPortMappings`, `ComposedPortMapping`, and
  `ComposedTracePolicy`.
- Added validation for composed external port mappings into internal primitive
  graph nodes without changing primitive `NodeTypeContract` serialization.
- Added contract-upgrade DTOs:
  `ContractUpgradeRecord`, `ContractUpgradeOutcome`,
  `DiagnosticsLineagePolicy`, `ContractUpgradeChange`,
  `ContractUpgradeDiagnostic`, and `ContractUpgradeRejectionReason`.
- Added typed validation errors for unknown internal composition nodes,
  unknown or missing external port mappings, missing upgrade changes, and
  typed rejections without diagnostics.
- Updated `pantograph-node-contracts` README coverage for composed-node
  mappings and migration records.

## Verification

- `cargo fmt -p pantograph-node-contracts -- --check`
- `cargo test -p pantograph-node-contracts`
- `cargo check -p pantograph-node-contracts`
- `cargo clippy -p pantograph-node-contracts --all-targets -- -D warnings`

## Notes

- This slice intentionally keeps composition data as standalone DTOs instead
  of adding fields to `NodeTypeContract`. Primitive contract serialization
  remains stable for existing workflow-service and binding consumers.
- Follow-up slices can project concrete `node-group` and `tool-loop`
  contracts into these DTOs without duplicating composition semantics outside
  `pantograph-node-contracts`.
