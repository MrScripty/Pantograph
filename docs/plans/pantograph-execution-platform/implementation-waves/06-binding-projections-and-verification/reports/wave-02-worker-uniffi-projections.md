# Wave 02 Worker Report: uniffi-projections

## Status

Complete locally. Commit is blocked because `.git` is mounted read-only.

## Scope

- `crates/pantograph-uniffi/src/runtime.rs`
- `crates/pantograph-uniffi/src/runtime_tests.rs`
- Stage `06` implementation notes and coordination ledger

## Changes

- Added additive `FfiPantographRuntime` JSON methods for registry-backed graph
  authoring discovery:
  - `workflow_graph_list_node_definitions`
  - `workflow_graph_get_node_definition`
  - `workflow_graph_get_node_definitions_by_category`
  - `workflow_graph_get_queryable_ports`
  - `workflow_graph_query_port_options`
- Projected node definitions through `pantograph-workflow-service`
  `NodeRegistry` rather than creating a UniFFI-owned node catalog.
- Projected queryable ports and port-option queries through the backend
  `node-engine` registry and the runtime's existing `ExecutorExtensions`.
- Restored direct workflow execution-session methods expected by the generated
  C# smoke and UniFFI metadata gate.
- Preserved workflow error-envelope mapping for unknown node types and
  non-queryable port option requests.
- Added direct runtime tests for discovery, grouping, queryable ports, and
  rejection envelope behavior.
- Extended the C# smoke harness to assert generated access to backend-owned
  discovery methods.
- Updated the UniFFI metadata gate to require the graph-authoring discovery
  exports.

## Verification

```bash
cargo test -p pantograph-uniffi direct_runtime_exposes_backend_owned_graph_authoring_discovery
cargo test -p pantograph-uniffi direct_runtime_runs_workflow_from_json
cargo test -p pantograph-uniffi
./scripts/check-uniffi-embedded-runtime-surface.sh
./scripts/check-uniffi-csharp-smoke.sh
PANTOGRAPH_PACKAGE_PROFILE=debug ./scripts/package-uniffi-csharp-artifacts.sh
./scripts/check-packaged-csharp-quickstart.sh
```

## Notes

- No generated binding artifacts were edited.
- Full C# artifact smoke and packaged quickstart checks remain pending for the
  Stage `06` integration wave.
