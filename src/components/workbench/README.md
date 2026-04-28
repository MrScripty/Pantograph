# src/components/workbench

## Purpose
This directory contains Pantograph’s run-centric workbench shell and first-pass
page wrappers. It gives the GUI one Scheduler-first navigation model while
later plan stages fill in richer page bodies.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `WorkbenchShell.svelte` | Top-level workbench frame, toolbar navigation, active-run summary, and page outlet. |
| `SchedulerPage.svelte` | Dense run-list view backed by the run-list projection service, active-run selection store, local table controls, policy/scope/date filters, scope columns, typed queue/estimate columns, backend-gated queue actions, and selected-run scheduler timeline projection. |
| `schedulerPagePresenters.ts` | Pure Scheduler page status, duration, timestamp, scope/date, queue-control gating, queue/estimate, filter, sorting, projection freshness, and timeline presenters. |
| `schedulerPagePresenters.test.ts` | Unit coverage for Scheduler table labels, status classes, filters, sorts, projection freshness, and timeline labels. |
| `GraphPage.svelte` | Workbench page that switches between the active run's immutable graph snapshot and the current editable workflow graph. |
| `RunGraphSnapshot.svelte` | Read-only run graph renderer backed by `workflowService.queryRunGraph`; it does not load historic graphs into the editor store. |
| `DiagnosticsPage.svelte` | Projection-backed selected-run diagnostics page with run detail facts, date-aware filtered comparison facets, mixed-version warnings, and scheduler timeline records. |
| `diagnosticsPagePresenters.ts` | Pure diagnostics page status, duration, projection freshness, run authority fact, comparison date/filter/facet, and timeline label presenters. |
| `diagnosticsPagePresenters.test.ts` | Unit coverage for diagnostics page labels, comparison filters/facets, and payload availability presentation. |
| `IoInspectorPage.svelte` | Projection-backed I/O artifact browser, retention detail surface, and global retention policy form. |
| `ioInspectorPresenters.ts` | Pure I/O media, payload availability, retention detail, byte-size, and projection freshness presenters. |
| `ioInspectorPresenters.test.ts` | Unit coverage for I/O Inspector presentation labels. |
| `LibraryPage.svelte` | Projection-backed Library usage and audit table with active-run highlighting and audited Pumas search/download/delete actions. |
| `libraryUsagePresenters.ts` | Pure Library category, active-run match, network byte, and projection freshness presenters. |
| `libraryUsagePresenters.test.ts` | Unit coverage for Library page presentation labels and active-run matching. |
| `runGraphPresenters.ts` | Pure run graph summary, topology table, and SVG snapshot layout presenters. |
| `runGraphPresenters.test.ts` | Unit coverage for run graph version/topology presentation without editor-store state. |
| `NetworkPage.svelte` | Local-first node capability, scheduler load, disk, network-interface, degradation, and peer status page. |
| `networkPagePresenters.ts` | Pure Network page byte, transport, degraded metric, scheduler load, and local capability presenters. |
| `networkPagePresenters.test.ts` | Unit coverage for Network page metric labels and degraded platform states. |
| `workflowErrorPresenters.ts` | Shared workbench formatter for typed workflow service errors so backend categories remain visible in page messages. |
| `workflowErrorPresenters.test.ts` | Unit coverage for backend error-envelope and transport-error formatting. |
| `NodeLabPage.svelte` | Reserved Node Editor page for future node authoring workflows. |

## Problem
The previous app root presented mutually exclusive canvas and workflow modes.
Pantograph needs persistent top-level pages that share a selected run, so
Scheduler, Diagnostics, Graph, I/O, Library, Network, and Node Editor do not
grow separate navigation and selection models.

## Constraints
- Scheduler must be the default page.
- Active-run context is transient frontend state and must not persist across
  GUI restart.
- Page bodies must consume backend projection services instead of raw
  diagnostic event ledger rows.
- Page bodies must display workflow command failures through presenters that
  preserve backend error categories.
- Diagnostics pages must consume run-detail and scheduler-timeline projections
  without parsing raw event ledger rows in the component.
- I/O pages must treat artifact rows as metadata projections. Payload bodies
  are not loaded unless a dedicated typed payload API exists.
- Library pages must render usage projections without issuing optimistic Pumas
  or Library mutations. Audited Pumas actions must call typed workflow service
  commands and refresh projection state after confirmed backend responses.
