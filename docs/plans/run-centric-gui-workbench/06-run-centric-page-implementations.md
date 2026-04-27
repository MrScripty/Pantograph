# 06: Run-Centric Page Implementations

## Status

Draft plan. Not implemented.

## Objective

Implement the top-level run-centric pages against backend-owned projections:
Scheduler, Diagnostics, Graph, I/O Inspector, Library, Network, and Node Lab.

## Scope

### In Scope

- Scheduler dense run table and selected-run actions.
- Diagnostics run/system views with scheduler decision details.
- Graph page run-view/edit-view distinction.
- I/O Inspector node-centric and artifact-centric views with retention state.
- Library page with active-run asset highlighting and Pumas management entry
  points.
- Network page local-only system/node stats with future peer-ready structure.
- Node Lab reserved/future page state.
- Page-level loading, error, empty, and no-active-run states.
- Focused frontend tests for presenters, stores, and page interactions.

### Out of Scope

- Full advanced comparison workflows.
- Full media viewers for every possible artifact type in the first pass.
- Full Iroh peer discovery.
- Full Node Lab local-agent authoring.
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
- Table and gallery UI must be dense, stable, and accessible.
- Network starts local-only but must not block future peer expansion.
- Node Lab is future-facing and should not imply unavailable authoring support
  is implemented.

### Assumptions

- The first page implementation can prioritize complete data flow and
  inspectability over advanced filtering polish.
- Existing diagnostics components can be adapted instead of rewritten from
  scratch where they consume the new run-centric projections cleanly.
- Existing graph components can render historic workflow versions if given the
  correct graph projection.

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
- Library highlights active-run assets and shows usage/audit summaries.
- Network shows local instance capabilities/load/cache state.
- Network highlights active-run relevant scheduler decisions and local
  runtime/model/device state.
- Node Lab has a truthful future/disabled surface.
- Frontend lint, typecheck, and focused tests pass.

## Milestones

### Milestone 1: Scheduler Page

**Goal:** Build the default dense operational run table.

**Tasks:**

- [ ] Render future, scheduled, queued, delayed, running, completed, failed,
  cancelled, and historic runs.
- [ ] Add columns for status, run id, workflow/version, scheduled time, queue
  position, priority, session/bucket, estimate, actual timing, progress,
  runtime/node, models, retention summary, delay reason, and error summary as
  data exists.
- [ ] Add sorting/filtering/search scaffolding.
- [ ] Add selected-run action surface with client-safe and GUI-admin actions
  gated by projection authority.
- [ ] Add scheduler event drawer or selected-run timeline entry point.
- [ ] Render scheduler timeline projection; do not parse raw scheduler event
  payloads in the component.

**Verification:**

- Presenter tests cover status labels, delay reasons, estimates, and retention
  summaries.
- Component tests cover row selection and active-run update.
- Accessibility tests cover table controls and actions.

**Status:** Not started.

### Milestone 2: Diagnostics Page

**Goal:** Present run diagnostics and version-aware aggregate diagnostics.

**Tasks:**

- [ ] Adapt existing diagnostics components to active-run detail projections.
- [ ] Show scheduler decision section: facts, estimates, selected/rejected
  runtime/device, delay reasons, model load/unload, actions, and observed
  result.
- [ ] Add filters for workflow/node/model/runtime versions, scheduler policy,
  graph settings, session/bucket/client, status, date, and retention
  completeness where data exists.
- [ ] Display mixed-version warnings/facets.
- [ ] Preserve comparison-ready labels/facets for future run, workflow-version,
  runtime-version, model-version, device, and input-profile comparisons.
- [ ] Render diagnostics projections built from typed event ledger data without
  embedding event-family-specific parsing in page components.

**Verification:**

- Presenter tests cover mixed-version labels and scheduler decision summaries.
- Component tests cover active-run and no-active-run states.

**Status:** Not started.

### Milestone 3: Graph Page

**Goal:** Show the workflow version associated with the active run and preserve
editable workflow behavior.

**Tasks:**

