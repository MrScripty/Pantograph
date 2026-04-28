# 06: Run-Centric Page Implementations

## Status

In progress. Stage `05` delivered the Scheduler-first shell, dense Scheduler
table, local Network status page, and Node Editor reserved page. This stage now
has first-pass Diagnostics, Graph, I/O Inspector, and Library pages backed by
projection services. Deeper diagnostics facets, node status graph overlays,
Pumas mutation actions, and richer artifact payload renderers remain open.

## Objective

Implement the top-level run-centric pages against backend-owned projections:
Scheduler, Diagnostics, Graph, I/O Inspector, Library, Network, and Node
Editor.

## Scope

### In Scope

- Scheduler dense run table and selected-run actions.
- Diagnostics run/system views with scheduler decision details.
- Graph page run-view/edit-view distinction.
- I/O Inspector node-centric and artifact-centric views with retention state.
- Library page with active-run asset highlighting and Pumas management entry
  points.
- Network page local-only system/node stats with future peer-ready structure.
- Node Editor reserved/future page state.
- Page-level loading, error, empty, and no-active-run states.
- Focused frontend tests for presenters, stores, and page interactions.

### Out of Scope

- Full advanced comparison workflows.
- Full media viewers for every possible artifact type in the first pass.
- Full Iroh peer discovery.
- Full Node Editor local-agent authoring.
- Advanced scheduler optimization tuning beyond displaying backend facts.

## Inputs

### Problem

Once backend projections and the shell exist, each page needs a concrete
implementation that makes the selected run useful without turning the frontend
into a second source of truth.

### Constraints

- Scheduler must show future/queued runs as first-class rows.
- Page data comes from Stage `04` services.
- Scheduler timelines, diagnostics summaries, I/O galleries, retention state,
  and Library usage are consumed as projections derived from typed diagnostic
  events. Pages do not query or interpret raw ledger rows.
- Pages consume materialized projections and projection freshness states from
  Stage `04`; they do not trigger full ledger replay or locally rebuild
  diagnostics on navigation.
- Table and gallery UI must be dense, stable, and accessible.
- Network starts local-only but must not block future peer expansion.
- Node Editor is future-facing and should not imply unavailable authoring support
  is implemented.

### Assumptions

- The first page implementation can prioritize complete data flow and
  inspectability over advanced filtering polish.
- Existing diagnostics components can be adapted instead of rewritten from
  scratch where they consume the new run-centric projections cleanly.
- Historic workflow versions can be rendered through an isolated read-only
  projection view until the graph editor package exposes a safe immutable graph
  store.

### Dependencies

- Stages `01` through `05`.
- Frontend design system and `lucide-svelte` icons.
- Existing workflow graph package/components.
- Existing diagnostics presenters/components.
- New run/scheduler/I/O/Library/Network services from Stage `04`.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Dense Scheduler table becomes unreadable or unstable. | Medium | Use fixed column sizing, resizable columns later, and presenter tests for compact labels. |
| I/O Inspector tries to render unavailable payloads as failures. | Medium | Treat expired/deleted payloads as explicit retention states. |
| Pages recreate diagnostic truth from raw events. | High | Consume backend projections only; keep raw event inspection out of normal page implementations. |
| Page navigation triggers expensive projection rebuilds. | High | Treat rebuilds as admin/maintenance flows; page services read materialized projections and show freshness/catching-up state where needed. |
| Graph page shows current workflow instead of historic version. | High | Require run-view graph projection by workflow version id. |
| Library management actions mutate state optimistically. | Medium | Wait for backend confirmation and refresh projections. |
| Network page overpromises future P2P behavior. | Low | Label local-only facts and reserve peer concepts structurally. |

## Definition of Done

- Scheduler page is usable as the default landing page.
- Selecting a Scheduler row updates active-run context and other pages.
- Page stores own only selection, filters, sort, layout, and transient
  presentation state. Scheduler, retention, Library, diagnostics, and run facts
  come from backend projections.
