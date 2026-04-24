# Coordination Ledger: 02 Node Contracts And Discovery

## Status

Wave `01` complete. Wave `02` is ready to begin with the
`canonical-contracts` slice.

## Branch Or Worktree Strategy

- Integration branch: `main`.
- Worker worktrees: subagents are not explicitly authorized in this turn, so
  the host will execute Wave `02` worker slices serially in the shared
  workspace unless authorization changes.

## Wave Status

| Wave | Status | Integration Notes |
| ---- | ------ | ----------------- |
| `wave-01` | Complete | Stage-start report, contract freeze, and current ownership inventory recorded in `02-node-contracts-and-discovery.md`. |
| `wave-02` | Pending | Parallel canonical crate and projection integration. |
| `wave-03` | Pending | Host-owned integration and gate. |

## Worker Reports

| Worker | Report Path | Status |
| ------ | ----------- | ------ |
| canonical-contracts | `reports/wave-02-worker-canonical-contracts.md` | Pending |
| workflow-service-projections | `reports/wave-02-worker-workflow-service-projections.md` | Pending |
| workflow-nodes-registration | `reports/wave-02-worker-workflow-nodes-registration.md` | Pending |

## Decisions

- 2026-04-24: Stage-start outcome is `ready_with_recorded_assumptions`.
- 2026-04-24: Existing dirty files are unrelated `assets/` changes and do not
  overlap the Stage `02` write set.
- 2026-04-24: Without explicit subagent authorization, the host may implement
  Wave `02` slices serially while preserving the recorded worker boundaries.
- 2026-04-24: First implementation slice is `canonical-contracts`:
  `crates/pantograph-node-contracts/`, workspace wiring, README, and targeted
  contract tests. Workflow-service projection integration follows only after
  that slice is committed.
- 2026-04-24: No new third-party dependency is expected for the first slice. If
  one becomes necessary, implementation stops for dependency-standard review
  before manifest edits.

## Verification Results

- 2026-04-24: Wave `01` verification passed by inspection: start outcome is
  recorded, dirty files are unrelated, and Wave `02` write sets are
  non-overlapping.