- [ ] Implement run view for selected historic/queued run workflow version.
- [ ] Implement or preserve edit view for current editable workflow.
- [ ] Use presentation revision when available and generated layout otherwise.
- [ ] Add visible distinction between run view and edit view.
- [ ] Overlay node status/output availability when diagnostics projection
  supports it.

**Verification:**

- Tests cover historic run view not loading current editable graph.
- Tests cover presentation fallback layout behavior.
- Existing graph interaction tests pass for edit view.

**Status:** Not started.

### Milestone 4: I/O Inspector Page

**Goal:** Browse workflow and node data for the active run with retention-aware
rendering.

**Tasks:**

- [ ] Add workflow inputs and outputs sections.
- [ ] Add node-centric input/output view.
- [ ] Add artifact-centric gallery view.
- [ ] Add no-active-run retained artifact browsing where backend projections
  support it.
- [ ] Render artifact gallery projection with event-derived retention state and
  payload availability labels.
- [ ] Add renderers for text, image metadata/preview, audio placeholder/player
  where available, video placeholder/player where available, tables, JSON,
  files, and unknown/raw fallback.
- [ ] Show retention state and cleanup/policy details for each item.
- [ ] Surface global retention settings for final outputs, workflow inputs,
  intermediate node I/O, failed-run data, maximum artifact size, maximum total
  storage, media behavior, compression/archive behavior, and cleanup
  trigger/status where Stage `04` exposes them.
- [ ] Add global retention policy controls for privileged GUI users if Stage
  `04` exposes them.

**Verification:**

- Presenter tests cover artifact type labels and retention state labels.
- Component tests cover expired/deleted payload states.
- Accessibility checks cover gallery navigation and policy controls.

**Status:** Not started.

### Milestone 5: Library Page

**Goal:** Combine Library/Pumas management with active-run asset provenance.

**Tasks:**

- [ ] Show asset categories: models, runtimes, workflows, nodes, templates,
  connectors, local additions, Pumas assets, Pantograph-owned assets.
- [ ] Highlight assets used by active run.
- [ ] Show source, version, fingerprint where available, usage count, last
  accessed, linked workflow/node versions, and audit summaries.
- [ ] Render Library usage projections derived from typed `library.*` events.
- [ ] Add Pumas search/download/delete actions where backend support exists.
- [ ] Avoid optimistic display of asset mutations.

**Verification:**

- Tests cover active-run asset highlighting.
- Tests cover backend error preservation for Pumas actions.
- Tests cover rejected Pumas/Library actions without optimistic local mutation.
- Accessibility checks cover asset action buttons and filters.

**Status:** Not started.

### Milestone 6: Network And Node Lab Pages

**Goal:** Add local-first system/node visibility and reserve future node
authoring page.

**Tasks:**

- [ ] Show local instance identity, CPU/memory/GPU/disk where available,
  runtimes, models, current load, active runs, queued work, cache state,
  capability summary, and scheduler/system events.
- [ ] Structure Network data so future peer nodes can appear without a page
  rewrite.
- [ ] Highlight active-run relevant local node/runtime/model state.
- [ ] Reserve peer pairing/trust state in the data and page model so future
  Iroh work can add discovered peers, codes/keys, and verification state
  without a page rewrite.
- [ ] Add Node Lab page with clear future/experimental state and no false
  authoring affordances.

**Verification:**

- Tests cover local-only Network empty/loaded/error states.
- Tests cover platform-degraded metrics states without page crashes or fake
  values.
- Tests cover Node Lab disabled/future state.

**Status:** Not started.

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

- None. Draft plan only.

### Deviations

- None.

### Follow-Ups

- Decide table virtualization threshold during Scheduler implementation.
- Decide first-pass media renderer depth for I/O Inspector.

### Verification Summary

- Not run. Draft plan only.

### Traceability Links

- Requirement sections: Scheduler Page Requirements, Diagnostics Requirements,
  Graph Page Requirements, I/O Inspector Requirements, Library Requirements,
  Network Page Requirements, Node Lab Requirements.
