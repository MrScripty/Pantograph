# node-engine

Core workflow graph execution engine for Pantograph.

## Purpose
This crate owns graph primitives, node descriptors, validation, execution
orchestration, undo/redo state, and backend workflow events. The boundary exists
so workflow execution semantics stay reusable by services, embedded runtime,
bindings, and tests without importing Tauri or frontend modules.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Cargo.toml` | Crate manifest and feature declarations for optional inference/audio node execution. |
| `src/` | Execution engine source modules and source-level README. |

## Problem
Pantograph workflows need a backend-owned engine that can validate graph shape,
run built-in node types, report execution events, and preserve graph-edit
semantics independently of any UI. Without this crate, adapters would have to
rebuild execution and graph mutation behavior locally.

## Constraints
- Keep UI and transport concerns out of graph execution.
- Preserve descriptor and event contracts consumed by workflow service and
  binding crates.
- Feature-gated runtime integrations must not make the core graph APIs depend
  on every optional backend.
- Public graph and event DTOs may be persisted by saved workflows or consumed
  by generated bindings, so changes must remain additive where possible.

## Decision
Keep graph execution in a reusable Rust crate with optional feature gates for
heavier runtime-backed nodes. `workflow-nodes` supplies built-in descriptors and
task implementations; `pantograph-workflow-service` composes this crate into
host-agnostic application operations.

## Alternatives Rejected
- Put graph execution in the frontend store: rejected because the backend is
  the source of truth for workflow semantics.
- Put node descriptors only in Tauri commands: rejected because bindings and
  tests need the same engine contracts without desktop transport.
- Always compile every runtime-backed node feature: rejected because consumers
  should not pay for Python or inference dependencies unless needed.

## Invariants
- Graph validation and execution events are backend-owned.
- Built-in node dispatch must match descriptor inventory from `workflow-nodes`.
- Optional features must preserve the base crate's ability to compile without
  optional runtime integrations.
- Blocking or runtime-specific execution must stay isolated from pure graph
  contracts.

## Revisit Triggers
- `core_executor.rs` decomposition changes the public facade.
- A new node family needs an execution path that cannot fit the current
  feature-contract model.
- Graph/event DTOs become generated schemas for non-Rust consumers.

## Dependencies
**Internal:** `pantograph-runtime-identity`, optional `inference`, and
`workflow-nodes` consumers through the workspace.

**External:** `graph-flow`, `tokio`, `serde`, `serde_json`, `thiserror`,
`log`, `inventory`, `uuid`, and optional runtime dependencies.

## Related ADRs
- `None identified as of 2026-04-21.`
- `Reason: This crate predates the current ADR set and documents an existing
  execution boundary.`
- `Revisit trigger: A future refactor changes graph execution ownership or
  introduces a persisted graph/event schema.`

## Usage Examples
```rust
use node_engine::WorkflowGraph;

let graph = WorkflowGraph::default();
```

## API Consumer Contract
- Inputs: workflow graph DTOs, node descriptors, task inputs, execution
  targets, and optional runtime-backed task configuration.
- Outputs: graph mutations, execution results, workflow events, and typed
  engine errors.
- Lifecycle: callers create and own executors; this crate owns per-executor
  graph state and emitted event semantics.
- Errors: public fallible paths return typed `NodeEngineError` values.
- Versioning: public graph/event changes should be additive unless a coordinated
  migration is accepted.

## Structured Producer Contract
- Stable fields: graph DTOs, node descriptors, events, and undo/redo payloads
  are machine-consumed by workflow service, frontend projections, and bindings.
- Defaults: missing optional fields must preserve existing workflow behavior.
- Enums and labels: node categories, port data types, and execution states are
  semantic contracts.
- Ordering: event ordering follows backend execution progression.
- Compatibility: saved workflows and generated bindings may consume these
  shapes across releases.
- Regeneration/migration: schema-affecting DTO changes must update frontend,
  binding, and workflow-service consumers in the same slice.

## Testing
```bash
cargo test -p node-engine
```

## Notes
- `core_executor.rs` is over the decomposition threshold and remains tracked in
  the standards compliance plan.
