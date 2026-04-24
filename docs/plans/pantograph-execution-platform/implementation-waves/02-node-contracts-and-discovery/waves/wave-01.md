# Wave 01: Contract Freeze And Inventory

## Objective

Freeze canonical node-contract shape and inventory current contract ownership
before workers edit source files.

## Workers

No parallel workers. The host owns this wave.

## Required Work

- Run `../../../08-stage-start-implementation-gate.md` for Stage `02`.
- Freeze `NodeTypeId`, `NodeInstanceId`, `PortId`, port value types,
  compatibility diagnostics, and effective-contract DTOs.
- Inventory `node-engine` metadata, workflow-service graph DTOs, effective
  definition code, and Rustler graph validation as read-only context.
- Decide which public facades are host-owned integration points.

## Write Set

- Stage-start report or implementation notes only.

## Forbidden Files

- Source files, manifests, lockfiles, and generated artifacts before start
  outcome is recorded.

## Verification

- Stage-start outcome is `ready` or `ready_with_recorded_assumptions`.
- Wave `02` worker write sets are non-overlapping.

## Report

Host updates `coordination-ledger.md`.
