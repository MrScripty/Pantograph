# M2 Diagnostics: report

Agent: Opus 5.5 (M2). Date: 2026-11-03. Branch `feat/workbench-redesign`, working
tree only, nothing committed.

## Changed files

| File | Change |
| --- | --- |
| `src/components/workbench/DiagnosticsPage.svelte` | Rewritten to the `Diagnostics` board: dimension line, 300px "Run facts" panel, "Events" panel with level and source segments, search, Follow, Refresh, ErrorCard, log lines, footer sentence. Same five service calls and the same invalidation subscription as before. |
| `src/components/workbench/diagnosticsPagePresenters.ts` | Removed the Tailwind colour helpers (`diagnosticsStatusClass`, `diagnosticsErrorSeverityClass`, `diagnosticsTimelineRowClass`), the old `all`/`errors` timeline filter and the unused `formatDiagnosticsTimestamp`. Enum formatters now return sentence-case words. Added the redesign presenters (listed below). |
| `src/components/workbench/diagnosticsPagePresenters.test.ts` | Removed tests of the deleted helpers, updated the formatter expectations, added 14 tests for the new presenters (30 tests in the file). |
| `docs/plans/workbench-redesign/reports/tools/fixture-diagnostics.js` | New CDP fixture, as the caller asked. It loads after `fixture.js`, reuses its runs and answers the diagnostics queries: run detail, scheduler timeline, node status, I/O retention, workflow-scoped run list with 12 earlier image-gen-sdxl runs, and event `listen`. |
| `docs/plans/workbench-redesign/reports/m2-diagnostics.md` | This report. |

New presenter functions:

- **Timing and words:** `formatDiagnosticsClock`, `formatDiagnosticsLogTime`, `formatDiagnosticsElapsed`, `formatDiagnosticsRunDuration`, `diagnosticsWords`, `diagnosticsSentenceCase`.
- **Header and dimension line:** `buildDiagnosticsDimensionLine`, `buildDiagnosticsPlacementLabel`, `buildDiagnosticsClientLabel`.
- **Left-panel blocks:** `buildDiagnosticsFactSections` (one layout table assigns all 27 fact rows), `buildDiagnosticsRetentionRows`, `buildDiagnosticsProjectionRows`, `buildDiagnosticsNodeStatusBars`, `groupDiagnosticsExecutionFacetRows`.
- **Filters:** the filter field descriptors `DIAGNOSTICS_EXECUTION_FILTER_FIELDS` (6) and `DIAGNOSTICS_COMPARISON_FILTER_FIELDS` (14 selects and 2 dates), `buildDiagnostics{Execution,Comparison}FilterChips`, `buildDiagnosticsComparisonShortcuts`.
- **Comparison:** `buildDiagnosticsComparisonSummary`.
- **Event log:** `buildDiagnosticsLogLines`, `buildDiagnosticsLevelOptions`, `filterDiagnosticsLogLines`, `buildDiagnosticsNodeErrorsOutsideTimeline`.
- **Deep-link focus:** `resolveDiagnosticsFocus`, `diagnosticsFocusKey`.
- **Error card:** `explainDiagnosticsError` and `buildDiagnosticsErrorCard`.
- **Refresh ownership:** `createDiagnosticsRefreshGate`.

## How the page-specific requirements are met

- **Facts stay reachable.**
  - A layout table in the presenter places each of the 27 `buildDiagnosticsFactRows` facts in exactly one block: Timing (lifecycle times, then the estimates), Placement, Retention (policy plus the retention-state counts), or the collapsible "More facts". A test checks that the layout covers every fact label once and that every fact value is placed.
  - "More facts" also lists the latest error event id, the fatal error event id and the terminal error when they differ from what the card shows.
  - The Projection section has one key-value row per projection: run detail, scheduler timeline, run list, node status, I/O retention.
