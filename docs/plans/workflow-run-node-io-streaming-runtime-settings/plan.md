# Plan: Workflow Run Node IO, Streaming, And Runtime Settings

## Objective

Restore Pantograph workflow run inspection so retained executed runs expose
per-node inputs, outputs, status, timing, cache state, and artifacts through the
existing retention policy, ArtifactStore, diagnostics ledger, and run-detail
projection. Fix text generation streaming so connected text outputs update
during execution and retain final generated text. Add backend-owned inference
runtime settings so local GGUF/llama.cpp execution can use GPU-capable device
configuration instead of being effectively CPU-bound with no user control.
Keep backend service/domain crates as the source of truth for execution facts,
runtime settings, and retained run inspection; Tauri remains an IPC/app-shell
adapter and must not become the authoritative contract owner.

## Scope

### In Scope

- Audit the existing node-engine, embedded-runtime, diagnostics ledger,
  ArtifactStore, run-detail projection, and graph page path before further code
  changes.
- Restore existing per-node IO retention/projection rather than creating a
  parallel storage system.
- Ensure cached node executions still produce inspectable node IO records with
  explicit cache status.
- Ensure text generation live chunks route to the connected text output using
  the canonical `response` path.
- Ensure final text generation output is retained as `response` and appears in
  node IO/run detail after completion.
- Add backend-owned llama.cpp runtime settings for device/offload and CPU
  behavior, with Pumas defaults and Pantograph workflow/run overrides.
- Add or refine a backend-owned run-inspection read model when the frontend
  currently has to compose multiple projections for one executed run view.
- Capture runtime settings in workflow run snapshots and diagnostics ledger
  events.
- Add vertical-slice tests before broad horizontal refactors.
- Update touched module READMEs or ADRs when contracts change.
- Add refactor/decomposition steps for oversized files when touched, using
  logical responsibility splits rather than treating 500 lines as a hard limit.

### Out of Scope

- Replacing the diagnostics ledger, ArtifactStore, retention policy, or
  run-centric workbench projection architecture.
- Adding a new node IO database outside existing projection/retention ownership.
- Implementing vLLM, MLX, or PyTorch device settings beyond preserving a clean
  backend-specific extension point.
- Changing Pumas into a Pantograph-specific runtime-policy provider.
- Persisting raw token stream chunks as final output JSON.
- Supporting backwards compatibility with retired Pantograph workflow behavior.

## Inputs

### Problem

User testing of the Maid workflow shows:

- The workflow reports completion, but the executed run graph page shows no
  inspectable inputs or outputs for any node.
- The text output node has no retained generated text.
- Connecting LLM `stream` to text output is normalized toward text output ports,
  but the executed run still has no inspectable node IO.
- Text streaming is not visible during generation.
- Local inference runtime settings are not user-configurable, leaving the
  workflow effectively bound to CPU behavior with no GPU/offload controls.

This is inconsistent with Pantograph's intended execution-platform design:
run snapshots, retention policy, ArtifactStore, diagnostics ledger projections,
and run-centric graph inspection already exist and should be the source of
truth.

### Current Codebase Findings

Initial read-only search found that the target functionality is partially
implemented and should be repaired through existing ownership boundaries:

- Diagnostics ledger already has `IoArtifactObserved` events,
  `IoArtifactRole::NodeInput`, `IoArtifactRole::NodeOutput`, and
  `IoArtifactProjectionRecord` with producer/consumer node and port fields.
- Workflow execution currently records workflow-level IO artifacts for
  submitted inputs and terminal outputs, but the inspected code path only emits
  `workflow_input` and `workflow_output` records from session execution.
- Node-engine already resolves execution inputs, emits `TaskCompleted` with
  serialized output maps, and records input snapshots, but cached-output return
  currently happens before completion/event projection work.
- Llama.cpp streaming already emits `TaskStream` on the canonical `response`
  port and final completed output is `response`; the likely missing slice is
  scheduler-run event delivery/frontend routing/reconciliation rather than a
  new stream port.
- Frontend run graph and IO inspector already query run graph snapshots, node
  statuses, and IO artifact projections. They do not need a parallel node IO
  store.
- Runtime device settings already exist at several layers as `device`,
  `gpu_layers`, and `context_size`, but workflow llama.cpp execution currently
  starts with `device = "auto"` and llama-server startup ignores the configured
  context size in favor of the default constant.

### Source Evidence From Code Search

- `crates/pantograph-diagnostics-ledger/src/event.rs` defines
  `RunDetailProjectionRecord`, `IoArtifactProjectionRecord`, and
  `IoArtifactRole::{NodeInput, NodeOutput, WorkflowInput, WorkflowOutput}`.
  This is the existing projection surface to extend or query.
- `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`
  records submitted workflow inputs and terminal workflow outputs as
  `DiagnosticEventPayload::IoArtifactObserved`, but the inspected path does
  not emit every resolved node input/output as `node_input`/`node_output`.
- `crates/node-engine/src/engine/execution_core.rs` resolves dependency inputs,
  computes input versions, records input snapshots, and stores completed
  output maps. Its cached-output path can return before the normal completion
  and event projection path.
- `crates/node-engine/src/engine/execution_events.rs` carries completed output
  maps in `TaskCompleted`, which is enough for the first retained-output
  vertical slice.
- `crates/node-engine/src/core_executor/llamacpp_nodes.rs` emits
  `WorkflowEvent::task_stream(..., "response", ...)` and returns final
  `response` output. This supports keeping `response` as the canonical text
  output instead of inventing a new retained stream output.
- `src-tauri/src/workflow/event_adapter/translation.rs` maps `TaskStream` to
  `NodeStream`, and `src/components/workflowToolbarEvents.ts` handles
  `NodeStream` and `NodeCompleted` runtime updates. The streaming bug should
  first be treated as delivery, ownership, or graph-handle reconciliation.
- `crates/inference/src/backend/mod.rs`, `src-tauri/src/config.rs`, and
  `src/components/DeviceConfig.svelte` already expose parts of runtime device
  configuration. `crates/inference/src/server.rs` starts llama-server with the
  default context size instead of `BackendConfig.context_size`, and
  `llamacpp_nodes.rs` currently supplies `device = "auto"` for workflow runs.

### Constraints

- Follow
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.
- Existing retention and ArtifactStore systems are authoritative.
- Backend service/domain crates are the source of truth for saved run facts,
  retained node IO, streaming semantics, and execution/runtime settings. Tauri
  can expose, translate, and transport backend contracts, but Tauri must not
  define final semantics for retained run facts, runtime settings, node IO, or
  streaming.
- Frontend must render backend projections and must not infer missing persisted
  run facts locally.
- Scheduler owns admission and runtime reservation policy. The inference crate
  owns backend execution contracts and runtime request shaping, not scheduling.
- Pumas is the canonical model source and can provide model/package defaults;
  Pantograph owns final workflow/run execution settings.
- Runtime settings that affect execution must be captured in immutable run
  snapshots.
- Streaming events must be bounded and must not put large/raw media bodies in
  event JSON.
- Existing unrelated dirty files must remain untouched.

### Assumptions

- Existing retention policy is intended to cover per-node IO for executed run
  inspection.
- Existing run-detail projection plus existing IO artifact projection queries
  are the correct read path for the executed run graph page.
- `RunDetailProjectionRecord` does not need inline node IO fields unless the
  first vertical slice proves the current artifact projection cannot serve the
  graph page. Avoid duplicating `IoArtifactProjectionRecord` facts in multiple
  persisted records.
