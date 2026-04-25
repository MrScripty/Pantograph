# Coordination Ledger: 03 Managed Runtime Observability

## Status

Stage `03` complete. Wave `01`, Wave `02`, and Wave `03` are integrated and
the stage-end refactor gate outcome is recorded as `not_warranted`.

## Branch Or Worktree Strategy

- Integration branch: `main`.
- Worker worktrees: subagents are not explicitly authorized in this turn, so
  the host will execute Wave `02` worker slices serially in the shared
  workspace unless authorization changes.

## Wave Status

| Wave | Status | Integration Notes |
| ---- | ------ | ----------------- |
| `wave-01` | Complete | Stage-start report, event adaptation decision, durable ledger boundary, and serial execution assumption recorded in `03-managed-runtime-observability.md`. |
| `wave-02` | Complete | Runtime context/capability contracts, diagnostics event adaptation, and cancellation/progress/guarantee recorder wiring are integrated. |
| `wave-03` | Complete | ADR-007, final verification, stale fixture repair, and stage-end refactor gate are complete. |

## Worker Reports

| Worker | Report Path | Status |
| ------ | ----------- | ------ |
| runtime-context-capabilities | `reports/wave-02-worker-runtime-context-capabilities.md` | Complete |
| diagnostics-event-adapter | `reports/wave-02-worker-diagnostics-event-adapter.md` | Complete |
| cancellation-progress-guarantee | `reports/wave-02-worker-cancellation-progress-guarantee.md` | Complete |

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
- 2026-04-24: The host implemented `runtime-context-capabilities` locally in
  the shared workspace. The slice adds `node_execution.rs`,
  `node_execution_capabilities.rs`, `node_execution_tests.rs`,
  embedded-runtime facade exports, README coverage, crate-local path
  dependencies on `pantograph-node-contracts` and
  `pantograph-runtime-attribution`, and focused context/capability/guarantee
  tests.
- 2026-04-24: Decomposition review split the initial combined
  context/capability/test module into focused sibling modules before commit to
  keep touched source files below the 500-line standards trigger.
- 2026-04-24: The host implemented `diagnostics-event-adapter` locally in the
  shared workspace. The slice adds transient runtime-owned node diagnostics
  DTOs and adapts node-engine lifecycle/progress/stream/cancellation facts into
  enriched attribution, contract, lineage, and guarantee events without adding
  durable ledger storage.
- 2026-04-24: The host implemented `cancellation-progress-guarantee` locally in
  the shared workspace. The slice adds an event-sink recorder that forwards
  original node-engine events and collects enriched diagnostics for registered
  runtime-created node contexts, including cancellation and reduced-guarantee
  classification.
- 2026-04-24: ADR-007 freezes embedded-runtime ownership of managed runtime
  observability: runtime-created node execution context, managed capabilities,
  transient diagnostics, cancellation/progress lifecycle, and guarantee
  classification.
- 2026-04-24: Stale `pantograph-embedded-runtime` public `workflow_run` test
  fixtures were corrected to omit caller-supplied run ids. This aligns the
  tests with the Stage `01` backend-owned run-id contract and restores the full
  package test suite.
- 2026-04-24: Stage-end refactor gate outcome is `not_warranted`. Reviewed
  touched files from `git diff --name-only 7d153f82...HEAD` plus current Wave
  `03` files; source files stayed below the 500-line decomposition trigger and
  no ownership, dependency, async, binding, or durable-ledger boundary refactor
  is needed before Stage `04`.

## Verification Results

- 2026-04-24: Wave `01` verification passed by inspection: start outcome is
  recorded, Stage `01` and Stage `02` end gates are recorded, dirty files are
  unrelated, durable ledger storage remains forbidden until Stage `04`, and
  Wave `02` write boundaries are explicit.
- 2026-04-24: `runtime-context-capabilities` verification passed:
  `cargo test -p pantograph-embedded-runtime node_execution`,
  `cargo check -p pantograph-embedded-runtime`,
  `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-embedded-runtime --all-targets -- -D warnings`.
- 2026-04-24: Full `cargo test -p pantograph-embedded-runtime` is not clean in
  this environment. The new `node_execution` tests passed; the package suite
  still reports Pumas SQLite read-only database failures and older
  workflow-run fixture failures where callers supply backend-owned run ids.
- 2026-04-24: `diagnostics-event-adapter` verification passed:
  `cargo test -p pantograph-embedded-runtime node_execution_diagnostics`,
  `cargo check -p pantograph-embedded-runtime`,
  `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-embedded-runtime --all-targets -- -D warnings`.
- 2026-04-24: `cancellation-progress-guarantee` verification passed:
  `cargo test -p pantograph-embedded-runtime node_execution_diagnostics`,
  `cargo check -p pantograph-embedded-runtime`,
  `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-embedded-runtime --all-targets -- -D warnings`.
- 2026-04-24: Wave `03` package verification passed:
  `cargo test -p pantograph-embedded-runtime`,
  `cargo test -p node-engine`, and
  `cargo test -p pantograph-workflow-service`.
- 2026-04-24: Wave `03` workspace verification passed:
  `cargo check --workspace --all-features`,
  `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and
  `cargo test --workspace --doc`.
