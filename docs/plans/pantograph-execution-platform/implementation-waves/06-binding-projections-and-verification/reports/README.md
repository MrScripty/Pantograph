# Stage 06 Worker Reports

## Purpose

This directory stores Stage `06` worker reports for binding projection and
host-language verification work.

## Contents

| File | Description |
| ---- | ----------- |
| `wave-02-worker-uniffi-projections.md` | UniFFI projection worker findings. |
| `wave-02-worker-rustler-projections.md` | Rustler projection worker findings. |
| `wave-03-worker-csharp-host-tests.md` | C# host verification findings. |
| `wave-03-worker-python-host-tests.md` | Python host verification findings. |
| `wave-03-worker-beam-host-tests.md` | BEAM host verification findings. |
| `stage-end-refactor-gate.md` | Stage-end refactor gate result. |

## Problem

Binding projection work crosses generated artifacts and host-language smoke
tests. Reports preserve support-tier evidence and blocked-lane status before
host integration.

## Constraints

- Reports must be written before host integration.
- Reports document findings; they do not turn unsupported host lanes into
  supported lanes.

## Decision

Keep worker reports beside the Stage `06` wave specs so generated artifact and
host verification evidence remains auditable.

## Alternatives Rejected

- Treat host smoke status as implied by source compilation: rejected because
  support tiers require actual artifact-loading evidence or explicit degraded
  status.

## Invariants

- Every worker report names its scope, changed files, verification, and
  residual risk.
- Supported host lanes require real native/generated artifact verification.

## Revisit Triggers

- A host lane request shape drifts from the native Rust contract.

## Dependencies

**Internal:** `../README.md`, `../coordination-ledger.md`, and
`../waves/`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../../adr/ADR-010-binding-projection-ownership-and-support-tiers.md`

## Usage Examples

Read each wave `02` or wave `03` report before integrating projection or host
verification output.

## API Consumer Contract

- These reports are planning artifacts, not runtime APIs.
- Host integration consumes them as evidence for verification and risk review.

## Structured Producer Contract

- Report filenames use `wave-XX-worker-<name>.md` or a gate-specific name.
- Reports record scope, files changed, verification, skipped checks, issues,
  and handoff notes.
