# Execution ledger: workbench redesign

Dated slice, verification, deviation and commit summaries. The plan owns
current state; this file owns history.

## 2026-11-03

- Planning: mockups (rounds 1–3) reviewed with the owner on the Design canvas
  and locally; direction E approved for every tab; run-context picker,
  clipboard shortcuts and Graph run/edit modes added at the owner's request.
  Mockup sources copied to `docs/design/mockups/`; spec written at
  `docs/design/workbench-redesign-spec.md`; branch `feat/workbench-redesign`
  created from `019caf4aa`.
- Standards: Coding-Standards checkout `5bce3110`; task facts routed through the
  Standards Engine CLI (application purpose), snapshot
  `snapshot:v1:58a8cb6f-8bc1-4f8b-9f84-caa364b3df2b`; closure recorded in the plan.
- M0 started: delegated to an Opus 5.5 agent with the write set in the plan.

### M0 Foundation: implemented, verified, uncommitted (2026-11-03)

What changed:

- Tokens and base styles: `src/styles.css` now defines the spec palette as
  `--pg-*` on `:root`, plus a few derived values taken from the mockup CSS
  (`--pg-ink`, `--pg-text-quiet`, `--pg-text-code`, `--pg-border-idle`,
  `--pg-border-danger`, `--pg-rule-log`, scrollbar thumbs). Body background is
  `var(--pg-ground)` with the Atkinson Hyperlegible stack at 14px/1.45; themed
  thin scrollbars and a 2px accent `:focus-visible` outline apply globally.
  The `!important` neutral overrides and the radial-gradient body hacks are
  deleted; one `!important` block for package graph components remains
  (WR-008). The mockup primitives are ported value for value as `pg-*`
  classes in `@layer components`.
- `tailwind.config.cjs`: `pg` colours keyed to the variables, font families
  (serif Newsreader, sans Atkinson Hyperlegible, mono JetBrains Mono), and the
  `neutral` scale retuned to the same greys (950 ground, 900 panel) so pages
  not yet restyled sit on the new palette without overrides.
- `index.html`: title "Pantograph", `bg-neutral-950` body class removed.
- Dependency decision (fonts): requirement is offline-safe bundled fonts for
  the desktop app. Installed `@fontsource/newsreader@5.3.0`,
  `@fontsource/atkinson-hyperlegible@5.3.0`, `@fontsource/jetbrains-mono@5.3.0`
  (all SIL OFL 1.1, `license: OFL-1.1` in the lockfile), nothing else
  installed or upgraded (one lockfile `peer` flag npm dropped on an unrelated
  entry was restored). Consumer: `@import` of weights 400/500 (Newsreader,
  JetBrains Mono) and 400/700 (Atkinson) at the top of `src/styles.css`.
  Verification: `vite build` emits 18 woff2 files plus woff fallbacks under
  `dist/assets/` and the CSS references them. The first `npm install` timed
  out (ETIMEDOUT); a retry with longer fetch timeouts succeeded, so WR-003 is
  not the font fallback and system stacks are only the in-stack fallback.
- Shell: `WorkbenchShell.svelte` rewritten: brand word, three tab groups with
  dividers (Sessions | Scheduler, Diagnostics, I/O Inspector, Network | Graph,
  Node Editor, Library), run-context picker, engine line, Settings pinned
  right, Assistant toggle (`aria-pressed`, drives the existing side panel via
  `toggleSidePanel`). The header and pages pad right by `$panelWidth` so the
  Assistant panel never covers them. Engine line reads the run-list query:
  `transport_error` shows "Engine unreachable since HH:MM, showing last
  projection" (or "nothing loaded yet") with Retry, and the page area dims to
  72%. Icon rail and "Active Run" label removed. `data-testid`
  `workbench-nav-<id>` kept for the e2e suite.
- Store: `workbenchStore.ts` gains `sessions` (first) and a `group` per page,
  `WORKBENCH_TAB_GROUPS`, `workbenchPagesInGroup`; Scheduler stays default.
  `SessionsPage.svelte` placeholder renders EmptyState "Sessions are coming in
  Milestone 8."