- Historic run graph rendering must use immutable run graph projections and
  must not mutate the current editor store.
- Network pages must distinguish unavailable platform metrics from zero values.
- Existing graph and diagnostics surfaces must remain usable while ownership
  moves into the workbench shell.
- Toolbar navigation must use semantic buttons with accessible names.

## Decision
Create a dedicated workbench component boundary under `src/components` and let
`App.svelte` mount it as the only root workspace. The shell reads page and
active-run state from `workbenchStore.ts`. Scheduler uses the typed run-list
projection service to populate the initial dense list and coalesces refreshes
triggered by workflow events rather than polling. Scheduler table filters, sort
order, and column visibility live in `schedulerRunListStore.ts` so they remain
transient UI state without becoming backend scheduler policy.

## Alternatives Rejected
- Keep the old canvas/workflow view-mode toggle as the root shell.
  Rejected because it preserves duplicate navigation semantics and prevents
  selected-run context from spanning pages.
- Persist the active run in backend state or local storage.
  Rejected because requirements make the active run a per-GUI-session
  navigation selection only.

## Invariants
- `WorkbenchShell.svelte` owns page routing, not page bodies.
- Scheduler row selection may set active-run context, but durable run data must
  still be fetched from projection services by each page.
- Scheduler table search, status filter, policy-field filters, scope filters,
  accepted-date filter, sort controls, and column visibility operate only on
  the materialized run-list projection returned by the backend. The control
  state is owned by
  `schedulerRunListStore.ts`, not by the backend scheduler and not by
  component-local durable state.
- Scheduler timeline rows come from `workflowService.querySchedulerTimeline`.
  Components render typed summary/detail fields and payload availability only.
- Scheduler client, session, bucket, and workflow execution-session facts come
  from run-list projection fields. Components must not recover those scope
  facts from raw events.
- Scheduler queue action buttons must be gated by projected workflow
  execution-session ids and backend run status. They must call workflow-service
  queue cancel or push-front commands and refresh projections after confirmed
  backend responses.
- Scheduler status presentation includes delayed rows from the run-list
  projection. Components must treat delayed as backend-authored state rather
  than inferring it from scheduler reason text.
- I/O artifact rendering must distinguish metadata-only rows from rows with
  payload references without treating missing payload references as failures.
- Library active-run highlighting must use explicit projection facts, not
  inferred workflow or asset name matches.
- Run graph snapshots are read-only projection views. Switching to the current
  editor keeps the selected run context for other pages but does not imply the
  editor graph matches that run.
- Diagnostics timeline rows render typed scheduler projection summaries and
  payload availability only; detailed payload parsing belongs in backend
  projections or future typed presenters.
- Diagnostics comparison facets are derived from selected-run detail and
  run-list projections, preferring backend-owned run-list facet counts when
  present. They must not parse diagnostic event payloads or depend on sampled
  page rows for comparison totals.
- Network local-node summaries must render only API-reported local facts and
  peer records. They must not synthesize future Iroh state.
- Workbench pages must not consume raw diagnostic ledger events.
- Reserved pages must not invent backend state; they should display only data
  available through typed services or explicit unavailable states.
- Workbench error messages must be formatted from typed workflow service
  errors. Components must not stringify backend envelopes directly.

## Revisit Triggers
- A router is introduced for deep links or browser-style navigation.
- Library or Node Editor graduate from reserved pages to full feature
  surfaces with their own subnavigation.
- The read-only snapshot renderer needs richer node status or artifact overlays.

## Dependencies
**Internal:** `src/stores/workbenchStore.ts`, `src/services/workflow`,
`src/services/diagnostics`, existing graph and diagnostics components.

**External:** Svelte, lucide-svelte.

## Related ADRs
- None identified as of 2026-04-27.
- Reason: this is the first committed workbench shell implementation slice.
- Revisit trigger: shell routing, active-run ownership, or projection
  consumption rules need a repository-wide decision record.

## Usage Examples
```svelte
<script lang="ts">
  import WorkbenchShell from './components/workbench/WorkbenchShell.svelte';
</script>

<WorkbenchShell />
```

