# 02 Node Contracts And Discovery Waves

## Purpose

Define concurrent waves for Stage `02`, canonical node contracts and discovery.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `coordination-ledger.md` | Host-owned Stage `02` status, decisions, verification, and handoff record. |
| `waves/` | Ordered wave specifications for contract freeze, implementation, and integration. |
| `reports/` | Worker reports required before host integration. |

## Problem

Stage `02` moves node and port semantics out of host-local projections and into
canonical backend contracts. The wave folder keeps node-contract, workflow
service, and workflow-node work aligned around one frozen contract boundary.

## Constraints

- GUI-local catalogs and host binding generation remain out of scope.
- Shared manifests, generated artifacts, facade exports, and ADRs are
  host-owned.
- Contract semantics must be frozen before parallel implementation begins.

## Decision

Use a contract-freeze wave, a bounded implementation wave, and a host
integration wave so all consumers project backend-owned contracts.

## Alternatives Rejected

- Let workflow-service, GUI, and node-engine keep separate contract shapes:
  rejected because it would keep compatibility semantics fragmented.

## Invariants

- Backend contracts own node/port compatibility.
- Frontend and host surfaces consume projections, not independent semantics.
- Worker output is not integrated before its report exists.

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

## Revisit Triggers

- Canonical contracts require GUI or binding semantics to be implemented early.
- Existing graph mutation APIs cannot consume canonical compatibility without a
  public facade redesign.
- Worker write sets overlap on shared graph DTOs or registry conversion files.

## Dependencies

**Internal:** `../../02-node-contracts-and-discovery.md`,
`../../10-concurrent-phased-implementation.md`, and
`coordination-ledger.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../adr/ADR-006-canonical-node-contract-ownership.md`

## Usage Examples

Start Stage `02` by reading `waves/wave-01.md`, then record the frozen contract
surface in `coordination-ledger.md` before assigning `waves/wave-02.md`.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume the wave specs as scope, write-set, verification, and
  reporting contracts.

## Structured Producer Contract

- Wave files use `waves/wave-XX.md` names.
- Worker reports live under `reports/`.
- The coordination ledger records wave status, integration decisions,
  verification, and deviations.
