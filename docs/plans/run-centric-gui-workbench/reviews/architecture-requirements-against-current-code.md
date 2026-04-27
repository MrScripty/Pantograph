# Architecture Requirements Against Current Code

## Status

Draft investigation record. Not an implementation plan replacement.

Last updated: 2026-04-27.

## Purpose

Record the architectural changes required by each plan in the parent directory
against the current Pantograph codebase. This file exists so later plan
iterations can separate:

- features already supported by current architecture
- current code that can be extended
- architectural gaps that must be resolved before implementation
- ownership decisions that likely need README or ADR updates

## Compatibility Policy

The run-centric workbench plan no longer requires backwards compatibility with
existing saved workflow files, diagnostics history, or run records. When the
new identity/version contracts land, old records that cannot satisfy those
contracts may be deleted, ignored, or regenerated. This investigation therefore
treats legacy support as optional cleanup, not an architectural requirement.

## Investigation Scope

Reviewed plan stages:

- `../00-overview-and-boundaries.md`
- `../01-workflow-identity-versioning-and-run-snapshots.md`
- `../02-scheduler-estimates-events-and-control.md`
- `../03-diagnostics-retention-and-audit-ledgers.md`
- `../04-api-projections-and-frontend-data-boundaries.md`
- `../05-app-shell-active-run-navigation.md`
- `../06-run-centric-page-implementations.md`
- `../07-verification-rollout-and-refactor-gates.md`

Reviewed code areas:

- `crates/pantograph-workflow-service`
- `crates/pantograph-diagnostics-ledger`
- `crates/pantograph-runtime-attribution`
- `crates/pantograph-node-contracts`
- `crates/pantograph-frontend-http-adapter`
- `src-tauri/src/workflow`
- `src/services`
- `src/stores`
- `src/components`
- `src/App.svelte`

## Cross-Cutting Findings

### Existing Strengths

- The workflow service already owns execution-session queue state and queue
  mutation through the scheduler store.
- The scheduler already has priority/FIFO ordering, admission diagnostics,
  queue status DTOs, runtime capacity posture, and runtime warmup diagnostic
  DTOs.
- The diagnostics ledger already persists model/license usage events, workflow
  timing observations, retention policy rows for usage events, and workflow run
  summaries.
- Tauri diagnostics already combine trace, scheduler, runtime, and timing
  overlays into a frontend projection.
- The frontend already has service boundaries for workflow, diagnostics, and
  managed runtimes.
- `App.svelte` already owns the root shell, so the eventual workbench shell has
  a clear migration point.

### Main Architectural Gaps

- There is no durable workflow version registry that maps stable workflow
  identity plus semantic version to execution fingerprint.
- Current graph fingerprints are topology/type fingerprints, not full workflow
  execution-version fingerprints that include node versions.
- Run submission creates an in-memory queued item but does not resolve or
  persist a full immutable run snapshot before queue insertion.
- Scheduler estimates do not exist as distinct pre-run records.
- Scheduler events exist only indirectly as queue state, trace events, or
  diagnostics snapshots. There is no durable typed diagnostic event ledger for
  scheduler, runtime, node, I/O, retention, or Library facts.
- Retention currently prunes model/license usage and timing observations, but
  there is no I/O artifact metadata ledger that survives payload deletion.
- Pumas/Library operations are command/helper oriented and are not audited as
  asset access records.
- Frontend TypeScript DTOs lag backend scheduler diagnostics in at least one
  area: Tauri exposes scheduler diagnostics, but frontend diagnostics types do
  not include that field.
- The app shell is still a two-mode canvas/workflow UI rather than a
  page-based workbench.

## 00: Overview And Boundaries

### Current Code

- `src/App.svelte` is the current shell owner. It starts diagnostics, loads the
  generated component workspace and last graph, registers global shortcuts, and
  switches between `$viewMode === 'canvas'` and `$viewMode === 'workflow'`
  surfaces ([src/App.svelte](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src/App.svelte:62),
  [src/App.svelte](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src/App.svelte:144)).
