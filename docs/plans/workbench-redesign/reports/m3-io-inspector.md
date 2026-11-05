# M3 I/O Inspector: report

Agent: M3 page agent (Opus 5.5), 2026-11-03. Branch `feat/workbench-redesign`,
uncommitted. Board: `docs/design/mockups/boards/IoInspector.dc.html`.

## Changed files

| File | Change |
| --- | --- |
| `src/components/workbench/IoInspectorPage.svelte` | Rewritten to the board: dimension line, 340px node and port tree, one-artifact detail panel; no-run EmptyState with the saved-workflow disclosure. Stops importing `RunGraphSnapshot.svelte`. |
| `src/components/workbench/ioInspectorPresenters.ts` | Existing exports unchanged; new M3 section appended (tree, filters, selection, labels, detail sections, backend filter, stale-fact cards, view-box parsing, download URL owner). |
| `src/components/workbench/ioInspectorPresenters.test.ts` | 14 new tests (42 in the file). |
| `src/components/workbench/SavedGraphInspectionSnapshot.svelte` | Rewritten: native `<button>` nodes over a decorative SVG edge layer, `pg` tokens, muted stale-fact cards. |
| `docs/plans/workbench-redesign/reports/tools/fixture-io.js` | New evidence fixture, loaded after `fixture.js` (see Screenshots). |
| `docs/plans/workbench-redesign/reports/m3-io-inspector.md` | This report. |

Nothing outside the write set was edited. `RunGraphSnapshot.svelte`, `runGraphPresenters.ts` and `graphInspectionPresenters.ts` (M5) are only imported. The page no longer imports `runGraphPresenters.ts`.

## What the page does now

- **Dimension line:** `run_id, N nodes, M retained artifacts, X of payloads kept, projection current at seq K`. The wrapper carries `data-run-id`, so Ctrl+C copies the run id. Projection freshness (ADR-014 guardrail) lives here, and so does "live updates off" when the invalidation subscription fails. That failure no longer becomes a page error.
- **Left panel ("Nodes and ports"):**
  - Top: search field, a `Filters` toggle (`aria-expanded`) whose disclosure holds the existing Backend text filter, and the All/Inputs/Outputs `SegmentedControl` with counts. Below them, a `FilterChip` for an applied backend and `Chip`s for the retention summary.
  - Tree: one disclosure row per node of the run snapshot (glyph, node id, mono type, "Succeeded, 2 retained ports" or "Queued, not reached yet", plus the stale-fact count). Under each node, its port rows (`← port` input, `→ port` output, with a hidden "Input"/"Output" word for screen readers; size; "Kind, retention"). Rows are native buttons with `aria-current`.
  - Tree sources: `run_graph.graph.nodes`, `node_statuses`, and `resolved_node_io` joined with `io_artifacts` through the existing `buildResolvedNodeIoDisplayRows`. Artifacts that no resolved row references fall back to their producer/consumer node and port metadata. Artifacts with no node go under a trailing "Workflow boundary" group. `_data` diagnostic ports stay hidden, as before.
  - Footer: Refresh and Retention settings (`focusSettingsSection('diagnostics_retention')`, as before).
- **Right panel ("Artifact"):**
  - Header: title `node:port`, the mono artifact id, and chips for role, kind, retention, and media type with size.
  - Buttons: Read body, Read stream, Download, Acknowledge. A disabled button carries a `title` saying why.
  - Preview region: the full image (`<img>`, full-body read), text or JSON in a `pg-code` block (bounded 64 KiB read, 32k-character decode, "text truncated" note), audio, or the binary-body message. A port with no readable body or stream shows "Only metadata was kept for this port." with the retention reason.
  - Sections: "Where it came from", "How it was made" and "Storage" (`KeyValue`), built from the existing descriptor rows.
  - Access errors render as an `ErrorCard` inside the panel. The consume result renders as a muted `role="status"` line.
