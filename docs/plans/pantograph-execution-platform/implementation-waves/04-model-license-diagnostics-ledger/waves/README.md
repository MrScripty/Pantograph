# Stage 04 Wave Specs

## Purpose

This directory contains the wave specifications for Stage `04`, durable
model/license diagnostics ledger work.

## Contents

| File | Description |
| ---- | ----------- |
| `wave-01.md` | Ledger schema and retention freeze. |
| `wave-02.md` | Ledger storage/retention, runtime submission, and workflow-service query projections. |
| `wave-03.md` | Integration, verification, and stage-end gate. |

## Problem

Diagnostics ledger work spans persistence, runtime event submission, retention,
and workflow-service projections. The wave files keep the ledger as one typed
write boundary while allowing bounded implementation slices.

## Constraints

- Stage `04` implementation must follow `../../../08-stage-start-implementation-gate.md`.
- Model/license facts must stay typed and validated.
- Retention cleanup must not delete the only audit evidence that an event
  happened.

## Decision

Freeze schema and retention policy before implementation, then integrate
storage, runtime submission, and query projections through one ledger boundary.

## Alternatives Rejected

- Add one independent table per diagnostic concern: rejected because it would
  recreate fragmented write models instead of typed projections.

## Invariants

- Durable model/license usage records remain queryable by run, model, license,
  client/session/bucket, and retention state.
- Projection tables are read models, not independent sources of audit truth.

## Revisit Triggers

- Ledger storage engine, retention policy, or projection ownership changes.
- Runtime submission cannot report explicit degraded-guarantee states.

## Dependencies

**Internal:** `../README.md`, `../coordination-ledger.md`,
`../../../04-model-license-diagnostics-ledger.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../../adr/ADR-008-durable-model-license-diagnostics-ledger.md`

## Usage Examples

Use `wave-01.md` to freeze schema/retention decisions before assigning
`wave-02.md` workers.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume them as wave contracts for ledger ownership,
  projection boundaries, and verification.

## Structured Producer Contract

- Wave specs use `wave-XX.md` names.
- Each wave records objective, worker/write boundaries, forbidden files,
  verification, report expectations, and integration order.