- **Filters.**
  - The six execution selects sit in a `Filters` disclosure inside "Nodes by status". Next to it, a `Breakdown` disclosure keeps the old per-facet node counts.
  - The 14 comparison selects and 2 date inputs sit in a `Filters` disclosure inside "Compared with earlier runs".
  - Both sections show removable `FilterChip`s for applied filters. The board's quick pills (`Only v0.9.1`, `Only cuda:1`, `Last 7 days`) are `aria-pressed` presets over the same filter fields.
- **Node errors.**
  - When a node status row's `canonical_error_event_id` or `error_event_id` points at a timeline event, that event's LogLine takes the node id as its kind, at least error level (so the failed glyph) and the node's error code.
  - Nodes whose error event is not in the timeline (or that have no event id) appear in a compact table under the ErrorCard. Nothing else is in that table.
- **`diagnosticsFocus` deep link.**
  - The page applies each focus request once (keyed by run id and `requested_at_ms`), after its run's projection has loaded. It sets the level filter to the focused line's level, resets source and search, turns Follow off, marks the line `aria-current` and scrolls it into view.
  - A request that names only a node resolves through the node's error event.
  - As before, the page never clears the focus itself. The store drops it when another run is selected.
  - "Show in log" uses the same select-and-scroll path, with the level set to Errors.
- **ErrorCard text.**
  - `explainDiagnosticsError` holds a table of codes from the workflow-service error registry (`diagnostic_errors.rs`): `runtime_model_load_failed`, `runtime_preflight_failed`, `runtime_launch_failed`, `scheduler_admission_failed`, `model_dependency_failed`, `node_execution_failed`, `output_validation_failed`, `run_timeout`.
  - Any other code gets a neutral title built from the phase, the explanation "The run reported: <message>." and a suggestion pointing to the log.
  - A warning on a run that did not fail is shown as one muted line, not as the danger card.
- **No page-wide red banners.**
  - A refresh failure keeps the last projection of the same run on screen and adds a muted sentence in the Events panel. A different run's projection is never shown.
  - A first-load failure gives a centred muted sentence and a `Try again` button.
  - A failed live-updates subscription gives a muted sentence ("Live updates are off…").
  - The shell keeps the engine-unavailable line and the dimming.
- **No ledger reads.** The page makes the same `queryRunDetail`, `querySchedulerTimeline`, `queryRunList({workflow_id})`, `queryNodeStatus` and `queryIoArtifacts` calls, and keeps the same `subscribeDiagnosticsProjectionInvalidations` subscription. Nothing parses `payload_json` beyond the existing presence check.
- **Ownership of async work and listeners.**
  - Overlapping refreshes go through a refresh gate created per component. It is closed on destroy, so a refresh that finishes after unmount writes nothing (tested).
  - The invalidation subscription is owned by `onMount` and released on destroy, as before.
  - Follow and scroll-into-view are `$effect`s with no timers or listeners.
  - `activeRunId` is a derived string, so re-selecting the same run does not trigger a refresh.

## Verification (verbatim, from the repo root, 2026-11-03)

```
$ npm run typecheck
> tsc --project tsconfig.json --noEmit --pretty false
(no output, exit 0)

$ npm run test:frontend
ℹ tests 685
ℹ pass 685
ℹ fail 0

$ npm run lint:a11y
svelte accessibility gate passed

$ npm run lint:critical
critical anti-pattern gate passed

$ npx eslint src/components/workbench/DiagnosticsPage.svelte src/components/workbench/diagnosticsPagePresenters.ts src/components/workbench/diagnosticsPagePresenters.test.ts --max-warnings 0
(0 problems)

$ TRACEABILITY_BASE_REF=codex/pantograph-rpc-workflow-2026-09-26 ./scripts/check-decision-traceability.sh
No changed files detected for decision traceability check.
```

As WR-003 says, range mode proves nothing for uncommitted work. I ran a scratch copy of the hook with an explicit file list
(`<scratch>/m2/traceability-m2.sh`):