- **Stale graph facts:** muted cards shaped like `ErrorCard` (left bar in `--pg-border-strong`, never danger or orange). Node-scoped facts appear inside the expanded node they concern. Graph-scope facts appear at the top of the tree.
- **No run selected:** `EmptyState` ("Whose inputs and outputs?", Pick a run → `openRunContextPicker()`). Directly under it is a disclosure, "Inspect a saved workflow instead" (`aria-expanded`). It holds the existing Saved workflow select, the "N saved workflows with stable ids" note, Refresh, and `SavedGraphInspectionSnapshot`. **This disclosure is where saved-graph inspection now lives.** It loads (`listWorkflows` then `inspectWorkflowGraph`) when opened, or again when the run is cleared while it is open. Picking a run supersedes a saved-graph load already in flight.

### Service semantics kept exactly

- **Run data:** `queryRunInspection({ workflow_run_id, artifact_limit: 250 })` with the same generation-token supersession. Reload triggers are unchanged: run change, Backend filter change, projection invalidation (`run_detail`, `node_status`, `io_artifact`), and Refresh. The client-side Backend filter moved verbatim into `applyIoInspectorBackendFilter` (tested).
- **Read body:** descriptor check (`verifyArtifactReadable`), then `readArtifactBody(buildIoArtifactPreviewReadRequest(...))`. Images are read in full; everything else is bounded to bytes 0 to 65 536. The `assertCompleteIoArtifactImageRead` check is kept.
- **Read stream:** `verifyArtifactStreamReadable`, then `readArtifactStream` (bounded), with the same completeness check.
- **Download:** `verifyArtifactReadable`, then a full `readArtifactBody`, with the same filename builder.
- **Acknowledge:** `acknowledgeArtifactConsumed({ consumer_id: 'pantograph-gui-io-inspector' })`, then the preview is released if the payload was not retained, then `refreshInspector()`.
- **Unchanged helpers:** the access-request authority (request serial, lifecycle generation, current-selection check), the image decode-error handling and the object-URL lifecycle were copied unchanged. The only change: "current selection" now means the one artifact shown on the right instead of any card of the selected node.
- **No ledger reads:** no new backend calls, no ledger reads, no optimistic mutation.

### WR-007 fixes (no ignore comments added)

- **`IoInspectorPage.svelte:1024`** (`svelte-ignore a11y_media_has_caption` on `<video>`): the `<video>` element is gone. A video preview read is bounded to its first 64 KiB, so inline playback was at best partial. Video now gets the binary-body message pointing at Download. This is a user-visible change and needs owner review (issue row below).
- **`IoInspectorPage.svelte:508`** (`appendChild`): Download now renders a declarative `<a download>` (inside `{#if downloadLink}`, `bind:this`), clicks it after `tick()`, and removes it.
  - A single `createIoArtifactDownloadUrlOwner` (presenter, tested) owns the 30 s revoke timers. On page destroy it cancels them and revokes the URLs. Before, the timers outlived the page.
  - A download whose body arrives after the page is gone is revoked at once and never clicked.
- **`SavedGraphInspectionSnapshot.svelte:77`** (`<g role="button">`): saved-graph nodes are now native `<button>`s with `aria-current` and an `aria-label` from `formatSavedGraphNodeAccessibleLabel`. They are absolutely positioned at graph coordinates over an `aria-hidden` SVG that draws only the edges. The `tick()` focus-restore workaround is no longer needed.
- **`RunGraphSnapshot.svelte`**: no longer imported by this page. `npm run lint:a11y` now passes for the whole tree, so M5 appears to have already handled line 156.

### Behaviour notes for review

- **Default selection:** the first image output, else the first output, else the first entry. The chosen entry stays selected across refreshes while it exists.
- **Resets:** selection and collapsed nodes reset when the run changes. Changing the selected entry clears pending reads, as changing the node did before.
- **Saved-graph view box:** `SavedGraphInspectionSnapshot` previously read `model.canvas.viewBox.x` and so on, but `buildRunGraphCanvasModel` returns `viewBox` as a string, so the old SVG got an `undefined` viewBox. The new `parseGraphCanvasViewBox` accepts the string or an object, so either shape M5 settles on works.
- **Retention chips:** these come from `retention_summary`, which the Backend filter does not narrow. That was already the case before this change.

