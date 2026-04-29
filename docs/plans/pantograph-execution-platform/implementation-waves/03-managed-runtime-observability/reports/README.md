# Stage 03 Worker Reports

## Purpose

This directory stores Stage `03` worker reports for managed runtime
observability work.

## Contents

| File | Description |
| ---- | ----------- |
| `wave-02-worker-runtime-context-capabilities.md` | Runtime context and managed capability worker findings. |
| `wave-02-worker-diagnostics-event-adapter.md` | Diagnostics adapter worker findings. |
| `wave-02-worker-cancellation-progress-guarantee.md` | Cancellation, progress, and guarantee worker findings. |

## Problem

Runtime observability crosses context, diagnostics, and lifecycle behavior.
Reports preserve worker evidence before host integration.

## Constraints

- Reports must be written before host integration.
- Reports document findings; they do not authorize durable ledger ownership in
  Stage `03`.

## Decision

Keep worker reports beside the Stage `03` wave specs so lifecycle and
diagnostics handoffs remain auditable.

## Alternatives Rejected

- Keep worker findings only in chat: rejected because runtime lifecycle changes
  need durable verification evidence.

## Invariants

- Every worker report names its scope, changed files, verification, and
  residual risk.

## Revisit Triggers

- A worker discovers spawned task or cancellation ownership outside its write
  set.

## Dependencies

**Internal:** `../README.md`, `../coordination-ledger.md`, and
`../waves/`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../../adr/ADR-007-managed-runtime-observability-ownership.md`

## Usage Examples

Read each wave `02` report before integrating runtime context, diagnostics, or
lifecycle worker output.

## API Consumer Contract

- These reports are planning artifacts, not runtime APIs.
- Host integration consumes them as evidence for verification and risk review.

## Structured Producer Contract

- Report filenames use `wave-XX-worker-<name>.md`.
- Reports record scope, files changed, verification, skipped checks, issues,
  and handoff notes.
