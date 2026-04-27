# src/components/workbench

## Purpose
This directory contains Pantograph’s run-centric workbench shell and first-pass
page wrappers. It gives the GUI one Scheduler-first navigation model while
later plan stages fill in richer page bodies.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `WorkbenchShell.svelte` | Top-level workbench frame, toolbar navigation, active-run summary, and page outlet. |
| `SchedulerPage.svelte` | Dense run-list view backed by the run-list projection service and active-run selection store. |
| `GraphPage.svelte` | Workbench page that switches between the active run's immutable graph snapshot and the current editable workflow graph. |
| `RunGraphSnapshot.svelte` | Read-only run graph renderer backed by `workflowService.queryRunGraph`; it does not load historic graphs into the editor store. |
| `DiagnosticsPage.svelte` | Projection-backed selected-run diagnostics page with run detail facts and scheduler timeline records. |
| `diagnosticsPagePresenters.ts` | Pure diagnostics page status, duration, projection freshness, fact, and timeline label presenters. |
| `diagnosticsPagePresenters.test.ts` | Unit coverage for diagnostics page labels and payload availability presentation. |
| `IoInspectorPage.svelte` | Projection-backed I/O artifact browser and global retention policy form. |
| `ioInspectorPresenters.ts` | Pure I/O media, payload availability, byte-size, and projection freshness presenters. |
| `ioInspectorPresenters.test.ts` | Unit coverage for I/O Inspector presentation labels. |
| `LibraryPage.svelte` | Projection-backed Library usage and audit table with active-run highlighting where the projection proves a last-run match. |
| `libraryUsagePresenters.ts` | Pure Library category, active-run match, network byte, and projection freshness presenters. |
| `libraryUsagePresenters.test.ts` | Unit coverage for Library page presentation labels and active-run matching. |
| `runGraphPresenters.ts` | Pure run graph summary, topology table, and SVG snapshot layout presenters. |
| `runGraphPresenters.test.ts` | Unit coverage for run graph version/topology presentation without editor-store state. |
| `NetworkPage.svelte` | Local-first node capability, scheduler load, disk, network-interface, degradation, and peer status page. |
| `networkPagePresenters.ts` | Pure Network page byte, transport, degraded metric, scheduler load, and local capability presenters. |
| `networkPagePresenters.test.ts` | Unit coverage for Network page metric labels and degraded platform states. |
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
- Diagnostics pages must consume run-detail and scheduler-timeline projections
  without parsing raw event ledger rows in the component.
- I/O pages must treat artifact rows as metadata projections. Payload bodies
  are not loaded unless a dedicated typed payload API exists.
- Library pages must render usage projections without issuing optimistic Pumas
  or Library mutations.
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
triggered by workflow events rather than polling.

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
- Network local-node summaries must render only API-reported local facts and
  peer records. They must not synthesize future Iroh state.
- Workbench pages must not consume raw diagnostic ledger events.
- Reserved pages must not invent backend state; they should display only data
  available through typed services or explicit unavailable states.

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
  events; callers should not add independent polling loops around it.
- Active-run selection contains identity and summary fields only. Consumers must
  query detail, timeline, graph, I/O, or Library projections for durable data.
- `GraphPage.svelte` reads historic workflow versions through
  `workflowService.queryRunGraph` and renders them through
  `RunGraphSnapshot.svelte`. It never applies that graph to the editor store.
- `DiagnosticsPage.svelte` reads selected-run facts through
  `workflowService.queryRunDetail` and scheduler history through
  `workflowService.querySchedulerTimeline`.
- `IoInspectorPage.svelte` reads artifact metadata through
  `workflowService.queryIoArtifacts` and global retention state through
  `workflowService.queryRetentionPolicy`.
- Retention policy saves call `workflowService.updateRetentionPolicy` and
  update displayed state only from the backend response.
- `LibraryPage.svelte` reads usage and audit summaries through
  `workflowService.queryLibraryUsage`.

## Structured Producer Contract
- Workbench navigation order comes from `WORKBENCH_PAGES` in
  `workbenchStore.ts`.
- Scheduler table rows are `RunListProjectionRecord` values returned by
  `workflowService.queryRunList`.
- I/O artifact cards render `IoArtifactProjectionRecord` metadata and may show
  `payload_ref` availability, but do not dereference payload bodies.
- Library usage rows render `LibraryUsageProjectionRecord` summaries and may
  highlight only rows whose `last_workflow_run_id` equals the active run.
- Run graph snapshot rows render `WorkflowRunGraphProjection` topology,
  presentation revision, graph settings, and execution fingerprint fields.
- Diagnostics fact rows render `RunDetailProjectionRecord` fields, and
  timeline rows render `SchedulerTimelineProjectionRecord` summaries.
- Network status cards are derived from `WorkflowLocalNetworkStatusQueryResponse`.
- Network disk and interface rows render reported local metrics and show
  unavailable states when platform probes do not provide rows.
- Reserved page unavailable states are not persisted and do not imply backend
  capability flags.