- **The three M2 files alone:** fails with "Changed directory without decision traceability update: src/components/workbench".
- **The same files plus `src/components/workbench/README.md`:** "Decision traceability check passed." That README is M0's uncommitted file, so it must go in the same commit as M2 or be touched for it.

```
$ npx vite build --outDir <scratch>/m2/dist
✓ built in 21.75s
```

`<scratch>` is `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad`.

### Headless walkthrough

Set-up:

- **Server:** `python3 -m http.server 8792 --directory <scratch>/m2/dist`, stopped afterwards.
- **Driver:** `CDP_PORT=9342 node docs/plans/workbench-redesign/reports/tools/cdp-init.cjs <steps>`.
- **Init steps:** `fixture.js`, then `fixture-diagnostics.js`.
- **Steps files:** `<scratch>/m2/steps.json`, `<scratch>/m2/steps-unavailable.json` (base fixture only), `<scratch>/m2/steps-nodes.json`.

Logged results:

```
dim, levels, lines: ["run_3f2504e0-4f89-11d3-9a0c-0305e82c3301, failed 00:36:23 after 22 s, 1 error in load_model, 1 warning",["All 14:true","Warnings 1:false","Errors 2:false"],14]
after Show in log (levels, selected line, line count, sources, follow): [["All 14:false","Warnings 1:false","Errors 2:true"],"diagnostics-event-1830",2,"true,false,false,false","false"]
comparison chips, controls, failed number: ["Version 0.9.1 ×","14 selects","2 dates","2 of 6"]
node filters, more facts (after render): ["6 node selects","15 more facts"]
page heights (doc, window, page scroll, page client): [900,900,860,860]
unavailable text: "Diagnostics for run_3f2504e0-4f89-11d3-9a0c-0305e82c3301 could not be loaded. Transport error: fixture host: workflow_run_detail_query is not mocked Try again"
bars: ["Completed","Failed","Cancelled"]
```

The page does not scroll as a whole (document height 900 at a 900 px window). The panels scroll inside.

## Screenshots

All under `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m2/`:

- `diag-empty.png`: no run selected; EmptyState with "Pick a run".
- `diag-failed.png`: failed run with the terminal `runtime_model_load_failed` ErrorCard, Timing, Placement, Retention and 14 log lines. The node error line `load_model` has the failed glyph.
- `diag-show-in-log.png`: after "Show in log". Errors is pressed, 2 of 14 lines are shown, the error line is selected and Follow is off.
- `diag-filters.png`: the comparison Filters disclosure open, with the `Version 0.9.1` chip and the facet rows. The error line stays selected after switching back to All.
- `diag-more-facts.png`: "More facts" open (15 rows) and the Projection rows.
- `diag-nodes.png`: "Nodes by status" bars, the Breakdown disclosure, and the comparison numbers and note.
- `diag-running.png`: running run; Duration reads "Running", no ErrorCard, Follow on.
- `diag-unavailable.png`: base fixture only. Diagnostics queries are rejected, and the page shows a muted in-place sentence with Try again, not a red banner.
- `board.png`: the board rendered at the same size, for comparison.

## Ledger entry text

