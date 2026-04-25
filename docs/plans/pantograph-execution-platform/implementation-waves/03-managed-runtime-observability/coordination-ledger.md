# Coordination Ledger: 03 Managed Runtime Observability

## Status

Stage `03` started. Wave `01` stage-start gate, contract freeze, event
adaptation decision, durable ledger boundary, and serial execution assumption
are recorded in `03-managed-runtime-observability.md`.

## Branch Or Worktree Strategy

- Integration branch: `main`.
- Worker worktrees: subagents are not explicitly authorized in this turn, so
  the host will execute Wave `02` worker slices serially in the shared
  workspace unless authorization changes.

## Wave Status

| Wave | Status | Integration Notes |
| ---- | ------ | ----------------- |
| `wave-01` | Complete | Stage-start report, event adaptation decision, durable ledger boundary, and serial execution assumption recorded in `03-managed-runtime-observability.md`. |
| `wave-02` | Pending | Parallel runtime and event adaptation. |
| `wave-03` | Pending | Host-owned integration and gate. |

## Worker Reports

| Worker | Report Path | Status |
| ------ | ----------- | ------ |
| runtime-context-capabilities | `reports/wave-02-worker-runtime-context-capabilities.md` | Pending |
| diagnostics-event-adapter | `reports/wave-02-worker-diagnostics-event-adapter.md` | Pending |
| cancellation-progress-guarantee | `reports/wave-02-worker-cancellation-progress-guarantee.md` | Pending |

## Decisions

- 2026-04-24: Stage-start outcome is
  `ready_with_recorded_assumptions`.
- 2026-04-24: Existing dirty files are unrelated `assets/` changes and do not
  overlap the Stage `03` write set.
- 2026-04-24: Without explicit subagent authorization, the host may implement
  Wave `02` slices serially while preserving the recorded worker boundaries.
- 2026-04-24: Stage `03` adapts node-engine workflow/task lifecycle,
  progress, waiting-for-input, stream, and graph mutation facts as low-level
  execution inputs when available, while `pantograph-embedded-runtime` owns
  enriched node execution diagnostics, attribution context, guarantee
  classification, managed capability routing facts, cancellation/progress
  handles, and lineage projection.
- 2026-04-24: Durable model/license ledger storage and query implementation
  remains forbidden until Stage `04`.
- 2026-04-24: First implementation slice is
  `runtime-context-capabilities`: `crates/pantograph-embedded-runtime/src/`
  context, execution result/error/input/output contracts, managed capability
  traits, lineage context, tests, README updates, and public facade exports
  only when needed.
- 2026-04-24: No new third-party dependency is expected for the first slice. If
  one becomes necessary, implementation stops for dependency-standard review
  before manifest edits.

## Verification Results

- 2026-04-24: Wave `01` verification passed by inspection: start outcome is
  recorded, Stage `01` and Stage `02` end gates are recorded, dirty files are
  unrelated, durable ledger storage remains forbidden until Stage `04`, and
  Wave `02` write boundaries are explicit.