- Diagnostics, Graph, I/O Inspector, Library, and Network have active-run and
  no-active-run states.
- Historic run graph view uses workflow version/presentation revision data, not
  current editable graph by accident.
- I/O Inspector distinguishes retained, expired, deleted, metadata-only,
  external, truncated, and too-large states where backend projects them.
- I/O Inspector no-active-run state supports general retained artifact browsing
  where backend projections support it.
- Warm projection lag is represented through backend-provided freshness or
  catching-up states rather than frontend event replay.
- Library highlights active-run assets and shows usage/audit summaries.
- Network shows local instance capabilities/load/cache state.
- Network highlights active-run relevant scheduler decisions and local
  runtime/model/device state.
- Node Editor has a truthful future/disabled surface.
- Frontend lint, typecheck, and focused tests pass.

## Milestones

### Milestone 1: Scheduler Page

**Goal:** Build the default dense operational run table.

**Tasks:**

- [ ] Render future, scheduled, queued, delayed, running, completed, failed,
  cancelled, and historic runs.
- [x] Add columns for status, run id, workflow/version, scheduled time, queue
  position, priority, session/bucket, estimate, actual timing, progress,
  runtime/node, models, retention summary, delay reason, and error summary as
  data exists.
- [x] Add sorting/filtering/search scaffolding.
- [ ] Add selected-run action surface with client-safe and GUI-admin actions
  gated by projection authority. Backend-supported cancel and push-to-front
  session queue controls are now wired; broader GUI-admin controls remain open.
- [x] Add scheduler event drawer or selected-run timeline entry point.
- [x] Render scheduler timeline projection; do not parse raw scheduler event
  payloads in the component.
- [x] Render projection freshness/catching-up state if the scheduler timeline
  or related warm summaries are behind the latest event cursor.

**Verification:**

- Presenter tests cover status labels, scheduler reasons, estimates, and retention
  summaries.
- Component tests cover row selection and active-run update.
- Accessibility tests cover table controls and actions.

**Status:** Partially complete from Stage `05`. The current Scheduler page
renders a dense projection-backed run table, first-class queued/future rows
where present in the run-list projection, selected-run actions, active-run
updates, projection freshness, search, status filtering, stable local sort
options, scheduler policy IDs, retention policy IDs, scheduler-policy and
retention-policy filters, client/session/bucket scope columns, workflow
execution-session projection facts for future queue controls, typed queue
position, priority, estimate, and scheduler-reason columns, delayed status
presentation, backend-supported cancel/front actions for queued selected runs,
a selected-run timeline panel, timeline projection
freshness, and timeline summary/detail rows without raw event parsing. Progress,
model/runtime summaries, richer delay categories, richer retention summaries,
and privileged queue action controls remain open.

### Milestone 2: Diagnostics Page

**Goal:** Present run diagnostics and version-aware aggregate diagnostics.

**Tasks:**

- [x] Adapt existing diagnostics components to active-run detail projections.
- [x] Show scheduler decision section: facts, estimates, selected/rejected
  runtime/device, delay reasons, model load/unload, actions, and observed
  result where the current scheduler timeline projection summarizes them.
- [ ] Add filters for workflow/node/model/runtime versions, scheduler policy,
  graph settings, session/bucket/client, status, date, and retention
  completeness where data exists. Status, scheduler policy, retention policy,
  client, client-session, bucket, and accepted-date comparison filters are
  wired from current run-list projection fields.
- [x] Display mixed-version warnings/facets.
- [ ] Preserve comparison-ready labels/facets for future run, workflow-version,
  runtime-version, model-version, device, and input-profile comparisons.
- [x] Render diagnostics projections built from typed event ledger data without
  embedding event-family-specific parsing in page components.
- [x] Render backend-provided projection freshness/catching-up state for warm
  diagnostics summaries.

**Verification:**

- Presenter tests cover mixed-version labels and scheduler decision summaries.
- Component tests cover active-run and no-active-run states.

