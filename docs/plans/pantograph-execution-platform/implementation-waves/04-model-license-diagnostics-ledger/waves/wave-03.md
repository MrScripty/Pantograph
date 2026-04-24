# Wave 03: Ledger Integration And Gate

## Objective

Complete Stage `04` integration, retention/recovery verification, ADR, and
stage-end refactor gate.

## Workers

No parallel workers. The host owns this wave.

## Required Work

- Read worker reports and record deviations.
- Add/update ADR for SQLite ledger persistence, license snapshots,
  measurements, and retention.
- Verify GUI and binding files were not implemented in this stage.
- Run final Stage `04` verification.
- Apply `../../../09-stage-end-refactor-gate.md`.

## Verification

```bash
cargo test -p pantograph-diagnostics-ledger
cargo test -p pantograph-embedded-runtime
cargo test -p pantograph-workflow-service
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```