- The backend already has clear boundaries for scheduler queue state
  (`crates/pantograph-workflow-service/src/scheduler`), durable diagnostics
  (`crates/pantograph-diagnostics-ledger`), attribution
  (`crates/pantograph-runtime-attribution`), and frontend/Tauri projections.
- Frontend state boundaries are documented in `src/README.md`, which says
  backend-owned workflow responses drive durable graph and execution state.

### Required Architecture Changes

- Add a new workbench boundary in frontend source, likely under `src/features`
  or `src/components`, with ownership over top-level page navigation,
  active-run UI selection, and page layout only.
- Keep backend-owned run/scheduler/diagnostics/version state out of the
  workbench shell stores. The shell should store selected page and active run
  id/context only.
- Define a backend read-model boundary for the workbench before page work.
  The current diagnostics projection is useful but is not broad enough for
  run list, I/O retention, Library usage, or Network page state.
- Update source READMEs when ownership moves from `App.svelte`/view modes into
  a page workbench structure.
- Add ADRs or README decisions for any new durable owner: workflow version
  registry, typed diagnostic event ledger, I/O artifact/retention projections,
  and projection rebuild ownership.

### Sequencing Requirement

Backend identity, scheduler, diagnostics, and API projections should land
before the frontend claims to render authoritative run-centric state. The
current code supports this order because frontend services can be extended
before the root shell is replaced.

## 01: Workflow Identity, Versioning, And Run Snapshots

### Current Code

- Workflow id validation only checks for non-empty text
  ([validation.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-workflow-service/src/workflow/validation.rs:17)).
- Workflow file lookup has a separate `sanitize_workflow_stem` helper that
  allows ASCII alphanumeric, hyphen, underscore, and spaces
  ([capabilities.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-workflow-service/src/capabilities.rs:409)).
- Current graph fingerprinting uses node id, node type, and edge endpoints,
  then sorts rows and hashes with FNV-1a
  ([capabilities.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-workflow-service/src/capabilities.rs:325)).
- The current fingerprint does not include node semantic versions or node
  behavior fingerprints.
- Run queue insertion generates a `WorkflowRunId` and stores in-memory inputs,
  output targets, override selection, timeout, priority, and queue metadata
  ([store_queue.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-workflow-service/src/scheduler/store_queue.rs:62)).
- Run execution enqueues first, then waits until admission, then runs with the
  queued request data
  ([session_execution_api.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-workflow-service/src/workflow/session_execution_api.rs:74),
  [session_execution_api.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-workflow-service/src/workflow/session_execution_api.rs:150)).
- Durable run summaries currently store `workflow_run_id`, `workflow_id`,
  optional `session_id`, optional `graph_fingerprint`, status, timing, node
  count, event count, and last error
  ([timing.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-diagnostics-ledger/src/timing.rs:198)).
- Attribution storage has durable `workflow_runs`, but it records only
  workflow id, client/session/bucket, status, and timestamps, not workflow
  versions or run snapshots
  ([schema.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-runtime-attribution/src/schema.rs:83)).

### Architectural Gaps

- No workflow registry table or service owns `workflow_identity`,
  `workflow_semver`, `execution_fingerprint`, and `presentation_revision`.
- No strict semantic version/fingerprint uniqueness rule exists.
- No server-computed workflow version id is attached before queue insertion.
- No immutable run snapshot is persisted at queue time.
- Current workflow id validation is too loose for the stated stable identity
  requirements and inconsistent with file stem sanitization.
- Current graph fingerprint is useful but insufficient for execution-version
  identity because it omits node versions and uses a local 64-bit hash.
- Presentation metadata is not versioned separately from execution topology.
- Diagnostics filters still key timing mainly by workflow id plus graph
  fingerprint, not workflow execution version plus node versions.

### Required Architecture Changes

