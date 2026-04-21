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
| `tool_loop.rs` | Tool-loop node descriptor and LLM loop behavior; fails explicitly when tool calls require disabled backend tool execution. |
| `tool_executor.rs` | Disabled tool-executor node descriptor that preserves saved-workflow compatibility without fabricating tool results. |

## Problem
Control nodes shape graph execution flow and can be mistaken for fully
implemented agent tooling. Tool-loop/tool-executor descriptors may appear in
saved workflows, but real backend tool execution is not implemented yet.

## Constraints
- Control nodes must use backend graph context keys and declared port metadata.
- Expected control-flow outcomes should remain deterministic.
- Tool nodes must not imply successful real tool execution while the backend
  tool runtime contract is unavailable.
- Public descriptor ids and port ids are saved workflow contracts.

## Decision
Keep control-flow nodes in this directory and disable tool execution behavior
until a backend-owned tool runtime contract exists. Descriptors remain
registered for saved workflow compatibility, but runtime paths fail instead of
emitting synthetic success.

## Alternatives Rejected
- Keep placeholder tool success: rejected because it can create false
  successful executions.
- Move control-flow nodes into frontend templates: rejected because graph
  execution behavior belongs in backend nodes.

## Invariants
- Conditional and merge nodes must preserve declared input/output semantics.
- Tool-loop/tool-executor must fail when real tool execution would be required.
- Disabled tool behavior must not emit successful placeholder results.

## Revisit Triggers
- Backend-owned tool execution contracts are implemented.
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
- Outputs: branch/merge context values and non-tool-loop LLM responses; tool
  execution requests fail until a backend tool runtime exists.
- Lifecycle: tasks are instantiated by graph execution and run once per
  scheduling decision.
- Errors: missing required inputs return `GraphError::TaskExecutionFailed`.
- Versioning: descriptor ids, port ids, and disabled/tool-runtime support
  status must migrate with templates and saved workflows.

## Structured Producer Contract
- Stable fields: node type ids, port ids, port data types, execution modes, and
  serialized tool-call/result shapes are machine-consumed.
- Defaults: `ToolLoopConfig::default()` values are observable while the node
  remains registered.
- Enums and labels: control node type ids and task labels carry behavior.
- Ordering: tool result arrays preserve input tool-call ordering.
- Compatibility: saved workflows may already reference these node ids, so
  descriptor registration remains while runtime behavior is disabled.
- Regeneration/migration: backend tool runtime support must update descriptors,
  tests, templates, saved workflows, and this README together.

## Testing
```bash
cargo test -p workflow-nodes --lib control
```

## Notes
- Tool execution is deliberately disabled rather than simulated.
