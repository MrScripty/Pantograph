# Stage 03 Wave Specs

## Purpose

This directory contains the wave specifications for Stage `03`, managed runtime
observability.

## Contents

| File | Description |
| ---- | ----------- |
| `wave-01.md` | Runtime context contract freeze. |
| `wave-02.md` | Runtime capability, diagnostics adapter, cancellation, progress, and guarantee implementation. |
| `wave-03.md` | Integration, verification, and stage-end gate. |

## Problem

Runtime observability touches execution context, diagnostics, cancellation, and
progress. The wave files keep runtime-owned facts separate from durable ledger
ownership that belongs to later stages.

## Constraints

- Stage `03` implementation must follow `../../../08-stage-start-implementation-gate.md`.
- Node authors must not be required to manually emit baseline diagnostics.
- Durable model/license ledger storage remains out of scope for this stage.

## Decision

Freeze the runtime context first, then split implementation by context,
diagnostics adaptation, and lifecycle guarantees before host integration.

## Alternatives Rejected

- Add observability only inside individual nodes: rejected because diagnostics
  would depend on node-author boilerplate.

## Invariants

- Runtime wrappers own baseline execution observations.
- Background runtime work has tracked lifecycle, cancellation, and shutdown.

## Revisit Triggers

- Diagnostics persistence is needed before Stage `04`.
- Runtime lifecycle changes require shared mutable state across worker write
  sets.

## Dependencies

**Internal:** `../README.md`, `../coordination-ledger.md`,
`../../../03-managed-runtime-observability.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../../adr/ADR-007-managed-runtime-observability-ownership.md`

## Usage Examples

Read `wave-01.md` before changing runtime context contracts; read `wave-02.md`
only after the coordination ledger records contract freeze.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume them as wave-level scope, lifecycle, and verification
  contracts.

## Structured Producer Contract

- Wave specs use `wave-XX.md` names.
- Each wave records objective, worker/write boundaries, verification, report
  expectations, and integration order.