> **2026-11-03, M2 Diagnostics (Opus 5.5 agent).**
>
> `DiagnosticsPage.svelte` now follows the `Diagnostics` board, built from the shared parts:
>
> - `DimensionLine`, a 300px `Panel` "Run facts", `Chip`, `KeyValue`, `GroupHeader`, `FilterChip`, `SegmentedControl`, `ErrorCard`, `LogLine` and `EmptyState`.
> - Timing, Placement, Retention, More facts and Projection key-value blocks hold all 27 run-detail facts and all 5 projection cursors.
> - The six execution selects and the 14 comparison selects plus 2 date inputs moved into `Filters` disclosures with applied-filter chips and quick presets.
> - Node errors are log lines with the failed glyph. A compact table lists only node errors that are missing from the timeline.
> - The ErrorCard explanation and "Try:" advice come from `explainDiagnosticsError`: a known-code table with a neutral fallback that restates the message.
> - The `diagnosticsFocus` deep link switches the level filter, selects the line and scrolls it into view, once per request.
> - Transport and subscription failures are muted sentences inside the panel.
> - Service calls are unchanged, and there are no ledger reads.
>
> Gates:
>
> - typecheck, 685/685 frontend tests, lint:a11y, lint:critical, eslint on the 3 files and vite build all pass.
> - Traceability passes with `src/components/workbench/README.md` in the same change.
> - Headless CDP walkthrough with `reports/tools/fixture-diagnostics.js`. Screenshots in the M2 scratch directory: empty, failed, show-in-log, filters, more facts, nodes, running, unavailable.

## Issue rows (issues.md format; IDs for the integration owner to assign)

| ID | Severity | Evidence | Relationship | Owner / boundary | Disposition | Verification | Revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WR-M2-1 | low | `SchedulerTimelineProjectionRecord` has no error code or node id. A `diagnostic_error_occurred` line reads "diagnostic error occurred <summary>…" unless a node status row links the event. The board shows the code as the line's kind. | Diagnostics log readability (A2) | backend scheduler-timeline projection | defer: the code shows when the summary carries it or a node row links the event | log line for a run-level error shows its code | projection adds `error_code` / `node_id` to timeline records |
| WR-M2-2 | low | The board's ErrorCard explanation cites cross-run memory facts ("needed 14.2 GB and 9.8 GB was free; image-gen-sdxl was running there"). No projection exposes them per error. The page explains from the code and restates the projected message. | ErrorCard fidelity (A2) | backend run-detail / diagnostics error payload | defer: known-code table plus message | `explainDiagnosticsError` tests | projection exposes requested and free memory at failure time |
| WR-M2-3 | low | The board shows retention as "7 days, metadata only". The run detail carries only `retention_policy_id`, so the Retention block shows the policy id and the retention-state counts. | Retention block (A2) | backend retention projection | defer | screenshot `diag-failed.png` | projection exposes policy duration |
| WR-M2-4 | low | The source segment offers All sources, Scheduler, Runtime and Nodes (as on the board). Events from `workflow_service`, `retention`, `library` and `local_observer` appear only under All sources. | Event filtering | MrScripty (design) | defer: matches the board | `filterDiagnosticsLogLines` test | owner wants an "Other" segment |
| WR-M2-5 | low | The `diagnosticsFocus` path was not driven in the browser: the built app raises it only from the Graph toolbar after a failed submit, which the fixture cannot reach. Presenter tests cover resolution and keying. "Show in log" exercised the shared select-and-scroll effect in headless Chromium. | A1/A2 deep link | integration owner | verify in A5 | submit a failing workflow in the desktop app, click its Diagnostics link, see the error line selected and in view with Errors pressed | A5 |
| WR-M2-6 | low | The "Compared with earlier runs" median is now the median of the *other* compared runs that completed, so the selected run is compared against it rather than with itself. The old page had no median. | Comparison semantics | MrScripty | confirm or revert | `buildDiagnosticsComparisonSummary` test | owner review |

## Shared-part changes requested from the integration owner

1. **`Panel`: expose the scroll element (for example `bind:scrollElement`) or accept a `role`/`aria-label` for the scroll area.** The Events panel needs the log container for Follow and scroll-into-view, and it must be `role="log"`. For now it uses the `pg-panel pg-col pg-ptop pg-scroll` classes directly instead of `Panel`.
2. **`LogLine`: an optional `title` (or `data-event-id`) prop.** This would expose the event id on hover. Today the event id is searchable and the line shows `seq N` in its source suffix.
3. **`src/components/workbench/README.md`: add a Diagnostics line if the README lists page decisions.** The traceability hook needs that README in the same commit as these files (see Verification).