- Add a backend workflow-version registry owner. Candidate owners:
  `pantograph-workflow-service` for application use cases, with a storage
  trait/repository, or a new narrow crate if the registry must be consumed by
  bindings and services outside workflow-service.
- Define validated workflow identity as a domain type rather than a loose
  string. Reuse it across workflow-service, attribution, diagnostics, and
  frontend projections.
- Replace current overloaded graph fingerprinting with explicit canonical
  execution fingerprinting:
  - include executable topology
  - include node identities
  - include node semantic versions or behavior fingerprints
  - exclude layout/display metadata
  - use a stronger named digest such as SHA-256 or BLAKE3 for long-term audit
    identity
- Add workflow semantic version records and a strict uniqueness constraint:
  same workflow identity plus semantic version cannot map to a different
  execution fingerprint.
- Add presentation revision records or references for layout/display metadata.
  These must be stored separately and never used as diagnostics grouping keys.
- Move run submission so version resolution happens before `enqueue_run`.
  Queue items should carry a run snapshot reference, not only request fields.
- Extend durable run summaries or introduce a run snapshot table that records:
  workflow identity, workflow semantic version, workflow execution version id,
  execution fingerprint, presentation revision, node versions, model choices,
  runtime versions, scheduler policy/version, retention policy/version, graph
  settings, input references, session, and bucket.
- Add a cutover cleanup path for existing runs where exact workflow/node
  versions cannot be recovered. These may be deleted, ignored, or regenerated
  instead of preserved as queryable legacy records.

### Verification Requirements

- Identity validation rejects invalid workflow names with explicit field errors.
- Fingerprint tests cover ordering-insensitive topology.
- Node version changes create a new execution version.
- Display-only changes create or update presentation revision without changing
  execution version.
- Queued run snapshots remain stable if the editable workflow changes later.

## 02: Scheduler Estimates, Events, And Control

### Current Code

- Scheduler queue state is in-memory inside `WorkflowExecutionSessionStore`.
  Queue items carry run id, inputs, output targets, override selection,
  timeout, priority, decision reason, enqueue tick, and starvation bypass count
  ([store.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-workflow-service/src/scheduler/store.rs:20)).
- Queue insertion is priority/FIFO based and mutates the session-local queue
  ([store_queue.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-workflow-service/src/scheduler/store_queue.rs:72)).
- Admission can set decision reasons such as waiting for runtime capacity
  ([store_queue.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-workflow-service/src/scheduler/store_queue.rs:177)).
- Runtime admission waits are handled by a 10ms polling loop around
  `can_load_session_runtime`
  ([session_execution_api.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-workflow-service/src/workflow/session_execution_api.rs:79)).
- Runtime load and unload are performed through host calls, but they are not
  recorded as durable scheduler events
  ([session_execution_api.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-workflow-service/src/workflow/session_execution_api.rs:143),
  [session_execution_api.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-workflow-service/src/workflow/session_execution_api.rs:170)).
- Scheduler DTOs include queue items, queue status, decision reasons, runtime
  capacity pressure, and runtime-registry diagnostics, but no estimate or event
  record family.
- Frontend `WorkflowService` can query current session queue and scheduler
  snapshot by session id, not all runs or historic/future runs
  ([WorkflowService.ts](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src/services/workflow/WorkflowService.ts:230)).

### Architectural Gaps

- No `SchedulerEstimate` contract exists.
- No persisted estimate history exists for pre-run audit.
- No scheduler event stream/ledger exists for submitted, estimated, queued,
  delayed, promoted, reservation, runtime selection, model load/unload, retry,
  fallback, client action, or admin override events.
- Queue authority is session-id based. It does not yet model "normal client can
  only affect own session" versus "GUI admin can affect all sessions" as an
  explicit API authority boundary.
- Current queue state is in-memory and session-scoped. The Scheduler page
  requires dense current and historic run listing across sessions/buckets.
