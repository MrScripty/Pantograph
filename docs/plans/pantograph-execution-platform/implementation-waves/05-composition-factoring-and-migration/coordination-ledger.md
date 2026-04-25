# Coordination Ledger: 05 Composition Factoring And Migration

## Status

Stage `05` in progress. Wave `01` and Wave `02` implementation slices are
integrated locally. Wave `03` migration integration, ADR, final verification,
and stage-end gate remain.

## Wave Status

| Wave | Status | Integration Notes |
| ---- | ------ | ----------------- |
| `wave-01` | Complete | Stage-start report, node/port inventory, saved-workflow artifact inventory, classification freeze, migration output semantics, and Wave `02` write boundaries recorded in `05-composition-factoring-and-migration.md`. |
| `wave-02` | Complete | Composition contracts, workflow-node factoring, and runtime lineage are integrated locally. |
| `wave-03` | Pending | Host-owned migration integration and gate. |

## Worker Reports

| Worker | Report Path | Status |
| ------ | ----------- | ------ |
| composition-contracts | `reports/wave-02-worker-composition-contracts.md` | Complete |
| workflow-nodes-factoring | `reports/wave-02-worker-workflow-nodes-factoring.md` | Complete |
| runtime-lineage | `reports/wave-02-worker-runtime-lineage.md` | Complete |

## Decisions

- 2026-04-24: Stage-start outcome is
  `ready_with_recorded_assumptions`.
- 2026-04-24: Existing dirty files are unrelated `assets/` changes and do not
  overlap the Stage `05` write set.
- 2026-04-24: Stage `01`, Stage `02`, Stage `03`, and Stage `04` are
  integrated and their stage-end refactor gates are recorded as
  `not_warranted`.
- 2026-04-24: Without explicit subagent authorization, the host may implement
  Wave `02` slices serially while preserving the recorded worker boundaries.
- 2026-04-24: The built-in node type inventory is frozen at 45 workflow-node
  descriptors discovered from `crates/workflow-nodes/src/`, plus the
  workflow-service-owned `node-group` graph grouping surface.
- 2026-04-24: Existing saved workflow artifact inventory includes
  `src/templates/workflows/gguf-reranker-workflow.json`, embedded-runtime
  graph fixtures, and workflow-service inline graph/session fixtures.
- 2026-04-24: Existing migration behavior canonicalizes legacy
  `system-prompt` nodes to `text-input` and rewrites legacy `prompt` handles
  to `text`.
- 2026-04-24: Classification freeze: simple input/output/control/storage/
  system utility nodes and specialized model-producing nodes remain primitive;
  `node-group` and `tool-loop` become stable composed authoring contracts with
  primitive trace preservation; `vision-analysis` and future unmanaged direct
  model execution nodes require split-or-reject migration policy before they
  can be treated as complete managed primitives.
- 2026-04-24: Migration outcomes are frozen as `upgraded`, `regenerated`, or
  `typed_rejection`. Temporary compatibility projections are allowed only
  inside migration code and must not remain public semantics after migration.
- 2026-04-24: Wave `02` non-overlap boundary is frozen:
  `composition-contracts` owns canonical contract metadata and migration error
  types, `workflow-nodes-factoring` owns concrete descriptor changes and
  workflow-node README coverage, and `runtime-lineage` owns embedded-runtime
  composed-parent lineage projection.
- 2026-04-24: The host implemented `composition-contracts` locally in the
  shared workspace. The slice adds standalone composed-node contract DTOs,
  external-to-internal port mapping validation, contract-upgrade and migration
  diagnostic DTOs, typed composition/migration validation errors, README
  coverage, and focused tests without changing primitive `NodeTypeContract`
  serialization.
- 2026-04-24: The host implemented `workflow-nodes-factoring` locally in the
  shared workspace. The slice adds `builtin_composed_node_contracts()`, a
  concrete `tool-loop` composed authoring registration over primitive
  `llm-inference`, `tool-executor`, and turn-state control nodes, crate-root
  exports, README coverage, and focused tests without changing primitive
  descriptor registration.
- 2026-04-24: The host implemented `runtime-lineage` locally in the shared
  workspace. The slice adds `NodeLineageContext` helpers for primitive lineage,
  entering composed execution scopes, preserving composed-parent stacks,
  carrying lineage segment metadata, README coverage, and focused tests.

## Verification Results

- 2026-04-24: Wave `01` verification passed by inspection: start outcome is
  recorded, dirty files are unrelated, prior-stage gates are recorded,
  node/port and saved-workflow artifact inventory is recorded,
  keep/split/compose classification is frozen, migration output semantics are
  frozen, and Wave `02` write boundaries are explicit.
- 2026-04-24: `composition-contracts` verification passed:
  `cargo fmt -p pantograph-node-contracts -- --check`,
  `cargo test -p pantograph-node-contracts`,
  `cargo check -p pantograph-node-contracts`, and
  `cargo clippy -p pantograph-node-contracts --all-targets -- -D warnings`.
- 2026-04-24: `workflow-nodes-factoring` verification passed:
  `cargo fmt -p workflow-nodes -- --check`,
  `cargo test -p workflow-nodes`,
  `cargo check -p workflow-nodes`, and
  `cargo clippy -p workflow-nodes --all-targets -- -D warnings`.
- 2026-04-24: `runtime-lineage` verification passed:
  `cargo fmt -p pantograph-embedded-runtime -- --check`,
  `cargo test -p pantograph-embedded-runtime node_execution`,
  `cargo check -p pantograph-embedded-runtime`, and
  `cargo clippy -p pantograph-embedded-runtime --all-targets -- -D warnings`.
