# Wave 01: Attribution Contract Freeze

## Objective

Complete host-owned preflight before any worker edits Stage `01` source files.

## Workers

No parallel workers. The host owns this wave.

## Required Work

- Run `../../../08-stage-start-implementation-gate.md` for Stage `01`.
- Record git status and overlapping dirty files.
- Freeze attribution IDs, lifecycle enums, command names, errors, and SQLite
  schema outline.
- Inventory public workflow-session entry points to replace, make internal, or
  remove.
- Complete SQLite and credential digest dependency review before manifest edits.

## Write Set

- Stage-start report or Stage `01` implementation notes.
- No source files unless the stage-start gate explicitly records a ready
  outcome.

## Forbidden Files

- Source code, manifests, lockfiles, generated files, and binding artifacts
  before the start outcome is recorded.

## Verification

- Stage-start outcome is `ready` or `ready_with_recorded_assumptions`.
- Wave `02` worker write sets do not overlap.

## Report

Host updates `coordination-ledger.md`.

## Integration Order

This wave must complete before wave `02` starts.