- Existing ArtifactStore descriptors are the correct representation for large
  or binary node IO values.
- Workflow-level IO artifact recording is already present and must remain
  distinct from per-node IO artifact recording.
- `TaskCompleted` output maps and resolved node inputs are available inside the
  node execution path; the repair should capture them there or at the nearest
  existing event/projection boundary.
- Text generation final output should remain `response`; text streaming should
  be a live event path associated with that canonical response.
- First runtime-settings slice should target llama.cpp because the blocking
  user workflow uses a GGUF model.

### Standards-Driven Simplification Findings

- One node IO evidence adapter should own conversion from resolved node
  inputs, completed outputs, cache status, node identity, and run context into
  existing `IoArtifactObserved` events. This avoids duplicating event-building
  logic across execution, workflow-service, Tauri, and frontend code.
- Inspectable small text/JSON node values should be retained through the
  existing ArtifactStore path when policy allows. Metadata-only ledger rows are
  useful for audit counts and hashes, but they are not sufficient when the run
  graph must show the value to the user.
- Artifact identity is run-scoped. Because a Pantograph run executes once,
  stable IDs based on run/role/node/port are the correct default for retained
  node IO inside one run. Cross-run comparison belongs in workflow history read
  models keyed by workflow/workflow version plus run ids, not in per-run
  artifact identity.
- Runs are immutable and execute once. Any "rerun" language in this plan means
  a new run submitted from the same workflow that may reuse cached node
  results, not a second execution attempt inside an existing run.
- The executed run graph should prefer a backend-owned read model if it needs
  graph snapshot, node statuses, and IO artifacts together. That read model
  must compose existing projections and must not add a second persistence
  system.
- The backend run-inspection read model must remain factual and presentation
  neutral. Backend owns graph/status/artifact facts, stable IDs, projection
  freshness, retention state, handles, and errors. Frontend owns layout,
  grouping, labels, visual hierarchy, selected node state, panels, empty
  states, and how those facts are displayed.
- Streaming is a delivery mode for a logical output, not a different persisted
  output identity. Live chunks can update clients and UI during execution, but
  final retained artifacts come from completed node outputs such as
  `response`; connecting `stream` instead of `response` must not change
  retained run inspection artifacts.
- Runtime setting semantics must live in backend service/inference contracts.
  Tauri may transport settings and expose commands, but it is not the source of
  truth for validation, defaults, or effective execution behavior.
- Oversized files touched by this work need decomposition review. The 500-line
  standard is a review trigger, not a hard limit; split only when there is a
  clear responsibility boundary that improves maintenance and testing.

### Design Quality Findings

- Run inspection should be descriptor-first. The graph page needs enough
  metadata to show status, ports, retention state, sizes, hashes, previews, and
  artifact handles, but it should not eagerly load every artifact body for the
  whole run. Artifact bodies should be fetched on demand through existing
  artifact read/stream APIs.
- Retained node IO should have explicit preview limits. Small text/JSON values
  can be materialized for inspection, but every inline value or preview needs a
  named byte/character limit, truncation metadata, and a full artifact handle
  when policy allows full retrieval.
- Run-inspection queries should be keyed by `workflow_run_id` and avoid
  frontend-driven N+1 reads across graph, status, artifact, and body endpoints.
  A backend read model is justified when it reduces those repeated calls while
  still returning descriptors instead of large bodies.
- Stream chunks are high-frequency delivery events. They should be sequenced,
  scoped to run/node/port, and coalesced or throttled at the frontend boundary
  when needed so rendering does not become token-rate work. Durable retention
  should store final `response` output and, at most, bounded stream summary
  metadata.
- Run-scoped artifact identity should represent the final logical fact for a
  port. If a port value is a collection, keep one stable port fact that points
  to a manifest or bounded item descriptors rather than inventing execution
  attempt identity.
- Runtime settings should have one normalized backend contract and one
  effective-settings snapshot. UI controls, Tauri IPC, workflow defaults, Pumas
  defaults, and run overrides should all converge into that backend contract
  instead of maintaining parallel setting semantics.
- Prefer additive repair of existing event/projection paths over new adapters
  unless the audit proves a boundary is missing. The plan should not create a
  new framework for node IO, streaming, or runtime settings.

### Standards Groups Reviewed

- `PLAN-STANDARDS.md`: requires objective, scope, ordered milestones,
  verification, re-plan triggers, affected contracts/artifacts, lifecycle
  review, dirty-worktree discipline, and explicit worker write sets when
  parallel implementation is used.
- `CODING-STANDARDS.md`: requires backend-owned data, single owners for
  stateful flows, bounded production diagnostics, validation at boundaries, DRY
  event/materialization logic, and decomposition review for large files.
- `ARCHITECTURE-PATTERNS.md`: requires executable boundary contracts where
  persisted/process-boundary DTOs can drift, structured producer-consumer
  semantics, composition-root lifecycle ownership, and read models over
  transient workflow internals.
- `FRONTEND-STANDARDS.md`: requires declarative UI state, event-driven
  synchronization where feasible, deterministic cleanup for any polling, and
  tests for gesture-heavy or reactive workflow controls touched by the work.

### Blast Radius Controls

- Primary source write areas must stay tied to the active vertical slice:
  `crates/node-engine`, `crates/pantograph-embedded-runtime`,
  `crates/pantograph-workflow-service`,
  `crates/pantograph-diagnostics-ledger`, `crates/inference`,
  `src-tauri/src/workflow`, `src/services/workflow`, and the graph/run
  inspection frontend components.
- `crates/node-engine` may expose normalized execution evidence for resolved
  inputs, completed outputs, stream events, and cache status. It must not own
  diagnostics persistence, retention policy, or ArtifactStore writes.
- `crates/pantograph-workflow-service` should own retained IO materialization,
  ArtifactStore use, run-inspection read-model assembly, and calls into the
  diagnostics ledger because it already owns run/session service contracts.
- `crates/pantograph-diagnostics-ledger` should change only when existing event
  payloads, projections, or query APIs cannot represent the needed facts. Prefer
  existing `IoArtifactObserved` roles before adding schema.
- `crates/inference` should own backend runtime settings, validation, effective
  settings snapshots, and llama.cpp config/argument application. It must not
  own scheduler admission policy or Pumas model-source semantics.
- `src-tauri` is limited to IPC/event transport, command adaptation, and app
  shell wiring. It must not become the authoritative DTO or validation owner.
- Frontend services and components may mirror backend DTOs, render
  presentation-specific view models, and own UI-only state. They must not
  fabricate missing backend facts or persist runtime settings locally.
- Explicitly outside this plan's blast radius: Pumas contracts or
  implementation, scheduler admission policy beyond consuming backend runtime
  facts, broad saved-workflow compatibility shims, unrelated backend support
  beyond extension points, and new persistence systems.

### Decomposition Targets

- If `session_execution_api.rs` is touched for node IO, extract event
  construction or retained-value materialization when the code would otherwise
  mix session orchestration, ArtifactStore writes, and diagnostics payload
  building in one large function.
- If diagnostics query APIs are touched, keep projection records and read-model
  assembly separate. Extract run-inspection query assembly when it becomes a
  distinct responsibility from lower-level ledger reads.
- If `llamacpp_nodes.rs` or `server.rs` is touched for runtime settings, prefer
  a small llama.cpp runtime/config builder over spreading argument/default
  rules across node execution and process startup code.
