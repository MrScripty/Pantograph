# crates/node-engine/src

## Purpose
This directory contains Pantograph's workflow execution and descriptor core. It
turns node definitions into runnable behavior, validates graph/runtime inputs,
and keeps execution dispatch aligned with the contracts published by
`workflow-nodes`.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `builder.rs` | Engine construction helpers and composition wiring. |
| `composite_executor.rs` | Executor composition for multi-stage task execution. |
| `core_executor.rs` | Main node-type dispatch, dependency-aware execution, and payload normalization. |
| `descriptor.rs` | Node descriptor contracts consumed by the graph and runtime layers. |
| `engine.rs` | Workflow engine entry points and orchestration helpers. |
| `engine/` | Focused graph-event and multi-demand helpers behind the stable engine facade. |
| `error.rs` | Shared engine and execution error types. |
| `events.rs` | Stable facade for workflow event contracts and sink implementations. |
| `events/` | Focused event contract, sink, and test modules behind the stable facade. |
| `extensions.rs` | Extension points used to add engine behavior without mutating the core API. |
| `groups.rs` | Group/node graph helpers. |
| `model_dependencies.rs` | Model dependency typing used by execution preflight and runtime selection. |
| `orchestration/` | Orchestration-specific execution and state modules. |
| `path_validation.rs` | Validation helpers for file and model-path inputs. |
| `port_options.rs` | Port metadata helpers used by graph editing and execution. |
| `registry.rs` | Built-in node registration and descriptor inventory. |
| `tasks/` | Task metadata and task-oriented helpers. |
| `types.rs` | Shared workflow graph and runtime DTOs. |
| `undo.rs` | Undo/redo support for workflow graph editing. |
| `validation.rs` | Graph validation and invariants. |

## Problem
Pantograph needs one execution layer that understands workflow node contracts
without hard-coding frontend assumptions. As node types expand from generation
and embeddings into reranking, execution dispatch must preserve semantic
boundaries instead of forcing new workloads through incompatible legacy paths.

## Constraints
- Node descriptors are shared across frontend, backend, and saved workflow
  artifacts, so runtime assumptions must stay append-only.
- Execution helpers must tolerate heterogeneous port payloads while still
  normalizing them into typed backend requests.
- Task-type inference drives dependency/runtime selection, so incorrect
  classification can start the wrong engine mode.
- Built-in dispatch must fail explicitly for disabled node behavior instead of
  synthesizing successful placeholder outputs.

## Decision
Keep `core_executor.rs` as the single dispatch boundary for built-in node types
and normalize node inputs there before handing them to downstream runtimes.
Reranking therefore enters as a first-class `reranker` node with dedicated
document parsing and task classification instead of overloading
`llamacpp-inference`. `engine.rs` also remains the backend-owned source for
graph-mutation and incremental-demand workflow events, so adapters only
translate emitted execution facts instead of inferring graph-change semantics
locally.

## Alternatives Rejected
- Reusing the generic llama.cpp inference node for reranking.
  Rejected because reranking expects query-plus-documents semantics and ordered
  scored output, not prompt completion.
- Letting the frontend classify reranker models independently.
  Rejected because runtime mode selection must stay backend-owned.

## Invariants
- Built-in node dispatch in `core_executor.rs` must match descriptor inventory
  published by `workflow-nodes`.
- Task-type inference must reflect execution semantics, not UI naming.
- Input normalization may be permissive for additive compatibility, but output
  shapes must stay stable once published.
- Graph mutation and incremental execution events must be emitted from executor
  state transitions, not synthesized by frontend or transport adapters.
- Execution events may carry additive `occurred_at_ms` timestamps, and adapter
  layers must preserve those backend-owned producer times when projecting trace
  or diagnostics state instead of restamping them locally.
- `tool-executor` dispatch is disabled until backend-owned tool execution
  contracts exist.

## Revisit Triggers
- A second reranker family requires materially different request normalization.
- Node execution dispatch becomes too large to keep maintainable in one file and
  needs an extracted per-capability executor split.
- Saved workflow migrations become necessary for structured document inputs.

## Dependencies
**Internal:** `workflow-nodes`, `inference`, `pantograph-workflow-service`,
graph/task modules in this crate.

**External:** `serde_json`, async runtime support, and dependencies declared in
the crate manifest.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
use node_engine::core_executor::CoreNodeExecutor;
```

## API Consumer Contract
- Hosts call into the engine/executor surface with workflow graphs whose node
  types and port IDs match descriptor inventory.
- Execution errors distinguish invalid workflow input from backend/runtime
  failures where possible.
- Disabled node behavior, including `tool-executor`, must surface as execution
  errors rather than successful placeholder outputs.
- Additive node inputs may be accepted for compatibility, but callers should
  prefer the canonical descriptor fields when constructing new workflows.

## Structured Producer Contract
- Built-in node descriptors and execution dispatch must evolve together.
- Task metadata such as `taskTypePrimary` is machine-consumed by dependency
  selection and must remain stable once introduced.
- Reranker outputs are published as ordered result lists plus convenience fields
  such as top score/document; consumers should not infer ranking from raw input
  order.
