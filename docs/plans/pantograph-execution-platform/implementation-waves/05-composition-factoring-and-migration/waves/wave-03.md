# Wave 03: Migration Integration And Gate

## Objective

Complete Stage `05` saved-workflow upgrade integration, documentation, ADR,
verification, and stage-end refactor gate.

## Workers

No parallel workers. The host owns this wave.

## Required Work

- Read worker reports and record deviations.
- Integrate saved-workflow upgrade or rejection fixtures.
- Add release notes for removed or renamed node/port public contracts.
- Add/update ADR for composed-node trace preservation and upgrade strategy.
- Run final Stage `05` verification.
- Apply `../../../09-stage-end-refactor-gate.md`.

## Verification

```bash
cargo test -p pantograph-node-contracts
cargo test -p workflow-nodes
cargo test -p pantograph-embedded-runtime
cargo test -p pantograph-workflow-service
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```