- If `GraphPage` or related frontend services are touched, move only
  presentation-neutral composition to backend read models. Frontend extraction
  should focus on view-model/presenter boundaries, not recreating backend data
  ownership in TypeScript.

### Dependencies

- `crates/node-engine/src/engine/`: dependency input resolution, demand
  execution, output cache, execution events, workflow execution sessions.
- `crates/pantograph-embedded-runtime/src/`: workflow host execution,
  embedded task executor, diagnostics/event adapter integration, runtime
  configuration.
- `crates/pantograph-workflow-service/src/workflow/`: run submission,
  retention policy, ArtifactStore, run snapshots, run-detail projection,
  artifact descriptors, scheduler/session APIs.
- `crates/pantograph-workflow-service/src/trace/` and
  `crates/pantograph-diagnostics-ledger/`: typed event ledger and materialized
  projections.
- `src-tauri/src/workflow/`: Tauri event adapter, diagnostics overlay,
  workflow commands, retained run projection bridge.
- `src/services/workflow/` and `src/components/`: TypeScript DTO mirrors,
  graph/run page, toolbar event routing, output node display.
- `crates/inference/src/`: llama.cpp backend config, managed runtime state,
  runtime lifecycle snapshots, generation/runtime request contracts.
- Pumas Library model package facts and selected model summaries.

### Affected Structured Contracts

- Node-engine `TaskStarted`, `TaskCompleted`, `TaskStream`, and cached-output
  execution events.
- Embedded-runtime workflow execution response and run output collection.
- Workflow-service run snapshot graph/runtime/settings payloads.
- Diagnostics ledger event payloads for node IO, runtime settings, retention,
  and streaming summaries, especially existing `IoArtifactObserved` records
  with `node_input` and `node_output` roles.
- Run-detail projection DTOs consumed by frontend graph/run pages.
- ArtifactStore descriptor metadata for retained node IO values.
- Llama.cpp runtime configuration DTOs and TypeScript command mirrors.

### Affected Persisted Artifacts

- Workflow run snapshots.
- Diagnostics ledger event rows and materialized run-detail projection rows.
- ArtifactStore descriptor metadata and retained artifact bodies.
- Managed runtime state only where runtime settings are stored or projected.
- Saved workflow graph data only where current node settings are explicitly
  stored as workflow defaults.

### Concurrency and Lifecycle Review

- Execution events can arrive while the frontend is switching active runs or
  graph pages. Backend projection records must remain keyed by
  `workflow_run_id`, `node_id`, and event sequence.
- Node execution may be cached. Cached paths must still emit or project
  inspectable node IO with explicit cache state and without pretending the node
  executed freshly.
- Streaming events may arrive before final `TaskCompleted`. UI must append live
  text from backend events and then reconcile to retained final output.
- Runtime settings may affect managed runtime restart/load behavior. The
  implementation must define who starts/stops runtime processes, when setting
  changes trigger restart, and how stale runtime instances are rejected.
- Scheduler reservations and inference runtime loading must not split ownership
  of runtime state. Scheduler selects/admits; inference runtime validates and
  applies backend config.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Workflow-level IO is wired but per-node IO artifact roles are not populated | High | Use existing `IoArtifactObserved` node roles and run graph artifact queries before adding any data shape. |
| Run-detail UI reads a projection that omits node IO despite ledger events existing | High | Add backend projection tests and frontend DTO tests against real projection fixtures. |
| Cached execution skips node IO events | High | Add cache-specific acceptance coverage because the cached-output branch can return before normal completion processing. |
| Streaming delivery is broken despite canonical `response` emission | High | Trace event adapter, scheduler-run transport, and toolbar routing before changing backend port contracts. |
| Runtime device settings accidentally become scheduler policy | High | Keep settings as execution/backend configuration; scheduler may inspect requirements but does not own backend-specific config semantics. |
| Existing runtime setting fields are present but not honored | High | First wire `device`, `gpu_layers`, and `context_size` end to end before adding thread/batch/ubatch fields. Verify `context_size` becomes llama-server `-c`, `gpu_layers` becomes `-ngl`, and `device` becomes the llama.cpp device argument when supported. |
| Tauri or frontend becomes de facto source of truth for backend contracts | High | Keep contract DTOs and validation in backend service/inference crates; Tauri only transports and adapts them. |
| Frontend has to compose too many projections for one run view | Medium | Add a factual backend read model that joins existing projections without duplicating persisted facts when GraphPage needs graph, status, and IO together. Keep display semantics in frontend. |
| Run-inspection queries eagerly load artifact bodies or create N+1 reads | High | Return descriptor-first run inspection data keyed by run id; load artifact bodies lazily through existing artifact APIs. |
| Token-rate stream events cause excessive frontend rendering or ledger writes | High | Sequence and scope stream events, coalesce UI updates where needed, and retain only final `response` plus bounded summary metadata. |
| Collection outputs force unstable artifact identity | Medium | Keep one run/role/node/port logical fact and represent collections as manifests or bounded item descriptors. |
| GPU settings vary by platform/backend | Medium | First slice targets llama.cpp generic settings: GPU layers/offload mode and context; add thread/batch/ubatch after the current config path is proven. |
| Large Rust or frontend files grow past standards thresholds | Medium | Treat 500 lines as a review trigger, not a hard limit. Split when there is a clear responsibility boundary; otherwise document why the current shape remains safe for that slice. |
| Artifact retention redacts or drops values needed for debugging | Medium | Record bounded JSON control-plane values inline and large/binary values as descriptors; expose retention state clearly. |

## Definition of Done

- A retained workflow run graph shows every executed node with status, timing,
  resolved inputs, outputs, artifacts, and cache state according to retention
  policy.
- A new run that reuses cached node output still shows node IO and explicitly
  reports cached output use.
- The Maid-style `Puma-Lib -> llm-inference -> text-output` path produces
  retained generated text in the run detail.
- Text generation streaming visibly updates the connected text output during
  execution and final retained output reconciles to the completed `response`.
- Llama.cpp runtime settings expose at least CPU thread count, GPU layer/offload
  configuration, context length, and batch/ubatch where supported by current
  backend config.
- Existing `device`, `gpu_layers`, and `context_size` settings are applied by
  workflow llama.cpp execution and visible in diagnostics before additional
  llama.cpp settings are added.
- Runtime settings are captured in run snapshots and diagnostics events.
- Runtime settings contracts and effective-setting snapshots are owned by
  backend service/inference crates, with Tauri acting only as transport.
- Frontend graph/run inspection reads backend-owned run-detail projections
  rather than transient runtime node data.
- The executed run graph can request one backend-owned inspection model for the
  run view when separate graph/status/artifact calls would create avoidable UI
  race and composition logic.
- Run-inspection responses are descriptor-first and do not eagerly read every
  artifact body. Full bodies remain available through existing artifact
  read/stream APIs when retention policy allows.
- Stream delivery does not make rendering or durable ledger writes scale with
  unbounded token-rate work.
- All touched Rust and frontend contracts have focused tests plus at least one
  vertical-slice acceptance test.
- Touched module READMEs or ADRs are updated when public, persisted, or
  host-facing contracts change.

## Milestones

### Milestone 1: Existing Retention And Projection Audit

**Goal:** Prove exactly where node inputs/outputs disappear in the existing
retention and run-detail path before implementation changes.

**Tasks:**
- [ ] Complete the retained-run trace through node-engine execution events,
  embedded-runtime event adaptation, diagnostics ledger writes, run-detail
  projection drain, and frontend graph page read.