- Model/cache state is present indirectly in capabilities/runtime diagnostics,
  but not as one scheduler-visible model/cache state machine.
- Intentional delay for better cache/model state exists only as admission wait
  reasons, not as an explicit estimate/event decision with facts considered.

### Required Architecture Changes

- Add scheduler estimate contracts and storage/query support:
  estimated start, duration, memory/VRAM, cache needs, candidates,
  missing/blocking assets, delay reason, model load cost, cache benefit,
  queue position, confidence/quality, timestamp, and estimate version.
- Add typed scheduler event contracts and durable query support through the
  typed diagnostic event ledger architecture. The scheduler owns event
  production, while persistence/projection ownership should sit with the event
  ledger boundary, not frontend diagnostics overlays.
- Extend queue/run records so scheduler estimates reference the immutable run
  snapshot from Stage `01`.
- Add a scheduler-visible model/cache state projection:
  available on disk, loading, loaded, warm, in use, unload pending, unloading,
  evicted, failed, and unknown where applicable.
- Add API authority boundaries:
  - client/session scoped queue actions
  - privileged GUI/admin actions
  - scheduler-denied or scheduler-normalized action results
- Make load/unload transitions observable by routing host/runtime callbacks
  through typed scheduler event builders and a single scheduler event pathway.
- Replace broad polling where practical with event-driven state changes. If
  polling remains for runtime admission, record wait events and ensure the
  lifecycle is documented.

### Verification Requirements

- Scheduler tests assert estimate creation at submission time.
- Scheduler tests assert typed event emission for queue, admission, delay,
  load, unload, cancel, reprioritize, and admin override paths.
- Event validation tests reject malformed scheduler events and disallowed
  producers.
- Authority tests prove normal clients cannot mutate another session/bucket.
- Restart/replay tests prove durable events remain queryable if persistence is
  implemented in this stage.

## 03: Diagnostics, Retention, And Audit Ledgers

### Current Code

- `pantograph-diagnostics-ledger` currently owns model/license usage events,
  license snapshots, output measurements, usage lineage, one diagnostics
  retention policy table, timing observations, and workflow run summaries
  ([schema.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-diagnostics-ledger/src/schema.rs:19),
  [schema.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-diagnostics-ledger/src/schema.rs:105),
  [schema.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-diagnostics-ledger/src/schema.rs:169),
  [schema.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-diagnostics-ledger/src/schema.rs:205)).
- Usage diagnostics query filters include client, session, bucket, workflow
  run, workflow, node, model, license, guarantee, and time fields, but not
  workflow version, node version, runtime version, scheduler policy, graph
  settings, or retention completeness.
- Workflow run summaries are timing/run summary records only and do not store
  artifact retention facts
  ([timing.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/crates/pantograph-diagnostics-ledger/src/timing.rs:198)).
- Tauri `WorkflowDiagnosticsStore` currently combines in-memory trace store
  state with optional durable timing ledger; its overlays are not durable
  audit ledgers
  ([store.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src-tauri/src/workflow/diagnostics/store.rs:93)).
- Pumas helper code hydrates nodes and resolves model metadata, but does not
  record audit events for search/download/delete/access
  ([puma_lib_commands.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src-tauri/src/workflow/puma_lib_commands.rs:1)).
- There is no typed diagnostic event envelope, allowlisted event family
  contract, payload schema-version registry, or backend-only event builder
  boundary.

### Architectural Gaps

- No I/O artifact metadata ledger exists for workflow inputs, node inputs,
  node outputs, intermediate artifacts, workflow outputs, or final artifacts.
- No typed append-only event ledger exists for scheduler, run, node, I/O,
  Library, runtime, or retention event families.
- No retention state exists per artifact. Current retention policy is for
  model/license usage events, not general run I/O payloads.
- Retroactive retention policy changes are not represented as policy versions
  that can explain old payload deletion.
