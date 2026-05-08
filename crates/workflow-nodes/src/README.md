# crates/workflow-nodes/src

Built-in workflow node descriptor and task source boundary.

## Purpose
This directory owns the built-in node definitions, composed-node
registrations, and task implementations registered into `node-engine`. It
keeps node metadata, port contracts, and runtime task behavior grouped by
workflow node family.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Crate export surface and built-in descriptor registration wiring. |
| `contracts.rs` | Canonical primitive contract projection plus composed authoring registrations for built-in workflow nodes. |
| `setup.rs` | Node registration helpers and optional host extension setup, including explicit Pumas selector access roles. |
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
- Disabled or experimental nodes must be explicitly documented until removed or
  completed.

## Decision
Group built-in nodes by workflow role and expose them through crate-level
registration helpers. Keep node descriptors and task implementations near each
other so graph metadata and runtime behavior can be reviewed together.

Composed authoring surfaces are exported through
`builtin_composed_node_contracts()`. The current built-in composed
registration is `tool-loop`, which maps its stable external contract onto
primitive `llm-inference`, `tool-executor`, and turn-state control nodes so
diagnostics can preserve primitive execution facts.

## Alternatives Rejected
- Put all node implementations in one file: rejected because node families have
  distinct runtime and contract concerns.
- Let frontend define built-in node metadata: rejected because backend execution
  and saved workflow compatibility require backend-owned descriptors.

## Invariants
- Node descriptor metadata must match task input/output behavior.
- Built-in node ids, port ids, categories, and data types are compatibility
  contracts.
- Composed registrations use `pantograph-node-contracts` mapping DTOs and must
  preserve primitive trace policy.
- Saved templates must not rely on frontend-only aliases for backend ports.
- Experimental control/tool nodes must not be presented as complete execution
  behavior while tool execution is disabled.
- Descriptor/task structs should derive standard defaults when the declared
  empty configuration exactly matches Rust's derived field defaults.

## Revisit Triggers
- Node definitions move to a generated registry format.
- Built-in node families need separate crates.
- Tool-loop/tool-executor behavior gains a backend-owned tool runtime or is
  removed from the descriptor set.

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
- Composition: `builtin_composed_node_contracts()` returns backend-owned
  mappings for stable composed authoring surfaces.
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
- Inference descriptors publish graph-authoring task and port payload metadata
  for canonical request/result families. The current `llm-inference`
  projection derives task fields from `inference::model_contracts`, then
  annotates text/chat, embedding, rerank, executable audio transcription, and
  executable image-generation payloads plus graph-visible generated-image,
  text/chat usage, and cache-handle outputs without changing frontend
  rendering, scheduler policy, or runtime backend selection.
- Canonical `llm-inference` declares `task_kind` and `runtime_hint` as
  optional graph-visible inputs because saved-workflow migration, preflight,
  and execution already consume those fields as canonical node data.
- Direct `diffusion-inference` is retired from the built-in descriptor
  inventory. New image-generation authoring must use canonical
  `llm-inference` with `task_kind = image_generation`; old direct-diffusion
  stubs are not graph-visible compatibility targets.
- Pumas model selectors use an explicit selector-access extension role:
  owner `PumasApi`, local `PumasLocalClient`, or read-only
  `PumasReadOnlyLibrary`; selector queries must not reconstruct that role from
  raw `PUMAS_API` extension injection.
  Read-only access must be opened against the model-library root that contains
  `models.db`, not a launcher/source root.
  The same selector-access adapter owns model-library update feed handoff:
  owner access lists updates directly, local-client access converts the Pumas
  subscription recovery handshake into an update feed, and read-only access
  reports update-feed unavailability without starting lifecycle work.
  Summary snapshot and single-summary reads also route through the adapter:
  owner access may call Pumas summary APIs directly, while local-client and
  read-only access project bounded selector-row summaries.
  The adapter also owns selected-model detail hydration: owner and local-client
  roles use Pumas batch detail APIs, while read-only access may project a
  bounded selector row without claiming package-summary or inference-settings
  detail ownership. Local-client selected-detail coverage uses an IPC fixture
  to prove the adapter calls the same Pumas batch detail methods rather than
  reconstructing owner API access.

## Testing
```bash
cargo test -p workflow-nodes --lib
```

## Notes
- Tool-loop and tool-executor now fail explicitly when tool execution is
  required; backend-owned tool runtime implementation remains future work.