- [ ] Complete the Maid-style graph trace through the same path, including cached and
  non-cached execution.
- [x] Search for existing retention, ArtifactStore, diagnostics ledger,
  run graph, streaming, and runtime setting surfaces.
- [x] Identify that workflow-service currently records workflow-level
  `workflow_input` and `workflow_output` artifacts, while durable per-node
  `node_input` and `node_output` capture still needs implementation or wiring.
- [x] Identify that node-engine has resolved inputs and completed outputs
  available at execution time, but the cached-output branch can bypass normal
  completion projection.
- [x] Identify that llama.cpp already emits streaming chunks on `response` and
  final output as `response`.
- [x] Identify that runtime config fields exist but workflow llama.cpp
  execution currently hardcodes `device = "auto"` and llama-server startup uses
  the default context size rather than `BackendConfig.context_size`.
- [x] Identify whether node inputs are never captured, captured but not
  persisted, persisted but not projected, projected but not rendered, or
  filtered by retention policy.
- [x] Identify whether node outputs and artifacts fail at a different layer
  than inputs.
- [ ] Identify current retention policy defaults and whether they intentionally
  suppress node IO.
- [ ] Identify whether current artifact IDs intentionally represent
  latest-value-per-run/node/port, or whether attempts/cache hits need distinct
  artifact identity.
- [x] Record source findings in this plan's execution notes with file/function
  references.
- [ ] Confirm whether the retained-run graph should consume node IO through an
  existing composite run-detail DTO, direct artifact query calls, or a small
  backend read-model that joins run detail and IO artifact projections without
  duplicating persisted facts.
- [ ] Identify whether GraphPage currently composes graph, node status, and IO
  artifact projections in frontend state in a way that should move behind a
  backend run-inspection read model.
- [x] Record any discovered standards violations, stubs, or dead projection
  paths as follow-up tasks in this plan before editing source code.

**Verification:**
- Read-only audit summary identifies the first broken hop for inputs and the
  first broken hop for outputs.
- At least one concrete existing test or missing-test gap is identified for the
  first implementation slice.
- No source code edits in this milestone.

**Status:** In progress

### Milestone 2: Standards Contract Freeze And Simplification Gate

**Goal:** Freeze the smallest backend-owned contracts and simplification steps
needed before implementation spreads across execution, diagnostics, runtime,
Tauri, and frontend code.

**Tasks:**
- [ ] Define one internal node IO evidence adapter contract that accepts
  resolved inputs, completed outputs, cache state, node identity, and run
  context, then emits existing `IoArtifactObserved` events.
- [x] Define one retained IO value materialization rule: bounded text/JSON
  values that must be inspectable are written through ArtifactStore; large,
  binary, redacted, or policy-suppressed values become descriptors or
  metadata-only records with explicit retention reasons.
- [ ] Document run-scoped artifact identity semantics:
  one final retained input/output fact per run/role/node/port. Do not add
  attempt/event artifact identity unless Pantograph later supports multiple
  executions of the same node inside one run.
- [ ] Document collection-output semantics: if one port produces multiple
  files/items, retain one stable port fact that points to a manifest or bounded
  item descriptors instead of introducing attempt identity or unstable IDs.
- [ ] Define a backend-owned run-inspection read model if Milestone 1 confirms
  the executed graph page needs graph snapshot, node statuses, and IO artifacts
  together. The read model must join existing projections without creating a
  second persisted store.
- [ ] Define the run-inspection performance shape: run-id-keyed query,
  descriptor-first response, no eager artifact body loads, no frontend N+1
  query pattern, and lazy artifact body retrieval through existing artifact
  APIs.
- [ ] Keep the run-inspection read model presentation-neutral. It may expose
  factual relationships and stable identifiers, but it must not define cards,
  badges, colors, display labels, panel grouping, visual ordering, or selected
  node behavior.
- [ ] Define one backend-owned llama.cpp runtime settings contract and
  validation path. Tauri and frontend may display or transport this contract
  but must not define the final setting semantics.
- [ ] Define the effective-settings snapshot shape once, including source
  attribution for each effective value where useful: Pumas default, workflow
  default, run override, backend default, or backend validation adjustment.
- [ ] Freeze the implementation blast radius for the first source slice:
  identify each touched file as contract owner, orchestration owner,
  persistence owner, transport adapter, or presentation consumer before
  editing it.
- [ ] Define executable boundary validation for persisted, IPC, and
  process-boundary DTOs touched by node IO, run inspection, streaming, or
  runtime settings.
- [x] Record decomposition choices for touched oversized files. Split by
  logical ownership, such as IO event building, retained value materialization,
  llama.cpp config building, or run-inspection read-model assembly. Do not
  split files just to satisfy a numeric line count.
- [ ] Update this plan with any explicit standards exceptions where a touched
  file stays over 500 lines because a split would reduce clarity or create
  unstable boundaries.
- [ ] Record any planned parallel-worker write sets before spawning workers;
  shared contracts, persisted schemas, lockfiles, generated files, and DTO
  fixtures must have one explicit owner or be handled serially.

**Verification:**
- Contract freeze notes identify producer, consumer, persistence behavior, and
  validation owner for node IO, run inspection, and runtime settings.
- No Tauri-owned DTO is the authoritative source for backend execution facts or
  runtime setting semantics.
- Blast radius notes list each touched implementation area and confirm no
  source changes are planned outside the active vertical slice.
- Boundary DTOs have a runtime validation/normalization owner before source
  implementation begins.
- Decomposition review records either a split target or a reason to defer for
  every touched oversized file.

**Status:** Not started

### Milestone 3: First Vertical Slice For Retained Node IO

**Goal:** Restore the thinnest end-to-end retained run IO path using existing
retention and projection systems.

**Tasks:**
- [ ] Add a failing vertical-slice test for
  `text-input -> pass-through/mock-processing -> text-output` that asserts
  run-detail node status, inputs, outputs, and terminal workflow output.
- [x] Add or update a workflow-service projection test proving
  `IoArtifactObserved` records with `node_input` and `node_output` roles are
  queryable by run id and grouped by node for the run graph page.
- [ ] Route retained node IO through the Milestone 2 evidence adapter rather
  than adding ad hoc event construction at each execution or UI boundary.
- [ ] Repair the existing execution-event, ledger, projection, or frontend read
  path identified by Milestone 1.
- [x] Emit or project existing
  `DiagnosticEventPayload::IoArtifactObserved` records with
  `IoArtifactRole::NodeInput` and `IoArtifactRole::NodeOutput` for each
  executed node, using the existing workflow-level artifact path as the shape
  reference.
- [x] Ensure retained node inputs are the resolved execution inputs after graph
  dependency resolution and explicit user bindings, not stale graph node data.
- [x] Ensure retained node outputs are the actual executor outputs, not only
  terminal workflow outputs.
- [x] Preserve terminal `workflow_output` records as run outputs while adding
  distinct per-node `node_output` records for inspection.
- [x] Store small structured/text node IO through the retained IO value
  materialization rule. Inspectable values should be retrievable through
  ArtifactStore descriptors; metadata-only rows must carry explicit retention
  reasons.
- [ ] Apply named preview limits and truncation metadata for inline text/JSON
  previews. Do not inline unbounded values into diagnostics events or run
  inspection DTOs.
- [x] Project node IO through the existing `workflow_io_artifact_query` and
  run graph artifact-summary path before extending run-detail DTOs.
