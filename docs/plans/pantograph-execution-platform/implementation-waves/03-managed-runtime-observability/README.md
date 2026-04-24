# 03 Managed Runtime Observability Waves

## Purpose

Define concurrent waves for Stage `03`, runtime-created node execution context,
managed capabilities, baseline diagnostics, cancellation, progress, and
guarantee classification.

## Stage Objective

Move ordinary node execution onto a runtime-created context without adding
node-authored diagnostics boilerplate or implementing durable ledger storage.

## Waves

| Wave | Purpose |
| ---- | ------- |
| `waves/wave-01.md` | Host-owned runtime context and event contract freeze. |
| `waves/wave-02.md` | Parallel runtime context/capabilities and event adaptation work. |
| `waves/wave-03.md` | Host-owned integration, cancellation/guarantee verification, ADR, and gate. |

## Global Host-Owned Files

- workspace manifests and lockfiles
- public runtime facade exports
- ADR files
- durable ledger implementation files

## Stage Verification

```bash
cargo test -p pantograph-embedded-runtime
cargo test -p node-engine
cargo test -p pantograph-workflow-service
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Re-Plan Triggers

- Managed capability routing requires durable ledger storage in Stage `03`.
- Node-engine must become the owner of durable attribution or compliance
  semantics.
- Cancellation or spawned task ownership cannot be isolated to one lifecycle
  owner.
