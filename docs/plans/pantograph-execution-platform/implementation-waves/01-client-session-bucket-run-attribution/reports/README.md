# Stage 01 Worker Reports

## Purpose

This directory stores Stage `01` worker reports for durable attribution work.

## Contents

| File | Description |
| ---- | ----------- |
| `wave-02-worker-attribution-domain-storage.md` | Attribution domain and storage worker findings. |
| `wave-02-worker-workflow-service-cutover.md` | Workflow-service cutover worker findings. |

## Problem

Host integration needs a durable record of what each worker changed, verified,
skipped, or escalated before Stage `01` changes are merged.

## Constraints

- Reports must be written before host integration.
- Reports document findings; they do not authorize edits outside wave write
  sets.

## Decision

Keep worker reports beside the Stage `01` wave specs so coordination and
integration evidence stays with the plan.

## Alternatives Rejected

- Keep worker findings only in chat or commit messages: rejected because Stage
  `01` integration needs durable, reviewable handoff records.

## Invariants

- Every worker report names its scope, changed files, verification, and
  residual risk.

## Revisit Triggers

- A worker changes files outside its assigned write set.

## Dependencies

**Internal:** `../README.md`, `../coordination-ledger.md`, and
`../waves/`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../../adr/ADR-005-durable-runtime-attribution.md`

## Usage Examples

Read the relevant worker report before integrating that worker's branch.

## API Consumer Contract

- These reports are planning artifacts, not runtime APIs.
- Host integration consumes them as evidence for verification and risk review.

## Structured Producer Contract

- Report filenames use `wave-XX-worker-<name>.md`.
- Reports record scope, files changed, verification, skipped checks, issues,
  and handoff notes.