- Run-context picker in `src/components/workbench/shell/`: `RunContextPicker.svelte`,
  pure helpers `runContextPicker.ts`, run-list source `runContextRunList.ts`
  (generation token; outcomes applied, failed, unreachable, superseded,
  detached), clipboard controller `runContextClipboard.ts` (one delegated
  `pointerover`, `pointerout` and `keydown` listener on `document`; second
  attach throws; stale paste reads classified superseded), shared actions in
  `runContextStore.ts` (`openRunContextPicker`, `closeRunContextPicker`,
  `selectRunContext`, `clearRunContext`, `refreshRunContextRunList`,
  `runContextRunList`, `runContextLiveUpdates`). Run list from
  `workflowService.queryRunList({ limit: 250 })` only.
- Shared parts in `src/components/workbench/ui/` (exported from `index.ts`):
  StatusGlyph (+ `statusGlyph.ts`), DimensionLine, Panel, GroupHeader, ListRow,
  SegmentedControl, Chip, FilterChip, DeviceLanes, KeyValue, LogLine, ErrorCard,
  TwoStepButton, EmptyState, Toast, plus `types.ts`.
- READMEs with the hook's headings: `src/components/workbench/`, `shell/`,
  `ui/`, and (outside the literal write set, under WR-001, see WR-003)
  `src/README.md` and `src/stores/README.md`.

Verification (run from the repo root, all on the final tree):

- `npm run typecheck`: pass.
- `npm run test:frontend`: pass, 574 tests, 0 failures (new: 5 glyph, 10
  picker helper, 7 run-list source, 8 clipboard, 2 store tests).
- `npx eslint --max-warnings 0` on every changed .ts/.svelte file and
  `tailwind.config.cjs`: pass.
- `npm run lint:a11y`: fails only on pre-existing items in untouched files
  (`IoInspectorPage.svelte:1024`, `RunGraphSnapshot.svelte:156`,
  `SavedGraphInspectionSnapshot.svelte:77`); no finding in M0 files (WR-007).
- `npm run lint:critical`: fails only on pre-existing
  `IoInspectorPage.svelte:508:20` `no-append-remove-child` (WR-007).
- `TRACEABILITY_BASE_REF=codex/pantograph-rpc-workflow-2026-09-26 ./scripts/check-decision-traceability.sh`:
  exit 0, "No changed files detected" (HEAD equals the base; nothing is
  committed). A scratch copy that reads the working-tree change list passes
  "Decision traceability check passed."; the same copy without
  `src/README.md` in the list fails as expected (WR-003).
- `npx vite build --outDir /tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m0/dist`: pass, no Svelte warnings.
- Headless screenshots of the built app (1440x900):
  `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m0/shell.png` (plain browser: engine unreachable, Scheduler shows its
  existing transport error), and with a scratch Tauri stand-in that answers
  only `workflow_run_list_query`: `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m0/picker-open.png`,
  `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m0/picker-copied.png`, `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m0/chip-selected.png`, `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m0/picker-unknown.png`,
  `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m0/sessions.png`, `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m0/diagnostics.png`.
- CDP walkthrough results: focus moves to the search field on open; groups
  Running 2, Queued 3, Finished today 2; Ctrl+C over a hovered row copied the
  id (headless denied the system clipboard, so the in-window fallback and its
  toast were used, WR-010); Escape closed and returned focus to the chip;
  Ctrl+V over the chip selected the copied run; `×` cleared it and focus
  stayed on the chip; an unknown `run_` id showed "No run with id … in recent
  history."; typing an exact id selected it and closed the popover; an outside
  click closed it.

Deviations and findings: WR-003 to WR-010 in `issues.md`. Device memory is
absent from the local network status projection (WR-006), so the plan's lanes
assumption resolves to its stop condition.
- M0 accepted by the integration owner after reviewing the shell and picker
  screenshots; WR-003 READMEs kept and added to the write set; WR-009
  deviations accepted. Walkthrough tooling moved to `reports/tools/`.
- M1–M8 started in parallel (one Opus 5.5 agent per page, distinct scratch
  directories, ports 8791–8798, CDP ports 9341–9348). Agents write
  `reports/m<N>-<page>.md`; the integration owner merges them here.