**Status:** Partially complete. `DiagnosticsPage.svelte` now queries
`workflowService.queryRunDetail` and
`workflowService.querySchedulerTimeline` for the active run, renders run detail
facts, status, timing, workflow execution-session authority, terminal error,
projection freshness, and scheduler timeline summaries without parsing raw
ledger rows or event-family payloads in the component. It also uses the
run-list projection to render selected-run
comparison facets, active facet counts across the current workflow's recent
runs, and mixed workflow-version warnings. Those counts now prefer backend
run-list facet summaries scoped to the selected workflow when no local
comparison filters are active. The page can filter comparison peers by status,
scheduler policy, retention policy, client, client session, bucket, and
accepted date using typed run-list projection fields; node/model and runtime
version facets, graph-setting filters, retention-completeness facets, date
range controls, and richer scheduler decision facets remain open pending
additional typed projection fields.

### Milestone 3: Graph Page

**Goal:** Show the workflow version associated with the active run and preserve
editable workflow behavior.

**Tasks:**

- [x] Implement run view for selected historic/queued run workflow version.
- [x] Implement or preserve edit view for current editable workflow.
- [x] Use presentation revision when available and generated layout otherwise.
- [x] Add visible distinction between run view and edit view.
- [x] Overlay node output availability when retained I/O artifact projections
  provide node-keyed metadata.
- [x] Overlay node runtime status when diagnostics projection supports it.

**Verification:**

- Tests cover historic run view not loading current editable graph.
- Tests cover presentation fallback layout behavior.
- Existing graph interaction tests pass for edit view.

**Status:** Partially complete. `GraphPage.svelte` now loads the selected run's
`WorkflowRunGraphProjection` through `workflowService.queryRunGraph`, defaults
to a read-only run snapshot when an active run is selected, and keeps the
current editable graph behind an explicit `Current Editor` mode. The snapshot
renders version identity, execution fingerprint, presentation revision,
captured topology, run settings availability, and an isolated SVG graph without
applying historic graphs to the editor store. `GraphPage.svelte` now queries the
I/O artifact projection for the selected run and passes node-keyed artifact
summaries into `RunGraphSnapshot.svelte`, allowing graph nodes and node rows to
show retained input/output availability without reading raw ledger rows or
payload bodies. The page also queries the typed node-status projection through
`workflowService.queryNodeStatus`, so graph nodes and node rows can show
queued/running/waiting/completed/failed/cancelled status without parsing raw
event payloads.

### Milestone 4: I/O Inspector Page

**Goal:** Browse workflow and node data for the active run with retention-aware
rendering.

**Tasks:**

- [x] Add workflow inputs and outputs sections.
- [x] Add node-centric input/output view.
- [x] Add artifact-centric gallery view.
- [x] Add no-active-run retained artifact browsing where backend projections
  support it.
- [x] Render artifact gallery projection with event-derived retention state and
  payload availability labels.
- [x] Render backend-provided projection freshness/catching-up state for
  retained-artifact galleries when the projection is warm or rebuilding.
- [x] Add renderers for text, image metadata/preview, audio placeholder/player
  where available, video placeholder/player where available, tables, JSON,
  files, and unknown/raw fallback.
- [x] Show retention state and cleanup/policy details for each item.
- [ ] Surface global retention settings for final outputs, workflow inputs,
  intermediate node I/O, failed-run data, maximum artifact size, maximum total
  storage, media behavior, compression/archive behavior, and cleanup
  trigger/status where Stage `04` exposes them.
- [x] Add global retention policy controls for privileged GUI users if Stage
  `04` exposes them.

**Verification:**

- Presenter tests cover artifact type labels and retention state labels.
- Component tests cover expired/deleted payload states.
- Accessibility checks cover gallery navigation and policy controls.

