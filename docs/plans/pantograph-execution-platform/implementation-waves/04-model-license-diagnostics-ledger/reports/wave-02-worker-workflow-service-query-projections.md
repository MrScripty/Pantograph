# Wave 02 Worker Report: workflow-service-query-projections

## Scope

- Primary write set: `crates/pantograph-workflow-service/` diagnostics query
  use cases.
- Host-owned shared files touched for integration:
  `Cargo.lock` and `crates/pantograph-workflow-service/Cargo.toml`.
- Forbidden files respected: GUI diagnostics views, host binding projections,
  and node factoring or migration logic were not touched.

## Changes

- Added an optional diagnostics ledger store to `WorkflowService`, matching the
  existing backend-owned optional attribution store pattern.
- Added `with_diagnostics_ledger`,
  `with_ephemeral_diagnostics_ledger`, and an internal diagnostics ledger
  guard.
- Added workflow diagnostics usage query DTOs:
  `WorkflowDiagnosticsUsageQueryRequest`,
  `WorkflowDiagnosticsUsageQueryResponse`, and
  `WorkflowDiagnosticsUsageSummary`.
- Added `workflow_diagnostics_usage_query`, which validates string filter ids,
  delegates durable usage event queries to `pantograph-diagnostics-ledger`,
  includes retention metadata, and returns grouped summaries by model, license,
  and guarantee level.
- Added `WorkflowServiceError` mapping for diagnostics ledger errors so invalid
  query bounds and malformed ids are caller-visible invalid requests while
  storage/schema failures remain internal errors.
- Updated workflow-service README coverage and public facade exports.

## Verification

- Passed:
  `cargo fmt -p pantograph-workflow-service -p pantograph-diagnostics-ledger -- --check`.
- Passed:
  `cargo test -p pantograph-workflow-service workflow_diagnostics_usage_query`.
- Passed: `cargo check -p pantograph-workflow-service`.
- Passed:
  `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`.
- Passed: `cargo test -p pantograph-workflow-service`.

## Notes

- This slice exposes backend-owned workflow-service query projections but does
  not implement GUI diagnostics views or host binding projections.
- The workflow service delegates storage, retention, and pruning semantics to
  `pantograph-diagnostics-ledger`.
