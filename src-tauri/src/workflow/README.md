# src-tauri/src/workflow

## Purpose
This directory contains Pantograph’s Rust-side workflow editing and execution
layer. It owns command handlers, session-backed graph mutation, execution
plumbing, and the canonical connection-eligibility rules that the frontend calls
through Tauri.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `connection_intent.rs` | Canonical candidate-discovery, revision-aware connection commits, and atomic insert-and-connect logic for interactive graph editing. |
| `commands.rs` | Tauri command registration for workflow editing APIs. |
| `workflow_execution_commands.rs` | Session-oriented command implementations used by the frontend graph editor. |
| `types.rs` | Rust DTOs mirrored into the TypeScript workflow contracts. |
| `validation.rs` | Shared lower-level workflow validation helpers. |
| `task_executor.rs` | Runtime execution path for workflow node execution once editing commits are accepted. |
| `model_dependencies.rs` | Dependency preflight, binding resolution, and runtime-environment selection for Python-backed models. |
| `python_runtime.rs` | Process-backed Python adapter that resolves venv-specific interpreters and launches workflow workers. |

## Problem
Pantograph previously exposed mostly pairwise connection validation. The frontend
now needs backend-owned candidate discovery, structured rejection reasons, and
revision-aware commit semantics so GUI and headless-style consumers follow one
eligibility model.

## Constraints
- Editing commands must operate against session-backed graph state.
- Rust DTOs must stay aligned with the mirrored TypeScript contracts.
- Expected incompatibility must not be reported as transport failure.
- Existing public facades such as `validate_connection` must keep working during
  the migration.

## Decision
Add a dedicated `connection_intent.rs` module and expose additive Tauri commands
for `get_connection_candidates`, `connect_anchors_in_execution`, and
`insert_node_and_connect_in_execution`. The command path computes eligible
targets from live session state, uses graph fingerprints for stale-intent
detection, and returns structured rejection reasons instead of boolean-only
failure. Workflow execution in this directory also owns dependency-aware,
process-backed Python execution for nodes such as `pytorch-inference`,
`diffusion-inference`, `audio-generation`, and `onnx-inference`. Execution
state is now stored as a per-session async mutex behind the global session map
so long-running mutation commands serialize within one session without blocking
lookups or edits for every other session. Session-scoped candidate lookup and
insert commands now also emit release-visible tracing at the command boundary so
interactive horseshoe failures can be diagnosed from `--run-release` logs.

## Alternatives Rejected
- Extend `workflow_get_io` to cover graph-editing intent.
  Rejected because workflow I/O surfaces are for execution boundaries, not
  internal graph editing.
- Keep the frontend as the primary source of compatibility truth.
  Rejected because capacity, cycle, and stale-revision checks depend on backend
  session state.

## Invariants
- `connection_intent.rs` is the canonical source of connection eligibility.
- Candidate discovery is source-anchor scoped and must not mutate the session.
- Commit commands must reject stale revisions and return structured rejection
  data for expected incompatibility cases.
- Insert-and-connect must mutate the session atomically so rejected inserts do
  not leave orphan nodes or disconnected edges.
- `workflow_execution_commands.rs` must refresh derived graph metadata when it
  returns graphs to the frontend.
- Session-scoped candidate and insert commands must log enough request/rejection
  context to diagnose release-only interaction failures without relying on
  browser-console access.
- The global execution registry lock must not be held across awaited graph
  mutations; only the per-session execution lock may span awaited workflow work.
- Python-backed execution stays out-of-process and is selected by resolved
  dependency `env_id`, not by frontend code.
- Bundle-capable model assets must resolve executable paths from Pumas
  execution descriptors rather than from raw library record paths.

## Revisit Triggers
- Headless editing moves to a transport boundary outside Tauri invoke.
- Eligibility rules expand enough to justify a dedicated policy module or ADR.
- Insert ranking/placement heuristics need a dedicated policy boundary.

## Dependencies
**Internal:** node-engine workflow types, session execution manager, Tauri
command registration, mirrored frontend contracts.

**External:** Tauri command runtime and serde serialization.

## Related ADRs
- None.
- Reason: the connection-intent change is still local to the workflow editing
  subsystem.
- Revisit trigger: editing/session APIs become a supported external embedding
  contract with explicit versioning.

## Usage Examples
```rust
let response = connection_intent::commit_connection(
    &workflow_registry,
    &mut execution.graph,
    source_anchor,
    target_anchor,
    &graph_revision,
);

let inserted = connection_intent::insert_node_and_connect(
    &workflow_registry,
    &mut execution.graph,
    source_anchor,
    "text-output",
    None,
    Position { x: 480.0, y: 160.0 },
    &graph_revision,
);
```

## API Consumer Contract (Host-Facing Modules)
- Frontend callers must create or load an execution session before calling the
  session-scoped editing commands in this directory.
- `get_connection_candidates` accepts a source anchor and optional graph
  revision, and returns compatible existing targets plus insertable node types.
- `connect_anchors_in_execution` and `insert_node_and_connect_in_execution`
  require the revision used to derive UI state and return either an updated
  graph or a structured rejection.
- Expected incompatibility is not exceptional; transport/session errors still
  surface as command failures.
- Session-scoped commands are serialized per execution, not across the entire
  workflow registry; callers should not assume mutations on one session block
  reads or edits on another session.
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
- Graph fingerprints are regenerated metadata; callers must not persist them as
  durable workflow configuration.
- `model_path` remains the workflow-facing field name, but for external bundle
  assets it must carry the Pumas execution descriptor `entry_path` so runtime
  consumers receive the executable root instead of the library stub directory.
