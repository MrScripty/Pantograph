# Coordination Ledger: 01 Client Session Bucket Run Attribution

## Status

Wave `01` complete. Wave `02` is integrated through durable attribution,
binding cutover, and execution-session terminology cleanup. Stage-end review
remains pending before moving to Stage `02`.

## Branch Or Worktree Strategy

- Integration branch: `main`.
- Worker worktrees: subagents are not explicitly authorized in this turn, so
  the host will execute Wave `02` worker slices serially in the shared
  workspace unless authorization changes.

## Wave Status

| Wave | Status | Integration Notes |
| ---- | ------ | ----------------- |
| `wave-01` | Complete | Stage-start report, contract freeze, cutover inventory, and dependency review recorded in `01-client-session-bucket-run-attribution.md`. |
| `wave-02` | Integrated | Attribution storage, workflow-service attributed runs, bucket selection, UniFFI JSON boundary projection, UniFFI/Rustler public workflow-session binding cutover, and execution-session terminology cleanup are integrated. |
| `wave-03` | Pending | Host-owned integration and stage-end gate. |

## Worker Reports

| Worker | Report Path | Status |
| ------ | ----------- | ------ |
| attribution-domain-storage | `reports/wave-02-worker-attribution-domain-storage.md` | Complete |
| workflow-service-cutover | `reports/wave-02-worker-workflow-service-cutover.md` | Integrated |

## Decisions

- 2026-04-24: Stage-start outcome is `ready_with_recorded_assumptions`.
- 2026-04-24: Existing dirty files are unrelated `assets/` changes and do not
  overlap the Stage `01` write set.
- 2026-04-24: `rusqlite` and `blake3` are already present in `Cargo.lock`;
  Wave `02` may add direct crate-local dependencies for
  `pantograph-runtime-attribution`.
- 2026-04-24: Without explicit subagent authorization, the host may implement
  Wave `02` slices serially while preserving the recorded write boundaries.
- 2026-04-24: The host implemented `attribution-domain-storage` locally in the
  shared workspace because no subagent authorization was given.
- 2026-04-24: The host implemented the first workflow-service cutover slice
  locally: native Rust attributed workflow-run orchestration. Legacy
  workflow-session public API removal remains pending.
- 2026-04-24: Public generic `workflow_run` now rejects caller-authored run ids;
  backend-generated ids remain available through the service-owned execution
  path and attributed run orchestration.
- 2026-04-24: Workflow-service now exposes native Rust client bucket
  create/delete operations backed by `pantograph-runtime-attribution`; explicit
  bucket selection is tested in attributed workflow runs.
- 2026-04-24: The host added a boundary-projection slice across attribution,
  embedded-runtime, and UniFFI so JSON callers can register clients, open or
  resume durable client sessions, create or delete buckets, and run attributed
  workflows before the legacy workflow-session API is removed.
- 2026-04-24: UniFFI and Rustler frontend public workflow-session wrappers were
  removed from the binding surface. Rustler now exposes the same durable
  attribution frontend-HTTP operations as UniFFI.
- 2026-04-24: Remaining workflow-service, embedded-runtime, Tauri diagnostics,
  and node-engine scheduler/runtime workflow-session terminology was renamed
  to execution-session terminology. The old workflow-session Rust source
  vocabulary no longer appears under `crates/` or `src-tauri/`.

## Verification Results

- 2026-04-24: Wave `01` verification passed by inspection: start outcome is
  recorded and Wave `02` write sets remain non-overlapping.
- 2026-04-24: `attribution-domain-storage` verification passed:
  `cargo fmt --all -- --check` and
  `cargo test -p pantograph-runtime-attribution`, and
  `cargo clippy -p pantograph-runtime-attribution --all-targets -- -D warnings`.
- 2026-04-24: `workflow-service-cutover` partial verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-workflow-service attribution`,
  `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`,
  `cargo test -p pantograph-workflow-service`, and
  `cargo check --workspace --all-features`.
- 2026-04-24: Workflow run boundary hardening verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-workflow-service workflow_run`,
  `cargo test -p pantograph-workflow-service`, and
  `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`.
- 2026-04-24: Workflow-service bucket selection verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-workflow-service attribution`, and
  `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`.
- 2026-04-24: Attribution boundary projection verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-attribution`,
  `cargo test -p pantograph-workflow-service`,
  `cargo test -p pantograph-uniffi --features frontend-http`,
  `cargo check --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- 2026-04-24: Binding workflow-session cutover verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-uniffi --features frontend-http`,
  `cargo check -p pantograph_rustler --features frontend-http`,
  `cargo check --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
  `cargo test -p pantograph_rustler --features frontend-http` was attempted
  and failed during test binary linking on unresolved Erlang NIF symbols
  (`enif_*`), which is an existing Rustler test-link environment limitation.
- 2026-04-24: Execution-session terminology cutover verification passed:
  `cargo fmt --all -- --check`,
  `cargo check --workspace --all-features`,
  `cargo test -p pantograph-workflow-service`,
  `cargo test -p pantograph-embedded-runtime workflow_runtime`,
  `cargo test -p pantograph-uniffi --features frontend-http`,
  `cargo test -p node-engine workflow_execution_session`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
  Source vocabulary checks for legacy `WorkflowSession*` and
  `workflow_session` forms under `crates/` and `src-tauri/` returned no
  matches.
