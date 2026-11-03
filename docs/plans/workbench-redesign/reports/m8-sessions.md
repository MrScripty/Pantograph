# M8 Sessions page report

Agent: Opus 5.5 page agent, 2026-11-03. Board: `docs/design/mockups/boards/Sessions.dc.html`.

## Changed files

| File | Change |
| --- | --- |
| `src/components/workbench/SessionsPage.svelte` | Replaces the M0 placeholder. Dimension line; 400px Sessions panel (search, disabled `New session`, All/Active/Saved segment, Active and Saved groups, footer); Session detail panel (title, id, chips, action row, Activity and "Runs from this session" key-value sections with `Show in Scheduler`, Recent runs, "Open in this session" with `Open graph`, "What resumes"). |
| `src/components/workbench/sessionsPresenters.ts` | New pure projection: listing outcomes, run outcome kinds and strip, date and last-activity wording, session entries for workflows and orchestrations, sorting, filtering and grouping, dimension line, failure explanation, detail model, detail empty text, plus `createLatestOnlyGate` (the page's invocation authority, kept here so it is tested). |
| `src/components/workbench/sessionsPresenters.test.ts` | 23 tests. |
| `docs/plans/workbench-redesign/reports/tools/fixture-sessions.js` | Walkthrough fixture (the caller asked for it next to `fixture.js`). Load it after `fixture.js`. It keeps that file's runs and answers `list_workflows` (4 workflows, 3 of them with runs in the base fixture) and `list_orchestrations` (1). Every other command goes to the base fixture. |
| `docs/plans/workbench-redesign/reports/m8-sessions.md` | This report. |

No file outside the M8 write set was edited. The fixture is outside it because the caller asked for it there.

## What exists: findings that set the design

- **Saved workflows.** `workflowService.listWorkflows()` calls `list_workflows` and returns `WorkflowMetadata { id (file stem, set by the backend), name, description?, created, modified, orchestrationId? }`. The dates are RFC 3339 strings (`crates/pantograph-workflow-service/src/graph/persistence.rs`). The Graph selector reads the same command through `refreshWorkflowList`. That function swallows errors into the shared `graphSessionError` and empties `availableWorkflows`, so the page calls `listWorkflows()` itself and keeps its own typed failure.
- **Orchestrations.** `GraphSelector.svelte` does **not** list orchestrations. It shows Workflows and System graphs only. The only frontend listing is `loadOrchestrationList()` in `src/stores/orchestrationStore.ts` (`list_orchestrations`, registered in `src-tauri/src/app_setup.rs`), and nothing called it until now. Its metadata is `{ id, name, description, nodeCount }` with **no dates**. No mounted page renders orchestrations: `UnifiedGraphView.svelte` and `OrchestrationGraph.svelte` are not used by `GraphPage.svelte`.
- **Run list.** `runContextRunList` (shell-owned, `workflow_run_list_query`, limit 250) carries `workflow_id`, `workflow_semantic_version`, `client_id`, `client_session_id`, `workflow_execution_session_id`, `selected_device_id`, timestamps and status.
- **Client sessions.** No frontend service lists client sessions from the attribution projection. A search of `src/` for `client_session` finds only run-record fields and Scheduler/Diagnostics filters. This window also has no client session id of its own. Each Run submit (`WorkflowToolbar.svelte`, then `workflow_create_execution_session`) makes the backend register a new attribution client ("Pantograph Desktop") and open a new client session with takeover (`src-tauri/src/workflow/headless_workflow_commands.rs` 140–185). The response's `attribution` is typed `unknown` and is discarded. As a result, no "current session id" exists to match runs against. **Active is therefore omitted with a muted sentence (plan stop condition), and an issue row is below.**
- **Scheduler filter store.** `schedulerRunListStore` has no exact workflow filter. Its `search` matches the workflow id as a substring, along with 15 other run fields. It also has `clientSession`.

## Where each group and field comes from

| Group / field | Built from |
| --- | --- |
| Saved group, one entry per workflow | `list_workflows` items. The id is `metadata.id` (the file stem), falling back to `name`. The name is `metadata.name`. The description is `metadata.description`. |
| Saved group, one entry per orchestration | `list_orchestrations` items (`id`, `name`, `description`, `nodeCount`). |
| Active group | No source. It shows `ACTIVE_SESSIONS_UNAVAILABLE_NOTE`: "Live client sessions are not listed by this engine build. Runs still show their client session in the Scheduler." The Active segment has no count, so it does not claim zero. |
| Runs of a session | Run-list rows whose `workflow_id` equals the entry id exactly, newest `last_updated_at_ms` first. |
| Row glyph | `running` when any of its runs is live (`running`, or `accepted`/`future`/`scheduled`/`queued`/`delayed`). Otherwise `idle`. |
| Row tag (the board's client tag) | The kind: `workflow` or `orchestration`. Saved entries have no client. |
| Row sub-line | With live runs: "N live run(s), <outcome summary>". Otherwise the description, or "Created <day>" for a workflow / "Orchestration, N nodes" for an orchestration. |
| Outcome strip | Counts by status in board order: running (accent), queued (hollow), succeeded (grey), failed (danger), cancelled (hollow, dashed). Widths are proportional to the counts. An `sr-only` sentence repeats the strip in words. |
| Run count, last activity (right column) | The number of matched runs. The time is "now" while a run is live, HH:MM today, "Oct 31" this year, "Jan 31, 2025" otherwise, and "never" when nothing is recorded. Last activity is the newer of the latest run `last_updated_at_ms` and the workflow's `modified`. |
| Sort | Live entries first, then by last activity, then by name. |
| Dimension line | Source counts, or "unavailable" / "Loading". Run total and live count from the run list. Runs whose `workflow_id` matches no saved workflow ("N from workflows not saved here"). |
| Detail chips | The state chip (kind + live count), the latest `workflow_semantic_version`, distinct `selected_device_id` values, and the node count for orchestrations. |
| Activity: Created, Modified | `metadata.created` / `metadata.modified`. Orchestrations show "Not recorded for orchestrations". |
| Activity: Last activity | As above, with what happened ("run completed", "run in progress", "workflow saved"). |
| Activity: Clients | Distinct `client_id` values on the session's runs and the count of distinct `client_session_id` values. Shows "None recorded on its runs" when there are none. |
| Runs from this session: Total, Live now, Versions run | Outcome counts; live run ids (short) with status and `scheduler_queue_position`; distinct semantic versions. |
| Recent runs | Up to 5 runs decorated by the shell's `decorateRunContextRow`, as `ListRow` with `data-run-id` (Ctrl+C copies). Selecting a row calls `selectRunContext`. A line gives "N more in Scheduler". |
| Open in this session | One item: the workflow itself (`.pantograph/workflows/<id>.json`, the path the graph loader reads) or the orchestration (button disabled). |
| What resumes | A sentence saying that nothing resumes because the engine keeps no session state. |

The board's "Artifacts kept" row and its "N graphs open" / "unsaved" chips are omitted: no projection reports artifact size or retention per workflow, or open graphs per session.

## Actions

| Control | Behaviour |
| --- | --- |
| `Show in Scheduler` | `setSchedulerRunFilters({ search: <workflow id> })`, then `setWorkbenchPage('scheduler')`. Verified: the Scheduler opens with its search field set to `image-gen-sdxl` and lists that workflow's two runs. This is a substring search, not an exact workflow filter (issue below). |
| `Open graph` (workflow) | `loadWorkflowByName(id)` from `graphSessionStore`, the loader the Graph selector uses. On success it calls `setWorkbenchPage('graph')`. On failure an `ErrorCard` appears in the section, using `graphSessionError` or the thrown message, with "Open again". A newer open, or leaving the page, supersedes the call, and a superseded call does not switch tabs. Like the Graph selector, it replaces the current edit graph without asking about unsaved edits. |
| `Delete workflow` (workflow only) | `TwoStepButton` "Delete workflow", then "Confirm delete" (4 s reset; verified armed, then reset after 4.3 s). It replaces the old `window.confirm("… This cannot be undone.")` in `WorkflowPersistenceControls.svelte` and calls the same `deleteWorkflowByName`, which owns the error text and the Graph reset when the deleted workflow is open. On success the lists reload. On failure an `ErrorCard` appears under the header. |
| `New session`, `Resume`, `Rename`, `Save a copy` | Native buttons with `aria-disabled="true"` and a `title` giving the reason. |
| `Delete` (orchestration), `Open graph` (orchestration) | Disabled with a title. The instruction allowed only `delete_workflow`, and the Graph tab cannot open orchestrations. |
| `Close session` | Not rendered, because no active sessions are listed. |

## Lifecycle

- One owner, `onMount`. It opens three `createLatestOnlyGate` gates (load, open, delete), loads both listings once with `Promise.allSettled`, and closes the gates on destroy. A listing, open or delete that completes after a newer call or after destroy is classified `superseded`/`closed` and writes nothing. This is covered by two tests: overlapping calls, close, and reopen not reviving an old token; gates are independent per operation.
- No timers, subscriptions or listeners are added. The run list is the shell's store, kept live by the shell. Date wording is recomputed whenever the sources or the run list change, so no clock is needed.
- The page never scrolls as a whole: the two `Panel`s scroll inside.

## Verification

Run from the repo root on 2026-11-03 with the final sources.

```
$ npm run typecheck
> Pantograph@0.0.0 typecheck
> tsc --project tsconfig.json --noEmit --pretty false
exit 0
```
(An earlier run in this session failed only in `diagnosticsPagePresenters.test.ts`, which M2 was editing at the time.)

```
$ npm run test:frontend
ℹ tests 685
ℹ pass 685
ℹ fail 0
```
`node --experimental-strip-types --test src/components/workbench/sessionsPresenters.test.ts`: 23 pass, 0 fail.

```
$ npm run lint:a11y
svelte accessibility gate passed
$ npm run lint:critical
critical anti-pattern gate passed
$ npx eslint src/components/workbench/SessionsPage.svelte src/components/workbench/sessionsPresenters.ts src/components/workbench/sessionsPresenters.test.ts --max-warnings 0
eslint exit 0
```
(Earlier in the session, `lint:a11y`/`lint:critical` failed only on M3's files, WR-007.)

```
$ TRACEABILITY_BASE_REF=codex/pantograph-rpc-workflow-2026-09-26 ./scripts/check-decision-traceability.sh
No changed files detected for decision traceability check.
```
This proves nothing for uncommitted work (WR-003). I also ran a scratch copy of the hook over the three M8 source files:
```
Changed directory without decision traceability update: src/components/workbench
Update src/components/workbench/README.md or add/update an ADR under docs/adr/.
Decision traceability check failed (1 issue(s)).
```
With `src/components/workbench/README.md` added to the list: `Decision traceability check passed.` That README is outside the M8 write set; see the shared-part change below.

```
$ npx vite build --outDir /tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m8/dist
✓ built in 20.69s   (no Svelte warnings)
```

Headless walkthrough: `CDP_PORT=9348 node docs/plans/workbench-redesign/reports/tools/cdp-init.cjs <steps>`, served by `python3 -m http.server 8798` (stopped afterwards). It injected `fixture.js`, then `fixture-sessions.js`. Logged results:
```
rows: ["image-gen-sdxl workflow 1 live run, 1 running, 1 failed Runs in recent history: 1 running, 1 failed 2 runs now",
       "ingest-docs-rag workflow 1 live run, 1 running Runs in recent history: 1 running 1 run now",
       "summarize-daily workflow 1 live run, 1 queued, 1 succeeded Runs in recent history: 1 queued, 1 succeeded 2 runs now",
       "Maid workflow Created Jan 31 Runs in recent history: no runs 0 runs May 1",
       "Coding Agent Orchestration orchestration Wraps Coding Agent with a tool loop Runs in recent history: no runs 0 runs never"]
dim line: "4 saved workflows, 1 orchestration. 7 runs in recent history, 5 live, 2 from workflows not saved here"
after open graph (tab, error): ["Sessions","The graph did not open load_workflow Failed to load workflow \"image-gen-sdxl\": fixture host: get_node_definitions is not mocked Try: … Open again"]
delete armed: true
armed after 4.3s: false
after show in scheduler (tab, search fields): ["Scheduler",["image-gen-sdxl"]]
segments: ["All 5=false","Active=true","Saved 5=false"]
```
The fixture does not mock the graph loader, so `Open graph` was checked only through its failure path. The successful tab switch needs the desktop app (A5).

## Screenshots (1440x900, headless shell)

All under `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m8/`:

- `sessions-populated.png`: All filter, a live workflow selected, with strips, recent runs, Open graph and What resumes.
- `sessions-saved-idle.png`: an idle saved workflow with no runs (Maid).
- `sessions-orchestration.png`: an orchestration, with "Not recorded" dates and disabled Open graph and Delete.
- `sessions-open-failed.png`: the in-place `ErrorCard` after Open graph failed.
- `sessions-delete-armed.png`: `Confirm delete` armed.
- `sessions-show-in-scheduler.png`: the Scheduler (M1's new page) after Show in Scheduler, with search `image-gen-sdxl`.
- `sessions-active-filter.png`: the Active segment, with the explanation and the matching detail empty text.
- `sessions-unavailable.png`: a plain browser (no host). The engine line shows unreachable, one `ErrorCard` covers both listings with Retry, the dimension line reads "Saved workflows unavailable, orchestrations unavailable. Run history not loaded", and the detail reads "Nothing to show while saved workflows cannot be listed."

## Ledger entry text

- M8 Sessions implemented (Opus 5.5 page agent). `SessionsPage.svelte` replaces the placeholder. It projects saved workflows (`list_workflows`) and orchestrations (`list_orchestrations`, through `orchestrationStore.loadOrchestrationList`, which had no caller before) as saved sessions. Each session shows its runs from the shell's run-list projection, matched on exact `workflow_id`. Active is omitted with a muted sentence because no service lists client sessions and the window holds no client session id (plan assumption "run list exposes a client session id per run": the field exists on runs, but there is no current-window id to match, so the stop condition applies). `Show in Scheduler` sets the Scheduler search to the workflow id and switches tab. `Open graph` uses the Graph selector's loader and switches to Graph. `Delete workflow` is a `TwoStepButton` over the existing `deleteWorkflowByName` (it replaces `window.confirm`). Session actions with no service render disabled with titles. New pure presenter `sessionsPresenters.ts` with 23 tests, including the page's latest-only invocation gate. Gates: typecheck, test:frontend (685 pass), lint:a11y, lint:critical, eslint on changed files, and vite build all pass. The traceability hook needs the `src/components/workbench/README.md` row update (integration owner). Headless walkthrough with `reports/tools/fixture-sessions.js` on port 8798 / CDP 9348; screenshots in `scratchpad/m8/`.

## Issue rows (issues.md format; IDs to be assigned by the integration owner)

| ID | Severity | Evidence | Relationship | Owner / boundary | Disposition | Verification | Revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WR-M8a | medium | No frontend service lists client sessions from the attribution projection, and the GUI holds no client session id: each submit registers a new attribution client and opens a new client session server-side (`src-tauri/src/workflow/headless_workflow_commands.rs` 140–185); the `attribution` in `WorkflowExecutionSessionCreateResponse` is typed `unknown` and discarded by `WorkflowToolbar.svelte`. Run records do carry `client_id` / `client_session_id`. | Sessions "Active" group (A2) | backend attribution projection plus desktop host session lifecycle | defer: Active shows the sentence "Live client sessions are not listed by this engine build…", with no count in the segment | `sessionsPresenters.test.ts` (grouping, filter options); `sessions-active-filter.png` | a client-session listing command exists, or the desktop window keeps one client session and exposes its id |
| WR-M8b | low | No session service or persistence: `New session`, `Resume`, `Rename`, `Save a copy`, `Close session` and session delete have no backend. The board's footer "Sessions are saved in .pantograph/sessions" and the "Artifacts kept" / "graphs open" facts have no source. | Sessions actions and detail (A2) | backend (out of scope by plan) | defer: controls are `aria-disabled` with a title; footer and resume text say what is true; facts omitted | presenter test "only Delete workflow is backed by a service"; screenshots | a session service is planned |
| WR-M8c | low | Orchestrations: `OrchestrationGraphMetadata` has no created/modified dates; the Graph page cannot open orchestrations (`UnifiedGraphView`/`OrchestrationGraph` are not mounted); `delete_orchestration` exists but was not wired (the M8 instruction allowed `delete_workflow` only). The Graph selector does not list orchestrations, contrary to the M8 brief. | Sessions orchestration entries | M5 (Graph) and the orchestration store owner | defer: dates read "Not recorded for orchestrations"; Open graph and Delete are disabled with titles | `sessions-orchestration.png`; presenter test "orchestration entry has no dates…" | Graph gains an orchestration view, or metadata gains dates |
| WR-M8d | low | `schedulerRunListStore` has no exact workflow filter; `Show in Scheduler` sets `search` to the workflow id, which also matches longer ids that contain it and other run fields | Show in Scheduler accuracy | M1 (Scheduler store) | defer: substring search is close enough for distinct ids; documented on `schedulerSearchForSession` | walkthrough "after show in scheduler" | M1 adds a workflow filter (then switch the page to it) |
| WR-M8e | low | Session run counts cover only the run-context run list (most recent 250 runs), so older runs of a workflow are not counted | Sessions run totals | shell run-list source | defer: the wording says "in recent history" | presenter tests | a per-workflow run count projection exists |
| WR-M8f | low | `Open graph` (like the Graph selector) replaces the current edit graph without asking about unsaved edits; there is no shared Graph mode, so Sessions cannot ask Graph to open in "Edit workflow" mode if M5 defaults to "As run" while a run is selected | Open graph from Sessions | M5 (Graph) with integration owner | defer: check during integration with M5 | desktop A5: Open graph lands in edit mode with the workflow loaded | M5 adds a mode store or an unsaved-changes guard |

## Shared-part changes needed from the integration owner

1. `src/components/workbench/README.md` (required by the traceability hook): in the Contents table, replace the row
   `| SessionsPage.svelte | Placeholder for the Sessions tab until Milestone 8 of docs/plans/workbench-redesign/plan.md. |`
   with
   `| SessionsPage.svelte | Sessions tab: saved workflows and orchestrations shown as sessions with their runs from the run-context run list. No session service exists, so session actions are disabled (M8 report). |`
2. `ui/ListRow.svelte` (optional, low): there is no whitespace between the name and version spans, so the row's text and accessible name read "image-gen-sdxlv0.9.2". A literal space between `</span>` and `{#if version}` fixes it. The Sessions rows have their own markup and already include the space.
