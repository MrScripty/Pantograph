# Wave 04: Binding Integration And Gate

## Objective

Complete Stage `06` artifact/version integration, support-tier docs, ADR,
verification, and stage-end refactor gate.

## Workers

No parallel workers. The host owns this wave.

## Required Work

- Read worker reports and record deviations.
- Verify C#, Python, and BEAM support tiers match actual language-native tests.
- Confirm generated bindings and native artifacts are version-matched.
- Add/update ADR for binding projection architecture and supported host tiers.
- Run final Stage `06` verification plus every supported host-lane command.
- Apply `../../../09-stage-end-refactor-gate.md`.

## Verification

```bash
cargo test -p pantograph-uniffi
cargo test -p pantograph-rustler
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

Host also runs the C#, Python, and BEAM commands recorded in wave `01` for
every supported lane.
