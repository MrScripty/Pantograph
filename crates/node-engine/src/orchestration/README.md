# crates/node-engine/src/orchestration

Backend orchestration graph execution boundary.

## Purpose
This directory owns the high-level orchestration graph model and executor used
to sequence data graphs through start, end, condition, loop, merge, and
data-graph nodes.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | Public orchestration module exports and boundary documentation. |
| `types.rs` | Orchestration graph, node, edge, config, and result DTOs. |
| `nodes.rs` | Node execution context and per-node orchestration behavior helpers. |
| `executor.rs` | Orchestration executor and event emission flow. |
| `executor_tests.rs` | Crate-local orchestration executor regression coverage for control flow and terminal event behavior. |
| `store.rs` | In-memory orchestration graph storage and metadata helpers. |

## Problem
Pantograph needs workflow-level control flow that can coordinate multiple data
graphs without moving orchestration semantics into frontend stores or host
adapters.

## Constraints
- Orchestration graph DTOs may be serialized in saved orchestration files.
- Data graph execution is delegated through a trait rather than hard-coded.
- Control-flow behavior must remain deterministic and backend-owned.
- Event payloads must stay compatible with workflow service and binding
  projections.
- Event/output projections should use direct typed constructors where possible
  so orchestration glue does not accumulate closure-only lint debt.

## Decision
Keep orchestration graph contracts and execution in this focused module tree.
The executor owns control-flow sequencing while data-graph execution remains an
injected dependency. Executor regression coverage now lives in a sibling
`executor_tests.rs` harness so production control-flow code does not grow under
an inlined test module.

## Alternatives Rejected
- Store orchestration control flow only in frontend graph state: rejected
  because backend execution must own runtime behavior.
- Merge orchestration graph DTOs into workflow-node descriptors: rejected
  because orchestration nodes describe control flow, not dataflow tasks.

## Invariants
- Orchestration graphs must have valid node/edge references before execution.
- DataGraph nodes call the injected data-graph executor.
- Orchestration events preserve backend execution order.
- Executor behavior tests stay in `executor_tests.rs` so `executor.rs` remains
  focused on production control-flow sequencing and event emission.
- Orchestration storage filters should use explicit option predicates for file
  extensions so saved-graph discovery remains easy to audit.
- Saved orchestration DTO changes require migration of tracked examples.

## Revisit Triggers
- Orchestration JSON receives a formal schema.
- Orchestration execution moves into workflow service.
- Durable orchestration stores replace the current in-memory store.

## Dependencies
**Internal:** node-engine graph/runtime types and saved orchestration examples
under `.pantograph/orchestrations`.

**External:** `serde`, `serde_json`, `async-trait`, and `tokio`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
use node_engine::orchestration::{OrchestrationGraph, OrchestrationNodeType};
```

## API Consumer Contract
- Inputs: orchestration graph DTOs, initial data, and a `DataGraphExecutor`.
- Outputs: orchestration results and ordered orchestration events.
- Lifecycle: callers create or load orchestration graphs, then execute them
  through `OrchestrationExecutor`.
- Errors: validation or data-graph execution failures propagate through the
  orchestration result/error path.
- Versioning: graph/node/edge DTO changes must migrate saved examples and
  adapters together.

## Structured Producer Contract
- Stable fields: orchestration graph ids, node ids, edge ids, node types,
  config keys, and event labels are machine-consumed.
- Defaults: omitted optional config fields must match executor defaults.
- Enums and labels: node types and event labels carry execution behavior.
- Ordering: event order follows backend control-flow progression.
- Compatibility: `.pantograph/orchestrations` examples may reference these DTOs
  across releases.
- Regeneration/migration: DTO changes require orchestration examples, tests,
  and consumers to update in the same slice.

## Testing
```bash
cargo test -p node-engine orchestration
```

## Notes
- Keep orchestration policy in backend Rust; host adapters should only load,
  save, and invoke it.