## API Consumer Contract
- App root mounts `WorkbenchShell.svelte` once per GUI runtime.
- Page selection is controlled through `workbenchStore.ts` actions.
- Scheduler refreshes the typed run-list projection on mount and workflow
  events. Supported status, assigned policy, assigned scope, and accepted-date
  filters are sent to the backend run-list query; search, sort, and
  `Unassigned` labels stay local presentation filters. Callers should not add
  independent polling loops around it.
- Active-run selection contains identity and summary fields only. Consumers must
  query detail, timeline, graph, I/O, or Library projections for durable data.
- `GraphPage.svelte` reads historic workflow versions through
  `workflowService.queryRunGraph` and renders them through
  `RunGraphSnapshot.svelte`. It reads selected-run artifact metadata through
  `workflowService.queryIoArtifacts` for node output-availability overlays and
  selected-run node status through `workflowService.queryNodeStatus` for
  runtime-status overlays. It never applies that graph to the editor store.
- `DiagnosticsPage.svelte` reads selected-run facts through
  `workflowService.queryRunDetail`, scheduler history through
  `workflowService.querySchedulerTimeline`, and comparison peers through
  `workflowService.queryRunList`.
- `IoInspectorPage.svelte` reads artifact metadata through
  `workflowService.queryIoArtifacts` and global retention state through
  `workflowService.queryRetentionPolicy`. Artifact retention labels come from
  `IoArtifactProjectionRecord.retention_state`, not from `payload_ref`
  inference. Retention completeness counts come from the response
  `retention_summary`, not from raw ledger events.
- Retention policy saves call `workflowService.updateRetentionPolicy` and
  update displayed state only from the backend response. The page may show a
  saving state, but it must not apply the requested policy as if it were
  accepted before the backend responds.
- Retention cleanup actions call `workflowService.applyRetentionCleanup`, show
  the backend cleanup count, and refresh artifact metadata from projections
  instead of removing artifact cards locally.
- `LibraryPage.svelte` reads usage and audit summaries through
  `workflowService.queryLibraryUsage`.

## Structured Producer Contract
- Workbench navigation order comes from `WORKBENCH_PAGES` in
  `workbenchStore.ts`.
- Scheduler table rows are `RunListProjectionRecord` values returned by
  `workflowService.queryRunList`.
- Scheduler table controls are frontend presentation filters and must not imply
  backend scheduler priority or queue mutations. Scheduler policy, retention
  policy, scope, and accepted-date filters use typed `RunListProjectionRecord`
  fields, and assigned filter values may also narrow the backend projection
  query before local presentation filtering. Queue position, priority,
  estimate, and scheduler reason columns also render typed projection fields,
  not scheduler payload JSON.
- Scheduler timeline rows are `SchedulerTimelineProjectionRecord` values and
  must not be rebuilt or interpreted from raw ledger rows in the frontend.
- I/O artifact cards render `IoArtifactProjectionRecord` metadata and may show
  typed retention-state, retention reason, runtime/model ids, and `payload_ref`
  availability, but do not dereference payload bodies.
- Library usage rows render `LibraryUsageProjectionRecord` summaries and may
  highlight only rows whose `last_workflow_run_id` equals the active run.
- Run graph snapshot rows render `WorkflowRunGraphProjection` topology,
  presentation revision, graph settings, and execution fingerprint fields.
  Node I/O overlays render summarized `IoArtifactProjectionRecord` metadata and
  must not dereference payload bodies or inspect raw ledger rows. Node status
  overlays render `NodeStatusProjectionRecord` rows, not raw diagnostic events.
- Diagnostics fact rows render `RunDetailProjectionRecord` fields, and
  comparison filters/facets use `RunListProjectionRecord` fields. Scheduler
  estimate and queue facts are read from typed projection fields. Timeline rows
  render `SchedulerTimelineProjectionRecord` summaries.
- Network status cards are derived from `WorkflowLocalNetworkStatusQueryResponse`.
- Network disk and interface rows render reported local metrics and show
  unavailable states when platform probes do not provide rows.
- Workflow command errors use `formatWorkflowCommandError`, preserving backend
  category labels such as `invalid_request`, `scheduler_busy`, and
  `queue_item_not_found` for users and tests.
- Queue and retention command results are backend-owned. Workbench components
  must refresh projections or use returned DTOs instead of editing queue rows
  or retention facts optimistically.
- Reserved page unavailable states are not persisted and do not imply backend
  capability flags.
