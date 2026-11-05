# Workbench redesign: implementation spec

This is the design contract for porting the approved mockups in
`docs/design/mockups/` into the Svelte workbench. The mockups are the visual
authority; this document states the tokens, shared parts, behaviours and
page-by-page mapping so that pages built in parallel come out as one system.
The plan that sequences the work is `docs/plans/workbench-redesign/plan.md`.

To view the mockups locally: `bash docs/design/mockups/viewer/build.sh --serve`
then open http://127.0.0.1:8765/. Boards `Revised` (Scheduler), `Sessions`,
`Diagnostics`, `IoInspector`, `Network`, `Graph`, `NodeEditor`, `Library` are
the targets. Boards `Main`, `Studio`, `Instrument`, `Dark` are earlier rounds
and are not targets.

## Design principles (from the Calm Tech review)

- One place for attention: the selected run is the context for every tab, shown
  once in the header chip. Pages do not repeat it in their own headers.
- Status is carried by a small glyph vocabulary, never by pulsing motion, bright
  LED-like colour, or uppercase tracked labels.
- Errors are explained in sentences with a suggested next action, inside the
  panel they belong to, not in page-wide red banners.
- Filters are progressive: a search field, one status segment, and a `Filters`
  disclosure. Applied secondary filters appear as removable chips.
- Lists are grouped under sticky collapsible headers and scroll inside their
  panel; the page itself never scrolls.
- Destructive actions are two-step (`Cancel run` → `Confirm cancel`) and reset
  after four seconds.
- Engine unavailability is shown in place: the header line changes and content
  dims to 72% opacity; pages keep showing the last projection.

## Tokens

Define these as CSS custom properties on `:root` in `src/styles.css` and expose
them to Tailwind through `tailwind.config.cjs` (`theme.extend.colors` keyed to
`var(--…)`). Replace the current `!important` neutral overrides; the new
palette is the only palette.

| Token | Value | Use |
| --- | --- | --- |
| `--pg-ground` | `#2A2E35` | page background |
| `--pg-panel` | `#33383F` | panels, cards |
| `--pg-panel-raised` | `#3A3F47` | hover rows |
| `--pg-panel-selected` | `#434A54` | selected row (plus 3px inset left bar `--pg-text`) |
| `--pg-panel-sunken` | `#23272D` | inputs, popovers, lane tracks, header bar |
| `--pg-panel-head` | `#2E333A` | group headers, panel footers, device lane strip |
| `--pg-border` | `#4A5059` | panel borders, dividers |
| `--pg-border-soft` | `#3B4048` | row separators |
| `--pg-border-strong` | `#8A909A` | hover borders, dimension lines |
| `--pg-text` | `#E4E6EA` | primary text |
| `--pg-text-bright` | `#FFFFFF` | titles, selected |
| `--pg-text-muted` | `#A3A9B4` | secondary text |
| `--pg-text-soft` | `#C9CDD4` | labels, mono ids |
| `--pg-accent` | `#D9B06A` | running state, focus ring, live wire (brass) |
| `--pg-danger` | `#E39A9A` | failed state, destructive text |
| `--pg-ok` | `#A7C4A0` | engine online dot |
| `--pg-info` | `#9FB7D9` | numbers in code views |

Fonts: `Newsreader` (serif, titles and the brand word), `Atkinson Hyperlegible`
(UI text), `JetBrains Mono` (ids, timestamps, code). Bundle them through
`@fontsource/*` packages so the desktop app works offline; if the packages
cannot be installed, fall back to the system stacks `Georgia, serif`,
`system-ui, sans-serif`, `ui-monospace, monospace` and record the gap in
`issues.md`. Base size 14px, line height 1.45. Titles 24px/500 serif, page
subtitles 20px.

Scrollbars: `scrollbar-width: thin; scrollbar-color: #5A6069 transparent` plus
the WebKit equivalents (10px, thumb `#5A6069` with a 2px `--pg-panel` border,
hover `#7A818C`). Apply globally.

