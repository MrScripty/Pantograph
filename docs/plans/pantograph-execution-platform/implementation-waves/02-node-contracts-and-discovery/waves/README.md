# Stage 02 Wave Specs

## Purpose

This directory contains the wave specifications for Stage `02`, canonical node
contracts and graph-authoring discovery.

## Contents

| File | Description |
| ---- | ----------- |
| `wave-01.md` | Contract freeze and inventory before implementation. |
| `wave-02.md` | Canonical contract crate, workflow-service projections, and workflow-node registration. |
| `wave-03.md` | Integration, graph compatibility verification, and stage gate. |

## Problem

Node contract ownership crosses node contracts, workflow-service projections,
and workflow-node registrations. The wave files prevent host-local discovery
semantics from diverging from backend-owned contracts.

## Constraints

- Stage `02` implementation must follow `../../../08-stage-start-implementation-gate.md`.
- Shared contract and projection semantics must be frozen before parallel work.
- Generated or host-binding files remain out of scope unless a later stage
  assigns them.

## Decision

Use a freeze wave, a bounded parallel implementation wave, and a host-owned
integration wave so canonical contracts land before downstream runtime and
binding work.

## Alternatives Rejected

- Let each consumer define its own discovery shape: rejected because it would
  recreate host-local contract semantics.

## Invariants

- Backend Rust owns node and port compatibility semantics.
- Frontend and host bindings consume projections only.

## Revisit Triggers

- A worker needs to change public contract fields outside the frozen boundary.
- Graph compatibility cannot be verified with the planned test set.

## Dependencies

**Internal:** `../README.md`, `../coordination-ledger.md`,
`../../../02-node-contracts-and-discovery.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../../adr/ADR-006-canonical-node-contract-ownership.md`

## Usage Examples

Start with `wave-01.md` to freeze the contract surface, then use `wave-02.md`
only after the coordination ledger records the frozen boundary.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume them as scope and verification contracts for each wave.

## Structured Producer Contract

- Wave specs use `wave-XX.md` names.
- Each wave records objective, worker/write boundaries, forbidden files,
  verification, report expectations, and integration order.
