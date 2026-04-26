# src/components/diagnostics

## Purpose
This directory contains the workflow diagnostics view for Pantograph's existing
GUI. It renders retained diagnostics snapshots from `src/stores/diagnosticsStore.ts`
without owning event subscriptions, runtime state machines, or workflow service
logic.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `DiagnosticsPanel.svelte` | Shell component that renders the bottom diagnostics panel, retained run list, and all diagnostics tabs. |
| `DiagnosticsOverview.svelte` | Overview tab with run-level summary cards and node detail panels. |
| `DiagnosticsNodeDetail.svelte` | Focused selected-node inspector that renders lifecycle, duration expectation, optional reported progress, messages, and errors. |
| `DiagnosticsTimingExpectation.svelte` | Small reusable duration-expectation badge used by overview and timeline views. |
| `DiagnosticsWorkflowHistory.svelte` | Opened-workflow timing history view shown before a retained run is selected. |
| `DiagnosticsTimeline.svelte` | Timeline tab that visualizes relative node spans inside a selected run. |
| `DiagnosticsEvents.svelte` | Events tab that shows retained workflow events and raw payload details. |
| `DiagnosticsScheduler.svelte` | Scheduler tab that renders session state and queue ordering from workflow service diagnostics snapshots. |
| `DiagnosticsRuntime.svelte` | Runtime tab that renders workflow capabilities, runtime requirements, and runtime install state. |
| `DiagnosticsGraph.svelte` | Graph tab that renders current graph metadata, graph-related diagnostics events, the latest backend-owned graph-memory impact summary, and additive workflow-session inspection state. |
| `presenters.ts` | Presentation helpers for durations, timestamps, status badges, overview counts, graph-memory summaries, and workflow-session inspection labels. |

## Problem
Pantograph needs a developer-facing diagnostics surface inside the workflow GUI,
but the panel should remain a renderer over diagnostics snapshots rather than a
second owner of workflow transport state.

## Constraints
- Components in this directory must consume diagnostics snapshots declaratively.
- The panel must fit inside the existing workflow editor rather than introducing
  a parallel top-level screen.
- Scheduler, runtime, and graph tabs should render workflow-service-backed or
  diagnostics-store-backed state instead of inventing component-local shadow
  models.
- Scheduler rendering must tolerate both authoritative workflow-service
  snapshots and the synthetic edit-session fallback state produced by the
  diagnostics store.
- Components may format diagnostics data for readability, but they must not
  mutate trace state directly.
- Graph diagnostics may display backend-owned workflow-session inspection facts
  such as residency, checkpoint summary, and per-node memory snapshots, but
  they must not infer or repair those facts locally.

## Decision
Render the diagnostics surface as a bottom panel under the workflow graph.
`DiagnosticsPanel.svelte` owns panel composition and delegates each active tab
to focused child components. Formatting logic stays in `presenters.ts` so Svelte
files mostly express layout and interaction. Duration expectation rendering is
shared through `DiagnosticsTimingExpectation.svelte` so the overview and
timeline views do not reinterpret backend timing fields independently.
When no retained run is selected, `DiagnosticsWorkflowHistory.svelte` renders
the backend-projected timing history for the opened workflow graph so previous
diagnostics remain visible before a new run starts.

## Alternatives Rejected
- Add diagnostics as a third top-level app mode.
  Rejected because the user needs graph editing and diagnostics in the same
  workflow workspace.
- Put event normalization directly into these components.
  Rejected because trace ownership belongs to the diagnostics service/store
  boundary.

## Invariants
- Components render diagnostics state supplied by the store and do not subscribe
  to workflow events directly.
- Tab switching and node/run selection use exported diagnostics store commands.
- The diagnostics store must not auto-select retained runs just because
  retained history exists. It may select a run when backend projection context
  marks that run relevant, and workflow switches must clear selected run/node
  state so opened-workflow timing history can render before a new run starts.
- Runtime, scheduler, and graph inspection rendering should stay read-only over
  store snapshots, not call workflow commands directly from the component tree.
- Duration expectation badges render backend-projected timing history only.
  Components must not calculate historical baselines from local run lists.
- Opened-workflow timing history must come from the diagnostics snapshot, not
  from retained run rows or component-local cache.
- Scheduler copy and layout should remain valid when queue ordering is
  synthesized for single-run edit sessions rather than backed by a real
  workflow-service queue.
- Session-inspection cards and node-memory lists render additive backend
  snapshots and must degrade cleanly when `currentSessionState` is `null`.

## Revisit Triggers
- Diagnostics needs detached windows or a second layout mode.
- The panel grows enough that tab content should move behind route-level code
  splitting.
- Runtime and scheduler traces become dense enough to require specialized
  visualizations beyond the current table and card layouts.

## Dependencies
**Internal:** `src/stores/diagnosticsStore.ts`,
`src/services/diagnostics`, `src/components/WorkflowToolbar.svelte`,
`src/App.svelte`.
**External:** Svelte 5 and Tailwind utility classes already used by the app.

## Related ADRs
- None yet.
- Reason: the panel is an additive frontend surface over the diagnostics plan.
- Revisit trigger: diagnostics becomes a durable product surface with its own
  routing or persistence concerns.

## Usage Examples
```svelte
<DiagnosticsPanel />
```

## API Consumer Contract
- `DiagnosticsPanel.svelte` assumes the app has already started the diagnostics
  store lifecycle.
- Child components receive already-selected diagnostics run/node props and emit
  selection changes back through callbacks or the store facade.
- Presentation helpers in `presenters.ts` should stay pure so they remain easy
  to test and safe to reuse across tabs.
- Generic diagnostics progress is not a universal node state. Optional reported
  progress may be shown only as node-specific telemetry when the backend
  reports it.

## Structured Producer Contract
- None.
- Reason: the directory renders frontend views and does not publish machine
  artifacts directly.
- Revisit trigger: diagnostics tab configuration or exported panel layouts
  become generated/shared metadata.