- [x] Carry resolved-input evidence through Tauri and frontend event transport
  as a handled no-op live event so execution ownership, diagnostics overlays,
  and consumer event stores stay exhaustive without redefining retained IO.
- [x] Ensure scheduler-session node IO and live node events are attributed to
  the backend workflow run id, not the transient workflow execution session id.
- [ ] Do not add node IO fields to `RunDetailProjectionRecord` unless the
  implementation proves a composite DTO is needed for read performance or API
  stability.
- [ ] Ensure retention policy gates node IO consistently and exposes the reason
  when IO is not retained.
- [x] Add bounded redaction/descriptor behavior for values too large or unsafe
  to retain inline.

**Verification:**
- New vertical-slice test fails before the fix and passes after the fix.
- Existing workflow-service run detail tests pass.
- Relevant Rust package checks pass for touched crates.
- Any frontend DTO/rendering changes include targeted TypeScript tests.

**Status:** Partially implemented. Executed node outputs now flow from
node-engine `TaskCompleted` events through embedded-runtime ledger projection
as `node_output` I/O artifacts, including retained small text/JSON ArtifactStore
bodies when available. Resolved node inputs now flow from node-engine
`TaskInputsResolved` events through the same ledger projection as `node_input`
I/O artifacts. Existing GraphPage read-side code already queries
`workflow_io_artifact_query` and summarizes node artifacts for the run snapshot.
Workflow-service projection coverage now proves `node_input` and `node_output`
records can be fetched together by run id and grouped by node for the run graph
page. Preview/truncation policy remains open.

### Milestone 4: Cached Execution IO And Artifact Retention

**Goal:** Ensure cached nodes and artifact-producing nodes remain inspectable in
executed run detail.

**Tasks:**
- [x] Add a new-run cache-hit test proving cached output use still produces
  inspectable node IO in run detail. The original run remains immutable and is
  not executed again.
- [x] Move or duplicate the necessary node IO event/projection work so cache
  hits cannot return before retained node output records are produced.
- [x] Ensure node-engine cache-hit completion events are explicitly marked as
  cache evidence, not fresh execution evidence.
- [x] Add or update cache status metadata: `fresh_execution`, `cache_hit`,
  `cache_invalidated`, or equivalent existing enum.
- [x] Project cache status from node completion events into durable run detail
  or artifact evidence where the run graph page needs it.
- [x] Verify ArtifactStore descriptors are retained for large/binary node IO
  and that run detail exposes lifecycle and retention state.
- [x] Verify retained artifact bodies are readable through existing artifact
  read/stream APIs when retention policy allows.
- [x] Ensure expired/deleted artifact bodies leave useful descriptor metadata in
  run detail.

**Verification:**
- Node-engine cache tests cover emitted cache status; workflow-service tests
  cover any later durable projection of that status.
- Workflow-service artifact retention tests cover node IO descriptors in run
  detail.
- Frontend IO inspector or graph run page tests cover descriptor display and
  missing-body states.

**Status:** Implemented for the current slice. Demand-engine cache hits now emit
`TaskCompleted` events with retained outputs and `cache_hit` status instead of
returning silently before node-output artifact retention. Fresh executor
completions emit `fresh_execution`. Durable node-status projection now carries
`fresh_execution`, `cache_hit`, and `cache_invalidated` from node completion
events into run detail and node-status queries, and the run graph page displays
the cache status when present. Descriptor-first large/binary node outputs are
covered by embedded-runtime tests that preserve ArtifactStore descriptor
metadata and read retained bodies through the existing artifact API. Oversized
inline node output values are covered as metadata-only records with size, hash,
retention reason, and no payload/read handle. Expired and deleted artifact
retention states are covered by workflow-service projection tests that preserve
artifact id, role/node filters, retention state, summary counts, and retention
reason after payload references are removed.

### Milestone 5: Text Generation Streaming Contract

**Goal:** Make live text generation visible while preserving final retained
`response` output.

**Tasks:**
- [x] Confirm llama.cpp currently emits text chunks on `TaskStream.port =
  "response"` and final completed output as `response`.
- [x] Audit scheduler-run event transport from node-engine `TaskStream` through
  embedded runtime, Tauri event adapter, frontend toolbar event handling, and
  active-run ownership filtering.
- [x] Verify scheduler-submitted retained runs use the same `NodeStream` path as
  toolbar/local execution, or define the smallest adapter needed to make them
  converge.
- [x] Keep backend stream semantics in backend/node-engine contracts. Tauri may
  translate events to frontend event names, but it must not redefine which port
  carries canonical text chunks.
- [x] Define the canonical text streaming rule: live chunks are associated with
  `response`; final retained text is `response`; `stream` remains a live/event
  or non-text/media stream surface, not the required text-output connection.
- [x] Define the persistence rule: choosing a live `stream` connection versus
  a `response` connection affects live delivery/display only, not final
  retained node output artifacts.
- [ ] Update backend stream emission only if a text backend other than
  llama.cpp emits on a noncanonical port.
- [ ] Update frontend event routing so a `response -> text` graph connection
  streams into a connected text output during execution.
- [ ] Verify dropping a stream-capable connection onto text output does not
  hide or discard the canonical final `response` path.
- [ ] Ensure final `TaskCompleted.response` reconciles the text output after
  streaming completes.
- [ ] Do not store raw token-by-token stream chunks as the only retained output;
  retain the final response and optional bounded stream summary.
- [ ] Add stream sequence/run/node/port reconciliation checks so stale stream
  events cannot update the wrong active run or node.
- [ ] Add frontend coalescing or throttling if direct token-rate updates create
  excessive renders; keep final `TaskCompleted.response` as the authoritative
  retained value.

**Verification:**
- Frontend test proves text stream chunks on `response` update a connected text
  output.
- Backend test proves llama.cpp/typed text stream events use the canonical
  response path.
- Vertical slice proves live text appears during execution and final run detail
  includes `response`.

**Status:** Not started

### Milestone 6: Llama.cpp Runtime Device Settings Slice

**Goal:** Add user-configurable llama.cpp execution settings without moving
runtime policy into Pumas or the frontend.

**Tasks:**
- [x] Audit current llama.cpp `BackendConfig`, managed runtime startup, and
  Pumas-selected inference settings.
- [x] Identify existing first-slice settings: `device`, `gpu_layers`, and
  `context_size`.
- [x] Identify current gaps: workflow llama.cpp execution hardcodes
  `device = "auto"`, and llama-server startup uses default context size rather
  than `BackendConfig.context_size`.
- [x] First wire existing `device`, `gpu_layers`, and `context_size` from
  backend-owned settings into workflow llama.cpp execution and sidecar startup.
- [ ] Move any authoritative runtime setting normalization out of Tauri-only
  config surfaces and into backend service/inference contracts where execution
  validation can own it.
- [x] Apply `context_size` to llama-server `-c`, `gpu_layers` to llama-server
  `-ngl`, and `device` to the supported llama.cpp device argument or validated
  no-op with a diagnostic when the installed binary does not support it.
- [ ] Define next-slice backend-owned settings: CPU threads, batch size, ubatch
  size, and any llama.cpp-specific validation required before exposing them.
- [ ] Define precedence: Pumas model defaults, workflow node defaults, run
  overrides, backend validation/defaults.
- [ ] Add settings to the canonical inference/runtime settings contract with
  backend-specific validation and bounded diagnostics.
- [ ] Capture effective runtime settings in run snapshots and diagnostics
  events.
