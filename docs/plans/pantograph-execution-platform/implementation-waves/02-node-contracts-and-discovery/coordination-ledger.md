# Coordination Ledger: 02 Node Contracts And Discovery

## Status

Wave `01` complete. Wave `02` is partially integrated; the
`canonical-contracts`, `workflow-nodes-registration`, and
`workflow-service-projections` slices are complete.

## Branch Or Worktree Strategy

- Integration branch: `main`.
- Worker worktrees: subagents are not explicitly authorized in this turn, so
  the host will execute Wave `02` worker slices serially in the shared
  workspace unless authorization changes.

## Wave Status

| Wave | Status | Integration Notes |
| ---- | ------ | ----------------- |
| `wave-01` | Complete | Stage-start report, contract freeze, and current ownership inventory recorded in `02-node-contracts-and-discovery.md`. |
| `wave-02` | Partial | Canonical contract crate, workflow-nodes registration, workflow-service projection integration, effective-contract resolution, direct incompatible connection diagnostics, and binding workflow validation projection are integrated; aggregate candidate diagnostics remain a design follow-up. |
| `wave-03` | Partial | Node-engine documentation alignment and ADR-006 are integrated; final verification and stage-end gate remain. |

## Worker Reports

| Worker | Report Path | Status |
| ------ | ----------- | ------ |
| canonical-contracts | `reports/wave-02-worker-canonical-contracts.md` | Complete |
| workflow-service-projections | `reports/wave-02-worker-workflow-service-projections.md` | Complete |
| workflow-nodes-registration | `reports/wave-02-worker-workflow-nodes-registration.md` | Complete |

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
- 2026-04-24: The host implemented `canonical-contracts` locally in the shared
  workspace because no subagent authorization was given. The slice adds
  `pantograph-node-contracts` and workspace wiring only; workflow-service and
  workflow-nodes integration remain separate follow-up slices.
- 2026-04-24: The host implemented `workflow-nodes-registration` locally in
  the shared workspace. Concrete built-in descriptors now project into
  canonical `NodeTypeContract` records through `workflow-nodes` without making
  `node-engine` the semantic owner of GUI or binding contracts.
- 2026-04-24: The host implemented `workflow-service-projections` locally in
  the shared workspace. Workflow-service graph definitions now consume
  canonical `NodeTypeContract` records from `workflow-nodes`, preserve
  canonical extended value types in projections, and delegate compatibility
  checks to `pantograph-node-contracts`.
- 2026-04-24: The host implemented the effective-contract resolution follow-up
  locally in the shared workspace. Dynamic `GraphNode.data["definition"]`
  overlays now resolve through canonical `EffectiveNodeContract` semantics and
  retain static ports unless explicitly overridden by stable port id.
- 2026-04-24: The host implemented direct compatibility diagnostic projection
  locally in the shared workspace. Incompatible direct connection rejections
  now include canonical source/target ids, port ids, value types, reason, and
  message under `contract_diagnostic`.
- 2026-04-24: The host implemented binding workflow validation projection
  locally in the shared workspace. Rustler and UniFFI workflow JSON validation
  now convert binding graph JSON into workflow-service graph DTOs and validate
  through backend-owned node contracts instead of calling node-engine workflow
  validation directly.
- 2026-04-24: Wave `03` documentation alignment added ADR-006 for canonical
  node contract ownership and updated node-engine READMEs to identify
  descriptors as execution inputs, not GUI/binding contract owners.

## Verification Results

- 2026-04-24: Wave `01` verification passed by inspection: start outcome is
  recorded, dirty files are unrelated, and Wave `02` write sets are
  non-overlapping.
- 2026-04-24: `canonical-contracts` verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-node-contracts`,
  `cargo clippy -p pantograph-node-contracts --all-targets -- -D warnings`,
  and `cargo check --workspace --all-features`.
- 2026-04-24: `workflow-nodes-registration` verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p workflow-nodes`,
  `cargo clippy -p workflow-nodes --all-targets -- -D warnings`, and
  `cargo check --workspace --all-features`.
- 2026-04-24: `workflow-service-projections` verification passed:
  `cargo test -p pantograph-workflow-service`,
  `cargo check --workspace --all-features`,
  `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`.
- 2026-04-24: effective-contract resolution verification passed:
  `cargo test -p pantograph-node-contracts`,
  `cargo test -p pantograph-workflow-service`,
  `cargo check --workspace --all-features`,
  `cargo fmt --all -- --check`,
  `cargo clippy -p pantograph-node-contracts --all-targets -- -D warnings`,
  and `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`.
- 2026-04-24: direct compatibility diagnostic projection verification passed:
  `cargo test -p pantograph-workflow-service`,
  `cargo check --workspace --all-features`,
  `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`.
- 2026-04-24: binding workflow validation projection verification passed:
  `cargo test -p pantograph-workflow-service graph::contract_validation`,
  `cargo test -p pantograph-uniffi test_validate_empty_workflow`,
  `cargo check -p pantograph_rustler -p pantograph-uniffi`,
  `cargo check --workspace --all-features`,
  `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-workflow-service -p pantograph-uniffi -p pantograph_rustler --all-targets -- -D warnings`.
- 2026-04-24: Rustler targeted test execution remains blocked at link time by
  missing Erlang NIF symbols, observed on `cargo test -p pantograph_rustler
  test_validation_empty_graph`; type checking and clippy for the Rustler crate
  pass.