## 2026-11-03, page milestones (merged from reports/m*.md)

### M1 Scheduler (report: reports/m1-scheduler.md)

- M1 Scheduler implemented (Opus 5.5 agent, 2026-11-03). The Scheduler is rebuilt on the `Revised` board with the shared parts. It reads the shell's `runContextRunList` as its only run-list source and no longer queries `workflow_run_list_query` (WR-005). Grouping comes from the picker helpers, with no memory-wait group and delayed runs under Queued (WR-004). The device strip shows one muted sentence behind the `showLanes` preference (WR-006).
- The 14 filter selects moved into the Filters disclosure with applied-filter chips. The status segment and search stay outside it.
- Every action is still reachable: Move to front, Priority/Set and two-step Cancel run in the footer; Resume and GUI admin (two-step Admin cancel, Front, Priority/Set) under More actions. Each has its existing guard, and a disabled control has a title explaining why.
- Gates: typecheck pass; test:frontend 685/685; lint:a11y pass; lint:critical pass; eslint pass on the changed files; vite build pass. The traceability hook says "No changed files" (WR-003). A scratch copy run on the M1 file list needs the `src/components/workbench/README.md` and `src/stores/README.md` updates listed in the M1 report.
- CDP walkthrough with `fixture-scheduler.js`: one run-list query with Scheduler open, selection follows the chip, cancel arms then confirms, 14 disclosure selects, the page does not scroll. Screenshots are in `scratchpad/m1/`.

### M2 Diagnostics (report: reports/m2-diagnostics.md)

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

### M3 I/O Inspector (report: reports/m3-io-inspector.md)

### M3 I/O Inspector: implemented, verified, uncommitted (2026-11-03)

- **Board:** `IoInspectorPage.svelte` is rebuilt to the IoInspector board.
  - Dimension line with run id, node and artifact counts, kept bytes and projection freshness (`data-run-id`).
  - A 340px node and port tree built from the run snapshot nodes, node statuses and resolved node I/O joined with the artifact projection. Artifacts with no resolved row fall back to producer/consumer metadata, and node-less artifacts go under a "Workflow boundary" group.
  - Direction segment, search, a Filters disclosure holding the Backend filter, and retention chips.
  - A one-artifact detail panel: title `node:port`, chips, Read body / Read stream / Download / Acknowledge with unchanged service semantics, full-image preview, bounded 64 KiB text preview, binary-body message, metadata-only panel, and the three descriptor sections.
- **Stale graph facts:** muted ErrorCard-shaped cards inside the node they concern.
- **Saved-graph inspection:** moved into an "Inspect a saved workflow instead" disclosure under the no-run EmptyState. The select, the stable-ids note and the restyled `SavedGraphInspectionSnapshot` (native button nodes) are kept.
- **Graph strip:** the compact run graph strip (`RunGraphSnapshot`) is removed from this page.
- **WR-007 resolved for M3's files:**
  - declarative download anchor, with one owner for the revoke timers;
  - `<video>` preview replaced by the binary-body message (WR-M3-3);
  - native buttons in the saved graph.
- **Presenters:** new projection logic, with 14 new presenter tests.
- **Gates:** typecheck, test:frontend (685/685), lint:a11y, lint:critical, eslint on the changed files and vite build all pass.
- **Evidence:** CDP walkthrough with `reports/tools/fixture-io.js` produced the empty, saved-workflow, image, text, metadata-only, filters and cleared screenshots (paths in `reports/m3-io-inspector.md`).

### M4 Network (report: reports/m4-network.md)

> **2026-11-03, M4 Network (Opus 5.5 page agent).** `NetworkPage.svelte`
> rebuilt to the Network board from the existing projections (local network
> status, node status, library usage, scheduler timeline); refresh ownership
> unchanged except a request serial on the status query. New pure presenters in
> `networkPagePresenters.ts` with 16 tests (675 frontend tests pass). Devices
> show GPU availability and reason with the WR-006 sentence; no lanes. Peers
> listed with transport glyphs; owner segments, pairing and forget are disabled
> with titles. Degraded metrics render as muted text inside the affected
> section. Gates: typecheck pass; eslint on the three files pass; lint:a11y and
> lint:critical fail only on WR-007 (IoInspectorPage); traceability "No changed
> files" (WR-003); vite build pass. Headless CDP walkthrough with
> `reports/tools/fixture-network.js`: screenshots of the local node with and
> without a run, events expanded, a degraded and a pairing-required peer, the
> failed-status and plain-browser states (paths in `reports/m4-network.md`).

