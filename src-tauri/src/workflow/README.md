# src-tauri/src/workflow

## Purpose
This directory contains Pantograph’s Tauri-side workflow transport and runtime
integration layer. It wires frontend commands onto the core
`pantograph-workflow-service` graph-edit/session APIs and hosts desktop-specific
execution concerns such as event streaming, dependency-aware runtime setup, and
process-backed task execution. As RuntimeRegistry work begins, this directory
must remain a consumer of app-owned runtime policy rather than becoming the
owner of that policy itself.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `commands.rs` | Shared Tauri workflow state aliases plus non-execution command registration entrypoints. |
| `workflow_execution_tauri_commands.rs` | Tauri execution/edit-session command entrypoints that forward to focused execution and graph-session helpers. |
| `workflow_execution_commands.rs` | Thin execution command-group facade that reuses focused runtime and edit-session helpers. |
| `workflow_execution_runtime.rs` | Desktop execution orchestration and diagnostics-emission helpers for edit-session workflow runs. |
| `workflow_edit_session.rs` | Backend-owned workflow edit-session graph operations surfaced through thin Tauri wrappers. |
| `connection_intent.rs` | Legacy local connection-intent implementation retained during migration; core now owns the canonical policy. |
| `effective_definition.rs` | Applies additive per-node `data.definition` overlays before legacy validation or candidate lookup reads port metadata. |
| `types.rs` | Legacy Rust DTO mirrors retained during migration; core DTOs are the source of truth for new editing surfaces. |
| `validation.rs` | Legacy local validation helpers retained during migration; core validation is authoritative for new editing surfaces. |
| `model_dependencies.rs` | Dependency preflight, binding resolution, and runtime-environment selection for Python-backed models. |
| `python_runtime.rs` | Process-backed Python adapter that resolves venv-specific interpreters and launches workflow workers. |
| `diagnostics/` | Backend-owned diagnostics contracts, trace projection helpers, and in-memory overlay/store state for workflow UI snapshots. |
| `headless_diagnostics.rs` | Backend-owned diagnostics projection and trace/scheduler snapshot adaptation for headless workflow transport. |
| `headless_diagnostics_transport.rs` | Host-facing diagnostics, trace, and history snapshot responses shared by workflow commands and runtime debug surfaces. |
| `headless_runtime.rs` | Shared host-resource composition for backend-owned embedded workflow runtime construction. |

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
- Runtime admission, reservation, retention, and eviction policy must not be
  implemented directly in Tauri workflow command handlers.
- Persisted graphs may contain additive `node.data.definition` port overlays for
  dynamic inference settings; the legacy Tauri path must interpret those the
  same way as core during the migration window.

