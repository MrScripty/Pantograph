# Wave 01: Inventory And Upgrade Policy Freeze

## Objective

Inventory existing node and port IDs and freeze the clean upgrade/rejection
policy before factoring work starts.

## Workers

No parallel workers. The host owns this wave.

## Required Work

- Run `../../../08-stage-start-implementation-gate.md` for Stage `05`.
- Confirm Stage `01` through Stage `04` outputs are integrated and gated.
- Inventory existing node type IDs, port IDs, saved workflow fixtures, and
  diagnostics lineage consumers.
- Freeze which coarse nodes are keep, split, or compose.
- Freeze migration output semantics: upgraded, regenerated, or typed rejection.

## Write Set

- Stage-start report, inventory notes, or implementation notes only.

## Verification

- Start outcome is `ready` or `ready_with_recorded_assumptions`.
- Wave `02` worker write sets are non-overlapping.
