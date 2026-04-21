# crates/workflow-nodes/src/system

System workflow node implementations.

## Purpose
This directory owns built-in system-level workflow nodes that interact with
process or host-adjacent capabilities through backend task contracts.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | System-node module exports and registration wiring. |
| `process.rs` | Process execution node behavior and metadata. |

## Problem
System nodes can cross the boundary from pure graph dataflow into host process
execution. That boundary needs explicit ownership and guardrails so workflow
graphs do not bypass backend runtime policy.

## Constraints
- Process behavior must stay backend-owned and explicit.
- Host/path/environment assumptions must be validated before execution.
- System node descriptors are saved workflow contracts.
- Security and sandbox policy must not be hidden in frontend code.

## Decision
Keep system node descriptors and behavior isolated in this directory. Treat
process execution as a backend capability that must remain reviewable and
bounded by host policy. `ProcessTask::new` is default-deny; hosts that accept
process execution risk must construct the task with an explicit
`ProcessExecutionPolicy` command allowlist.

## Alternatives Rejected
- Put process execution in frontend or Tauri-only code: rejected because graph
  execution can run through non-desktop hosts.
- Merge system nodes into generic processing nodes: rejected because host
  process interaction needs distinct security review.

## Invariants
- System node ids and ports are compatibility contracts.
- Process execution must preserve backend error reporting.
- Process execution must be authorized by backend-owned host policy, not by
  workflow graph fields or frontend state.
- Any expansion of host-side effects requires security and lifecycle review.

## Revisit Triggers
- System nodes need sandbox profiles.
- Process execution moves behind a dedicated host trait.
- Additional host-side-effect nodes are added.

## Dependencies
**Internal:** `node-engine`, `graph-flow`, and host runtime policy.

**External:** standard process/filesystem APIs and `async-trait`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
let task = ProcessTask::new("process-1");
```

## API Consumer Contract
- Inputs: process command/configuration values supplied by graph context.
- Outputs: process result values and task errors.
- Lifecycle: process tasks run during graph execution and must not own
  long-lived host services.
- Errors: unauthorized commands, process launch failures, and exit failures
  surface as graph execution errors.
- Versioning: process node ids and ports must migrate with saved workflows.

## Structured Producer Contract
- Stable fields: node type ids, port ids, data types, and process result shapes
  are machine-consumed.
- Defaults: process defaults must be explicit in descriptors or task code.
- Policy: default task construction denies execution; host integrations must
  provide a command allowlist before running process-backed workflows.
- Enums and labels: process result/status labels carry behavior.
- Ordering: process output streams should preserve emitted order where exposed.
- Compatibility: saved workflows may reference system node ids.
- Regeneration/migration: descriptor changes require saved workflow, template,
  and tests updates together.

## Testing
```bash
cargo test -p workflow-nodes --lib system
```

## Notes
- Treat new system nodes as security-relevant changes.
