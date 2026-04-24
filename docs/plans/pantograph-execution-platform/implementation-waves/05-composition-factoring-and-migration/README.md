# 05 Composition Factoring And Migration Waves

## Purpose

Define concurrent waves for Stage `05`, composed-node semantics, concrete node
factoring, primitive lineage, and clean saved-workflow upgrade.

## Stage Objective

Improve graph authoring through primitive/composed nodes while preserving
primitive diagnostics facts and upgrading, regenerating, or rejecting old
workflow artifacts without indefinite compatibility shims.

## Waves

| Wave | Purpose |
| ---- | ------- |
| `waves/wave-01.md` | Host-owned inventory and upgrade policy freeze. |
| `waves/wave-02.md` | Parallel composition contracts, workflow-node factoring, and runtime lineage work. |
| `waves/wave-03.md` | Host-owned migration integration, release notes, ADR, and gate. |

## Global Host-Owned Files

- workspace manifests and lockfiles
- public facade exports
- saved workflow fixtures shared across workers
- ADR and release note files

## Stage Verification

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

## Re-Plan Triggers

- Old graph artifacts cannot be cleanly upgraded, regenerated, or rejected.
- Temporary migration projections would need to remain as supported public
  semantics.
- Composed-node lineage cannot preserve primitive model/license attribution.