- [ ] Capture effective settings with source attribution when available so
  operators can tell whether a value came from Pumas, workflow defaults, run
  overrides, backend defaults, or validation adjustment.
- [x] Ensure changing settings that require reload makes runtime readiness
  fail closed or restarts through the existing runtime lifecycle owner.
- [ ] Add frontend controls in the existing model/runtime settings surface, not
  ad hoc graph-only state.

**Verification:**
- Backend tests prove settings are parsed, validated, snapshotted, and applied
  to llama.cpp config.
- Frontend tests prove controls write backend-owned settings and render
  validation errors.
- Manual acceptance on a GGUF model confirms GPU/offload settings are visible
  in diagnostics and runtime startup metadata.

**Status:** Partially implemented. Existing llama.cpp `device`, `gpu_layers`,
and `context_size` settings are wired through workflow execution and sidecar
startup. Effective-settings diagnostics, source attribution, additional
settings, and frontend controls remain open.

### Milestone 7: Backend Run-Inspection Read Model

**Goal:** Simplify executed-run UI state by letting the backend assemble the
factual read model for a single run inspection view from existing persisted
projections while leaving display decisions in the frontend.

**Tasks:**
- [ ] Add a backend query shape only if Milestone 1 confirms GraphPage needs
  graph snapshot, node statuses, and IO artifacts together for one coherent
  executed-run view.
- [ ] Keep diagnostics ledger projections and ArtifactStore as the persisted
  sources. The new query must be a read composition, not a new persistence
  subsystem.
- [ ] Return factual data only: graph snapshot, node statuses, IO artifact
  records or summaries, retention state, handles, stable IDs, errors, and
  projection freshness.
- [ ] Return artifact descriptors, previews, and handles by default. Do not
  include full artifact bodies in the run-inspection read model unless a
  bounded inline preview limit explicitly allows it.
- [ ] Ensure the read model query is keyed by run id and performs bounded
  joins/queries rather than forcing the frontend to issue one query per node or
  one artifact-body read per port.
- [ ] Do not return presentation data such as cards, badges, colors, display
  labels, panel grouping, selected-node state, or frontend visual ordering.
- [ ] Include projection state/cursor information for each joined source so the
  frontend can tell whether the view is current or partially stale.
- [ ] Replace frontend-side multi-query composition in GraphPage only after the
  backend read model is covered by contract and frontend DTO tests.
- [ ] Preserve existing lower-level projection queries for diagnostics pages,
  filtering, and advanced inspection use cases.

**Verification:**
- Backend tests prove the read model returns graph, node statuses, and
  artifacts for the same workflow run id and reports projection freshness.
- Frontend tests prove GraphPage renders from the backend read model and no
  longer owns race-prone composition of those facts.
- Frontend tests prove display grouping/labels/empty states remain owned by
  frontend presenters/components rather than backend DTOs.

**Status:** Not started

### Milestone 8: Standards, Boundaries, And Regression Gate

**Goal:** Close the repair with standards-compliant boundaries and regression
coverage.

**Tasks:**
- [ ] Review touched Rust files against file-size and responsibility thresholds.
- [ ] Split oversized modules only when there is a stable logical ownership
  boundary; otherwise record the explicit decomposition decision and revisit
  trigger.
- [ ] Review the final diff against the blast radius controls: no
  Tauri-owned backend semantics, no frontend-fabricated persisted facts, no
  scheduler ownership of backend runtime settings, no Pumas implementation
  changes, and no new node IO persistence subsystem.
- [ ] Confirm every cross-boundary DTO touched by this work is produced,
  validated, normalized, and consumed from the documented owner rather than
  copied as an independent contract.
- [ ] Confirm backend run-inspection output remains presentation-neutral and
  frontend presenters/components own labels, grouping, visual order, selected
  node behavior, and empty states.
- [ ] Update module READMEs for host-facing APIs, structured producer
  contracts, and persisted DTOs touched by this work.
- [ ] Add or update ADR only if implementation changes ownership of retention,
  ArtifactStore, diagnostics ledger, scheduler, or runtime settings.
- [ ] Run focused backend and frontend suites plus release smoke where affected.
- [ ] Update this plan's execution notes, deviations, and verification summary.

**Verification:**
- `cargo fmt --all`
- Targeted `cargo test` suites for node-engine, embedded-runtime,
  workflow-service, diagnostics ledger, and inference touched by the work.
- Frontend typecheck/test suites for touched workflow graph/run pages.
- Release build or release smoke when Tauri/runtime settings are touched.
- Plan execution notes include final standards review results for
  `PLAN-STANDARDS.md`, `CODING-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`, and
  `FRONTEND-STANDARDS.md`.

**Status:** Not started

## Execution Notes

- 2026-05-08: Plan created after user testing showed completed workflow runs
  without retained per-node IO, missing visible text streaming, and missing
  inference device settings. No implementation source edits are part of this
  plan creation.
- 2026-05-08: Read-only search found existing systems to reuse:
  diagnostics-ledger `IoArtifactObserved`/`node_input`/`node_output`,
  workflow-service workflow-level IO artifact recording, node-engine resolved
  input/output availability, llama.cpp `response` stream emission, frontend
  `NodeStream` routing, run graph artifact queries, and shallow runtime config
  fields. The plan now treats missing per-node IO as a wiring/projection
  repair, not a new persistence system.
- 2026-05-08: Standards review added a contract-freeze/simplification gate,
  backend-source-of-truth constraint, backend run-inspection read-model
  milestone, and explicit decomposition guidance. The 500-line standard is
  treated as a review trigger; implementation should split files only at
  stable responsibility boundaries.
- 2026-05-08: Standards compliance pass added reviewed standards groups,
  blast-radius controls, decomposition targets, executable-boundary contract
  requirements, final diff review gates, and additional re-plan triggers for
  Pumas changes, diagnostics schema changes, retention-policy changes,
  presentation leakage into backend DTOs, and frontend-fabricated run facts.
- 2026-05-08: Software design quality pass added descriptor-first
  run-inspection performance constraints, lazy artifact body loading, bounded
  inline previews, stream sequencing/coalescing guidance, immutable-run wording
  for cache-hit tests, collection-output artifact identity, and one normalized
  effective-settings snapshot with source attribution.
- 2026-05-08: First implementation slice found the first durable node IO
  broken hop in workflow-service session completion: `TaskCompleted` and host
  run responses already carry output values, but
  `session_execution_api.rs::record_workflow_io_artifact_events_if_configured`
  only emitted `workflow_input` and `workflow_output` ledger events. The slice
  now records submitted external inputs as `node_input` and terminal outputs as
  `node_output`, and materializes bounded text/JSON bodies through
  ArtifactStore via `session_io_artifacts.rs` when configured. Full resolved
  per-node input capture remains open because the current node-engine
  completion event does not expose the resolved input map for every node.
- 2026-05-08: Decomposition review for the first slice extracted retained IO
  artifact metadata/materialization into `session_io_artifacts.rs` instead of
  growing `session_execution_api.rs` with ArtifactStore body and descriptor
  conversion details.
- 2026-05-08: Verification passed:
  `cargo test -p pantograph-workflow-service workflow::tests::session_execution::`.
- 2026-05-08: Llama.cpp runtime settings slice found that
  `BackendConfig.context_size` was included in KV-cache fingerprints but not in
  llama-server startup or sidecar reuse identity. The slice made inference
  sidecar `context_size` part of runtime identity, passes it to `llama-server
  -c`, maps workflow `device`, `gpu_layers`, and `context_size`/`context_length`
  settings into `BackendConfig`, and rejects reuse when runtime settings differ.