### M6 Node Editor (report: reports/m6-node-editor.md)

- M6 Node Editor implemented (`NodeLabPage.svelte`, `nodeLabPresenters.ts` +
  test).
  - The page projects the built-in definitions from the Graph palette's
    `nodeDefinitions` store: one "Built-in" group, a read-only definition
    form, the canvas preview card and Checks ("Built-in node, always valid").
    It adds no service call.
  - The in-place notice names the Authoring API, the Local Agent and Runtime
    Publishing, and says they would be enabled in Settings.
  - Every authoring control is disabled with a reason in its `title`: six
    `aria-disabled` action buttons, and natively disabled fields and remove
    buttons. Nothing saves, publishes or tries.
  - Fields the registry lacks are omitted: version, runtime, device, retention.
  - Gates: typecheck pass; test:frontend 660/660; eslint pass on the three
    files. lint:a11y and lint:critical fail only on the WR-007 items. vite
    build passes. Traceability exit 0 (no committed changes).
  - Headless walkthrough with a scratch `get_node_definitions` fixture
    (17 mock definitions): selection, search, collapse, disabled titles and a
    non-scrolling page were checked.
  - Screenshots: `scratchpad/m6/node-editor-unavailable.png`,
    `node-editor.png`, `node-editor-llm.png`, `node-editor-search.png`.

### M7 Library (report: reports/m7-library.md)

2026-11-03, M7 Library implemented (Opus 5.5 page agent). `LibraryPage.svelte` rebuilt on the shared
parts per the Library board: Pumas selector rows joined with the library usage projection by
`pumas://models/<model_id>`; groups by last access (Used this month / Unused for 30 days / No usage
recorded, or Models when no usage exists) because device residency is not projected; kind segment from
Pumas model type, task and pipeline tag; Hugging Face search mode over the existing search and download
calls with a download-options disclosure; two-step delete keyed by model and blocked while the active run
last used the model or an action is busy; `Use in a new graph` and `Open folder` disabled with titles.
Presenters gained the join, grouping, detail, search and request-gate logic with 15 new tests.
Gates: eslint (changed files), typecheck, test:frontend 685/685, lint:a11y, lint:critical, vite build pass;
traceability range mode reports no changed files (WR-003). Headless walkthrough with
`reports/tools/fixture-library.js` recorded populated, active-run, delete, search, unavailable and error
states (screenshots in the M7 scratch directory).

### M8 Sessions (report: reports/m8-sessions.md)

- M8 Sessions implemented (Opus 5.5 page agent). `SessionsPage.svelte` replaces the placeholder. It projects saved workflows (`list_workflows`) and orchestrations (`list_orchestrations`, through `orchestrationStore.loadOrchestrationList`, which had no caller before) as saved sessions. Each session shows its runs from the shell's run-list projection, matched on exact `workflow_id`. Active is omitted with a muted sentence because no service lists client sessions and the window holds no client session id (plan assumption "run list exposes a client session id per run": the field exists on runs, but there is no current-window id to match, so the stop condition applies). `Show in Scheduler` sets the Scheduler search to the workflow id and switches tab. `Open graph` uses the Graph selector's loader and switches to Graph. `Delete workflow` is a `TwoStepButton` over the existing `deleteWorkflowByName` (it replaces `window.confirm`). Session actions with no service render disabled with titles. New pure presenter `sessionsPresenters.ts` with 23 tests, including the page's latest-only invocation gate. Gates: typecheck, test:frontend (685 pass), lint:a11y, lint:critical, eslint on changed files, and vite build all pass. The traceability hook needs the `src/components/workbench/README.md` row update (integration owner). Headless walkthrough with `reports/tools/fixture-sessions.js` on port 8798 / CDP 9348; screenshots in `scratchpad/m8/`.

