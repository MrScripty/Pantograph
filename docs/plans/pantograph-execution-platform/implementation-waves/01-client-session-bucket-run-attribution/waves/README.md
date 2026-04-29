# Stage 01 Wave Specs

## Purpose

This directory contains the wave specifications for Stage `01`, durable
client/session/bucket/run attribution.

## Contents

| File | Description |
| ---- | ----------- |
| `wave-01.md` | Contract freeze, dependency review, and API cutover inventory. |
| `wave-02.md` | Attribution domain/storage and workflow-service integration work. |
| `wave-03.md` | Integration, verification, ADR updates, and stage-end gate. |

## Problem

Attribution touches credential storage, workflow-service entry points, and run
identity. The wave files keep contract freeze, implementation, and integration
ordered so diagnostics and scheduler work can depend on stable attribution
facts.

## Constraints

- Stage `01` implementation must follow `../../../08-stage-start-implementation-gate.md`.
- Public workflow-session compatibility shims are not preserved.
- Shared manifests, generated artifacts, and ADR files remain host-owned.

## Decision

Keep Stage `01` split into three waves: freeze contracts, implement the
domain/service cutover, then integrate and verify before later stages consume
the attribution APIs.

## Alternatives Rejected

- One combined wave: rejected because attribution storage and workflow-service
  cutover need separate write sets before integration.

## Invariants

- Workflow execution starts only after validated attribution context exists.
- Worker write sets must not overlap shared host-owned files.

## Revisit Triggers

- Credential persistence or workflow-service cutover requires binding edits in
  the same wave.
- Dirty files overlap the Stage `01` write sets.

## Dependencies

**Internal:** `../README.md`, `../coordination-ledger.md`,
`../../../01-client-session-bucket-run-attribution.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../../adr/ADR-005-durable-runtime-attribution.md`

## Usage Examples

Read `wave-01.md`, then update `../coordination-ledger.md` before starting
implementation work for the stage.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume them as wave contracts for scope, write sets,
  verification, and report expectations.

## Structured Producer Contract

- Wave specs use `wave-XX.md` names.
- Each wave records objective, workers or required work, write boundaries,
  verification, report path, and integration order.