## Verification (from the repo root, 2026-11-03)

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

$ npx eslint src/components/workbench/IoInspectorPage.svelte src/components/workbench/SavedGraphInspectionSnapshot.svelte src/components/workbench/ioInspectorPresenters.ts src/components/workbench/ioInspectorPresenters.test.ts --max-warnings 0
(no output, exit 0)

$ TRACEABILITY_BASE_REF=codex/pantograph-rpc-workflow-2026-09-26 ./scripts/check-decision-traceability.sh
No changed files detected for decision traceability check.
(range mode on uncommitted work; see WR-003)

$ npx vite build --outDir /tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m3/dist --emptyOutDir
✓ built in 21.88s   (no Svelte or Vite warnings for the changed files)
```

An earlier typecheck run showed TS2305 errors in `diagnosticsPagePresenters.test.ts`, which belongs to M2 and was in progress at the time. The final run above is clean.

A grep over the two changed components finds no `RunGraphSnapshot`, `appendChild`, `removeChild`, `svelte-ignore`, `role="button"`, `cyan`, `uppercase`, `tracking-` or `animate-`.

The new presenter tests cover:

- tree order, input/output order, `_data` exclusion, metadata fallback and the boundary group;
- no duplicate entries;
- direction and search filters and their counts;
- the selection keep/fallback rule;
- node, entry, chip and dimension labels, including freshness;
- the Backend filter;
- the detail sections;
- the preview mode, including video falling back to the binary message;
- the disabled-action reasons;
- stale-fact cards and their dedupe, and diagnostics grouped by node;
- view-box parsing;
- the download URL owner: delay, dispose, cancelled timers, and a release after dispose revoking at once.

## Screenshots

The built app was served with `python3 -m http.server 8793` and driven with `CDP_PORT=9343 node docs/plans/workbench-redesign/reports/tools/cdp-init.cjs <steps>`, injecting `tools/fixture.js` and then `tools/fixture-io.js`.

`fixture-io.js` keeps the base fixture's runs and answers:

- `workflow_run_inspection_query`: five nodes, an image, a text, a JSON and a metadata-only artifact, resolved I/O, and one node-scoped stale graph fact;
- `workflow_artifact_descriptor`;
- `workflow_read_artifact_body`: a 1024×1024 canvas-generated PNG, the prompt text and the JSON;
- `workflow_acknowledge_artifact_consumed`;
- `list_workflows` and `workflow_graph_inspect`;
- `plugin:event|listen` and `plugin:event|unlisten`.

The steps file is `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m3/steps-io.json`. The server was stopped afterwards and port 8793 is free.

| Screenshot | What it shows |
| --- | --- |
| `.../scratchpad/m3/io-empty.png` | No run: centred question, Pick a run, closed saved-workflow disclosure under it. |
| `.../scratchpad/m3/io-saved.png` | Disclosure open: Saved workflow select, stable-ids note, graph with native node buttons, `unload` selected with a muted stale-fact card. |
| `.../scratchpad/m3/io-image.png` | Run picked via the header picker; `decode:image` selected by default; Read body loaded the full 1024×1024 PNG (`naturalWidth` 1024, `data-preview-complete=true`). |
| `.../scratchpad/m3/io-text.png` | `prompt:text` read, text in a code block, extent note. |
| `.../scratchpad/m3/io-metadata.png` | `sample:latents` (metadata only): actions disabled with reasons, "Only metadata was kept for this port." with the retention reason; tree scrolled to show the muted stale-fact card inside `unload`. |
| `.../scratchpad/m3/io-filters.png` | Filters open, Backend `cuda` applied: `Backend cuda ×` chip, retention chips, the text artifact filtered out. |
| `.../scratchpad/m3/io-empty-after-clear.png` | Run cleared with the × on the chip: back to the empty state with the disclosure still open and reloaded. |

`...` = `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad`.

Walkthrough log (verbatim):

```
saved (expanded, nodes, path): ["true",5,".pantograph/workflows/image-gen-sdxl.json"]
populated (dim, rows, selected): ["run_1b4e28ba-2fa1-11d2-883f-0016d3cca427, 5 nodes, 4 retained artifacts, 4.1 MiB of payloads kept, projection current at seq 412",7,"art_7f21a5"]
image: [1024,1024,"true"]
text: ["A pantograph on a drafting table, brass ","text/plain · 110 B"]
meta: ["Only metadata was kept for this port. Intermediate node I/O keeps metadata only under the standard policy.",true,"Metadata retained only, so there is no body to read."]
chips: ["Backend cuda ×","Payload retained3","Metadata retained only1"]
after clear (empty, saved open, nodes): [true,"true",5]
```

## Ledger entry text

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

## Issue rows (IDs to be assigned by the integration owner)

| ID | Severity | Evidence | Relationship | Owner / boundary | Disposition | Verification | Revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WR-M3-1 | low | The board's "kept for 7 days" (dimension line) and "Payloads kept 7 days" (tree footer) need the retention duration. `workflow_run_inspection_query` exposes `retention_summary` counts but no policy duration, and the page may not add a policy query (no new backend calls). | I/O Inspector dimension line and footer | backend run inspection projection | defer: the dimension line reads "X of payloads kept"; the footer offers Retention settings and Refresh with no duration text | screenshot shows the duration once the projection carries it | inspection projection exposes the effective retention window |
| WR-M3-2 | medium | The I/O Inspector e2e harnesses assume the old layout. `tests/e2e/io-inspector-image-preview/run.mjs` expects one `io-artifact-card` per artifact, all visible at once, the Refresh button as `[data-testid=io-inspector-page] button:nth-of-type(2)`, and `button[aria-label^="Read artifact stream"]`. `tests/e2e/workflow-editor-image-generation/...e2e.mjs` looks for an image card. The page now shows one artifact at a time, so a row must be clicked first. | A4/A5 browser evidence for artifact reads | integration owner (tests are outside M3's write set) | fix-now when integrating: click `[data-testid=io-artifact-row][data-artifact-id=…]` before the card assertions, use `[data-testid=io-artifact-stream-button]` and the footer Refresh. The `io-artifact-card`, `io-artifact-read-button`, `io-artifact-image-preview` and `io-artifact-access-error` test ids and their data attributes are kept. | both WebKit e2e runs pass (not run here: they need WebKitWebDriver and a display) | integration of M3 |
| WR-M3-3 | low | Removing `svelte-ignore a11y_media_has_caption` (WR-007) without a fake caption track means the `<video>` inline preview is gone; video bodies now show the binary-body message and Download. The preview read for video was already bounded to 64 KiB, so playback was partial. | Media preview parity | MrScripty | defer: confirm, or add a full-body video read with real captions or a reviewed exemption | presenter test `preview mode follows the media family` (video → binary) | owner review, or video artifacts become common |

## Shared-part changes requested (not made)

- **`EmptyState`:** an optional `children` snippet rendered under the button. With it, the saved-workflow disclosure could sit inside the empty-state card. Today the page wraps `EmptyState` in a fixed-height slot and groups the two vertically.
- **`ErrorCard`:** a `tone="muted"` variant (left bar `--pg-border-strong`, no failed glyph). Stale graph facts would then use the shared part instead of the small `.fact` style now duplicated in `IoInspectorPage.svelte` and `SavedGraphInspectionSnapshot.svelte`. It would also need to accept a `data-testid` or rest props, since the page wraps it to keep `io-artifact-access-error`.
- **M5 (`graphInspectionPresenters.ts` / `runGraphPresenters.ts`):** `GraphInspectionCanvasModel.viewBox` is a string. M3 parses either a string or an object, so either shape is fine, but please keep `formatSavedGraphNodeAccessibleLabel` and the `SavedGraphInspectionDisplayModel` fields that M3 reads (`canvas.nodes[].{id,nodeType,x,y,width,height,staleBadgeLabel}`, `canvas.edges[].{id,sourceX,sourceY,targetX,targetY,staleDiagnosticCount}`, `diagnostics`, `selectedNodeId`, `selectedNodeDiagnostics`).