Issue IDs assigned at merge: WR-M1-1 → WR-011; WR-M1-2 → WR-012; WR-M1-3 → WR-013; WR-M1-4 → WR-014; WR-M1-5 → WR-015; WR-M2-1 → WR-016; WR-M2-2 → WR-017; WR-M2-3 → WR-018; WR-M2-4 → WR-019; WR-M2-5 → WR-020; WR-M2-6 → WR-021; WR-M3-1 → WR-022; WR-M3-2 → WR-023; WR-M3-3 → WR-024; WR-M4-1 → WR-025; WR-M4-2 → WR-026; WR-M4-3 → WR-027; WR-M6-1 → WR-028; WR-M6-2 → WR-029; WR-M6-3 → WR-030; WR-M6-4 → WR-031; WR-M6-5 → WR-032; WR-M7-1 → WR-033; WR-M7-2 → WR-034; WR-M7-3 → WR-035; WR-M7-4 → WR-036; WR-M7-5 → WR-037; WR-M7-6 → WR-038; WR-M8a → WR-039; WR-M8b → WR-040; WR-M8c → WR-041; WR-M8d → WR-042; WR-M8e → WR-043; WR-M8f → WR-044.

## 2026-11-03, remaining milestones (merged from reports)

### M5 Graph (report: reports/m5-graph.md)

### M5 Graph (report: reports/m5-graph.md)

2026-11-03, M5 Graph implemented (Opus 5.5 page agent). The Graph page has two modes in a
segmented control: `Edit workflow` (the existing SvelteFlow editor restyled: palette tree with
category and `in:out` counts, dotted canvas, BaseNode with the status glyph in its header,
bottom-left pill controls, properties panel) and `As run <last 6>` (the run graph projection
in the read-only `RunGraphSnapshot`: glyph cards, live wire in accent, failed wires dashed
danger, legend, zoom/fit; properties show captured settings, ports and "In this run" with I/O
and Diagnostics). Default is run mode when the selected run is of the open workflow,
otherwise edit mode with a one-line "Show `<wf>` as run" offer. Run mode reads only
`queryRunGraph` and `queryNodeStatus` (+ `node_status` invalidations) and never writes the
editable graph store. Workflow selector, Save/New/Clear/Delete (same confirms), version,
`+patch` and Submit kept; submit-unavailable reasons as title plus a muted sentence; the
active-run banner removed; `Save as new workflow` disabled (no service creates a workflow
from a run snapshot). `RunGraphSnapshot.svelte:156` a11y finding fixed by native buttons
(WR-007 item). `WorkflowGraph.css` has no `!important`; the WR-008 block in `src/styles.css`
can be deleted. 20 new presenter tests. Gates: typecheck, test:frontend 687/687, lint:a11y,
lint:critical, eslint (changed files), vite build pass; traceability range mode reports no
changed files (WR-003). Headless walkthrough with `reports/tools/fixture-graph.js`
(port 8795 / CDP 9345); screenshots in `scratchpad/m5/`.

### M9a Settings restyle (report: reports/m9-settings.md)

> **2026-11-03, M9 Settings restyle (Opus 5.5 page agent).** `SettingsPage.svelte`
> restyled without redesign: two `Panel`s (Settings, scrolling inside; 320px
> summary), `pg-sec` sections, `pg-fl`/`pg-field` fields, `pg-card` groups,
> `KeyValue` rows, `Chip` readiness with a status glyph, `pg-btn` actions; errors
> and status as muted sentences in their section; no neutral, cyan, emerald,
> amber or red classes, no uppercase labels, no spinner. Same service calls,
> handlers, controls, messages, ids and `settingsFocus` deep link.
> `settingsPagePresenters.ts` emits `KeyValueRow`s with sentence-case copy, a
> glyph instead of Tailwind status classes, and capability rows; 10 presenter
> tests (687 frontend tests pass). Gates: typecheck, lint:a11y, lint:critical,
> eslint on the three files and vite build pass; traceability "No changed files"
> (WR-003). Headless CDP walkthrough with `reports/tools/fixture-settings.js`:
> populated sections, save/validation/cleanup/refresh messages, deep link from
> I/O Inspector scrolling the panel (document stays 900px), plain-browser state
> (paths in `reports/m9-settings.md`). The embedded app panels keep their old
> styling (WR-M9-2).