- No durable typed retention cleanup event family exists.
- No durable typed Pumas/Library audit event family exists.
- Diagnostics history cannot yet report retention completeness or mixed
  workflow/node/model/runtime version facets.
- Payload storage references, hashes, sizes, content types, and deletion
  reasons are not captured in durable query records.

### Required Architecture Changes

- Decide whether to extend `pantograph-diagnostics-ledger` or create one shared
  diagnostics-event crate for the typed diagnostic event ledger. Given current
  ownership, the diagnostics ledger is a strong candidate for typed event
  persistence and projection metadata, while payload blobs can remain outside
  it. Do not create per-event-family sibling repositories.
- Add a typed event envelope with event id, event kind, schema version, source
  component, timestamps, run/workflow/node/runtime/model/client correlation
  fields, privacy class, retention class, payload hash/size, optional payload
  reference, and validated payload JSON.
- Add typed event families for `scheduler.*`, `run.*`, `node.*`, `io.*`,
  `library.*`, `runtime.*`, and `retention.*` events.
- Add event builders that validate required envelope fields, allowed source
  components, payload schema version, maximum embedded payload size, and
  payload reference rules before persistence.
- Add `RunIoArtifactProjection` or equivalent derived from typed `io.*` and
  `retention.*` events:
  run id, workflow id/version, node id, port id, direction, artifact kind,
  content type, size, hash, payload reference, producer/consumer relationships,
  timestamps, retention state, and deletion/expiration reason.
- Add global retention policy version records and typed retention cleanup
  events. Cleanup must update metadata before deleting payloads.
- Add version-aware diagnostics filters and result facets:
  workflow execution version, node version, model version, runtime version,
  scheduler policy, graph settings, session/bucket/client, retention
  completeness, date, and status.
- Add typed Pumas/Library audit events:
  operation type, asset identity/version/source, run/session/bucket/client or
  GUI actor, timestamps, network bytes, cache hit/miss, success/failure, and
  error details.
- Add query projections for Library usage:
  used by active run, used by N runs, last accessed, access count, linked
  workflow versions, linked node versions.
- Define cleanup lifecycle. Prefer explicit/admin cleanup first; a background
  worker would need ownership, cancellation, overlap prevention, and tests.

### Verification Requirements

- Migration tests cover new event/projection tables and indexes.
- Event validation tests reject unsupported event kinds, missing required
  fields, unsupported schema versions, disallowed producers, and oversized
  embedded payloads.
- Projection rebuild tests prove page read models can be regenerated from the
  typed event ledger.
- Repository tests prove artifact metadata survives payload deletion.
- Tests cover retroactive policy changes on old runs.
- Tests cover Pumas/Library audit query by asset and by run.
- Diagnostics tests cover mixed-version facets and retention completeness.

## 04: API Projections And Frontend Data Boundaries

### Current Code

- `WorkflowService.ts` exposes workflow/session APIs, session queue APIs,
  scheduler snapshot, diagnostics snapshot, trace snapshot, and workflow
  persistence APIs
  ([WorkflowService.ts](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src/services/workflow/WorkflowService.ts:51)).
- Tauri diagnostics DTOs include `DiagnosticsSchedulerSnapshot.diagnostics`
  ([types.rs](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src-tauri/src/workflow/diagnostics/types.rs:300)).
- Frontend diagnostics TypeScript does not include the scheduler diagnostics
  field in `DiagnosticsSchedulerSnapshot`
  ([types.ts](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src/services/diagnostics/types.ts:154)).
- Frontend workflow types have runtime requirements and runtime capabilities,
  but no run list, run detail, run snapshot, scheduler estimate, scheduler
  event, I/O artifact, retention policy, Library usage, or Network node
  projections
  ([types.ts](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src/services/workflow/types.ts:192)).
- The frontend HTTP adapter is a `WorkflowHost` for proxying workflow runs and
  metadata, not a full GUI read-model API for the run-centric workbench.

### Architectural Gaps

