# crates/workflow-nodes/src/output

Workflow output node implementations.

## Purpose
This directory owns built-in terminal output nodes that publish workflow results
from graph context into typed output ports.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | Output-node module exports and registration wiring. |
| `text_output.rs` | Text output node behavior and metadata. |
| `image_output.rs` | Image output node behavior and metadata. |
| `audio_output.rs` | Audio output node behavior and metadata. |
| `vector_output.rs` | Vector/embedding output node behavior and metadata. |
| `component_preview.rs` | Generated component preview output node behavior and metadata. |
| `point_cloud_output.rs` | Point-cloud output node behavior and metadata. |

## Problem
Output nodes define the graph boundary that frontend consumers and workflow
sessions read after execution. Their port names and value shapes must stay
aligned with UI presenters and service output binding contracts.

## Constraints
- Output port ids are saved workflow and execution binding contracts.
- Terminal values must be JSON-serializable or otherwise backend-projectable.
- Media output nodes must preserve metadata needed by frontend playback and
  display components.

## Decision
Keep output node descriptors and task behavior grouped here. Treat output node
ports as public workflow contracts consumed by templates, sessions, and UI
output components.

## Alternatives Rejected
- Let frontend output components define port contracts: rejected because backend
  execution and saved workflows must own output binding truth.
- Collapse all output nodes into one generic node: rejected because media and
  component outputs carry distinct metadata and presenter expectations.

## Invariants
- Output node metadata must match frontend output presenters.
- Audio/image/component/point-cloud values must preserve any metadata required
  for playback, rendering, or preview.
- Output bindings must remain stable across workflow-service and frontend
  migrations.

## Revisit Triggers
- Output binding DTOs are versioned.
- More media outputs need shared metadata helpers.
- Generated component preview output moves to a dedicated plugin boundary.

## Dependencies
**Internal:** `node-engine`, `graph-flow`, workflow service output binding
contracts, and frontend output components.

**External:** `serde_json` and `async-trait`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
let task = TextOutputTask::new("text-output-1");
```

## API Consumer Contract
- Inputs: context values for declared output input ports.
- Outputs: typed context values consumed by workflow service output bindings.
- Lifecycle: output tasks run during graph execution and publish terminal
  values for the current run.
- Errors: missing or incompatible required inputs should return graph execution
  errors.
- Versioning: output node type ids and port ids must migrate with templates and
  frontend presenters.

## Structured Producer Contract
- Stable fields: output node type ids, port ids, data types, media metadata
  labels, and descriptor execution modes are machine-consumed.
- Defaults: optional output metadata defaults must match frontend presenter
  expectations.
- Enums and labels: output type ids and data type labels carry behavior.
- Ordering: output arrays should preserve graph/output target ordering where
  exposed.
- Compatibility: saved workflows and templates may reference output node ids
  across releases.
- Regeneration/migration: descriptor changes require workflow service,
  frontend presenter, template, saved workflow, and tests updates together.

## Testing
```bash
cargo test -p workflow-nodes --lib output
```

## Notes
- Keep output metadata additions additive unless all consumers migrate together.
