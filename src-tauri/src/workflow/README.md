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
| `execution_manager.rs` | Tauri execution-manager facade for host-local execution handles and stale-cleanup ownership. |
| `execution_manager/` | Focused execution-state lifecycle helpers behind the public execution-manager facade. |
| `event_adapter.rs` | Stable facade that bridges `node-engine` workflow events onto Tauri channels. |
| `event_adapter/` | Focused translation and diagnostics-bridge helpers behind the stable event-adapter facade. |
| `event_adapter/tests/` | Focused adapter regression tests split by translation/projection, channel transport, and executor integration behavior. |
| `workflow_edit_session.rs` | Backend-owned workflow edit-session graph operations surfaced through thin Tauri wrappers. |
| `types.rs` | Legacy Rust DTO mirrors retained during migration; core DTOs are the source of truth for new editing surfaces. |
| `model_dependencies.rs` | Dependency preflight, binding resolution, and runtime-environment selection for Python-backed models. |
| `python_runtime.rs` | Process-backed Python adapter that resolves venv-specific interpreters and launches workflow workers. |
| `diagnostics/` | Backend-owned diagnostics contracts, trace projection helpers, and in-memory overlay/store state for workflow UI snapshots. |
| `headless_diagnostics.rs` | Backend-owned diagnostics projection and trace/scheduler snapshot adaptation for headless workflow transport. |
| `headless_diagnostics_transport.rs` | Host-facing diagnostics, trace, and history snapshot responses shared by workflow commands and runtime debug surfaces. |
| `headless_runtime.rs` | Shared host-resource composition for backend-owned embedded workflow runtime construction. |
| `headless_workflow_commands_tests.rs` | Shared fixtures and module index for headless workflow command diagnostics, trace, scheduler, runtime metadata, and transport tests. |
| `headless_workflow_commands_tests/` | Focused headless workflow command tests split by diagnostics helper recording, transport responses/errors, and diagnostics projection/storage behavior. |

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
- Tauri execution-handle lifecycle and undo/redo projection must stay thin
  wrappers around backend-owned `node-engine` behavior rather than becoming a
  second owner of workflow-session policy.
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
task execution for workflow/orchestration paths. The legacy Tauri-local
connection-intent, effective-definition, and validation modules have been
removed; graph compatibility, dynamic definition overlays, and connection
rejection semantics now belong to `pantograph-workflow-service`. Workflow
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
`event_adapter/` now splits pure node-engine-to-Tauri event translation from
diagnostics-store bridge logic so event-contract completion work does not grow
inside one oversized adapter file or blur the transport-versus-backend
ownership boundary.
`event_adapter/tests/` now splits backend event translation/projection,
channel-transport, and executor-backed integration coverage so adapter
regressions no longer accumulate in one large root test module.
Workflow event construction should stay with backend event translation or active
diagnostics snapshot emitters; `events.rs` should not grow unused convenience
constructors that bypass those owners.
`workflow_execution_tauri_commands.rs` now owns the Tauri-facing execution
entrypoints, while `workflow_execution_commands.rs` remains a thin command-
group facade over `workflow_execution_runtime.rs` and `workflow_edit_session.rs`
so edit-session graph operations and runtime execution sequencing stop growing
inside the general command root.
`workflow_execution_runtime.rs` uses grouped execution, session, and runtime
state inputs for internal orchestration. Tauri command entrypoints retain their
framework-injected state signatures for registration compatibility, with scoped
lint expectations documenting that boundary exception instead of propagating
long positional argument lists through runtime helpers.
The legacy Tauri-local node registry mirror has also been removed; definition
commands now use the service-owned registry directly.
The legacy Tauri-local execution manager has been removed; undo/redo and
session execution state now stay with the backend-owned workflow service rather
than a parallel desktop state map.
The legacy Tauri-local workflow type mirror has also been removed; graph,
connection, node-definition, file, and port DTOs should come from
`pantograph-workflow-service` so command payloads do not drift from backend
contracts.
`headless_diagnostics_transport.rs` owns the host-facing diagnostics snapshot,
trace snapshot, and clear-history responses so runtime debug commands and
workflow command wrappers do not depend on the broader headless workflow
session adapter.
That diagnostics snapshot path may forward additive backend-owned
workflow-session inspection state, but it must not reconstruct node-memory or
checkpoint policy inside the Tauri adapter.
When the planned `RuntimeRegistry` is introduced, this directory should request
registry-backed runtime operations through injected host state while keeping
policy ownership outside the Tauri adapter boundary. Execution-path runtime
snapshot overrides should be projected into the shared registry by backend
helpers in `crates/pantograph-embedded-runtime`, not by local Tauri sequencing.
Stored diagnostics-runtime replay and runtime-event projection sequencing
should likewise stay in backend helpers so the Tauri diagnostics transport
remains a thin caller.
Diagnostics projections now carry backend-authored context for requested
snapshot filters, event source workflow run id, relevant workflow run id, and
relevance, so GUI stores do not need adapter-local diagnostics event claiming.
Serialized workflow events also include a backend-authored `ownership` payload
that exposes event execution id, active execution id, and baseline relevance for
frontend execution reducers.
Workflow execution diagnostics emission must likewise route through a backend
helper that synchronizes the shared runtime registry before projecting the
execution snapshot, so `workflow_execution_runtime.rs` does not own a second
sync-before-snapshot sequence.
The legacy Tauri-local workflow persistence module has been removed; path
boundary tests now live with `FileSystemWorkflowGraphStore` in the workflow
service crate, which is the active owner for save/load/list behavior.

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
- Core workflow service is the canonical source of workflow persistence and
  filesystem path validation.