- No backend projection exists for "all runs" across future, queued, running,
  completed, failed, cancelled, and historic states.
- No run detail projection merges run snapshot, scheduler estimate, scheduler
  event projections, workflow version, retention state, Library assets, and
  Network/local execution facts.
- No graph-by-run workflow-version projection exists for the Graph page.
- No I/O artifact API exists.
- No global retention policy read/update API exists for privileged GUI
  surfaces.
- No Library/Pumas audit query API exists.
- No local Network/system node API exists that combines CPU/memory/GPU/disk,
  runtimes, models, current load, queue, and cache state.
- Error taxonomy needs expansion for invalid workflow identity,
  version/fingerprint conflict, unauthorized queue action, expired payload, and
  retention cleanup failures.

### Required Architecture Changes

- Define backend read-model DTOs before frontend pages:
  - `RunListProjection`
  - `RunDetailProjection`
  - `SchedulerEstimateProjection`
  - `SchedulerEventProjection`
  - `DiagnosticTimelineProjection`
  - `WorkflowVersionGraphProjection`
  - `RunIoArtifactProjection`
  - `RetentionPolicyProjection`
  - `LibraryAssetUsageProjection`
  - `LocalNetworkNodeProjection`
- Add frontend service modules around these projections instead of expanding
  `WorkflowService.ts` into a broad catch-all. Candidate services:
  `runWorkbenchService`, `schedulerService`, `ioInspectorService`,
  `libraryService`, `networkNodeService`.
- Align backend DTO serialization with TypeScript contracts. Consider DTO
  generation if drift continues.
- Preserve explicit backend errors through adapters and presenters.
- Prefer event-driven updates for run/scheduler state. If polling is used,
  centralize it in one store and add cleanup tests.
- Keep raw typed event rows out of normal page APIs. Expose rebuildable
  projections; reserve raw event inspection for a future privileged developer
  surface if needed.
- Update `crates/pantograph-frontend-http-adapter`, Tauri command READMEs, and
  frontend service READMEs when projections become stable.

### Verification Requirements

- Backend projection tests for each read model.
- TypeScript tests for DTO normalization and error preservation.
- Cross-layer acceptance from backend fixture/state to frontend service for run
  list and selected run detail.
- Cross-layer acceptance proving a typed event updates a projection consumed by
  a frontend service without exposing raw ledger storage details.
- Typecheck must catch scheduler DTO drift, including backend fields like
  scheduler diagnostics.

## 05: App Shell And Active Run Navigation

### Current Code

- `App.svelte` has a root two-mode design: canvas mode and workflow mode
  ([src/App.svelte](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src/App.svelte:144)).
- Global keyboard shortcuts are registered directly in `App.svelte`, including
  undo variants, `Ctrl+\`` view toggle, and canvas-only Tab behavior
  ([src/App.svelte](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src/App.svelte:78)).
- Diagnostics store startup/shutdown is tied to app mount/unmount
  ([src/App.svelte](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src/App.svelte:62)).
- Existing frontend service state tracks current execution/session ids inside
  workflow service and svelte-graph session stores, but there is no global
  active-run page context.

### Architectural Gaps

- No page routing or workbench page store exists.
- No top-level navigation model exists for Scheduler, Diagnostics, Graph, I/O
  Inspector, Library, Network, and Node Lab.
- No transient active-run store exists that is independent of backend-owned run
  state.
- Existing diagnostics selection is local to diagnostics UI state, not a shared
  selected run for all pages.
- Existing drawing-to-Svelte and workflow graph surfaces are coupled to the
  root mode switch.
- Global keyboard shortcuts are mode-specific and will need ownership review
  when page navigation is introduced.

### Required Architecture Changes

- Introduce a workbench shell boundary with:
  - selected page id
  - transient active run id/context
  - top bar summary state
  - page layout slots
  - no-active-run page states
