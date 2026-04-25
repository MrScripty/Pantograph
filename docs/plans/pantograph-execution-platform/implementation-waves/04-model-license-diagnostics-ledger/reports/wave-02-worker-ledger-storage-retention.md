# Wave 02 Worker Report: ledger-storage-retention

## Scope

- Primary write set: `crates/pantograph-diagnostics-ledger/`.
- Host-owned shared files touched for integration:
  `Cargo.toml`, `Cargo.lock`, and
  `crates/pantograph-runtime-attribution/Cargo.toml`.
- Forbidden files respected: GUI diagnostics views, host binding projections,
  and node factoring or migration logic were not touched.

## Changes

- Added `pantograph-diagnostics-ledger` as the canonical durable
  model/license diagnostics ledger crate.
- Added ledger DTOs for model/license usage events, model identity, license
  snapshots, typed output measurements, usage lineage, execution guarantee
  level, retention policy, bounded diagnostics queries, and prune commands.
- Added `DiagnosticsLedgerRepository` as the narrow storage/query/pruning
  boundary consumed by later runtime and workflow-service slices.
- Implemented `SqliteDiagnosticsLedger` with versioned schema initialization,
  unsupported schema rejection, event insertion, bounded event queries,
  default retention policy lookup, and transactional pruning over complete
  usage events.
- Added SQLite tables for `ledger_schema_migrations`,
  `model_license_usage_events`, `license_snapshots`,
  `model_output_measurements`, `usage_lineage`, and
  `diagnostics_retention_policy`.
- Centralized `rusqlite` in workspace dependencies and switched
  `pantograph-runtime-attribution` to `rusqlite.workspace = true`.
- Added crate README coverage for ownership, persistence, and retention.

## Verification

- Passed: `cargo fmt -p pantograph-diagnostics-ledger -p pantograph-runtime-attribution`.
- Passed: `cargo test -p pantograph-diagnostics-ledger`.
- Passed: `cargo check -p pantograph-diagnostics-ledger`.
- Passed:
  `cargo clippy -p pantograph-diagnostics-ledger --all-targets -- -D warnings`.

## Notes

- Full `cargo fmt --all` attempted during this slice failed because Cargo fmt
  tried to write
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pumas-Library/rust/crates/pumas-core/src/conversion/manager.rs`,
  which is outside this writable workspace and mounted read-only in this
  environment. Package-scoped formatting for touched crates passed.
- Runtime submission and workflow-service query integration remain pending for
  later Wave `02` slices.