- 2026-05-08: Verification passed:
  `cargo test -p inference server::tests:: --features backend-llamacpp`,
  `cargo test -p inference backend::llamacpp::tests:: --features backend-llamacpp`,
  and `cargo test -p node-engine core_executor::llamacpp_nodes::tests:: --features inference-nodes`.
- 2026-05-08: Durable intermediate node-output slice now records node-engine
  `TaskCompleted` output port values through
  `NodeExecutionWorkflowLedgerSink` as `IoArtifactObserved(node_output)` events.
  Small text/JSON values are materialized into ArtifactStore bodies when a
  store is configured; otherwise the ledger receives metadata-only rows with
  retention reasons. The slice kept materialization helpers in
  `node_io_artifacts.rs` so `node_execution_ledger.rs` remains focused on
  event-to-ledger adaptation.
- 2026-05-08: Verification passed:
  `cargo test -p pantograph-embedded-runtime node_execution_ledger::tests:: --features backend-llamacpp`.
- 2026-05-08: Demand-engine cache-hit slice now emits `TaskCompleted` events
  with actual cached outputs and `TaskExecutionCacheStatus::CacheHit` before
  returning from cache. Fresh executor completions emit
  `TaskExecutionCacheStatus::FreshExecution`, and runtime transient node
  diagnostics preserve the completion cache status for graph/run consumers.
- 2026-05-08: Verification passed:
  `cargo test -p node-engine engine::tests::demand::`,
  `cargo test -p node-engine events::tests::`,
  `cargo test -p pantograph-embedded-runtime node_execution_diagnostics::tests:: --features backend-llamacpp`,
  and `cargo test -p pantograph-embedded-runtime node_execution_ledger::tests::node_execution_workflow_sink_records_task_completed_outputs_as_retained_node_artifacts --features backend-llamacpp`.
- 2026-05-08: Resolved node-input slice added node-engine
  `TaskInputsResolved` events after dependency input resolution and node-data
  preparation, including cache-hit replay from demand-engine input snapshots.
  Embedded runtime now projects those events into `IoArtifactObserved`
  `node_input` records using the same ArtifactStore materialization helper as
  node outputs.
- 2026-05-08: Verification passed:
  `cargo test -p node-engine engine::tests::demand::test_demand_cache_hit_emits_completed_outputs_with_cache_status`,
  `cargo test -p node-engine events::tests::`,
  `cargo test -p node-engine engine::tests::human_input::`,
  `cargo test -p node-engine engine::tests::demand::`,
  `cargo test -p pantograph-embedded-runtime node_execution_ledger::tests::node_execution_workflow_sink_records_resolved_inputs_as_retained_node_artifacts --features backend-llamacpp`,
  `cargo test -p pantograph-embedded-runtime node_execution_ledger::tests::node_execution_workflow_sink_records_task_completed_outputs_as_retained_node_artifacts --features backend-llamacpp`,
  and `cargo test -p pantograph-embedded-runtime node_execution_ledger::tests:: --features backend-llamacpp`.
- 2026-05-08: Read-side audit found the run graph page already uses
  `WorkflowProjectionService.queryIoArtifacts`, `buildRunGraphNodeArtifactSummaries`,
  and `RunGraphSnapshot` to show node I/O artifact summaries from the backend
  projection. No `RunDetailProjectionRecord` expansion is needed for the
  current node I/O summary path.
- 2026-05-09: Resolved-input transport slice added Tauri/frontend
  `NodeInputsResolved` handling for node-engine `TaskInputsResolved` events.
  The event is serialized with ownership projection, participates in backend
  diagnostics overlay summarization, and is explicitly handled as a frontend
  no-op for live node state so retained input evidence can flow without
  confusing it with progress, stream, or completion output.
- 2026-05-09: Verification passed:
  `node --experimental-strip-types --test packages/svelte-graph/src/stores/workflowExecutionEvents.test.ts`,
  `cargo test --manifest-path src-tauri/Cargo.toml workflow::event_adapter::tests::translation_projection::translated_task_inputs_resolved_event_preserves_inputs_without_trace_noise`,
  `cargo test --manifest-path src-tauri/Cargo.toml workflow::event_adapter::tests::`,
  and `cargo test --manifest-path src-tauri/Cargo.toml workflow::diagnostics::tests::`.
- 2026-05-09: Scheduler live-event slice found that the app-facing submit path
  used headless `workflow_run_execution_session` without a Tauri channel, while
  the existing `applyWorkflowToolbarEvent` stream handler was only reachable
  through subscribed workflow events. The slice adds a channel to scheduler
  session runs, forwards node-engine events through `TauriEventAdapter`, and
  keeps `response` as the backend-owned canonical text stream/output port.
- 2026-05-09: The same slice found a run-inspection attribution bug:
  embedded session execution only knew the workflow execution session id, so
  node-level ledger artifacts and live events could be keyed to the session
  instead of the backend-generated workflow run id. `WorkflowRunOptions` now
  carries the workflow run id, and embedded runtime uses it for node IO ledger
  attribution and live event ownership while still using the session id for
  warm executor residency.
- 2026-05-09: Decomposition review for the scheduler live-event slice moved
  workflow event ownership rewriting into `workflow_event_identity.rs` instead
  of growing `embedded_workflow_host.rs`; the host remains focused on runtime
  execution while the new module owns node-engine event id projection.
- 2026-05-09: Verification passed:
  `cargo test -p pantograph-workflow-service workflow::tests::session_execution::`,
  `cargo test -p pantograph-embedded-runtime scheduler_session_live_events_use_backend_workflow_run_id --features backend-llamacpp`,
  `cargo test --manifest-path src-tauri/Cargo.toml workflow::event_adapter::tests:: --features backend-llamacpp`,
  `node --experimental-strip-types --test src/services/workflow/WorkflowService.commands.test.ts src/components/workflowToolbarEvents.test.ts`,
  `npm run typecheck`, and
  `cargo check --manifest-path src-tauri/Cargo.toml --features backend-llamacpp`.
- 2026-05-09: Canonical response-stream frontend slice added a regression test
  proving `NodeStream(port="response")` appends live text to a connected
  text-output node while `NodeCompleted.outputs.response` still reconciles the
  final retained `text` runtime value through the same `response -> text` edge.
- 2026-05-09: Durable cache-status slice added
  `NodeExecutionCacheStatus` to diagnostics-ledger node execution status
  payloads and projections, bumps the node-status projection version, records
  node-engine `TaskCompleted.cache_status` through embedded runtime, and
  exposes the resulting cache status on the run graph page without making the
  frontend authoritative for cache semantics.
- 2026-05-09: Verification passed:
  `cargo test -p pantograph-diagnostics-ledger node_status_projection_preserves_execution_cache_status`,
  `cargo test -p pantograph-diagnostics-ledger node_status_projection_keeps_latest_status_per_node`,
  `cargo test -p pantograph-workflow-service workflow::tests::diagnostics::workflow_run_detail_query_drains_and_reads_projection`,
  `cargo test -p pantograph-workflow-service --test contract contract_snapshot`,
  `cargo test -p pantograph-embedded-runtime node_execution_workflow_sink_records_task_completed_outputs_as_retained_node_artifacts`,
  and
  `node --experimental-strip-types --test src/components/workbench/runGraphPresenters.test.ts src/services/workflow/WorkflowService.projections.test.ts src/components/workbench/networkPagePresenters.test.ts`.