- Move existing canvas and graph surfaces into explicit pages or feature slots,
  or retire them before deleting `viewMode`.
- Move active-run behavior out of diagnostics-specific state. Diagnostics can
  still own selected node/tab, but the run id should come from the workbench
  context.
- Define keyboard shortcut ownership per page. Root shortcuts should be limited
  to global commands that remain valid across pages.
- Add route/page tests for default Scheduler landing, page switching,
  active-run retention during the current session, and no persistence across
  restart.
- Update frontend READMEs for the new shell and any moved surfaces.

### Verification Requirements

- Frontend tests cover default page selection.
- Tests cover Scheduler row selection updating active-run context.
- Tests cover page switch preserving active-run in memory.
- Tests cover app startup with no active-run selection.
- Accessibility tests cover toolbar/rail controls.

## 06: Run-Centric Page Implementations

### Current Code

- There is an embedded Diagnostics Scheduler panel, but it is session-focused
  and card/table oriented, not a dense all-runs Scheduler page
  ([DiagnosticsScheduler.svelte](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src/components/diagnostics/DiagnosticsScheduler.svelte:1)).
- Diagnostics history currently displays workflow id and graph fingerprint
  timing history, not workflow semantic/execution versions
  ([DiagnosticsWorkflowHistory.svelte](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src/components/diagnostics/DiagnosticsWorkflowHistory.svelte:1)).
- The workflow graph component can show the current workflow graph, but there
  is no run-view projection that guarantees historic workflow version display.
- Managed runtime services can list and mutate managed runtimes, but they do
  not expose local compute node stats such as CPU/memory/GPU/disk/load
  ([ManagedRuntimeService.ts](/media/jeremy/OrangeCream/Linux%20Software/repos/owned/ai-systems/Pantograph/src/services/managedRuntime/ManagedRuntimeService.ts:74)).
- There is no I/O Inspector page, artifact gallery, Library usage page, Network
  page, or Node Lab page.

### Architectural Gaps

- Scheduler page needs a global run read model. Current queue views are
  session-scoped.
- Diagnostics page needs run-centric and aggregate version-aware filters.
- Graph page needs a historic run workflow-version graph projection and a clear
  run view versus edit view boundary.
- I/O Inspector needs artifact metadata, payload fetch/open behavior, and
  retention policy projections.
- Library page needs asset registry/search/download/delete projections and
  active-run asset highlighting.
- Network page needs a local system/node state backend source that is broader
  than managed runtime install state.
- Node Lab needs an honest placeholder route without implying authoring support
  exists.

### Required Architecture Changes

- Add page-specific presenter modules before complex Svelte components:
  scheduler run row presenters, diagnostics facet presenters, artifact
  presenters, Library asset presenters, and local Network presenters.
- Add a dense table abstraction or targeted Scheduler table component with
  stable column sizing, sorting/filtering state, selected row state, and
  accessible controls.
- Adapt diagnostics components to consume active-run context and version-aware
  filters.
- Add a Graph run-view adapter that consumes `WorkflowVersionGraphProjection`
  and cannot accidentally load the current editable graph for historic runs.
- Add I/O Inspector renderers for text, image, audio, video, table, JSON, file,
  unknown/raw, and expired/deleted/metadata-only states. Full media rendering
  can be incremental, but retention state cannot be a generic error.
- Add Library UI that waits for backend confirmation on model search/download
  and delete actions.
- Add Network UI around local node facts first, with data shape ready for peer
  nodes later.

### Verification Requirements

- Presenter tests for status labels, delay reasons, estimates, retention
  states, version facets, and Library usage labels.
- Component tests for Scheduler selection and action enablement.
- Component tests for expired/deleted I/O payload states.
- Tests proving Graph run view does not load current editable workflow for a
  historic run.
- Accessibility checks for dense table controls, toolbar navigation, gallery
  navigation, and Library actions.

## 07: Verification, Rollout, And Refactor Gates

### Current Code

