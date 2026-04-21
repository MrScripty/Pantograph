# crates/workflow-nodes/src/control

Control-flow workflow node implementations.

## Purpose
This directory owns built-in control-flow node descriptors and task behavior for
branching, merging, and tool-call orchestration inside workflow graphs.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | Control-node module exports and registration wiring. |
| `conditional.rs` | Conditional branch node behavior and metadata. |
| `merge.rs` | Merge node behavior and metadata. |
| `tool_loop.rs` | Experimental tool-loop node; currently produces synthetic loop output rather than calling a real LLM/tool runtime. |
| `tool_executor.rs` | Experimental tool-executor node; currently validates inputs and emits placeholder tool results instead of executing tools. |

## Problem
Control nodes shape graph execution flow and can be mistaken for fully
implemented agent tooling. The current tool-loop/tool-executor behavior is not
complete and must be called out until M2 resolves the contract.

## Constraints
- Control nodes must use backend graph context keys and declared port metadata.
- Expected control-flow outcomes should remain deterministic.
- Experimental tool nodes must not imply successful real tool execution.
- Public descriptor ids and port ids are saved workflow contracts.

## Decision
Keep control-flow nodes in this directory and document tool-loop/tool-executor
as experimental placeholder behavior. M2 must either implement real backend
tool execution contracts or remove/disable the nodes from supported workflows.

## Alternatives Rejected
- Leave placeholder tool behavior undocumented: rejected because it can create
  false successful executions.
- Move control-flow nodes into frontend templates: rejected because graph
  execution behavior belongs in backend nodes.

## Invariants
- Conditional and merge nodes must preserve declared input/output semantics.
- Tool-loop/tool-executor must remain treated as incomplete until the M2
  backend-owned tool contract is implemented or the nodes are disabled.
- Placeholder tool results must not be used as evidence of real tool execution.

## Revisit Triggers
- M2 completes or removes tool-loop/tool-executor behavior.
- Agent tooling moves into a dedicated backend tool runtime.
- Control-flow descriptor ids or port ids change.

## Dependencies
**Internal:** `node-engine`, `graph-flow`, and workflow context key helpers.

**External:** `serde`, `serde_json`, and `async-trait`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
let task = ToolExecutorTask::new("tool-executor-1");
```

## API Consumer Contract
- Inputs: context values for declared control-node input ports.
- Outputs: branch/merge/tool-loop/tool-executor context values.
- Lifecycle: tasks are instantiated by graph execution and run once per
  scheduling decision.
- Errors: missing required inputs return `GraphError::TaskExecutionFailed`.
- Versioning: descriptor ids, port ids, and placeholder/experimental status
  must migrate with templates and saved workflows.

## Structured Producer Contract
- Stable fields: node type ids, port ids, port data types, execution modes, and
  serialized tool-call/result shapes are machine-consumed.
- Defaults: `ToolLoopConfig::default()` values are observable while the node
  remains registered.
- Enums and labels: control node type ids and task labels carry behavior.
- Ordering: tool result arrays preserve input tool-call ordering.
- Compatibility: saved workflows may already reference these node ids.
- Regeneration/migration: M2 changes must update descriptors, tests, templates,
  saved workflows, and this README together.

## Testing
```bash
cargo test -p workflow-nodes --lib control
```

## Notes
- The placeholder tool behavior is a recorded non-standards bug/risk and must
  be resolved before these nodes are considered production-supported.