Focus: `outline: 2px solid var(--pg-accent); outline-offset: 1px` on every
focusable control via `:focus-visible`.

## Status glyph vocabulary

One component, `StatusGlyph`, 10px circle, 1.5px border:

| State | Rendering |
| --- | --- |
| running | filled accent |
| waiting | left half accent (`linear-gradient(90deg, accent 50%, transparent 50%)`), accent border |
| queued / next / not reached | hollow, muted border |
| succeeded / done | filled muted |
| failed | filled danger, 1px radius, rotated 45° (a small diamond) |
| cancelled / skipped | hollow, dashed border |
| idle (no run) | hollow, soft border |
| warn (log level) | filled accent diamond |

Map backend statuses: `accepted`, `future`, `scheduled`, `queued`, `delayed`
→ queued; `running` → running; `completed` → succeeded; `failed` → failed;
`cancelled` → cancelled. Node statuses map the same way; `Waiting` → waiting.

## Shell

`WorkbenchShell.svelte` becomes a top bar, 40px tall, background
`--pg-panel-sunken`, with a 1px `#1F2227` bottom border:

```
Pantograph | Sessions | │ | Scheduler Diagnostics I/O Inspector Network | │ | Graph Node Editor Library | … | [run chip ▾ ×] [■ Local engine] Settings [Assistant]
```

- Tabs are text, 13.5px, 11px horizontal padding, 2px bottom border on the
  current tab (`--pg-text`), hover tint `rgba(255,255,255,.04)`. Keyboard
  reachable, `aria-current="page"` on the active one. The lucide icon rail and
  the "Active Run" label block are removed.
- Vertical dividers (1px, 18px tall) separate the three groups. Settings is
  pinned to the right as a tab. The Assistant toggle keeps its existing
  behaviour; style it as a `tgl` (26px, bordered).
- Engine line: a 7px square dot, `--pg-ok` when online, `--pg-danger` when the
  transport reports unavailable, followed by "Local engine" or "Engine
  unreachable since HH:MM, showing last projection" and a small Retry.
- The run-context picker (below) sits between the tabs and the engine line.

Page ids gain `sessions`. Order: sessions, scheduler, diagnostics, io_inspector,
network, graph, node_lab, library, settings. Scheduler stays the default page.

## Run-context picker

The run chip in the header is the single control for the shared selection
(ADR-014: transient frontend state shared across pages).

- Source of runs: the scheduler run-list projection (`schedulerRunListStore`,
  fed by `workflowService.queryRunList`). The picker never queries the ledger.
- Chip when a run is selected: glyph, workflow id, version (`v1.4.0`, mono,
  muted), `▾`; a separate `×` button clears the selection. Chip when none:
  "Pick a run ▾" with a dashed border.
- Clicking the chip opens a popover (400px, anchored right) with a search field
  (placeholder "Search runs, or paste a run id") and the grouped list:
  Running, Waiting for memory, Queued, Finished today. Rows: glyph, workflow
  id + version, one-line status context, run id in mono on the right.
  Selecting a row sets the active run and closes the popover. Escape and an
  outside click close it. Focus goes to the search field on open and returns
  to the chip on close.
- Search matches workflow id, run id and version. If the typed or pasted text
  equals a known run id, select it immediately. If it looks like a run id but
  is unknown, show "No run with id … in today's history."
- Clipboard: while the pointer is over a run row in the Scheduler list, a
  picker row, or any element that renders a run id with `data-run-id`, Ctrl+C
  (or Cmd+C) copies the run id and shows a 1.6s toast "Copied run_…". While
  the pointer is over the chip, Ctrl+V reads the clipboard and selects that
  run if known, otherwise opens the popover with the text in the search field.
  Clipboard permission failures fall back to opening the popover. Keystrokes
  inside inputs are ignored.
