# Wave 03: Attribution Integration And Gate

## Objective

Finish Stage `01` integration, verification, documentation, ADR, and
stage-end refactor gate.

## Workers

No parallel workers. The host owns this wave.

## Required Work

- Integrate worker reports and record deviations.
- Add/update ADR for durable attribution ownership and SQLite persistence.
- Verify old workflow-session public entry points were removed, replaced, or
  made internal.
- Run final Stage `01` verification.
- Apply `../../../09-stage-end-refactor-gate.md`.

## Verification

```bash
cargo test -p pantograph-runtime-attribution
cargo test -p pantograph-workflow-service
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Report

Host updates `coordination-ledger.md` with final verification, integrated
branches or commits, deviations, and stage-end gate outcome.
