# 04 Model License Diagnostics Ledger Waves

## Purpose

Define concurrent waves for Stage `04`, durable model/license usage ledger,
retention, pruning, and query projections.

## Stage Objective

Add `pantograph-diagnostics-ledger`, persist time-of-use license snapshots and
typed output measurements, expose bounded query projections, and keep durable
ledger storage separate from transient trace storage.

## Waves

| Wave | Purpose |
| ---- | ------- |
| `waves/wave-01.md` | Host-owned ledger schema, retention, and dependency freeze. |
| `waves/wave-02.md` | Parallel ledger storage, runtime submission, and query projection work. |
| `waves/wave-03.md` | Host-owned integration, retention verification, ADR, and gate. |

## Global Host-Owned Files

- workspace manifests and lockfiles
- public facade exports
- ADR files
- GUI implementation files
- host binding projection files

## Stage Verification

```bash
cargo test -p pantograph-diagnostics-ledger
cargo test -p pantograph-embedded-runtime
cargo test -p pantograph-workflow-service
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Re-Plan Triggers

- SQLite cannot satisfy migration, retention, or indexed query requirements.
- Pumas cannot supply stable time-of-use license facts.
- Runtime integration requires ordinary nodes to hand-author compliance records.
