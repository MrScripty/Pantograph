# src-tauri/src/workflow

## Purpose
This directory contains Pantograph’s Tauri-side workflow transport and runtime
integration layer. It wires frontend commands onto the core
`pantograph-workflow-service` graph-edit/session APIs and hosts desktop-specific
execution concerns such as event streaming, dependency-aware runtime setup, and
process-backed task execution.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `commands.rs` | Tauri command registration for workflow editing and execution APIs. |
| `workflow_execution_commands.rs` | Thin Tauri wrappers that delegate graph edits to core and execute runtime snapshots for desktop event streaming. |
| `connection_intent.rs` | Legacy local connection-intent implementation retained during migration; core now owns the canonical policy. |
| `effective_definition.rs` | Applies additive per-node `data.definition` overlays before legacy validation or candidate lookup reads port metadata. |
| `types.rs` | Legacy Rust DTO mirrors retained during migration; core DTOs are the source of truth for new editing surfaces. |
| `validation.rs` | Legacy local validation helpers retained during migration; core validation is authoritative for new editing surfaces. |
| `task_executor.rs` | Runtime execution path for workflow node execution once editing commits are accepted. |
| `model_dependencies.rs` | Dependency preflight, binding resolution, and runtime-environment selection for Python-backed models. |
| `python_runtime.rs` | Process-backed Python adapter that resolves venv-specific interpreters and launches workflow workers. |

## Problem
Pantograph’s standalone GUI still needs a native bridge, but graph editing can
no longer live in Tauri because headless and embedded clients need the same
editing contract. Desktop-specific code must stop owning workflow graph state
while still handling desktop runtime execution concerns.

## Constraints
- Tauri must not own canonical graph mutation or persistence rules.
- Editing commands must delegate to the host-agnostic core service.
- Rust DTOs must stay aligned with the mirrored TypeScript contracts.
- Expected incompatibility must not be reported as transport failure.
- Existing public facades such as `validate_connection` must keep working during
  the migration.
- Persisted graphs may contain additive `node.data.definition` port overlays for
  dynamic inference settings; the legacy Tauri path must interpret those the
  same way as core during the migration window.

## Decision
Delegate workflow graph editing, connection intent, undo/redo, and persistence
to `pantograph-workflow-service`. Tauri commands now translate invoke payloads
into core requests, return core graph snapshots to the GUI, and keep only the
desktop-specific runtime execution path here. Execution still owns
dependency-aware, process-backed Python execution for nodes such as
`pytorch-inference`, `diffusion-inference`, `audio-generation`, and
`onnx-inference`. Where legacy local validation or candidate lookup still
exists, `effective_definition.rs` merges registry metadata with additive
per-node `inputs`/`outputs` overlays so dynamic expand-setting ports behave the
same way as the core service. Workflow diagnostics projections now adapt
backend-owned `WorkflowTraceStore` snapshots from
`pantograph-workflow-service`; Tauri retains only projection-only overlays such
as retained event history, progress text, and runtime/scheduler snapshots.

## Alternatives Rejected
- Extend `workflow_get_io` to cover graph-editing intent.
  Rejected because workflow I/O surfaces are for execution boundaries, not
  internal graph editing.
- Keep the frontend as the primary source of compatibility truth.
  Rejected because capacity, cycle, and stale-revision checks depend on backend
  session state.

## Invariants
- Core workflow service is the canonical source of graph mutation and connection
  eligibility.
- Candidate discovery is source-anchor scoped and must not mutate the session.
- Commit commands must reject stale revisions and return structured rejection
  data for expected incompatibility cases.
- Insert-and-connect must mutate the session atomically so rejected inserts do
  not leave orphan nodes or disconnected edges.
- Tauri editing commands must return the graph snapshots received from core
  rather than reconstructing local state.
- Session-scoped candidate and insert commands must log enough request/rejection
  context to diagnose release-only interaction failures without relying on
  browser-console access.
- Legacy local validation must treat `node.data.definition` as an additive
  overlay on top of registry metadata, not as an unchecked replacement for the
  node type contract.
- Python-backed execution stays out-of-process and is selected by resolved
  dependency `env_id`, not by frontend code.
- Bundle-capable model assets must resolve executable paths from Pumas
  execution descriptors rather than from raw library record paths.
- Task-type-derived backend selection must preserve distinct execution modes
  such as llama.cpp reranking rather than collapsing them into text generation.

## Revisit Triggers
- Legacy local graph-edit helpers are removed after all callers use core-owned
  contracts.
- Desktop execution needs a reusable host implementation outside Tauri.
- Insert ranking/placement heuristics need a dedicated policy boundary.

## Dependencies
**Internal:** `pantograph-workflow-service`, node-engine workflow types, Tauri
command registration, mirrored frontend contracts.

**External:** Tauri command runtime and serde serialization.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- Reason: graph editing is now part of the supported core service boundary.
- Revisit trigger: the Tauri runtime host itself needs an ADR-level split from
  transport command wiring.

## Usage Examples
```rust
let response = workflow_service
    .workflow_graph_connect(request)
    .await?;

let snapshot = workflow_service
    .workflow_graph_get_runtime_snapshot(&session_id)
    .await?;
```

## API Consumer Contract (Host-Facing Modules)
- Frontend callers must create or load an execution session before calling the
  session-scoped editing commands in this directory.
- `get_connection_candidates` accepts a source anchor and optional graph
  revision, and returns compatible existing targets plus insertable node types.
- `connect_anchors_in_execution` and `insert_node_and_connect_in_execution`
  require the revision used to derive UI state and return either an updated
  graph or a structured rejection.
- Node add/update/remove/move commands also return updated graph snapshots so
  the GUI can render backend-owned state directly.
- Expected incompatibility is not exceptional; transport/session errors still
  surface as command failures.
- Session-scoped commands are serialized per core edit session; callers should
  not assume mutations on one session block reads or edits on another session.
- Compatibility policy is additive: existing commands remain while new editing
  capabilities are introduced.
- Workflow dependency resolution and execution treat Pumas as the source of
  truth for executable model asset paths when bundle metadata requires it.

## Structured Producer Contract (Machine-Consumed Modules)
- `ConnectionCandidatesResponse` always includes `graph_revision`,
  `revision_matches`, `source_anchor`, `compatible_nodes`, and
  `insertable_node_types`.
- `ConnectionCommitResponse` uses `accepted` plus optional `graph`/`rejection`
  rather than overloading `Result` for expected validation failure.
- Rejection enums are stable snake_case labels shared with TypeScript.
- Graph fingerprints and returned graph snapshots come from core; Tauri must not
  invent or persist adapter-owned edit metadata.
- Canonical run/node lifecycle timing for diagnostics must come from
  `WorkflowTraceStore`; Tauri may only adapt that trace data into the existing
  GUI projection shape and attach additive UI-only overlay fields.
- `node.data.definition.inputs` and `node.data.definition.outputs` are additive
  port overlays used only when their `node_type` matches the containing node.
- `model_path` remains the workflow-facing field name, but for external bundle
  assets it must carry the Pumas execution descriptor `entry_path` so runtime
  consumers receive the executable root instead of the library stub directory.
- Pumas pipeline tags and node-type hints may classify reranking additively, but
  they must still resolve to executable backend/runtime metadata before
  execution starts.
