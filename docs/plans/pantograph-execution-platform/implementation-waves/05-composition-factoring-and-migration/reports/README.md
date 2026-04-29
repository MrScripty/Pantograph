# Stage 05 Worker Reports

## Purpose

This directory stores Stage `05` worker reports for composition, factoring, and
migration work.

## Contents

| File | Description |
| ---- | ----------- |
| `wave-02-worker-composition-contracts.md` | Composition contract worker findings. |
| `wave-02-worker-workflow-nodes-factoring.md` | Workflow-node factoring worker findings. |
| `wave-02-worker-runtime-lineage.md` | Runtime lineage worker findings. |

## Problem

Stage `05` affects contracts, runtime lineage, and saved workflow migration.
Reports preserve each worker's evidence before host integration.

## Constraints

- Reports must be written before host integration.
- Reports document findings; they do not authorize indefinite legacy workflow
  compatibility.

## Decision

Keep worker reports beside the Stage `05` wave specs so migration and lineage
handoffs remain auditable.

## Alternatives Rejected

- Keep worker findings only in branch history: rejected because migration
  decisions need explicit durable evidence.

## Invariants

- Every worker report names its scope, changed files, verification, and
  residual risk.

## Revisit Triggers

- A worker finds a saved workflow artifact that cannot be upgraded or rejected
  with typed errors.

## Dependencies

**Internal:** `../README.md`, `../coordination-ledger.md`, and
`../waves/`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../../adr/ADR-009-composed-node-contracts-and-migration.md`

## Usage Examples

Read each wave `02` report before integrating composition, factoring, or
lineage output.

## API Consumer Contract

- These reports are planning artifacts, not runtime APIs.
- Host integration consumes them as evidence for verification and risk review.

## Structured Producer Contract

- Report filenames use `wave-XX-worker-<name>.md`.
- Reports record scope, files changed, verification, skipped checks, issues,
  and handoff notes.