**Status:** Partially complete. `IoInspectorPage.svelte` now queries
`workflowService.queryIoArtifacts`, renders artifact metadata cards, separates
payload-reference availability from metadata-only rows, displays projection
freshness, browses retained artifacts across runs when no run is active, and
exposes the global retention policy read/update command without optimistic
local mutation. Workflow input/output sections now group retained metadata by
artifact role, and the node I/O section groups retained node artifacts by
producer node with input/output counts. Artifact cards now include media-family
renderer surfaces for text, image, audio, video, table, JSON, file, and unknown
metadata states without dereferencing payload bodies. Detailed payload retention
state and cleanup/storage controls remain open pending richer backend
projections.

### Milestone 5: Library Page

**Goal:** Combine Library/Pumas management with active-run asset provenance.

**Tasks:**

- [x] Show asset categories: models, runtimes, workflows, nodes, templates,
  connectors, local additions, Pumas assets, Pantograph-owned assets.
- [x] Highlight assets used by active run.
- [x] Show source, version, fingerprint where available, usage count, last
  accessed, linked workflow/node versions, and audit summaries.
- [x] Render Library usage projections derived from typed `library.*` events.
- [x] Render backend-provided projection freshness/catching-up state for
  Library usage counts.
- [x] Add Pumas search/download/delete actions where backend support exists.
- [x] Avoid optimistic display of asset mutations.

**Verification:**

- Tests cover active-run asset highlighting.
- Tests cover backend error preservation for Pumas actions.
- Tests cover rejected Pumas/Library actions without optimistic local mutation.
- Accessibility checks cover asset action buttons and filters.

**Status:** Complete for the first audited UI pass. `LibraryPage.svelte` now queries
`workflowService.queryLibraryUsage`, renders a dense usage/audit table, shows
projection freshness, formats asset categories from explicit id prefixes, and
highlights rows whose `last_workflow_run_id` exactly matches the active run.
It also exposes audited Pumas HuggingFace search, download start, and model
delete controls through typed workflow service commands. Action results refresh
the backend projection and do not optimistically mutate Library usage state.

### Milestone 6: Network And Node Editor Pages

**Goal:** Add local-first system/node visibility and reserve future node
authoring page.

**Tasks:**

- [x] Show local instance identity, CPU/memory/GPU/disk where available,
  current load, active runs, queued work, and capability summary.
- [ ] Show runtimes, models, cache state, and scheduler/system events when the
  local status API exposes those facts.
- [x] Structure Network data so future peer nodes can appear without a page
  rewrite.
- [ ] Highlight active-run relevant local node/runtime/model state.
- [x] Reserve peer pairing/trust state in the data and page model so future
  Iroh work can add discovered peers, codes/keys, and verification state
  without a page rewrite.
- [x] Add Node Editor page with clear future/experimental state and no false
  authoring affordances.

**Verification:**

- Tests cover local-only Network empty/loaded/error states.
- Tests cover platform-degraded metrics states without page crashes or fake
  values.
- Tests cover Node Editor disabled/future state.

**Status:** Partially complete. The Network page displays local identity,
transport state, CPU, memory, GPU availability/degradation, disks, network
interfaces, scheduler load/capacity, selected-run context, and future-ready peer
records through `workflowService.queryLocalNetworkStatus`. The page now treats
unavailable probes as explicit degraded/unavailable states instead of fake zero
values. Runtime/model/cache highlights for the selected run remain open because
the local status API does not yet expose run-keyed residency or cache facts.
The Node Editor page has a truthful unavailable state.

## Ownership And Lifecycle Note

Page data refresh should be centralized through Stage `04` services and stores.
Avoid each page creating independent polling loops for the same run/scheduler
facts. If a page-specific refresh loop is needed, it must have teardown tests.

## Re-Plan Triggers

- The Scheduler table needs virtualization earlier than expected.
- I/O payload size or media preview behavior requires a dedicated artifact
  viewer package.
- Existing graph components cannot render immutable historic versions without a
  larger refactor.
- Library/Pumas actions require authentication or network policy not captured
  in this plan.

## Completion Summary

### Completed

- Stage `05` added the Scheduler-first shell, dense Scheduler run table,
  active-run row selection, Graph and Diagnostics page wrappers, local Network
  status cards, and Node Editor unavailable state.
