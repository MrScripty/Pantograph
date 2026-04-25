# Wave 02 Worker Report: workflow-nodes-factoring

## Status

Complete.

## Scope

- Primary write set: `crates/workflow-nodes/`.
- Report path:
  `docs/plans/pantograph-execution-platform/implementation-waves/05-composition-factoring-and-migration/reports/wave-02-worker-workflow-nodes-factoring.md`.
- Shared files touched: none.
- Forbidden files touched: none.

## Changes

- Added `builtin_composed_node_contracts()` to expose built-in composed
  authoring registrations using `pantograph-node-contracts` DTOs.
- Added a concrete `tool-loop` composed authoring registration that maps the
  stable external `tool-loop` contract onto internal primitive
  `llm-inference`, `tool-executor`, and turn-state control nodes.
- Preserved primitive descriptor registration and `NodeTypeContract`
  serialization.
- Re-exported `builtin_composed_node_contracts()` from the crate root.
- Updated workflow-nodes README coverage for composed registrations and
  primitive trace preservation.

## Verification

- `cargo fmt -p workflow-nodes -- --check`
- `cargo test -p workflow-nodes`
- `cargo check -p workflow-nodes`
- `cargo clippy -p workflow-nodes --all-targets -- -D warnings`

## Notes

- The `tool-loop` composed registration is an authoring and diagnostics
  contract. It does not claim the current runtime has complete backend-owned
  tool-call continuation; existing tool-loop/tool-executor disabled-runtime
  behavior remains documented until the tool runtime exists.
