# Wave 01: Ledger Schema And Retention Freeze

## Objective

Freeze ledger storage, retention, pruning, and dependency decisions before
parallel implementation.

## Workers

No parallel workers. The host owns this wave.

## Required Work

- Run `../../../08-stage-start-implementation-gate.md` for Stage `04`.
- Confirm Stage `01` through Stage `03` outputs are integrated and gated.
- Freeze SQLite schema, migration strategy, retention defaults, pruning command
  semantics, query bounds, and unavailable-measurement enums.
- Complete SQLite dependency/linking/audit/release review.

## Write Set

- Stage-start report or implementation notes only.

## Verification

- Start outcome is `ready` or `ready_with_recorded_assumptions`.
- Wave `02` worker write sets are non-overlapping.
