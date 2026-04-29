# Stage 05 Wave Specs

## Purpose

This directory contains the wave specifications for Stage `05`, composed node
contracts, node factoring, runtime lineage, and workflow migration.

## Contents

| File | Description |
| ---- | ----------- |
| `wave-01.md` | Inventory and upgrade policy freeze. |
| `wave-02.md` | Composition contracts, workflow-node factoring, and runtime lineage. |
| `wave-03.md` | Migration integration, verification, and stage-end gate. |

## Problem

Composition and migration affect saved workflows, node contracts, runtime
lineage, and diagnostics continuity. The wave files make upgrade/rejection
policy explicit before source changes touch persisted workflow artifacts.

## Constraints

- Stage `05` implementation must follow `../../../08-stage-start-implementation-gate.md`.
- Legacy workflow artifacts are upgraded or rejected cleanly; indefinite
  compatibility shims are not preserved.
- Primitive trace facts must remain available for diagnostics.

## Decision

Freeze upgrade policy first, implement composition/factoring/lineage in bounded
write sets, then integrate migration behavior through a host-owned gate.

## Alternatives Rejected

- Preserve all old workflow shapes indefinitely: rejected because it would keep
  stale compatibility logic in the execution path.

## Invariants

- Composed nodes preserve primitive execution traceability.
- Saved workflow migration either produces a valid current artifact or a typed
  rejection.

## Revisit Triggers

- Existing workflow artifacts cannot be upgraded or rejected with typed errors.
- Runtime lineage requires contract changes outside the assigned wave.

## Dependencies

**Internal:** `../README.md`, `../coordination-ledger.md`,
`../../../05-composition-factoring-and-migration.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../../adr/ADR-009-composed-node-contracts-and-migration.md`

## Usage Examples

Use `wave-01.md` to document artifact upgrade policy before assigning
composition or factoring workers from `wave-02.md`.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume them as wave contracts for migration policy, write
  boundaries, and verification.

## Structured Producer Contract

- Wave specs use `wave-XX.md` names.
- Each wave records objective, worker/write boundaries, forbidden files,
  verification, report expectations, and integration order.