### M9b I/O Inspector e2e harnesses (report: reports/m9-e2e-harnesses.md)

### M9 e2e harness port: selectors updated, static checks pass, runs pending (2026-11-03)

- **Harnesses ported to the M3 I/O Inspector.** `tests/e2e/io-inspector-image-preview/run.mjs` and
  `tests/e2e/workflow-editor-image-generation/workflow-editor-image-generation.e2e.mjs` now:
  - select an artifact's `io-artifact-row` before card assertions;
  - use `io-artifact-stream-button` and the footer Refresh;
  - scope error checks to `io-artifact-access-error`.

  Every assertion is kept. The stale-read check now waits for the body read before removing the
  artifact.
- **Gates:** ESLint (`--max-warnings 0`) on all four harness files and `node --check` pass. On the
  unmodified files the same ESLint command reported 92 errors (missing globals for `tests/e2e`), cleared
  with in-file `/* global */` headers. `npm run typecheck` passes but does not cover `tests/e2e`.
- **Open:** WR-023 stays open until the owner runs
  `node tests/e2e/io-inspector-image-preview/run.mjs` and `npm run test:workflow-editor-image-gui`
  (with the four smoke env vars) on the desktop.

Issue IDs assigned at merge: WR-M5-1 → WR-045; WR-M5-2 → WR-046; WR-M5-3 → WR-047; WR-M5-4 → WR-048; WR-M5-5 → WR-049; WR-M5-6 → WR-050; WR-M5-7 → WR-051; WR-M5-8 → WR-052; WR-M5-9 → WR-053; WR-M5-10 → WR-054; WR-M9-1 → WR-055; WR-M9-2 → WR-056; WR-M9-3 → WR-057.

## 2026-11-03, integration slice (integration owner)

- Accepted M1–M8 and M9a/M9b after reviewing each report and screenshot set.
  Delegated two follow-ups found during acceptance: M9c restyles the seven
  panels embedded in Settings (WR-056), M9d restyles the graph node, edge and
  minimap components and removes the unused `GraphSelector` and run-graph
  presenter exports (WR-049, M5 requests 2–5).
- Shared-part additions requested by page reports, all additive:
  `SegmentOption.disabled`/`title` honoured by `SegmentedControl` (plus a
  `.pg-seg button:disabled` style); `KeyValueRow.runId` renders
  `data-run-id`; `Panel` takes `scrollRole`, `scrollLabel` and a bindable
  `scrollElement`; `EmptyState` renders optional `children` under the action;
  `ErrorCard` takes `tone="muted"` and `testId`; `LogLine` takes `title`;
  `ListRow.glyph` is optional and keeps its column; `.pg-ver` no longer wraps
  and has a leading space; `.pg-fl textarea.pg-field` grows (WR-M9 request 1).
- `src/styles.css`: the last `!important` block (graph package overrides) is
  removed on the M5 agent's confirmation that every selector it covered now
  styles itself (WR-008 closed). `!important` count in the file is 0.
- `PumasHfModelSearchResult` now mirrors the producer, Pumas
  `HuggingFaceModel` (`rename_all = "camelCase"`: `repoId`, `totalSizeBytes`,
  …); the Library presenter drops its `id` fallback and its test case for it
  (WR-037 closed).
- I/O Inspector Refresh button gained `data-testid="io-inspector-refresh"`;
  the WebKit harness selector uses it (M9b request).
- Settings: the deep-link id `settings-diagnostics-retention-policy` moved
  from the ArtifactStore policy form to the Diagnostics retention form, so
  "Retention settings" lands on the right section (WR-055 fixed; behaviour
  change accepted by the integration owner as a defect repair).
- Shell: the engine line shrinks with an ellipsis instead of overlapping
  Settings when the Assistant panel is open (M5 request 8).