- 2026-05-09: Descriptor-first node I/O slice added an embedded-runtime
  regression test proving a large binary `ArtifactDescriptor` emitted as a
  node output is projected as a retained `node_output` descriptor with payload
  kind, lifecycle state, retention state, read handle, payload reference, size,
  and producer port metadata intact. The same test reads a byte range through
  the existing ArtifactStore body API, so run inspection does not need to
  inline large/binary bodies.
- 2026-05-09: Verification passed:
  `cargo test -p pantograph-embedded-runtime node_execution_workflow_sink_projects_descriptor_node_outputs_without_body_inline`.
- 2026-05-09: Oversized inline node-output slice added an embedded-runtime
  regression test proving a value above the retained inline threshold is
  projected as `metadata_only` with media type, size, content hash, retention
  reason, and producer port metadata while omitting payload and read handles.
  This locks the backend-owned large-value behavior without adding frontend
  filtering or unbounded diagnostics payloads.
- 2026-05-09: Verification passed:
  `cargo test -p pantograph-embedded-runtime node_execution_workflow_sink_records_oversized_inline_outputs_as_metadata_only`.
- 2026-05-09: Deleted-retention projection slice added workflow-service
  coverage alongside the existing expired-retention test so artifact records
  remain queryable by run, node, role, and retention state after payload
  references are removed. The projection preserves artifact id, retention
  reason, and retention summary counts for run inspection.
- 2026-05-09: Verification passed:
  `cargo test -p pantograph-workflow-service workflow_io_artifact_query_exposes`.
- 2026-05-09: Run-graph node I/O projection slice added workflow-service
  coverage proving retained `node_input` and `node_output` records can be
  queried together by workflow run id, then grouped by backend node id for the
  run graph page without a per-node query loop.
- 2026-05-09: Verification passed:
  `cargo test -p pantograph-workflow-service workflow_io_artifact_query_groups_node_input_and_output_records_by_run_node`.

## Follow-Up Findings

- Full per-node input inspection now has a node-engine/embedded-runtime event
  boundary for resolved node inputs and a Tauri/frontend transport path.
  Remaining work is preview limits and descriptor behavior rather than event
  availability or graph-page summary query integration.
- Full per-node output inspection for every executed or cached intermediate
  node should now use the existing node execution event/ledger path. Durable
  run-detail and graph-page node-status projections now carry cache-hit versus
  fresh-execution evidence; remaining artifact work is descriptor lifecycle
  coverage for deleted or expired bodies.
- Scheduler live streaming now has a channel from backend node events to the
  existing frontend stream handler. Remaining streaming work is persistence-rule
  documentation/tests for `stream` versus `response` connections and any
  user-facing run view refresh needed after the terminal command response.
- Effective llama.cpp settings are applied but not yet emitted as a structured
  runtime settings snapshot with source attribution in run diagnostics. That is
  still needed before frontend controls can show whether values came from
  Pumas defaults, workflow defaults, run overrides, or backend defaults.

## Commit Cadence Notes

- Commit each verified logical slice atomically.
- Start implementation with Milestone 1 audit, Milestone 2 contract freeze, and
  then a failing vertical-slice test before broad repairs.
- Keep unrelated fixes separate.
- Follow `COMMIT-STANDARDS.md`.

## Optional Subagent Assignment

Use only after Milestone 1 identifies non-overlapping ownership slices and
Milestone 2 freezes shared contracts. Workers must use isolated worktrees or
branches created from the same clean integration commit. Shared contracts,
persisted schemas, DTO fixtures, generated files, lockfiles, and composition
root wiring must be handled serially unless one explicit owner is assigned.

| Owner/Agent | Primary Write Set | Allowed Adjacent Write Set | Forbidden/Shared Files | Output Contract | Handoff Checkpoint |
| ----------- | ----------------- | -------------------------- | ---------------------- | --------------- | ------------------ |
| Backend run-detail worker | `crates/pantograph-workflow-service/src/workflow/`, relevant workflow-service tests | Diagnostics-ledger query tests only when existing query coverage is insufficient | Diagnostics schema migrations, Tauri commands, frontend DTOs, lockfiles | Patch plus report listing changed files, projection contracts, retained-value behavior, and verification | After Milestone 2 contracts are frozen |
| Node-engine worker | `crates/node-engine/src/engine/`, node-engine cache/execution tests | Embedded-runtime event adapter tests when needed to prove event handoff | Workflow-service persistence, diagnostics schema, frontend code, lockfiles | Patch plus report proving fresh and cached node IO evidence records | After backend projection contract is frozen |
| Frontend workbench worker | `src/services/workflow/`, graph/run inspection frontend components and tests | Tauri TypeScript command mirrors only after backend DTO shape is stable | Backend contract owners, diagnostics schema, generated files, lockfiles | Patch plus report with DTO/test coverage and confirmation that presentation remains frontend-owned | After backend run-inspection DTO response shape is stable |
| Inference runtime worker | `crates/inference/src/`, inference tests | Workflow-service runtime snapshot tests when needed for effective settings visibility | Scheduler policy, Pumas code/contracts, frontend-only setting authority, lockfiles | Patch plus report with effective settings examples, diagnostics fields, and llama.cpp argument application | After Milestone 6 contract is frozen |

Worker reports should be written under this plan directory using names such as
`worker-report-<scope>.md` when parallel work is used.

## Re-Plan Triggers

- Milestone 1 proves node IO retention was never implemented in the existing
  architecture.
- The existing diagnostics ledger schema cannot represent node IO without a
  migration.
- ArtifactStore retention policy conflicts with inspectable node IO for normal
  workflow runs.
- Text streaming requires a public contract break rather than a routing/projection
  repair.
- Llama.cpp runtime settings require managed runtime process ownership changes
  beyond inference config application.
- Pumas does not expose enough model facts/defaults for runtime setting
  initialization.
- The implementation requires changing Pumas contracts or implementation rather
  than only consuming Pumas model facts/defaults.
- A backend run-inspection read model starts carrying presentation fields such
  as labels, colors, card layout, selected-node state, visual grouping, or UI
  ordering.
- A source slice needs a diagnostics-ledger schema migration instead of using
  existing `IoArtifactObserved` events and projections.
- Retaining node IO requires a retention-policy change rather than applying the
  current policy through ArtifactStore materialization.
- Frontend changes start deriving authoritative run facts that are absent from
  backend projections.
- Any touched source module crosses standards thresholds and cannot be safely
  decomposed or explicitly justified inside the current slice.

## Recommendations

- Treat Milestone 1 as a blocker before more symptom fixes. The current bug
  looks like a projection/retention pipeline break, and local edge repairs will
  not fix missing run-detail IO if the persistence path is not populated.
- Keep live streaming and retained final output separate. Live chunks should
  improve user experience during execution; final `response` remains the
  retained inspectable output.
- Implement runtime settings for llama.cpp first. It is the shortest useful
  slice for the current GGUF workflow and creates the extension shape for later
  vLLM, MLX, and PyTorch settings.

## Completion Summary

### Completed

- Not started.

### Deviations

- None.

### Follow-Ups

- None recorded yet.

### Verification Summary

- Plan and design review only; source verification not run.

### Traceability Links

- Module README updated: N/A for plan creation.
- ADR added/updated: N/A for plan creation.
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A.
