# Wave 03: Contract Integration And Gate

## Objective

Complete Stage `02` integration, documentation alignment, ADR, verification,
and stage-end refactor gate.

## Workers

No parallel workers. The host owns this wave.

## Required Work

- Read worker reports and record deviations.
- Update `node-engine` documentation so it no longer claims canonical GUI or
  binding semantics.
- Add/update ADR for canonical node contract ownership.
- Run final Stage `02` verification.
- Apply `../../../09-stage-end-refactor-gate.md`.

## Verification

```bash
cargo test -p pantograph-node-contracts
cargo test -p workflow-nodes
cargo test -p pantograph-workflow-service
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Report

Host updates `coordination-ledger.md` with verification, integrated branches or
commits, deviations, and stage-end gate outcome.