- Added `src/components/workbench/IoInspectorPage.svelte` with active-run
  artifact metadata browsing from `workflowService.queryIoArtifacts`.
- Extended the I/O artifact projection query contract so `workflow_run_id` is
  an optional filter, allowing the I/O Inspector to browse retained artifact
  metadata across runs when no run is selected.
- Added workflow input and workflow output summary sections in the I/O
  Inspector, derived from typed artifact roles in the retained artifact
  projection.
- Added a node-centric I/O summary in the I/O Inspector, grouping retained
  artifact metadata by `node_id` and surfacing node input/output counts without
  reading raw payload bodies.
- Added metadata-only media renderer surfaces in artifact cards for text,
  image, audio, video, table, JSON, file, and unknown media families, preserving
  the payload-reference boundary until typed payload body access exists.
- Added global retention policy read/update controls using
  `workflowService.queryRetentionPolicy` and
  `workflowService.updateRetentionPolicy` with no optimistic mutation.
- Added an I/O Inspector cleanup action backed by
  `workflowService.applyRetentionCleanup`; the page displays the backend
  expired-artifact count and refreshes artifact metadata from projections.
- Added `src/components/workbench/ioInspectorPresenters.ts` and tests for media
  labels, payload-reference availability labels, byte labels, and projection
  freshness labels.
- Added `src/components/workbench/LibraryPage.svelte` with projection-backed
  Library usage/audit rendering from `workflowService.queryLibraryUsage`.
- Added `src/components/workbench/libraryUsagePresenters.ts` and tests for
  asset category labels, exact active-run match highlighting, network byte
  labels, and projection freshness labels.
- Added `src/components/workbench/GraphPage.svelte` run-snapshot mode backed by
  `workflowService.queryRunGraph`, while preserving the current editable graph
  page behind an explicit mode switch.
- Added `src/components/workbench/RunGraphSnapshot.svelte` and
  `src/components/workbench/runGraphPresenters.ts` with tests for immutable run
  graph summary, topology rows, presentation fallback labels, and stable SVG
  layout inputs.
- Added graph node output-availability overlays from
  `workflowService.queryIoArtifacts`, keeping artifact metadata as a typed
  projection read model and avoiding payload dereferencing in the graph view.
- Added graph node runtime-status overlays from
  `workflowService.queryNodeStatus`, backed by the durable `node_status`
  projection and presenter tests for latest-status selection.
- Wired `WorkflowTraceStore` to append bounded typed
  `node.execution_status` ledger events for node start, waiting, completion,
  failure, and cancellation transitions. Progress and stream observations do
  not create diagnostic ledger events.
- Added `src/components/workbench/DiagnosticsPage.svelte` with active-run
  `queryRunDetail` and `querySchedulerTimeline` projection rendering.
- Added `src/components/workbench/diagnosticsPagePresenters.ts` and tests for
  run status classes, duration labels, projection freshness labels, run fact
  rows, comparison-ready run-list facet counts, mixed-version warnings, typed
  timeline labels, and payload availability labels.
- Added Diagnostics comparison filters for status, scheduler policy, retention
  policy, client, client session, bucket, and accepted date using run-list
  projection fields; filtered comparisons stay centered on the selected run and
  avoid backend aggregate facet totals while local filters are active.
- Expanded `src/components/workbench/NetworkPage.svelte` to render local
  capabilities, degradation warnings, disks, network interfaces, scheduler
  load/capacity, selected-run context, and future-ready peer records.
- Added `src/components/workbench/networkPagePresenters.ts` and tests for byte
  labels, transport labels, degraded CPU/GPU states, scheduler load labels, and
  local capability rows.
- Added Scheduler table search, status filtering, and local sort controls over
  the materialized run-list projection.
- Added `src/components/workbench/schedulerPagePresenters.ts` and tests for
  Scheduler timestamp/duration labels, status classes, filtering, and sorting.
- Added Scheduler table columns for the typed scheduler policy and retention
  policy IDs already available on `RunListProjectionRecord`.