- Pages with no run selected show an empty state centred in the page: a short
  serif question ("Which run do you want to look into?"), one sentence, and a
  primary "Pick a run" button that opens the popover.

## Shared parts (`src/components/workbench/ui/`)

Build these once; pages compose them. Names are suggestions, the behaviour is
the contract.

- `DimensionLine`: a 1px rule with tick ends and a centred mono label on the
  page background. Used as the page summary line under the header.
- `Panel`: `--pg-panel` background, `--pg-border` border, 4px radius; a column
  variant with a fixed top (`ptop`), scrolling middle (`scroll`) and fixed
  footer (`pfoot`, `--pg-panel-head` background, 48px min height).
- `GroupHeader`: sticky, collapsible, label + mono count + chevron, `aria-expanded`.
- `ListRow`: grid `14px 1fr auto`, glyph, name (bold) + mono version, two
  muted sub-lines, time on the right; `aria-current="true"` when selected.
- `SegmentedControl`: bordered group, each button with a label and mono count,
  `aria-pressed` on the active one; wraps instead of clipping.
- `FilterChip`: pill with a `×` button; `Chip`: static pill.
- `DeviceLanes`: per device, name + "x of y GB in use" and a track of blocks
  whose widths are proportional to GB: running (accent fill), waiting (accent
  hatch), next (hollow), free (empty). Legend to the right.
- `KeyValue`: two-column grid of muted keys and values.
- `LogLine`: grid `96px 14px 1fr`, mono timestamp, level glyph, message with a
  bold kind and a muted source suffix. Error lines get a 6% danger tint.
- `ErrorCard`: left 3px danger bar, bold title, muted code line, one sentence
  of explanation, one sentence starting "Try:", and an action button.
- `TwoStepButton`: `danger` → `confirm` with a 4s reset.
- `EmptyState`: the centred question + sentence + primary button.
- `Toast`: fixed bottom centre, mono, 1.6s.

## Pages

Each page keeps its presenters and service calls. What changes is the
projection into the new layout. Every page: dimension line first, then the
two-panel inbox layout (fixed-width list left, detail right) unless noted.

### Scheduler (`Revised` board)

- Dimension line: "N runs waiting, about M min of queued work on D devices"
  from the scheduler estimate projection; omit the minutes when no estimate.
- Device lanes strip (collapsible via a `showLanes` preference) from the
  local Network status projection where device memory is available; hide the
  strip when the projection has no device facts.
- Left panel 380px: search, status segment (All, Active, Queued, Done, Failed
  with counts), `Filters` disclosure holding the remaining 11 selects from the
  current page in a two-column grid, applied-filter chips, then the grouped
  list (Running, Waiting for memory, Queued, Finished today). Footer: `Move to
  front`, `Priority` number input, two-step `Cancel run` for live runs; "has
  finished. No actions available." for finished ones; "Select a run to act on
  it. The other tabs follow this selection." when none.
- Right panel: title (workflow id, version), run id (copyable), status chip
  plus placement/priority/queue chips, buttons Diagnostics / Graph / I/O that
  switch tabs; three cells Progress, Placement, Retention; then the scheduler
  timeline (`querySchedulerTimeline`) as a time/event grid with the sentence
  "Node-level events for this run are in Diagnostics."
- Selecting a row sets the active run (same store as the chip).

### Diagnostics (`Diagnostics` board)

- Dimension line: "run_id, failed 19:35:42 after 22 s, 1 error in load_model"
  style sentence from run detail.
- Left panel 300px: title, run id, chips (status, placement, client/session);
  Timing and Placement key-value sections; collapsible "Nodes by status" with
  thin bars; collapsible "Compared with earlier runs" using the comparison
  facets (two big serif numbers and a sentence); Projection freshness line.
- Right panel: level segment (All, Warnings, Errors with counts), source
  segment (All sources, Scheduler, Runtime, Nodes), search, Follow toggle;
  the ErrorCard when the run has a terminal error, with "Show in log" which
  switches the level filter to Errors; then the timeline as LogLines; a
  footer sentence "N of M events shown. Payload bodies for this run are in
  I/O Inspector."