- READMEs: workbench README gained the Graph row and the run-mode invariant;
  tools README lists the page fixtures.
- Gates after these edits: `npm run typecheck` pass; `npm run test:frontend`
  689/689; `lint:a11y` pass; `lint:critical` pass; eslint on the touched
  files pass. Vite build and the combined walkthrough follow M9c/M9d.

## 2026-11-03, follow-up slices M9c and M9d (merged from reports)

### M9c Settings embedded panels (report: reports/m9c-settings-panels.md)

> **2026-11-03, M9c Settings embedded panels (Opus 5.5 follow-up agent,
> WR-M9-2).** `ServerStatus` (and `server-status/` parts), `ModelConfig`,
> `DeviceConfig`, `RagStatus`, `SandboxSettings`, `ManagedRuntimePanel` (and
> its `runtime-manager/` parts) and `ManagedBinaryOverviewPanel` restyled onto
> `pg-card`, `pg-btn`, `pg-field`, `pg-tbl`, `pg-meter`, `KeyValue`, `Chip`,
> `SegmentedControl` and `StatusGlyph`: each app panel is a disclosure card
> (`h3` button with `aria-expanded`) with a glyph plus status word; errors are
> muted `role="alert"` sentences; no neutral, cyan, colour, uppercase,
> spinner, pulse or lucide classes remain in those files. Same service calls,
> handlers, ids, disabled conditions and messages; copy sentence case;
> previously unlabelled inputs gained labels and per-row buttons distinct
> names. New pure presenters `serverStatusPresenters.ts` and
> `managedRuntimePresenters.ts` (6 tests; 688 frontend tests pass). New
> READMEs for `src/components/`, `server-status/` and `runtime-manager/`.
> Gates: typecheck, lint:a11y, lint:critical, eslint on the changed files and
> vite build pass; traceability "No changed files" (WR-003). CDP walkthrough
> with a scratch fixture: populated and no-host states, document stays 900px
> (paths in `reports/m9c-settings-panels.md`). `BackendSelector.svelte`
> (outside this write set) still carries old classes (WR-M9c-1).

### M9d Graph node, edge and minimap components (report: reports/m9d-graph-components.md)

### M9d Graph canvas components (report: reports/m9d-graph-components.md)

2026-11-03, M9d implemented (Opus 5.5 follow-up agent). WR-M5-5 closed for the app canvas. All 21 workflow node
cards (and their 6 header and panel parts) drop category colours, coloured header tiles, pulsing dots and per-type focus colours. They render
their title through the new `NodeHeaderTitle` inside `BaseNode`'s glyph header and no longer override the
card border, so selection (a `--pg-text` border plus a 1px ring) is the same everywhere. `NodeGroupNode` is
neutral with a dashed border. `ReconnectableEdge` draws neutral grey wires (selected `--pg-text`) and keeps
the accent for the live insert preview; drift is dashed danger. Minimap nodes use `--pg-border-idle` (groups
`--pg-text-muted`). The package constants follow M5 request 2: mask `rgba(35,39,45,.6)` and no default edge
`style`. `GraphSelector.svelte` is deleted. The ten unused `runGraphPresenters` exports are gone, with two
kept private because `buildRunGraphCanvasModel` uses them. Traceability READMEs were added for
`src/components/nodes/`, `nodes/workflow/`, `edges/` and `packages/svelte-graph/src/`. Header height (37px)
and handle offsets (`52 + 20·i`) were measured unchanged on every fixture node. Gates: typecheck,
test:frontend 688/688, lint:a11y, lint:critical, eslint on the changed files and vite build all pass;
traceability in range mode reports no changed files (WR-003). Screenshots are in `scratchpad/m9d/`
(port 8801, CDP 9351).

Issue IDs assigned at merge: WR-M9-2 was already WR-056 (dropped duplicate WR-058); WR-M9c-1 → WR-059; WR-M9c-2 → WR-060; WR-M9c-3 → WR-061; WR-M5-5 → WR-062; WR-M9d-1 → WR-063; WR-M9d-2 → WR-064; WR-M9d-3 → WR-065.

## 2026-11-03, closure (integration owner)