- Added Scheduler table filters for the typed scheduler policy and retention
  policy row fields without parsing scheduler event payloads.
- Added Scheduler table columns for typed queue position, priority, estimate,
  and scheduler reason fields promoted into run-list projections.
- Added a Scheduler selected-run timeline panel backed by
  `workflowService.querySchedulerTimeline`, including projection freshness and
  typed summary/detail rendering without raw payload parsing.

### Deviations

- First-pass I/O media rendering is metadata-only. The page shows media-family
  renderer surfaces and `payload_ref` availability but does not dereference
  payload bodies because there is no typed payload body API in Stage `04`.
- No-active-run retained artifact browsing now uses an optional
  `workflow_run_id` query filter and still returns metadata only; payload body
  dereferencing remains blocked on a typed payload body API.
- Pumas search/download/delete UI is limited to the audited commands currently
  exposed by the frontend workflow service. The page refreshes projections after
  confirmed backend responses and does not optimistically mutate local usage
  rows.
- The first run graph view uses a lightweight read-only SVG snapshot instead
  of the full graph editor package because the current editor components are
  bound to the live workflow store.
- The first run diagnostics page renders scheduler timeline summary/detail
  fields and payload availability only. It does not parse `payload_json` because
  richer decision facets should be promoted into typed projection fields.
- Network selected-run context is visible but not yet linked to runtime/model
  residency because `WorkflowLocalNetworkStatusQueryResponse` does not expose
  run-keyed resource placement or cache facts.
- Scheduler timeline rows currently show typed event labels, summary/detail
  text, and payload availability only. The run table shows scheduler and
  retention policy IDs, queue position, priority, estimate, and scheduler
  reason fields. Richer delay categories and detailed retention summary
  columns need additional typed projection fields before being shown as dense
  table columns.

### Follow-Ups

- Decide table virtualization threshold during Scheduler implementation.
- Add richer artifact retention state once backend projections distinguish
  retained, expired, deleted, external, truncated, and too-large payloads.
  First-pass typed `retention_state` and `retention_reason` fields are now
  exposed on I/O artifact projections and rendered by the I/O Inspector.
  Projection-backed retention summary counts are also displayed for the current
  run or global artifact scope.
- Add payload-body media previews after typed payload body access is exposed
  through a service.
- Add richer Library/Pumas progress projections for downloads after Pumas
  exposes typed download status, byte, and cache-state facts.
- Audit any future executor path that bypasses `WorkflowTraceStore` before it
  can drive graph runtime-status overlays.
- Add typed diagnostics facet projections for scheduler estimates, selected and
  rejected runtime/device choices, model load/unload decisions, graph settings,
  node/model/runtime versions, date ranges, retention completeness, and
  selected/rejected runtime-device comparisons.
- Add local status fields for runtime/model/cache residency and run-keyed
  scheduler placement before adding active-run Network highlights.

### Verification Summary

- `node --experimental-strip-types --test src/components/workbench/ioInspectorPresenters.test.ts`
  passed.
- `node --experimental-strip-types --test src/components/workbench/libraryUsagePresenters.test.ts`
  passed.
- `node --experimental-strip-types --test src/components/workbench/runGraphPresenters.test.ts`
  passed.
- `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`
  passed.
- `node --experimental-strip-types --test src/components/workbench/networkPagePresenters.test.ts`
  passed.
- `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts`
  passed.
- `npm run test:frontend` passed.
- `npm run typecheck` passed.
- `npm run lint:full` passed.
- `npm run build` passed.
- `cargo test -p pantograph-diagnostics-ledger` passed.
- `cargo test -p pantograph-workflow-service` passed.
- `npm run format:check` passed.
- `git diff --check` passed for the Stage `06` page slices.

### Traceability Links

- Requirement sections: Scheduler Page Requirements, Diagnostics Requirements,
  Graph Page Requirements, I/O Inspector Requirements, Library Requirements,
  Network Page Requirements, Node Editor Requirements.