- Candidate discovery is source-anchor scoped and must not mutate the session.
- Commit commands must reject stale revisions and return structured rejection
  data for expected incompatibility cases.
- Insert-and-connect must mutate the session atomically so rejected inserts do
  not leave orphan nodes or disconnected edges.
- Node group create, ungroup, and port-mapping edits must call the core
  edit-session APIs and return core graph snapshots rather than using legacy
  Tauri group helpers as a second source of graph truth.
- Tauri editing commands must return the graph snapshots received from core
  rather than reconstructing local state.
- Diagnostics snapshot commands and event bridge emissions must preserve
  backend-authored projection context rather than requiring frontend-local
  execution-id claiming for diagnostics relevance.
- Workflow run commands must submit through the scheduler and return the
  scheduler-generated `workflow_run_id`; Tauri command payloads must not accept
  caller-authored run ids or workflow-name diagnostics side channels.
- Workflow-event serialization must include backend-authored ownership context
  for execution-scoped events so GUI reducers do not infer event execution ids
  from raw payload fields first.
- Session-scoped candidate and insert commands must log enough request/rejection
  context to diagnose release-only interaction failures without relying on
  browser-console access.
- Workflow command handlers may coordinate with the future `RuntimeRegistry`,
  but they must not become the long-lived owner of runtime residency or
  admission state.
- Workflow command handlers must preserve backend-owned workflow error
  envelopes for registry admission and runtime-unavailable failures rather than
  rewriting them into generic transport/internal errors.
- Headless workflow command diagnostics projection, trace, scheduler snapshot,
  runtime metadata, and clear-history tests stay under
  `headless_workflow_commands_tests/`, while
  `headless_workflow_commands_tests.rs` retains shared fixtures and module
  registration so `headless_workflow_commands.rs` remains focused on request
  orchestration.
- Tauri must not reintroduce local graph validation, connection-intent, or
  effective-definition policy; those behaviors belong in
  `pantograph-workflow-service`.
- Node definition discovery for graph editing and palettes must come from the
  service-owned registry or the active `node_engine::NodeRegistry` state, not a
  Tauri-local mirror.
- Undo/redo and execution session state must stay with
  `pantograph-workflow-service`; Tauri command handlers must not recreate a
  parallel execution-state manager.
- Workflow command/event adapters must use backend-owned workflow-service graph,
  connection, node-definition, file, and port DTOs rather than reintroducing
  Tauri-local mirrors.
- Python-backed execution stays out-of-process and is selected by resolved
  dependency `env_id`, not by frontend code.
- Bundle-capable model assets must resolve executable paths from Pumas
  execution descriptors rather than from raw library record paths.
- Task-type-derived backend selection must preserve distinct execution modes
  such as llama.cpp reranking rather than collapsing them into text generation.

## Revisit Triggers
- Core graph-edit contracts need a new transport projection that the current
  service DTOs cannot represent.
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

## API Consumer Contract
- Frontend callers must create or load an execution session before calling the
  session-scoped editing or execution commands in this directory. Raw graph
  execution commands are intentionally not registered as public Tauri invokes.
- `get_connection_candidates` accepts a source anchor and optional graph
  revision, and returns compatible existing targets plus insertable node types.
