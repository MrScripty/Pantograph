# M1 Scheduler: report

Milestone 1 of `plan.md`: the Scheduler page now matches the `Revised` board
and is built from the shared parts and the real projection types. Nothing has
been committed.

## Changed files

| File | Change |
| --- | --- |
| `src/components/workbench/SchedulerPage.svelte` | Rewritten as the inbox layout: dimension line, device-memory strip (one sentence), 380px Runs panel (search, Filters disclosure, refresh, status segment, applied-filter chips, grouped list, action footer), Run detail panel (title, copyable run id, chips, Diagnostics/Graph/I-O buttons, Progress/Placement/Retention cells, scheduler timeline). It no longer calls `workflow_run_list_query`: rows come from `runContextRunList` (WR-005). |
| `src/components/workbench/schedulerPagePresenters.ts` | New pure projections: status segment, grouping (built on the picker's `groupRunsForContextPicker`), row time and detail line, dimension-line sentence, the 14 filter fields and their chips, detail cell text, the estimate sentence, footer state and the reason text for unavailable actions, timeline attempt line, freshness line. Removed what only the old table used: `schedulerStatusClass` (cyan/amber classes), `buildSchedulerRunListQuery` and its date-range helpers, `buildSchedulerEstimateRows`, `formatSchedulerEstimateLabel`, `formatSchedulerEstimateDuration`, `formatSchedulerDuration`, `formatSchedulerQueuePosition`, `formatSchedulerPriority`, `formatSchedulerReasonLabel`. Kept `formatSchedulerProjectionFreshness` and `schedulerTimelinePayloadLabel`. Nothing imports them now, but `NetworkPage.svelte` imported them before M4 started. |
| `src/components/workbench/schedulerPagePresenters.test.ts` | Removed the tests for the deleted functions. Added 10 tests for the new projections. The file has 23 tests. |
| `src/stores/schedulerRunListStore.ts` | Added the `statusSegment` filter field, `SCHEDULER_SECONDARY_FILTER_KEYS` (the 14 disclosure filters), `withSchedulerSecondaryFiltersCleared` / `clearSchedulerSecondaryFilters`, and the `schedulerViewPreferences` store with `showLanes` (default true). Removed the column-visibility state, which only the deleted table used. The semantics of the existing filters are unchanged. |
| `src/stores/schedulerRunListStore.test.ts` | Covers the segment normalisation, the 14 disclosure keys, clearing the disclosure filters, and the view preference. The column tests are gone. |
| `docs/plans/workbench-redesign/reports/tools/fixture-scheduler.js` | Inject after `fixture.js`. It reuses that file's runs, adds the scheduler facts the page shows, answers the timeline, estimate and I/O artifact queries, and counts calls in `window.__fixtureCalls`. |
| `docs/plans/workbench-redesign/reports/m1-scheduler.md` | This report. |

## Behaviour against the M1 requirements

- **Run list ownership (WR-005).** The Scheduler reads `runContextRunList` without filters. Refresh and `run_list` invalidations stay with the shell (`startRunContextSync`). After an action, and from its Refresh button, the page calls `refreshRunContextRunList()`, which is the same source that Retry and opening the picker use, so overlapping refreshes go through the source's generation token. The page's own subscription now covers only `scheduler_timeline`, `run_detail` and `io_artifact`. In the walkthrough, `workflow_run_list_query` ran once with Scheduler open on load (the old page added a second query). Selecting a row did not query it. One Refresh added one query.
- **Grouping (WR-004).** The groups are Running, Queued, Finished today and Finished earlier. They come from the picker's `groupRunsForContextPicker`. Inside each group the rows keep the order of the Sort filter. Row text and glyphs come from `decorateRunContextRow`. There is no "Waiting for memory" group. Delayed runs appear under Queued with the scheduler reason as their context. The Active segment holds running runs only, and a test checks that the segments agree with the picker's groups for every status.
- **Device lanes (WR-006).** When `showLanes` is on, the strip region shows "local" followed by "Device memory is not reported by this engine build." No lane data is invented. The preference lives in `schedulerViewPreferences`. Nothing in the UI toggles it yet.
- **Filters.** The 14 existing selects sit in the Filters disclosure in a two-column grid that scrolls at 32vh. They are Status, Sort, Policy, Retention, Client, Session, Bucket, Runtime, Runtime variant, Backend, Device class, Device, Network node and Accepted. Each applied value shows as a chip with ×, and "Clear all" appears when there are two or more. Search and the segment stay outside the disclosure. The filter predicates are the existing `filterAndSortSchedulerRuns`. Segment counts are taken after all other filters.
- **Actions.** All the old actions are still there, with the same guards (`schedulerRunSupports*`), the same scheduler messages and the same error text from `formatWorkflowCommandError`. Afterwards the run list and detail refresh as before.
  - The footer row holds Move to front, Priority with Set, and More actions next to the two-step Cancel run (it changes to Confirm cancel and resets after 4 s).
  - More actions opens Resume (dependency readiness) and the GUI admin row: two-step Admin cancel, Front, and Priority with Set.
  - When an action does not apply, its control stays visible but disabled, and its `title` says why.
  - Action feedback goes to one `role="status"` line that stays mounted. Errors are in danger colour.
  - With a finished run selected, the footer says "… has finished. No actions available." With nothing selected it says "Select a run to act on it. The other tabs follow this selection."
- **Selection.** Clicking a row calls `selectRunContext`. The chip, the detail panel and the footer follow it, and the selection survives a tab round trip. Rows and the detail run id carry `data-run-id`.
- **Dimension line.** The line reads "N runs waiting, about M min of queued work on D devices". M is the sum of `estimated_duration_ms` across the waiting runs; those fields come from the scheduler estimate projection on each run-list row. If none of the waiting runs has an estimate, the line falls back to "N runs waiting". When only some have one, it adds "(estimated for k of them)". It also has loading and unreachable variants.
- **Timeline and estimate.** `querySchedulerTimeline`, `querySchedulerEstimate` and `queryIoArtifacts` run as before, with per-request serials that discard stale results.
  - The timeline is a time/event grid with the existing Kind and Source filters, which reset when the run changes. Each event shows its attempt facts on one mono line. The section ends with "Node-level events for this run are in Diagnostics."
  - A muted line shows how fresh the timeline, estimate and artifact projections are (ADR-014 guardrail).
  - Errors appear inside the panel as an ErrorCard or a cell sentence. The page has no red banners.
- **Engine unavailable.** In a plain browser the list shows "The run list is unavailable because the engine did not answer." with Retry, and the dimension line says the same. The shell dims the page. The page itself never scrolls; the walkthrough measured `scrollHeight` 900 at a 900px viewport.

## Verification (from the repo root)

```
$ npm run typecheck
> tsc --project tsconfig.json --noEmit --pretty false
(exit 0)

$ npm run test:frontend
ℹ tests 685
ℹ pass 685
ℹ fail 0
(exit 0)

$ npm run lint:a11y
svelte accessibility gate passed

$ npm run lint:critical
critical anti-pattern gate passed

$ npx eslint src/components/workbench/SchedulerPage.svelte src/components/workbench/schedulerPagePresenters.ts src/components/workbench/schedulerPagePresenters.test.ts src/stores/schedulerRunListStore.ts src/stores/schedulerRunListStore.test.ts --max-warnings 0
(exit 0, no output)

$ TRACEABILITY_BASE_REF=codex/pantograph-rpc-workflow-2026-09-26 ./scripts/check-decision-traceability.sh
No changed files detected for decision traceability check.
(exit 0; HEAD equals the base, see WR-003)

$ npx vite build --outDir /tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m1/dist --emptyOutDir
✓ built in 20.83s (no warnings)
```

`node --check` passes on `fixture-scheduler.js`.

I also ran a scratch copy of the traceability hook against the M1 file list (`scratchpad/m1/trace.sh`). It fails with "Changed directory without decision traceability update: src/components/workbench" and the same message for `src/stores`, because both READMEs are outside the M1 write set. The same copy passes ("Decision traceability check passed.") once both READMEs are in the list. The README edits needed are listed in the integration-owner requests below.

At the time of the run, `npm run typecheck` and `test:frontend` passed with the sibling agents' in-progress files. An earlier `tsc` run showed one transient error in `networkPagePresenters.test.ts`, which belongs to M4 and is not in this write set.

## Screenshots

The build was served on 127.0.0.1:8791 and driven with `cdp-init.cjs` (`CDP_PORT=9341`) using `fixture.js` followed by `fixture-scheduler.js`. The server has been stopped.

- `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m1/scheduler-none.png`: populated list, no run selected.
- `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m1/scheduler-queued.png`: queued run selected, with actions in the footer.
- `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m1/scheduler-more-actions.png`: Cancel armed (Confirm cancel) and More actions open.
- `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m1/scheduler-filters.png`: Filters disclosure, two chips, and the in-place action error after the confirmed cancel. The fixture does not mock the cancel command.
- `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m1/scheduler-running.png`: running run selected, with queue actions disabled and titled.
- `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m1/scheduler-failed.png`: failed run with the "has finished" footer.
- `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m1/scheduler-unavailable.png`: plain browser with no host, so the engine is unreachable.

Walkthrough log for `steps.json`:

- Selecting a row set the chip to `summarize-daily v1.0.3` and marked the row `aria-current`.
- After the first click, "Confirm cancel" was visible.
- Feedback after confirming: "Transport error: fixture host: workflow_cancel_execution_session_queue_item is not mocked".
- The disclosure held 14 selects.
- Chips: "Status: Queued", "Runtime: llama.cpp".
- Groups: Running 2, Queued 3, Finished today 2.
- Segments: All 7, Active 2, Queued 3, Done 1, Failed 1.
- Dimension line: "3 runs waiting, about 19 min of queued work on 2 devices".
- `scrollHeight` 900 at viewport 900.

`steps-calls.json` checked the call counts:

- On load: `workflow_run_list_query` 1 and `plugin:event|listen` 2 (shell `run_list` plus the Scheduler detail subscription).
- After selecting a row: run list still 1; timeline, estimate and artifacts 1 each.
- After Refresh: run list 2.
- The picker listed the same 7 rows.
- The selection was still there after switching to Diagnostics and back.

## Ledger entry text

- M1 Scheduler implemented (Opus 5.5 agent, 2026-11-03). The Scheduler is rebuilt on the `Revised` board with the shared parts. It reads the shell's `runContextRunList` as its only run-list source and no longer queries `workflow_run_list_query` (WR-005). Grouping comes from the picker helpers, with no memory-wait group and delayed runs under Queued (WR-004). The device strip shows one muted sentence behind the `showLanes` preference (WR-006).
- The 14 filter selects moved into the Filters disclosure with applied-filter chips. The status segment and search stay outside it.
- Every action is still reachable: Move to front, Priority/Set and two-step Cancel run in the footer; Resume and GUI admin (two-step Admin cancel, Front, Priority/Set) under More actions. Each has its existing guard, and a disabled control has a title explaining why.
- Gates: typecheck pass; test:frontend 685/685; lint:a11y pass; lint:critical pass; eslint pass on the changed files; vite build pass. The traceability hook says "No changed files" (WR-003). A scratch copy run on the M1 file list needs the `src/components/workbench/README.md` and `src/stores/README.md` updates listed in the M1 report.
- CDP walkthrough with `fixture-scheduler.js`: one run-list query with Scheduler open, selection follows the chip, cancel arms then confirms, 14 disclosure selects, the page does not scroll. Screenshots are in `scratchpad/m1/`.

## Issue rows (provisional IDs; the integration owner assigns final numbers)

| ID | Severity | Evidence | Relationship | Owner / boundary | Disposition | Verification | Revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WR-M1-1 | low | The old page sent its filters to `workflow_run_list_query` (`buildSchedulerRunListQuery`, limit 250), so a filter could find matching runs older than the 250 newest. Scheduler now filters the shell's unfiltered 250-row list on the client with the same predicates (`filterAndSortSchedulerRuns`). | Filter semantics after WR-005 | integration owner / shell run-list source | accept: one list, one refresh. If older runs must be filterable, the shell source would need a filter-aware query owned there, not by the page. | Filter results match the old predicates (existing filter tests unchanged and passing) | A user needs runs beyond the 250 newest |
| WR-M1-2 | low | `RunContextRunListState` does not carry the run-list `projection_state`, so Scheduler cannot show run-list freshness (cursor or status). It shows "loaded HH:MM" in the Refresh title. Freshness for the timeline, estimate and artifact projections is shown. | ADR-014 freshness guardrail for the run list | integration owner / `shell/runContextRunList.ts` | defer: see request 1 below | The Scheduler freshness line includes the run list | Shell store change |
| WR-M1-3 | low | Before M1, `workflowService.subscribeEvents` also refreshed the Scheduler run list. Now only `run_list` invalidations (shell), opening the picker, Retry and Refresh do. Detail projections still refresh on workflow events. | Run-list freshness when invalidation events are delayed | integration owner / `startRunContextSync` | defer: see request 2; add it only if A5 shows the list lagging behind local executions | A5 desktop check: a run submitted from the editor appears in Scheduler without pressing Refresh | A5 |
| WR-M1-4 | low | The board shows "Cancel run", "Move to front" and "Priority" for every live run, including running ones. The scheduler accepts session queue actions only for `queued`/`delayed` runs with an execution session, and admin actions only for `queued`/`delayed`. For running runs, and for queued runs without a session, the controls render disabled with a title giving the reason. | Scheduler footer (A2) | backend scheduler commands | defer: no service cancels a running run | Screenshot `scheduler-running.png` shows the disabled controls | The scheduler gains cancel for running runs |
| WR-M1-5 | low | The board's detail cells show node progress ("embed_chunks 61%"), "based on 14 earlier runs", requested GB, retention size on disk and an expiry date. The run-list, estimate and I/O artifact projections expose none of these. Progress uses run-level facts and the estimate. Retention uses the per-state artifact counts and the retention policy id. | Scheduler detail (A2) | backend projections | defer | Detail screenshots show projection facts only | The projections add node progress or retention size and expiry |

Suggested updates to existing rows (for the owner to apply):

- **WR-005:** resolved by M1. Verification: `steps-calls.json` shows one `workflow_run_list_query` on load with Scheduler open, and `SchedulerPage.svelte` has no `queryRunList` call.
- **WR-006:** its verification ("page screenshots show the sentence, not empty lanes") is met for Scheduler by `scheduler-*.png`.
- **WR-004:** its Scheduler half is implemented as specified. Tested by "status segments agree with the picker groups and have no memory-wait bucket".

## Requests for the integration owner (outside the M1 write set; nothing was edited)

1. **Shell run-list freshness (WR-M1-2).** In `src/components/workbench/shell/runContextRunList.ts`, add `projectionState: ProjectionStateRecord | null` to `RunContextRunListState`, set it from `response.projection_state` in the success branch of `refresh()`, and keep the previous value in both failure branches. Initial value: `null`. Scheduler would then add "Run list current at seq N" to its freshness line, through the existing `formatProjectionFreshnessState`.
2. **Optional event-driven refresh (WR-M1-3).** If wanted, add `const unsubscribeEvents = workflowService.subscribeEvents(() => void runListSource.refresh());` inside `startRunContextSync()` in `shell/runContextStore.ts`, and call it in the returned stop function. That keeps the single owner. The page should not add it.
3. **Traceability READMEs (required before commit).**
   - `src/stores/README.md`, Contents row for `schedulerRunListStore.ts`: change "Scheduler run-list filters, sort and visible columns" to "Scheduler filters (search, status segment, the 14 disclosure filters, sort) and view preferences (`showLanes`); the run rows come from the shell's `runContextRunList`".
   - `src/components/workbench/README.md`: add an invariant such as "The Scheduler reads the shell's `runContextRunList` as its only run-list source; it never queries `workflow_run_list_query` itself (WR-005)."
   - `shell/README.md`: the revisit trigger "Milestone 1 moves the scheduler run list into a shared store…" is resolved the other way round (Scheduler reads the picker's store). Replace it with the WR-M1-1 trigger.
4. **The `showLanes` toggle** has no control yet, by design. When WR-006 is resolved, add a `pg-tgl` with `aria-pressed` bound to `schedulerViewPreferences.showLanes` and feed `DeviceLanes` from a presenter.
