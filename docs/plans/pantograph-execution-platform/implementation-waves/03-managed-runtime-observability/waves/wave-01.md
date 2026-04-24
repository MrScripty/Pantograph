# Wave 01: Runtime Context Contract Freeze

## Objective

Freeze `NodeExecutionContext`, managed capability, baseline event, cancellation,
progress, lineage, and guarantee contracts before implementation workers start.

## Workers

No parallel workers. The host owns this wave.

## Required Work

- Run `../../../08-stage-start-implementation-gate.md` for Stage `03`.
- Confirm Stage `01` and Stage `02` outputs are integrated and their end gates
  are recorded.
- Decide which node-engine events are adapted and which are replaced at the
  embedded-runtime boundary.
- Confirm durable ledger storage remains forbidden until Stage `04`.

## Write Set

- Stage-start report or implementation notes only.

## Verification

- Start outcome is `ready` or `ready_with_recorded_assumptions`.
- Wave `02` write sets are non-overlapping.