- All milestones M0–M9 (including M9c and M9d) implemented. Final tree gates:
  `npm run typecheck` pass; `npm run test:frontend` 688/688; `lint:a11y`
  pass; `lint:critical` pass; eslint `--max-warnings 0` on every changed
  source file (106 files) pass; `npx vite build` pass.
- Combined walkthrough (headless Chromium 1440x900 over the built app, all
  page fixtures stacked on `reports/tools/fixture.js`): Scheduler started
  with "Pick a run"; clicking the failed run set the chip to
  image-gen-sdxl v0.9.1; Diagnostics, I/O Inspector, Network, Graph (edit mode
  with the "Show image-gen-sdxl as run" note, since the loaded workflow
  differs), Node Editor, Library, Sessions and Settings all rendered with that
  context; Ctrl+C over a Scheduler row showed "Copied run_7c9e6679…"; Ctrl+V
  over the chip on I/O Inspector selected ingest-docs-rag v1.4.0; clearing
  showed the I/O and Diagnostics empty states; the picker opened from the
  empty chip; document scrollHeight stayed 900 at a 900px viewport.
  Screenshots: `reports/evidence/01-scheduler-no-run.png` … `14-picker-open.png`.
- WebKitGTK I/O Inspector harness run on this machine (`DISPLAY=:0.0 node
  tests/e2e/io-inspector-image-preview/run.mjs`): "PASS real WebKitGTK: PNG
  360210 bytes decoded 320x320; full-read, malformed/partial/failure, text
  bound, stale read, and URL lifecycle checks passed", exit 0. Log at
  `reports/evidence/webkit-io-inspector-harness.log`. Environment: the
  operator's X display and installed WebKitWebDriver; recorded as
  representative, not the desktop app.
- Plan moved to `Verifying`; A1–A4 satisfied, A5 pending the owner.
- Working tree: 105 modified, 20 new, 1 deleted (`GraphSelector.svelte`);
  nothing committed. The pre-commit traceability hook was simulated by the
  agents over their file lists; the real hook runs at commit time (WR-003).

## 2026-11-03, port onto main and pull requests (integration owner)

- The source branch `feat/workbench-redesign` sits on
  `codex/pantograph-rpc-workflow-2026-09-26` (forked from main 2026-09-09, 37
  unmerged commits, 417 behind), so its commits were cherry-picked with `-x`
  onto `origin/main` (46aad5841) in a worktree. Replacement lineage:
  4352fce1b → e7d22d224 (design docs); cc2260d76 → 6fa30ce9f (source);
  fca20d6ab → 753b94fe4 (e2e harnesses); 2c5aae1d0 → 3af3f09d0 (plan).
  Twelve files conflicted with fixes main gained since September; each was
  resolved by keeping the redesign and re-applying main's behavioural change
  (toolbar typed submit inputs and seed checks, image output node artifact
  decode and URL lifecycle, number input seed validation, text output empty
  branch, external connection `canConnect` gating, Library browser-host
  capability gating, I/O Inspector page-lifecycle fencing on download, shell
  "Loopback browser host" chip). Branch-only commit 6126c5a34's complete-image
  guard (`assertCompleteIoArtifactImageRead`) reaches main through the port
  because the redesigned page depends on it.
- Follow-ups on the ported branch: a08e027fa adds an eslint environment for
  the plan's tooling scripts; b90eb6349 removes `artifactDownloadLifecycle.ts`,
  which nothing imports after the redesign.
- Gates on the ported branch: typecheck pass; test:frontend 873/873;
  lint:a11y pass; lint:critical pass; eslint `--max-warnings 0` on every
  changed file pass; `TRACEABILITY_MODE=range` against origin/main pass
  (225 changed paths); vite build pass; combined walkthrough identical to the
  source branch; WebKitGTK I/O harness PASS.
- Pull requests: #96 `docs/workbench-redesign-design` → `main` (design docs);
  #97 `feat/workbench-redesign-main` → `docs/workbench-redesign-design`
  (implementation, stacked). The source branch `feat/workbench-redesign` is
  superseded by the port and can be retired once #97 is integrated
  (disposition: `superseded`, its commits are patch-equivalent to the port).