- The repo already has broad frontend and backend verification scripts in
  `package.json`, including `npm run lint:full`, `npm run typecheck`, and
  `npm run test:frontend`.
- Existing plan standards require worktree hygiene, logical commits, README
  updates, and cross-layer acceptance checks for multi-layer work.
- Current working tree contains unrelated deleted/generated files outside this
  plan work. They must remain untouched unless the user explicitly assigns
  ownership.

### Architectural Gaps

- The plan set now needs stage-specific implementation-wave files only if work
  is split across parallel implementers. None exist yet.
- Several likely ownership changes need ADR decisions before code changes:
  workflow version registry, typed diagnostic event ledger, I/O artifact
  retention metadata, and privileged GUI admin authority.
- Existing tests are focused on current session/diagnostics behavior. New
  acceptance paths are needed for run list, run detail, scheduler events, and
  historic graph projection.

### Required Architecture Changes

- Add stage-start checklists during implementation that identify write sets and
  block overlapping dirty source/test/config files.
- Add ADRs or README updates in the same logical slice as accepted ownership
  changes.
- Add cross-layer acceptance tests before page implementation depends on new
  backend projections.
- Relocate or intentionally retire old app surfaces under the workbench shell.
- Convert any broad implementation stage into `implementation-waves/` only
  after shared contracts are frozen and write sets are non-overlapping.

### Verification Requirements

- Stage `01`: workflow-service, diagnostics-ledger, and node-contract tests as
  applicable.
- Stage `02`: scheduler policy/store tests plus event persistence tests.
- Stage `03`: diagnostics-ledger migration, typed event validation,
  projection rebuild, retention, and audit tests.
- Stage `04`: backend projection tests, frontend service tests, typecheck, and
  cross-layer acceptance.
- Stage `05` and `06`: frontend lint, typecheck, frontend tests, accessibility
  checks, and targeted backend tests for projection gaps.

## Recommended Plan Edits Based On Investigation

- Stage `01` should explicitly decide storage ownership for the workflow
  version registry before any schema work. The current split between
  attribution, diagnostics, and workflow-service does not make this automatic.
- Stage `02` should treat scheduler events as a typed durable backend event
  family, not as diagnostics overlay events. The current overlay path is useful
  for UI display but not enough for audit.
- Stage `03` should implement the typed diagnostic event ledger pattern and
  separate payload storage from artifact metadata. The current diagnostics
  ledger is a good event/projection metadata candidate, but payload lifecycle
  needs a distinct owner or reference contract.
- Stage `04` should include a DTO drift cleanup item because frontend
  diagnostics types already omit backend scheduler diagnostics.
- Stage `05` should include a relocation-or-retirement decision for the
  drawing-to-Svelte tool before root shell work starts.
- Stage `06` should avoid starting Scheduler table work until the global run
  list projection exists.
- Stage `07` should require ADR decisions for workflow versioning and typed
  diagnostic event ledger ownership before implementation begins.

## Open Questions For Iteration

- Should workflow version registry records live in `pantograph-workflow-service`
  storage, `pantograph-runtime-attribution`, `pantograph-diagnostics-ledger`, or
  a new crate?
- Should the typed diagnostic event ledger live in the diagnostics ledger
  database, or should it have a new shared crate/repository with
  diagnostics-ledger projections built on top? This must be one shared event
  owner decision, not a per-family split.
- What is the exact workflow identity grammar?
- Do node contracts currently expose enough semantic version information to
  support workflow execution fingerprints, or is a node-version contract stage
  required first?
- Should initial I/O payload storage use existing workflow output locations, a
  new artifact store directory, or metadata-only records until payload storage
  is formalized?
- Should the first Network page collect system metrics through Rust/Tauri
  platform APIs, existing runtime registries, or both?

## Verification Summary

- Documentation/investigation only.
- No code tests were run.
- Current working tree had unrelated dirty/generated files before this record
  was created; this investigation did not modify them.
