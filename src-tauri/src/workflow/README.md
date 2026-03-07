# src-tauri/src/workflow

## Purpose
This directory contains Pantograph’s Rust-side workflow editing and execution
layer. It owns command handlers, session-backed graph mutation, execution
plumbing, and the canonical connection-eligibility rules that the frontend calls
through Tauri.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `connection_intent.rs` | Canonical candidate-discovery and revision-aware connection-commit logic for interactive graph editing. |
| `commands.rs` | Tauri command registration for workflow editing APIs. |
| `workflow_execution_commands.rs` | Session-oriented command implementations used by the frontend graph editor. |
| `types.rs` | Rust DTOs mirrored into the TypeScript workflow contracts. |
| `validation.rs` | Shared lower-level workflow validation helpers. |
| `task_executor.rs` | Runtime execution path for workflow node execution once editing commits are accepted. |

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
for `get_connection_candidates` and `connect_anchors_in_execution`. The command
path computes eligible targets from live session state, uses graph fingerprints
for stale-intent detection, and returns structured rejection reasons instead of
boolean-only failure.

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
- `workflow_execution_commands.rs` must refresh derived graph metadata when it
  returns graphs to the frontend.

## Revisit Triggers
- Insert-and-connect becomes a first-class backend command.
- Headless editing moves to a transport boundary outside Tauri invoke.
- Eligibility rules expand enough to justify a dedicated policy module or ADR.

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
```

## API Consumer Contract (Host-Facing Modules)
- Frontend callers must create or load an execution session before calling the
  session-scoped editing commands in this directory.
- `get_connection_candidates` accepts a source anchor and optional graph
  revision, and returns compatible existing targets plus insertable node types.
- `connect_anchors_in_execution` requires the revision used to derive UI state
  and returns either an updated graph or a structured rejection.
- Expected incompatibility is not exceptional; transport/session errors still
  surface as command failures.
- Compatibility policy is additive: existing commands remain while new editing
  capabilities are introduced.

## Structured Producer Contract (Machine-Consumed Modules)
- `ConnectionCandidatesResponse` always includes `graph_revision`,
  `revision_matches`, `source_anchor`, `compatible_nodes`, and
  `insertable_node_types`.
- `ConnectionCommitResponse` uses `accepted` plus optional `graph`/`rejection`
  rather than overloading `Result` for expected validation failure.
- Rejection enums are stable snake_case labels shared with TypeScript.
- Graph fingerprints are regenerated metadata; callers must not persist them as
  durable workflow configuration.