- The existing `diagnosticsFocus` behaviour (deep link to an event) is kept:
  the focused event line gets the selected row treatment and is scrolled into
  view.

### I/O Inspector (`IoInspector` board)

- Dimension line: "run_id, N nodes, M retained artifacts, X MB kept for 7 days".
- Left panel 340px: search, direction segment (All, Inputs, Outputs); a tree of
  nodes (glyph, node id, mono type, "N retained ports") each expandable to its
  artifacts ("→ port" for outputs, "← port" for inputs, size, kind and
  retention). Footer: "Payloads kept 7 days. Retention settings" linking to
  Settings as today.
- Right panel: title `node:port`, artifact id, chips (role, kind, retention,
  media + size); buttons Read body / Download / Acknowledge with the existing
  semantics; the preview region (image, text/JSON in a code block, or the
  "Only metadata was kept for this port." panel); sections "Where it came
  from", "How it was made", "Storage" from the artifact descriptor fields.
- The compact run graph strip is removed from this page; Graph owns graphs.
- Saved-graph mode (no run) becomes the empty state with "Pick a run".

### Network (`Network` board)

- Dimension line: "N nodes, c connected, d degraded, p waiting to be paired.
  G GPUs, V GB VRAM reachable" from local status plus peers; omit parts that
  the projection does not provide.
- Left panel 340px: segment (All nodes, Shared with me, Mine), groups Local
  and Peers with rows (glyph by transport state, display name, node id,
  two sub-lines, latency on the right). Footer "Pair a node" (disabled with
  a title until pairing exists) and "Trusted peers only".
- Right panel: title, OS and captured time, chips; four stat cells (CPU with
  meter, memory with meter, running/queued, runtime sessions); Devices as
  DeviceLanes; the selected run's placement on this node ("<workflow> on this
  node": run id, state, runtime session, assets read) or a sentence that the
  run is placed elsewhere; Disks and Interfaces as small tables; Identity.
- Degraded metrics render as muted text in their section, not an amber list.

### Graph (`Graph` board)

- Toolbar row (no dimension line of its own; the dimension line sits in the
  toolbar between the status text and the panel toggles): a two-way mode
  switch `Edit workflow` / `As run <last 6 of id>`; the workflow selector
  (bold name, mono version, popover of saved workflows with "Picking a
  workflow switches to editing it…" footer); "Saved HH:MM" or "Unsaved
  changes" in edit mode, "read-only, as it ran" in run mode; `Nodes` and
  `Properties` panel toggles; Save, Version input, `+patch`, `Submit run` in
  edit mode; `Edit current vX` and `Save as new workflow` in run mode.
- Edit mode is the current editor (SvelteFlow canvas, palette, properties)
  restyled: palette as a tree with category counts and `in:out` port counts,
  dotted canvas background, node cards with the glyph in the header, a
  bottom-left zoom/fit/snap pill group, minimap bottom right.
- Run mode renders the run graph projection in an isolated read-only surface
  (ADR-014): node glyphs from node status, the live wire in accent, failed
  wires dashed danger, a legend top right, and the properties panel showing
  values "as they were when the run started" plus an "In this run" section
  with I/O and Diagnostics buttons. Entering run mode never touches the
  editable graph store.
- If the selected run belongs to a different workflow than the one being
  edited, a one-line note offers "Show <wf> as run" or keeps editing.
- Submit disabled reasons stay as today but render as the button's title and
  a muted sentence under the toolbar, not a banner.

### Node Editor (`NodeEditor` board)

Node authoring has no backend in this build. Keep the capability represented
at its owning boundary: the page shows the built-in node definitions read-only
(list left with "Built-in" group, definition form centre with inputs/outputs
tables and an implementation block reading "built-in, source not editable",
live canvas preview and Checks right). Authoring controls (`New node`, `Add
input`, `Save draft`, `Publish to runtime`, `Try with sample inputs`) render
with `aria-disabled` and a title, and the page opens with the in-place notice
"Node authoring is not available in this build…" naming Settings as the place
where it would be enabled. Do not advertise saving or publishing as working.
The "Your nodes" group is omitted until a definition store exists.

### Library (`Library` board)

- Dimension line: "N models, X GB on disk, L loaded on devices, U unused for
  30 days (Y GB)". Omit facts the projection lacks.
- Left panel 400px: search + `Find on Hugging Face` toggle, kind segment
  (All, Text, Image, Audio) when the model kind is known; groups "Loaded on a
  device", "On disk, used this month", "On disk, unused 30 days"; rows with
  two cache squares (accent = loaded on that device, grey = disk only), name,
  family, usage sub-line, size and last-used on the right. Footer legend and
  free disk space.
- Search mode replaces the list with Hugging Face results (mono repo id,
  sub-line, size, Download button that reads "Queued" once requested) using
  the existing search/download service calls.
- Right panel: title, pumas id, chips; `Use in a new graph` (disabled until
  supported), `Open folder` (disabled until supported), two-step `Delete from
  disk` blocked with a title while a run uses the model; sections "Where it
  lives", "How it is used", "Workflows that use it" (table from usage
  projection), "Source".

