# Stage 02 Worker Reports

## Purpose

This directory stores Stage `02` worker reports for canonical node contract and
discovery work.

## Contents

| File | Description |
| ---- | ----------- |
| `wave-02-worker-canonical-contracts.md` | Canonical contract crate worker findings. |
| `wave-02-worker-workflow-service-projections.md` | Workflow-service projection worker findings. |
| `wave-02-worker-workflow-nodes-registration.md` | Workflow-node registration worker findings. |

## Problem

Node contract work is split across multiple packages. Reports preserve worker
scope, verification, and escalations before host integration.

## Constraints

- Reports must be written before host integration.
- Reports document findings; they do not change frozen contract ownership.

## Decision

Keep worker reports beside the Stage `02` wave specs so integration evidence
stays with the canonical contract plan.

## Alternatives Rejected

- Record worker outcomes only in commit messages: rejected because contract
  ownership changes need durable review evidence.

## Invariants

- Every worker report names its scope, changed files, verification, and
  residual risk.

## Revisit Triggers

- A worker changes shared contract semantics outside its assigned write set.

## Dependencies

**Internal:** `../README.md`, `../coordination-ledger.md`, and
`../waves/`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../../adr/ADR-006-canonical-node-contract-ownership.md`

## Usage Examples

Read each wave `02` report before integrating the matching worker output.

## API Consumer Contract

- These reports are planning artifacts, not runtime APIs.
- Host integration consumes them as evidence for verification and risk review.

## Structured Producer Contract

- Report filenames use `wave-XX-worker-<name>.md`.
- Reports record scope, files changed, verification, skipped checks, issues,
  and handoff notes.
