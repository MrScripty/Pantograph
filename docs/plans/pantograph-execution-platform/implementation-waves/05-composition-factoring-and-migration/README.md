# 05 Composition Factoring And Migration Waves

## Purpose

Define concurrent waves for Stage `05`, composed-node semantics, concrete node
factoring, primitive lineage, and clean saved-workflow upgrade.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `coordination-ledger.md` | Host-owned Stage `05` status, decisions, verification, and handoff record. |
| `waves/` | Ordered wave specifications for inventory freeze, implementation, and integration. |
| `reports/` | Worker reports required before host integration. |

## Problem

Stage `05` changes composed-node semantics, workflow-node factoring, runtime
lineage, and saved workflow migration. The wave folder makes upgrade policy
explicit before persisted workflow artifacts are touched.

## Constraints

- Old graph artifacts must be upgraded, regenerated, or rejected cleanly.
- Primitive diagnostics and model/license attribution must remain traceable.
- Shared manifests, public facades, fixtures, ADRs, and release notes are
  host-owned.

## Decision

Freeze inventory and upgrade policy first, then split composition contracts,
workflow-node factoring, and runtime lineage before host-owned migration
integration.

## Alternatives Rejected

- Preserve all old workflow shapes indefinitely: rejected because it would keep
  stale compatibility logic in the execution path.

## Invariants

- Composed nodes preserve primitive runtime facts.
- Saved workflow migration produces valid current artifacts or typed
  rejections.
- Worker output is not integrated before its report exists.

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

## Revisit Triggers

- Old graph artifacts cannot be cleanly upgraded, regenerated, or rejected.
- Temporary migration projections would need to remain as supported public
  semantics.
- Composed-node lineage cannot preserve primitive model/license attribution.

## Dependencies

**Internal:** `../../05-composition-factoring-and-migration.md`,
`../../10-concurrent-phased-implementation.md`, and
`coordination-ledger.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../adr/ADR-009-composed-node-contracts-and-migration.md`

## Usage Examples

Start Stage `05` by reading `waves/wave-01.md`, then record upgrade policy in
`coordination-ledger.md` before assigning implementation workers.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume the wave specs as scope, write-set, verification, and
  reporting contracts.

## Structured Producer Contract

- Wave files use `waves/wave-XX.md` names.
- Worker reports live under `reports/`.
- The coordination ledger records wave status, integration decisions,
  verification, and deviations.