### Sessions (new, `Sessions` board)

Sessions are a new tab over existing authority only:

- Active: the client sessions the attribution projection exposes (this GUI
  window's session, CLI sessions) with their runs. If no service lists client
  sessions, show only the current window's session derived from the run list
  (runs whose client session matches) and record the gap in `issues.md`.
- Saved: the saved workflows (`list_workflows`) and orchestrations, each shown
  as a session card with created/modified dates and the runs of that workflow
  from the run list.
- Rows: glyph (running if any live run, succeeded if active and idle, idle if
  saved), name, client tag, sub-line, a thin outcome strip (accent running,
  grey succeeded, danger failed, hollow queued), run count and last activity.
- Detail: title, id, chips; Activity and "Runs from this session" key-value
  sections with "Show in Scheduler" (switches tab and applies the session
  filter); "Open in this session" list with Open graph buttons (loads the
  saved workflow into Graph edit mode); "What resumes" sentence.
- Actions: `Rename`, `Save a copy`, `Close session`, `Resume`, `Delete` only
  where a service exists. Others render `aria-disabled` with a title. Do not
  invent persistence.

### Settings

Not redesigned in this effort. It must still look right under the new tokens:
remove its dependence on the old `bg-neutral-*` overrides and use the shared
`Panel`, `KeyValue` and field styles for its existing sections.

## Accessibility contract (selected for this change)

Supported users operate the desktop app with pointer and keyboard; screen
reader semantics are preserved through native elements. Every control is a
`button`, `a`, `input`, `select` or `textarea` with a visible or hidden name.
Lists of selectable rows are buttons with `aria-current`. Toggles use
`aria-pressed`, disclosure uses `aria-expanded`, the picker popover is a
`role="dialog"` with a label, the log is `role="log"`. Decorative glyphs are
`aria-hidden` and the status is also in text. Focus is visible (accent
outline) and never depends on colour alone. `scripts/check-svelte-a11y.mjs`
must pass on every changed file.

## Evidence

- Deterministic: presenter and store tests under `src/**/*.test.ts`
  (`npm run test:frontend`), `npm run typecheck`, `npm run lint:a11y`,
  `npm run lint:critical`, `npx eslint <changed files> --max-warnings 0`.
- Representative browser: `npx vite build` then a headless Chromium walkthrough
  with `docs/design/mockups/viewer/cdp.js` against the built site, capturing a
  screenshot per page with mocked services where a live engine is absent.
- Real desktop: `npm run dev:desktop` by the owner, checking each tab with the
  local engine running. Recorded as manual evidence in the ledger.
