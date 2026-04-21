# crates/workflow-nodes/src

Built-in workflow node descriptor and task source boundary.

## Purpose
This directory owns the built-in node definitions and task implementations
registered into `node-engine`. It keeps node metadata, port contracts, and
runtime task behavior grouped by workflow node family.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Crate export surface and built-in descriptor registration wiring. |
| `setup.rs` | Node registration helpers used by hosts and tests. |
| `input/` | User/model input node task definitions and metadata. |
| `processing/` | Inference, transformation, dependency, and model-processing nodes. |
| `output/` | Terminal output node definitions for text, image, audio, vector, component, and point-cloud values. |
| `storage/` | File and KV-cache persistence nodes. |
| `control/` | Conditional, merge, tool-loop, and tool-executor control-flow nodes. |
| `system/` | Process/system task nodes. |
| `tool/` | Agent tool node descriptors and helper contracts. |

## Problem
Workflow graph execution depends on stable node type ids, port ids, and task
behavior. If built-in nodes are not documented as contracts, frontend templates,
saved workflows, and backend execution can drift.

## Constraints
- Node type ids and port ids are serialized in saved workflows and templates.
- Task behavior must use backend-owned context keys and `node-engine` metadata.
- Runtime-family-specific behavior should stay in focused node families.
- Placeholder or experimental nodes must be explicitly documented until removed
  or completed.

## Decision
Group built-in nodes by workflow role and expose them through crate-level
registration helpers. Keep node descriptors and task implementations near each
other so graph metadata and runtime behavior can be reviewed together.

## Alternatives Rejected
- Put all node implementations in one file: rejected because node families have
  distinct runtime and contract concerns.
- Let frontend define built-in node metadata: rejected because backend execution
  and saved workflow compatibility require backend-owned descriptors.

## Invariants
- Node descriptor metadata must match task input/output behavior.
- Built-in node ids, port ids, categories, and data types are compatibility
  contracts.
- Saved templates must not rely on frontend-only aliases for backend ports.
- Experimental control/tool nodes must not be presented as complete execution
  behavior until M2 resolves their placeholder path.

## Revisit Triggers
- Node definitions move to a generated registry format.
- Built-in node families need separate crates.
- Tool-loop/tool-executor behavior is either completed or removed.

## Dependencies
**Internal:** `node-engine`, `graph-flow`, backend inference/runtime crates, and
Pantograph workflow templates.

**External:** `serde`, `async-trait`, `inventory`, and node-family-specific
runtime dependencies.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
let mut extensions = node_engine::ExecutorExtensions::new();
workflow_nodes::setup_extensions(&mut extensions).await;
```

## API Consumer Contract
- Inputs: graph context values keyed by task id and port id.
- Outputs: context values, stream events, and task metadata consumed by
  `node-engine`, workflow service, frontend templates, and saved workflows.
- Lifecycle: descriptors are registered during host setup; task instances run
  during graph execution.
- Errors: task failures should use `GraphError` categories that workflow
  service/adapters can project.
- Versioning: node type ids and port ids should change only with coordinated
  workflow/template migrations.

## Structured Producer Contract
- Stable fields: node type ids, labels, categories, port ids, data types,
  required/multiple flags, and execution modes are machine-consumed.
- Defaults: descriptor defaults must match task behavior and frontend template
  assumptions.
- Enums and labels: node categories, execution modes, backend ids, and task
  labels carry behavior.
- Ordering: descriptor registration ordering should remain deterministic where
  displayed.
- Compatibility: saved workflows and templates may reference descriptors across
  releases.
- Regeneration/migration: descriptor changes require frontend registry,
  template, saved workflow, and tests updates in the same slice.

## Testing
```bash
cargo test -p workflow-nodes --lib
```

## Notes
- Tool-loop and tool-executor completion remains tracked under M2.
