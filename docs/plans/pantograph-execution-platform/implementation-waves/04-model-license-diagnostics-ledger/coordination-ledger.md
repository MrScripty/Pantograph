# Coordination Ledger: 04 Model License Diagnostics Ledger

## Status

Stage `04` in progress. Wave `01` is complete. Wave `02` implementation
slices are ready to begin from `ledger-storage-retention`.

## Branch Or Worktree Strategy

- Integration branch: `main`.
- Worker worktrees: subagents are not explicitly authorized in this turn, so
  the host will execute Wave `02` worker slices serially in the shared
  workspace unless authorization changes.

## Wave Status

| Wave | Status | Integration Notes |
| ---- | ------ | ----------------- |
| `wave-01` | Complete | Stage-start report, SQLite dependency/linking review, schema freeze, retention default, pruning semantics, query bounds, and worker write boundaries recorded in `04-model-license-diagnostics-ledger.md`. |
| `wave-02` | In Progress | `ledger-storage-retention` and `runtime-ledger-submission` are integrated locally; workflow-service query projections remain. |
| `wave-03` | Pending | Host-owned integration and gate. |

## Worker Reports

| Worker | Report Path | Status |
| ------ | ----------- | ------ |
| ledger-storage-retention | `reports/wave-02-worker-ledger-storage-retention.md` | Complete |
| runtime-ledger-submission | `reports/wave-02-worker-runtime-ledger-submission.md` | Complete |
| workflow-service-query-projections | `reports/wave-02-worker-workflow-service-query-projections.md` | Pending |

## Decisions

- 2026-04-24: Stage-start outcome is
  `ready_with_recorded_assumptions`.
- 2026-04-24: Existing dirty files are unrelated `assets/` changes and do not
  overlap the Stage `04` write set.
- 2026-04-24: Stage `01`, Stage `02`, and Stage `03` are integrated and their
  stage-end refactor gates are recorded as `not_warranted`.
- 2026-04-24: Without explicit subagent authorization, the host may implement
  Wave `02` slices serially while preserving the recorded worker boundaries.
- 2026-04-24: Stage `04` uses `rusqlite` `0.32.1` with the existing `bundled`
  feature. Because both `pantograph-runtime-attribution` and
  `pantograph-diagnostics-ledger` will directly use it, implementation should
  centralize the dependency in `[workspace.dependencies]` and declare it in
  each owning crate with `{ workspace = true }`.
- 2026-04-24: The version `1` ledger schema owns
  `ledger_schema_migrations`, `model_license_usage_events`,
  `license_snapshots`, `model_output_measurements`, `usage_lineage`, and
  `diagnostics_retention_policy`.
- 2026-04-24: The default local retention policy is `standard` usage-event
  retention for 365 days. Policy changes that require legal hold,
  export-before-prune, per-client retention, or a different default duration
  are re-plan triggers.
- 2026-04-24: Pruning is explicit and command-shaped. It deletes complete
  eligible usage events and their associated snapshot, measurement, and lineage
  rows in one transaction and must not rewrite retained historical facts.
- 2026-04-24: Query inputs are bounded by validated filters, inclusive start
  and exclusive end timestamps, explicit pagination, maximum page size 500,
  and maximum time-series bucket count 366.
- 2026-04-24: Wave `02` non-overlap boundary is frozen:
  `ledger-storage-retention` owns canonical ledger DTOs and persistence,
  `runtime-ledger-submission` consumes the ledger trait and DTOs from embedded
  runtime integration, and `workflow-service-query-projections` delegates to
  the ledger without owning persistence semantics.
- 2026-04-24: The host implemented `ledger-storage-retention` locally in the
  shared workspace. The slice adds `pantograph-diagnostics-ledger`, canonical
  ledger DTOs, `DiagnosticsLedgerRepository`, `SqliteDiagnosticsLedger`, schema
  migration initialization, event insertion/querying, retention policy lookup,
  transactional pruning, crate README coverage, and focused persistence tests.
- 2026-04-24: `rusqlite` is now centralized in workspace dependencies and
  `pantograph-runtime-attribution` inherits it through `rusqlite.workspace =
  true`.
- 2026-04-24: The host implemented `runtime-ledger-submission` locally in the
  shared workspace. The slice adds an embedded-runtime managed model usage
  submission boundary, durable event construction from runtime-created node
  context and model capability routes, guarantee-level mapping, unavailable
  measurement downgrade behavior, public facade exports, README coverage, and
  focused tests.

## Verification Results

- 2026-04-24: Wave `01` verification passed by inspection: start outcome is
  recorded, dirty files are unrelated, prior-stage gates are recorded,
  dependency/linking review is recorded, schema/retention/pruning/query bounds
  are frozen, and Wave `02` write boundaries are explicit.
- 2026-04-24: Dependency review commands passed:
  `cargo tree -i rusqlite`,
  `cargo tree -p pantograph-runtime-attribution --depth 1`,
  `cargo tree -p rusqlite --depth 1`, and
  `cargo tree -p pantograph-runtime-attribution --prefix none --no-dedupe | sort -u | wc -l`.
- 2026-04-24: `ledger-storage-retention` verification passed:
  `cargo fmt -p pantograph-diagnostics-ledger -p pantograph-runtime-attribution`,
  `cargo test -p pantograph-diagnostics-ledger`,
  `cargo check -p pantograph-diagnostics-ledger`, and
  `cargo clippy -p pantograph-diagnostics-ledger --all-targets -- -D warnings`.
- 2026-04-24: Full `cargo fmt --all` attempted during this slice failed
  because it tried to write a read-only file in the external Pumas checkout.
  Package-scoped formatting for touched crates passed.
- 2026-04-24: `runtime-ledger-submission` verification passed:
  `cargo fmt -p pantograph-embedded-runtime -p pantograph-diagnostics-ledger`,
  `cargo test -p pantograph-embedded-runtime node_execution_ledger`,
  `cargo check -p pantograph-embedded-runtime`,
  `cargo clippy -p pantograph-embedded-runtime --all-targets -- -D warnings`,
  and `cargo test -p pantograph-embedded-runtime`.
