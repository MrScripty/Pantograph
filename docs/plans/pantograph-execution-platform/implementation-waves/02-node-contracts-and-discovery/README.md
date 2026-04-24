# 02 Node Contracts And Discovery Waves

## Purpose

Define concurrent waves for Stage `02`, canonical node contracts and discovery.

## Stage Objective

Add `pantograph-node-contracts`, move compatibility and effective-contract
semantics out of workflow-service/node-engine ownership, and expose backend
discovery projections.

## Non-Goals

- No diagnostics ledger.
- No host binding generation.
- No GUI-local node catalogs.

## Waves

| Wave | Purpose |
| ---- | ------- |
| `waves/wave-01.md` | Host-owned contract freeze and node-engine/workflow-service inventory. |
| `waves/wave-02.md` | Parallel implementation of canonical contracts and workflow-service projection integration. |
| `waves/wave-03.md` | Host-owned integration, documentation alignment, ADR, and stage-end gate. |

## Global Host-Owned Files

- `Cargo.toml`
- `Cargo.lock`
- root crate exports and public facades
- generated artifacts
- ADR files

## Stage Verification

```bash
cargo test -p pantograph-node-contracts
cargo test -p workflow-nodes
cargo check --workspace --all-features
cargo test -p pantograph-workflow-service
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Re-Plan Triggers

- Canonical contracts require GUI or binding semantics to be implemented early.
- Existing graph mutation APIs cannot consume canonical compatibility without a
  public facade redesign.
- Worker write sets overlap on shared graph DTOs or registry conversion files.