- `connect_anchors_in_execution` and `insert_node_and_connect_in_execution`
  require the revision used to derive UI state and return either an updated
  graph or a structured rejection.
- Node add/update/remove/move commands also return updated graph snapshots so
  the GUI can render backend-owned state directly.
- Group create, ungroup, and update-port commands are session-scoped graph
  mutations and return the same graph mutation response shape as node and edge
  edits.
- Expected incompatibility is not exceptional; transport/session errors still
  surface as command failures.
- Session-scoped commands are serialized per core edit session; callers should
  not assume mutations on one session block reads or edits on another session.
- Compatibility policy is additive: existing commands remain while new editing
  capabilities are introduced.
- Workflow dependency resolution and execution treat Pumas as the source of
  truth for executable model asset paths when bundle metadata requires it.

## Structured Producer Contract
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
- Tauri diagnostics collection should pass grouped embedded-runtime diagnostics
  input objects into the backend projection helpers rather than rebuilding long
  positional argument lists locally.
- Workflow execution runtime helpers should pass grouped execution/session and
  runtime-state inputs internally; expanded argument lists are only acceptable
  at Tauri command registration boundaries with a scoped reason.
- Workflow helper lint cleanup should remove needless adapter allocations while
  leaving model dependency, runtime shutdown, and diagnostics contracts owned by
  backend services or their existing command DTOs.
- Model dependency commands accept the backend-owned
  `ModelDependencyRequest` DTO as one request envelope and sanitize it at the
  helper boundary, rather than keeping parallel Tauri-only argument lists.
- Runtime, scheduler, and diagnostics snapshot events use named backend-owned
  input structs and boxed large internals so event constructors stay stable
  without changing the frontend event JSON shape.
- Headless diagnostics helpers use grouped projection and runtime snapshot
  inputs so scheduler/runtime/trace facts stay named as they cross the Tauri
  adapter boundary.
- When a scheduler snapshot includes backend-owned `workflow_run_id`,
  adapters must attribute runtime/scheduler snapshot events to that execution
  instead of falling back to `session_id`. If the field is absent, the adapter
  may only use the requested session identity or update overlay-only state; it
  must not infer a concrete run id locally.
- When backend or node-engine failures are cancellation-shaped, the adapter
  contract must already be explicit in backend-owned workflow events; Tauri may
  only forward `Cancelled` and preserve that outcome into diagnostics and trace
  projections instead of inferring it from generic failure strings.
- When `WaitingForInput`, `GraphModified`, or `IncrementalExecutionStarted`
  cross this directory, Tauri must preserve backend-owned execution identity,
  prompt/task semantics, dirty-task overlays, and incremental-resume task ids
  through both the translated workflow event DTOs and diagnostics projection
  path.
- When scheduler snapshots associate a run execution with a different workflow
  session id, adapters must preserve that backend-owned `session_id` on trace
  summaries and diagnostics projections rather than inferring the relationship
  in TypeScript.
- After restart, restore, cleanup, or replay-shaped transitions, this
  directory must resynchronize diagnostics and runtime-registry views from the
  backend-owned trace store and runtime-registry snapshot rather than keeping
  adapter-local recovery bookkeeping as a second source of truth.
- Duplicate terminal workflow events must likewise flow through to the
  backend-owned trace store and rely on its idempotent replay handling; this
  directory must not restamp or locally coalesce them into a second policy
  layer.
- When the backend trace store resets an execution into a new attempt, the
  diagnostics overlay for that execution must reset alongside it so stale
  progress text, node overlays, and retained event history do not leak across
  retries.
- The workflow event adapter must preserve that retry/reset behavior when
  translating `node-engine` events, so restarted executions remain one backend-
  owned trace with fresh attempt-local diagnostics state instead of a layered
  transport reconstruction.
- That reset rule also applies when the prior terminal state was cancellation;
  retries after cancellation must clear stale overlay state the same way as
  retries after failure or completion.
- `node.data.definition.inputs` and `node.data.definition.outputs` are additive
  port overlays used only when their `node_type` matches the containing node.
- `model_path` remains the workflow-facing field name, but for external bundle
  assets it must carry the Pumas execution descriptor `entry_path` so runtime
  consumers receive the executable root instead of the library stub directory.
- Pumas pipeline tags and node-type hints may classify reranking additively, but
  they must still resolve to executable backend/runtime metadata before
  execution starts.
