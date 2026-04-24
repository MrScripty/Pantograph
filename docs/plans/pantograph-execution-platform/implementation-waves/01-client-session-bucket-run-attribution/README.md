# 01 Client Session Bucket Run Attribution Waves

## Purpose

Define safe concurrent waves for Stage `01`, durable client/session/bucket/run
attribution.

## Stage Objective

Add `pantograph-runtime-attribution`, replace affected workflow-session public
entry points with durable client-session APIs, and require workflow-run
attribution before execution starts.

## Non-Goals

- No GUI implementation.
- No model/license diagnostics ledger.
- No node contract registry implementation.
- No backward-compatible workflow-session public wrappers.

## Waves

| Wave | Purpose |
| ---- | ------- |
| `waves/wave-01.md` | Host-owned contract freeze, dependency review, and API cutover inventory. |
| `waves/wave-02.md` | Parallel implementation of attribution domain/storage and workflow-service integration. |
| `waves/wave-03.md` | Host-owned integration, public API cutover verification, docs, ADR, and stage-end gate. |

## Global Host-Owned Files

- `Cargo.toml`
- `Cargo.lock`
- generated binding artifacts
- public facade exports that cross worker boundaries
- ADR files

## Stage Verification

```bash
cargo test -p pantograph-runtime-attribution
cargo test -p pantograph-workflow-service
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Re-Plan Triggers

- SQLite or credential digest dependency review fails.
- Workflow-session public API cutover cannot be completed without host binding
  edits in the same wave.
- Existing dirty files overlap attribution or workflow-service write sets.
