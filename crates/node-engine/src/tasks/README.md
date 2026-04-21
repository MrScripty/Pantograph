# crates/node-engine/src/tasks

Graph context key helper boundary.

## Purpose
This directory owns `ContextKeys`, the helper used by task implementations to
build stable graph context keys for inputs, outputs, streams, and metadata.
Task implementations themselves now live in `workflow-nodes`.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | `ContextKeys` helper and key convention tests. |

## Problem
Task implementations need consistent string keys when reading and writing graph
context. If every task built those keys ad hoc, workflow execution and tests
would drift.

## Constraints
- Context key formats are consumed by task implementations and tests.
- This module must stay lightweight and free of workflow-node implementation
  dependencies.
- Key format changes are breaking for task code and serialized/debug output.

## Decision
Keep context key construction in `node-engine` and keep task implementations in
`workflow-nodes`. This preserves one key convention while avoiding a reverse
dependency from the engine onto built-in node implementations.

## Alternatives Rejected
- Build context key strings directly in every task: rejected because repeated
  format strings drift.
- Move `ContextKeys` to `workflow-nodes`: rejected because custom task
  implementations also need engine-owned key conventions.

## Invariants
- Input keys use `{task_id}.input.{port}`.
- Output keys use `{task_id}.output.{port}`.
- Stream keys use `{task_id}.stream.{port}`.
- Metadata keys use `{task_id}.meta.{field}`.

## Revisit Triggers
- Context storage moves away from string keys.
- Task ids or port ids become structured typed keys.
- External task SDKs need generated key helpers.

## Dependencies
**Internal:** node-engine context and task execution conventions.

**External:** This helper uses only the Rust standard library.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
use node_engine::ContextKeys;

let prompt_key = ContextKeys::input("inference_1", "prompt");
```

## API Consumer Contract
- Inputs: task ids, port ids, and metadata field names.
- Outputs: stable context key strings.
- Lifecycle: helpers are pure and stateless.
- Errors: helpers do not validate ids; callers are responsible for valid task
  and port names.
- Versioning: key format changes require coordinated task, test, and saved
  diagnostics migrations.

## Structured Producer Contract
- Stable fields: generated context key strings are machine-consumed by graph
  execution and task code.
- Defaults: no implicit defaults are applied.
- Enums and labels: key segment labels `input`, `output`, `stream`, and `meta`
  are semantic.
- Ordering: not applicable to single key construction.
- Compatibility: task implementations across crates rely on this format.
- Regeneration/migration: key format changes require all task implementations,
  tests, and diagnostics consumers to migrate together.

## Testing
```bash
cargo test -p node-engine tasks
```

## Notes
- Keep task behavior in `workflow-nodes`; this directory is only key
  convention support.
