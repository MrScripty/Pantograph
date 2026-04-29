# Stage 04 Worker Reports

## Purpose

This directory stores Stage `04` worker reports for model/license diagnostics
ledger work.

## Contents

| File | Description |
| ---- | ----------- |
| `wave-02-worker-ledger-storage-retention.md` | Ledger storage and retention worker findings. |
| `wave-02-worker-runtime-ledger-submission.md` | Runtime ledger submission worker findings. |
| `wave-02-worker-workflow-service-query-projections.md` | Workflow-service query projection worker findings. |

## Problem

Ledger work crosses persistence, runtime submission, and query projections.
Reports preserve verification and risk evidence before host integration.

## Constraints

- Reports must be written before host integration.
- Reports document findings; they do not create independent ledger write
  models.

## Decision

Keep worker reports beside the Stage `04` wave specs so durable ledger
integration remains traceable.

## Alternatives Rejected

- Record worker outcomes only through test output: rejected because skipped
  checks and projection risks need durable handoff notes.

## Invariants

- Every worker report names its scope, changed files, verification, and
  residual risk.

## Revisit Triggers

- A worker needs a separate diagnostics write model outside the typed ledger
  boundary.

## Dependencies

**Internal:** `../README.md`, `../coordination-ledger.md`, and
`../waves/`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../../adr/ADR-008-durable-model-license-diagnostics-ledger.md`

## Usage Examples

Read each wave `02` report before integrating ledger storage, runtime
submission, or query projection output.

## API Consumer Contract

- These reports are planning artifacts, not runtime APIs.
- Host integration consumes them as evidence for verification and risk review.

## Structured Producer Contract

- Report filenames use `wave-XX-worker-<name>.md`.
- Reports record scope, files changed, verification, skipped checks, issues,
  and handoff notes.
