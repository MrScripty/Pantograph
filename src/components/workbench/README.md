# src/components/workbench

## Purpose
This directory contains Pantograph’s run-centric workbench shell and first-pass
page wrappers. It gives the GUI one Scheduler-first navigation model while
later plan stages fill in richer page bodies.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `WorkbenchShell.svelte` | Top-level workbench frame, toolbar navigation, active-run summary, and page outlet. |
| `SchedulerPage.svelte` | Dense run-list view backed by the run-list projection service, active-run selection store, local table controls, future/scheduled/queued status filters, policy/scope/placement/date filters, scope and placement columns, typed queue/estimate columns, selected-run estimate and retention projection panels, backend-gated queue actions including session priority controls, and selected-run scheduler timeline projection with typed kind/source filters. |
| `schedulerPagePresenters.ts` | Pure Scheduler page status labels/classes, duration, timestamp, future/scheduled status, scope/placement/date, queue-control gating, selected-run estimate and retention rows, queue/estimate, filter, sorting, projection freshness, and typed timeline filter presenters. |
| `schedulerPagePresenters.test.ts` | Unit coverage for Scheduler table labels, status classes, filters, sorts, projection freshness, and timeline labels. |
| `GraphPage.svelte` | Workbench page that switches between the active run's immutable graph snapshot and the current editable workflow graph. |
| `RunGraphSnapshot.svelte` | Read-only run graph renderer backed by `workflowService.queryRunGraph`; it does not load historic graphs into the editor store. |
| `DiagnosticsPage.svelte` | Projection-backed selected-run diagnostics page with run detail facts, workflow-version/date-range/placement filtered comparison facets, mixed-version warnings, and scheduler timeline records. |
| `diagnosticsPagePresenters.ts` | Pure diagnostics page status labels/classes, duration, projection freshness, run authority/placement facts, workflow-version/date-range/filter/facet, and timeline label presenters. |
| `diagnosticsPagePresenters.test.ts` | Unit coverage for diagnostics page labels, comparison filters/facets, and payload availability presentation. |
| `IoInspectorPage.svelte` | Projection-backed I/O artifact browser, retention detail surface, cleanup status surface, and global retention policy form. |
| `ioInspectorPresenters.ts` | Pure I/O media, payload availability, retention policy/cleanup detail, byte-size, and projection freshness presenters. |
| `ioInspectorPresenters.test.ts` | Unit coverage for I/O Inspector presentation labels. |
| `LibraryPage.svelte` | Projection-backed Library usage and audit table with active-run highlighting and audited Pumas search/download/delete actions. |
| `libraryUsagePresenters.ts` | Pure Library category, active-run match, network byte, and projection freshness presenters. |
| `libraryUsagePresenters.test.ts` | Unit coverage for Library page presentation labels and active-run matching. |
| `runGraphPresenters.ts` | Pure run graph summary, topology table, and SVG snapshot layout presenters. |
| `runGraphPresenters.test.ts` | Unit coverage for run graph version/topology presentation without editor-store state. |
| `NetworkPage.svelte` | Local-first node capability, scheduler load, selected-run placement with scheduler model-cache posture, selected-run execution state, selected-run Library resources with typed status highlighting, scheduler events, disk, network-interface, degradation, and peer status page. |
| `networkPagePresenters.ts` | Pure Network page byte, transport, degraded metric, scheduler load, selected-run placement/model-cache/execution/resource labels and status classes, projection freshness, and local capability presenters. |
| `networkPagePresenters.test.ts` | Unit coverage for Network page metric labels and degraded platform states. |
| `workflowErrorPresenters.ts` | Shared workbench formatter for typed workflow service errors so backend categories remain visible in page messages. |
| `workflowErrorPresenters.test.ts` | Unit coverage for backend error-envelope and transport-error formatting. |
| `NodeLabPage.svelte` | Reserved Node Editor page with a presenter-backed unavailable state and no authoring controls. |
| `nodeLabPresenters.ts` | Pure Node Editor unavailable-state rows and message presenter. |
| `nodeLabPresenters.test.ts` | Unit coverage for Node Editor disabled-state presentation. |

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
- Existing graph editing surfaces must remain usable while ownership moves into
  the workbench shell.
- The workbench Diagnostics page owns the active diagnostics surface. Graph page
  toolbars must keep diagnostics navigation inside the workbench page model.
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
  placement filters, accepted-date filter, sort controls, and column visibility
  operate only on the materialized run-list projection returned by the backend.
  The control state is owned by
  `schedulerRunListStore.ts`, not by the backend scheduler and not by
  component-local durable state.
- Scheduler timeline rows come from `workflowService.querySchedulerTimeline`.
  Components render typed summary/detail fields and payload availability only.
  Timeline filtering uses typed event kind and source fields from the
  projection, not payload JSON.
- Scheduler selected-run estimate facts come from
  `workflowService.querySchedulerEstimate` and presenter-built rows. Components
  must not parse `latest_estimate_json` for Scheduler page display.
- Scheduler selected-run retention counts come from
  `workflowService.queryIoArtifacts` retention summaries and presenter-built
  rows. Components must not dereference payload bodies or replay ledger rows for
  Scheduler retention display.
- Scheduler client, session, bucket, and workflow execution-session facts come
  from run-list projection fields. Components must not recover those scope
  facts from raw events.
