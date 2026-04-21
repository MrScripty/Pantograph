# crates/workflow-nodes/src/tool

Agent tool workflow node implementations.

## Purpose
This directory owns built-in node descriptors and helper contracts for exposing
Pantograph agent tools inside workflow graphs.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | Tool-node module exports and registration wiring. |
| `agent_tools.rs` | Agent tool descriptor/task contracts. |

## Problem
Tool nodes bridge workflow execution into agent/tool capabilities. Their
descriptors need to stay aligned with backend tool execution policy and must
not imply that experimental control nodes perform real tool calls.

## Constraints
- Tool node descriptors are saved workflow contracts.
- Backend tool execution policy must be explicit before nodes are advertised as
  production-supported.
- Tool payloads must remain JSON-serializable across graph context boundaries.

## Decision
Keep tool-node definitions here and treat them as backend descriptors consumed
by workflow execution. M2 must align these descriptors with the final
tool-loop/tool-executor policy.

## Alternatives Rejected
- Define tool node contracts only in frontend UI: rejected because backend graph
  execution owns task semantics.
- Hide tool nodes inside control-flow files: rejected because tool descriptors
  need their own ownership boundary.

## Invariants
- Tool node ids, port ids, and payload shapes are compatibility contracts.
- Tool payloads must not bypass backend authorization or execution policy.
- Experimental tool behavior must stay documented until resolved.

## Revisit Triggers
- Backend tool execution contracts are completed.
- Agent tool nodes move to a dedicated plugin/runtime crate.
- Tool payload schemas become formal JSON Schema artifacts.

## Dependencies
**Internal:** `node-engine`, `graph-flow`, control tool nodes, and agent tool
contracts.

**External:** `serde`, `serde_json`, and `async-trait`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
let task = AgentToolsTask::new("agent-tools-1");
```

## API Consumer Contract
- Inputs: tool request/configuration values supplied by graph context.
- Outputs: tool descriptor or result values consumed by downstream nodes.
- Lifecycle: tool nodes run during workflow graph execution.
- Errors: tool contract failures should surface as graph execution errors.
- Versioning: tool node ids, port ids, and JSON payload shapes must migrate
  with saved workflows and frontend UI.

## Structured Producer Contract
- Stable fields: tool node type ids, port ids, payload keys, and descriptor
  metadata are machine-consumed.
- Defaults: any default tool behavior must be explicit in descriptors or task
  code.
- Enums and labels: tool names, ids, and status labels carry behavior.
- Ordering: tool lists/results should preserve backend ordering where exposed.
- Compatibility: saved workflows may reference tool node descriptors across
  releases.
- Regeneration/migration: tool contract changes require node descriptors,
  control-node behavior, templates, saved workflows, and tests to update
  together.

## Testing
```bash
cargo test -p workflow-nodes --lib tool
```

## Notes
- This README intentionally records the dependency on M2 tool execution
  hardening.
