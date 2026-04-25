# Wave 02 Worker Report: runtime-ledger-submission

## Scope

- Primary write set: `crates/pantograph-embedded-runtime/` ledger submission
  boundaries.
- Host-owned shared files touched for integration:
  `crates/pantograph-embedded-runtime/Cargo.toml` and public facade exports.
- Forbidden files respected: GUI diagnostics views, host binding projections,
  and node factoring or migration logic were not touched.

## Changes

- Added `node_execution_ledger.rs` as the runtime-owned managed model usage
  submission boundary.
- Added `ManagedModelUsageSubmission`, `SubmittedModelUsageEvent`, and
  `RuntimeLedgerSubmissionError`.
- Added `ModelExecutionCapability::build_usage_event` and
  `ModelExecutionCapability::submit_usage_event` so model usage records are
  built from a runtime-created `NodeExecutionContext` plus managed capability
  route instead of node-authored attribution.
- Mapped Stage `03` node execution guarantee levels into durable ledger
  guarantee levels and downgraded otherwise full guarantees to
  `managed_partial` when output measurement facts are explicitly unavailable.
- Projected runtime-owned attribution, workflow id, node id/type, effective
  contract version/digest, output ports, composed-node lineage, and lineage
  segment metadata into durable `ModelLicenseUsageEvent` records.
- Added focused tests covering persisted submission, unavailable-measurement
  downgrade, context/capability mismatch rejection, and unavailable capability
  rejection.
- Updated the embedded-runtime source README and crate facade exports.

## Verification

- Passed:
  `cargo fmt -p pantograph-embedded-runtime -p pantograph-diagnostics-ledger`.
- Passed: `cargo test -p pantograph-embedded-runtime node_execution_ledger`.
- Passed: `cargo check -p pantograph-embedded-runtime`.
- Passed:
  `cargo clippy -p pantograph-embedded-runtime --all-targets -- -D warnings`.
- Passed: `cargo test -p pantograph-embedded-runtime`.

## Notes

- This slice establishes the runtime submission boundary and testable managed
  capability API. Direct task-executor call-site interception and
  workflow-service query projections remain pending in Stage `04`.