- Scheduler selected runtime, selected device, and selected network-node facts
  come from run-list projection fields. Components must not recover those
  placement facts from scheduler payload JSON.
- Scheduler session-scoped queue action buttons must be gated by projected
  workflow execution-session ids and backend run status. Session priority
  changes use `workflowService.reprioritizeSessionQueueItem`; GUI-admin queue
  action buttons are gated by backend run status and call run-id admin command
  boundaries. Both paths must refresh projections after confirmed backend
  responses.
- Scheduler future, scheduled, queued, delayed, running, and terminal status
  presentation comes from the run-list projection. Components must treat those
  statuses as backend-authored state rather than inferring them from scheduler
  reason text.
- I/O artifact rendering must distinguish metadata-only rows from rows with
  payload references without treating missing payload references as failures.
- I/O Inspector node grouping and endpoint filters must use producer and
  consumer projection fields. Components may send backend producer/consumer
  filters, but must not infer endpoint ownership from raw payload JSON.
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
- Diagnostics comparison filters and facets use typed selected runtime, device,
  and network-node projection fields when present. Components must not recover
  placement context from scheduler payload JSON.
- Network local-node summaries must render only API-reported local facts and
  peer records. They must not synthesize future Iroh state.
- Network selected-run placement rows must come from local status placement
  records, including scheduler-owned model-cache posture when reported.
  Components must not infer runtime/model state from workflow ids or scheduler
  event payloads.
- Network selected-run execution rows must come from node-status projections.
  They are execution facts, not local cache-residency facts.
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
  runtime-status overlays. It never applies that graph to the editor store, and
  its editor toolbar stays focused on workflow persistence/execution rather
  than reopening separate diagnostics controls.
- `DiagnosticsPage.svelte` reads selected-run facts through
  `workflowService.queryRunDetail`, scheduler history through
  `workflowService.querySchedulerTimeline`, and comparison peers through
  `workflowService.queryRunList`. Comparison filters use projected workflow
  version, status, policy, scope, bucket, accepted-date, and accepted-date
  range fields. Selected-run retention completeness uses
  `workflowService.queryIoArtifacts` response `retention_summary` counts and
  I/O projection freshness, not raw ledger events. Selected-run execution
  facets use `workflowService.queryNodeStatus` node status projection rows for
  node, runtime, and model version labels and filters.
- `IoInspectorPage.svelte` reads artifact metadata through
  `workflowService.queryIoArtifacts` and global retention state through
  `workflowService.queryRetentionPolicy`. Artifact retention labels come from
  `IoArtifactProjectionRecord.retention_state`, not from `payload_ref`
  inference. Retention completeness counts come from the response
  `retention_summary`, not from raw ledger events. Retention policy setting
  rows render backend-provided policy settings rather than hardcoded page
  categories.
- Retention policy saves call `workflowService.updateRetentionPolicy` and
  update displayed state only from the backend response. The page may show a
  saving state, but it must not apply the requested policy as if it were
  accepted before the backend responds.
- Retention cleanup actions call `workflowService.applyRetentionCleanup`, show
  the backend cleanup count and detail rows, and refresh artifact metadata from
  projections instead of removing artifact cards locally.
- `LibraryPage.svelte` reads usage and audit summaries through
  `workflowService.queryLibraryUsage`. Audited HuggingFace search results use
  the typed diagnostics service result shape and render model ids without
  treating provider rows as arbitrary `unknown` values.

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
  estimate and queue facts are read from typed projection fields. Selected-run
  node, runtime, and model execution facets render `NodeStatusProjectionRecord`
  fields and filter only those selected-run projection rows. Timeline rows
  render `SchedulerTimelineProjectionRecord` summaries.
- Network status cards are derived from `WorkflowLocalNetworkStatusQueryResponse`.
- Network disk and interface rows render reported local metrics and show
  unavailable states when platform probes do not provide rows.
- Network selected-run placement uses backend-provided local active/queued run
  id lists from scheduler-load facts. It must not infer runtime/model/cache
  residency from selected-run context alone.
- Network selected-run placement details render backend-provided run-placement
  records for session id, runtime-loaded posture, and required backend/model
  facts. These are scheduler facts, not cache-residency claims.
- Network selected-run resources render active-run Library usage projection
  rows and last cache observations. They are audit facts, not local
  cache-residency claims. Cache-observation badges are derived only from typed
  Library usage projection fields.
- Network selected-run execution rows render `NodeStatusProjectionRecord`
  fields for node status, runtime id/version, and model id/version. They are
  not cache-residency claims. Status badges are derived only from typed
  node-status projection fields.
- Network selected-run events use `workflowService.querySchedulerTimeline` and
  render typed scheduler projection summaries. The page must not read raw
  diagnostic ledger rows to explain local scheduler activity.
- Workflow command errors use `formatWorkflowCommandError`, preserving backend
  category labels such as `invalid_request`, `scheduler_busy`, and
  `queue_item_not_found` for users and tests.
- Queue and retention command results are backend-owned. Workbench components
  must refresh projections or use returned DTOs instead of editing queue rows
  or retention facts optimistically.
- Reserved page unavailable states are not persisted and do not imply backend
  capability flags. Node Editor disabled-state copy and rows come from
  `nodeLabPresenters.ts` so the unavailable surface stays testable without
  exposing authoring controls.
