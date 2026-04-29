# Stage 06 Wave Specs

## Purpose

This directory contains the wave specifications for Stage `06`, binding
projections and host-language verification.

## Contents

| File | Description |
| ---- | ----------- |
| `wave-01.md` | Native base API and support-tier freeze. |
| `wave-02.md` | UniFFI and Rustler projection implementation. |
| `wave-03.md` | C#, Python, and BEAM host-language verification. |
| `wave-04.md` | Binding integration, status reconciliation, and stage-end gate. |

## Problem

Bindings project native Rust contracts into host languages. The wave files keep
native API freeze, generated/projection work, and host smoke verification
separate so support tiers are based on tested artifacts.

## Constraints

- Stage `06` implementation must follow `../../../08-stage-start-implementation-gate.md`.
- Generated binding artifacts are not hand-edited.
- Unsupported host lanes must remain explicitly marked rather than implied by
  partial fixtures.

## Decision

Freeze native API/support tiers first, implement binding projections, run host
verification, then reconcile status through an integration gate.

## Alternatives Rejected

- Treat generated host bindings as source-owned hand edits: rejected because it
  breaks reproducibility and artifact version matching.

## Invariants

- Native Rust remains the canonical API.
- Host bindings are projections over backend-owned contracts.
- Support claims require real artifact-loading verification or an explicit
  unsupported/degraded status.

## Revisit Triggers

- UniFFI, Rustler, or host fixture request shapes drift from native Rust.
- A supported lane cannot load the generated/native artifact.

## Dependencies

**Internal:** `../README.md`, `../coordination-ledger.md`,
`../../../06-binding-projections-and-verification.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../../adr/ADR-010-binding-projection-ownership-and-support-tiers.md`

## Usage Examples

Do not start `wave-02.md` until `wave-01.md` records the frozen native API and
support-tier expectations.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume them as wave contracts for binding ownership, generated
  artifact policy, host verification, and integration order.

## Structured Producer Contract

- Wave specs use `wave-XX.md` names.
- Each wave records objective, worker/write boundaries, forbidden files,
  verification, report expectations, and integration order.