## Decision
Delegate workflow graph editing, connection intent, undo/redo, and persistence
to `pantograph-workflow-service`. Tauri commands now translate invoke payloads
into core requests, return core graph snapshots to the GUI, and delegate
runtime execution through backend-owned helpers in
`crates/pantograph-embedded-runtime`. Desktop-specific code still injects event
channels, app state, and other host resources, but it no longer owns composite
task execution for workflow/orchestration paths. Where legacy local validation
or candidate lookup still exists, `effective_definition.rs` merges registry
metadata with additive per-node `inputs`/`outputs` overlays so dynamic
expand-setting ports behave the same way as the core service. Workflow
diagnostics projections now adapt backend-owned `WorkflowTraceStore` snapshots
from `pantograph-workflow-service`; the `diagnostics/` module now splits
contracts, trace/projection helpers, and retained overlay/store state by
concern so diagnostics logic does not accumulate in one transport file.
`headless_diagnostics.rs` owns the headless projection glue so
`headless_workflow_commands.rs` stays focused on request orchestration. Tauri
retains only projection-only overlays such as retained event history, progress
text, and runtime/scheduler snapshots.
`headless_runtime.rs` owns the desktop-side resource composition needed to
construct `pantograph-embedded-runtime` instances for headless workflow,
session, and orchestration entry points, keeping that host wiring out of
individual command files.
`workflow_execution_tauri_commands.rs` now owns the Tauri-facing execution
entrypoints, while `workflow_execution_commands.rs` remains a thin command-
group facade over `workflow_execution_runtime.rs` and `workflow_edit_session.rs`
so edit-session graph operations and runtime execution sequencing stop growing
inside the general command root.
`headless_diagnostics_transport.rs` owns the host-facing diagnostics snapshot,
trace snapshot, and clear-history responses so runtime debug commands and
workflow command wrappers do not depend on the broader headless workflow
session adapter.
When the planned `RuntimeRegistry` is introduced, this directory should request
registry-backed runtime operations through injected host state while keeping
policy ownership outside the Tauri adapter boundary. Execution-path runtime
snapshot overrides should be projected into the shared registry by backend
helpers in `crates/pantograph-embedded-runtime`, not by local Tauri sequencing.
Stored diagnostics-runtime replay and runtime-event projection sequencing
should likewise stay in backend helpers so the Tauri diagnostics transport
remains a thin caller.
Workflow execution diagnostics emission must likewise route through a backend
helper that synchronizes the shared runtime registry before projecting the
execution snapshot, so `workflow_execution_runtime.rs` does not own a second
sync-before-snapshot sequence.

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
- Workflow command handlers may coordinate with the future `RuntimeRegistry`,
  but they must not become the long-lived owner of runtime residency or
  admission state.
- Workflow command handlers must preserve backend-owned workflow error
  envelopes for registry admission and runtime-unavailable failures rather than
  rewriting them into generic transport/internal errors.
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
- RuntimeRegistry introduction requires a distinct host-state injection path or
  changes the lifetime of workflow runtime helpers.

## Dependencies
**Internal:** `pantograph-workflow-service`, node-engine workflow types, Tauri
command registration, mirrored frontend contracts.

**External:** Tauri command runtime and serde serialization.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- Reason: graph editing is now part of the supported core service boundary.
- Reason: workflow commands will consume runtime-registry operations but must
  stay transport/adaptation code rather than the owner of runtime policy.
- Revisit trigger: the Tauri runtime host itself needs an ADR-level split from
  transport command wiring or workflow execution leaves the Tauri app boundary.

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
- When `node-engine` emits additive `occurred_at_ms` timestamps on workflow
  events, this directory must forward those timestamps into backend-owned trace
  storage instead of replacing them with adapter-local observation time.
- Runtime and scheduler snapshots carried through this directory must preserve
  the distinction between backend/runtime producer facts and future
  RuntimeRegistry policy decisions.
- When a scheduler snapshot includes backend-owned `trace_execution_id`,
  adapters must attribute runtime/scheduler snapshot events to that execution
  instead of falling back to `session_id`. If the field is absent, the adapter
  may only use the requested session identity or update overlay-only state; it
  must not infer a concrete run id locally.
- When backend or node-engine failures are cancellation-shaped, the adapter
  must emit explicit cancelled workflow events and preserve that outcome into
  diagnostics and trace projections instead of collapsing it into a generic
  failure badge.
- When scheduler snapshots associate a run execution with a different workflow
  session id, adapters must preserve that backend-owned `session_id` on trace
  summaries and diagnostics projections rather than inferring the relationship
  in TypeScript.
- After restart, restore, cleanup, or replay-shaped transitions, this
  directory must resynchronize diagnostics and runtime-registry views from the
  backend-owned trace store and runtime-registry snapshot rather than keeping
  adapter-local recovery bookkeeping as a second source of truth.
- `node.data.definition.inputs` and `node.data.definition.outputs` are additive
  port overlays used only when their `node_type` matches the containing node.
- `model_path` remains the workflow-facing field name, but for external bundle
  assets it must carry the Pumas execution descriptor `entry_path` so runtime
  consumers receive the executable root instead of the library stub directory.
- Pumas pipeline tags and node-type hints may classify reranking additively, but
  they must still resolve to executable backend/runtime metadata before
  execution starts.
