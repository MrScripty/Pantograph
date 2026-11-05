# M5 Graph: report

Agent: Opus 5.5 page agent, 2026-11-03. Board: `docs/design/mockups/boards/Graph.dc.html`.
No git state changed. `src/styles.css` and `packages/svelte-graph` were not edited.

## Changed files

| File | Change |
| --- | --- |
| `src/components/workbench/GraphPage.svelte` | Rewritten. Owns the mode (`Edit workflow` / `As run <last 6>` as a `SegmentedControl`), panel toggles, the run-mode load of `queryRunGraph` + `queryNodeStatus` for the selected run (two request gates, one `node_status` invalidation subscription while in run mode), the "Show `<wf>` as run" note, `Edit current vX`, both properties panels (edit: label, type · id, ports, `Duplicate` disabled, two-step `Remove node` through `deleteSelection`; run: node facts, settings as captured, ports, "In this run" with `I/O` and `Diagnostics`, snapshot facts). The active-run banner is removed. The explicit mode choice is kept in module state keyed by run id so it survives a tab switch. |
| `src/components/workbench/RunGraphSnapshot.svelte` | Rewritten as the read-only run surface: dotted canvas, cards with status glyph, label and type, connected ports with dots, error or stale note, wires (live in accent, failed dashed danger, stale dotted), legend, zoom `+`/`−`/`Fit` with a fit-to-viewport default. Nodes are native `<button>`s (fixes `RunGraphSnapshot.svelte:156`, `reviewed-a11y-ignore`, by markup; no ignore remains). Props dropped that no consumer passes any more (`compact`, `artifactSummaries`, `focusedDiagnosticEventId`; I/O Inspector stopped importing the component in M3). |
| `src/components/workbench/runGraphPresenters.ts` | Existing exports unchanged. Added the snapshot layout (positions, ports, wire paths and kinds), node glyph and run sentence, node facts for the panel, captured-setting rows, selected-node default, fit/clamp scale, duration format, and `createRunGraphRequestGate`. |
| `src/components/workbench/runGraphPresenters.test.ts` | 11 new tests. |
| `src/components/workbench/graphInspectionPresenters.ts` | Existing exports unchanged. Added `resolveGraphPageMode`, run-id suffix and mode/dimension/edit-current/save-line labels, workflow-list time format, editor execution state to glyph, and the palette tree builder. |
| `src/components/workbench/graphInspectionPresenters.test.ts` | 9 new tests. |
| `src/components/WorkflowToolbar.svelte` | Same script (submit, validation summary, event and lifecycle subscriptions, version storage); new props `mode`, `run`, `runDimLabel`, `modeSwitch`, `panelToggles`, `onworkflowpicked`, `oneditcurrent`. One toolbar row per the board; stays mounted in both modes so its subscriptions never drop. Submit-unavailable reason is the button `title` plus one muted sentence (`id`/`data-testid` kept); submit error, persistence errors and `graphSessionError` are one-line sentences with a failed glyph, not banners. Run mode shows `Edit current vX` and `Save as new workflow` (`aria-disabled`, titled). The dimension line hides its words below 140px (container query) and keeps them in its `title`. |
| `src/components/WorkflowPersistenceControls.svelte` | Now the workflow selector (bold name, mono version; popover with saved workflows, system graphs, `New` / `Clear canvas` / `Delete` with the same confirm dialogs, footer sentence) plus `Save`, rendered without a wrapper so the parts sit in the toolbar row. Uses the existing loader `switchGraph`; `data-testid="workflow-graph-selector-toggle"` / `-option` and `data-workflow-id` kept for the e2e. Escape and an outside click close the popover (listeners live only while open). |
| `src/components/NodePalette.svelte` | Tree with category counts and `in:out` port counts, search (opens matching categories), drag and double-click/Enter to add unchanged. |
| `src/components/nodes/BaseNode.svelte` | Token colours; status glyph (with sr-only text) at the left of the header replaces the pulsing dot; no glow or pulse animation; error message as a danger line in the card. Header height, ports padding and handle positions (`52 + 20·i`), ids, types and classes unchanged. Handles are neutral (filled when connected); the data type stays in the port label's title. |
| `src/components/WorkflowGraph.css` | No `!important`. Tokens reach SvelteFlow through the `--xy-*` variables; dotted ground on `.svelte-flow`; neutral edge paths (CSS outranks ReconnectableEdge's presentation attributes); Controls as a horizontal pill group bottom left; minimap border and mask fill. |
| `docs/plans/workbench-redesign/reports/tools/fixture-graph.js` | New: wraps `fixture.js` (keeps its runs) and answers `get_node_definitions`, `list_workflows`, `load_workflow`, `create_workflow_execution_session`, `workflow_run_graph_query`, `workflow_node_status_query`; seeds `pantograph.lastGraph` and one stored semantic version. |
| `docs/plans/workbench-redesign/reports/m5-graph.md` | This report. |

## Data authority

Edit mode: unchanged (graph session and workflow stores, the same toolbar service calls).
Run mode adds only typed projection reads for the selected run:
`workflowService.queryRunGraph({ workflow_run_id })` and
`workflowService.queryNodeStatus({ workflow_run_id, limit: 250 })`, plus a `node_status`
invalidation subscription that re-reads node status. Run mode never touches the editable
graph store (ADR-014): it renders a copy of the projection in `RunGraphSnapshot` and the
selected run node is page-local state. `Edit current vX` loads the run's workflow through
the existing `switchGraph` loader (asks "Discard unsaved changes?" first when the editor is
dirty), or just switches mode when that workflow is already open.

Mode rule (`resolveGraphPageMode`, tested): no run → edit; a run whose `workflow_id` equals
the open saved workflow → run; otherwise edit with the one-line note offering
"Show `<wf>` as run". An explicit choice holds for that run id; selecting another run
returns to the default. Picking a workflow in the selector always switches to editing it.

`Save as new workflow`: checked. `save_workflow(name, graph)` is the only service that writes
a workflow, and `FileSystemWorkflowGraphStore::save_workflow`
(`crates/pantograph-workflow-service/src/graph/persistence.rs`) silently replaces the graph
of an existing file with the same identity and records no run provenance; the run snapshot
also keeps settings apart from the graph (`graph_settings`). No service creates a workflow
from a run snapshot, so the button is `aria-disabled` with a title (WR-M5-1).

Lifecycle: the run-mode `$effect` is the single owner of the load and the subscription; it
starts on entering run mode or changing run, and its cleanup unsubscribes and invalidates
both gates, so a load or status refresh that completes after a run change, mode change or
unmount is not applied. Failed status refreshes keep the last statuses and say so in the
panel. The selector popover listeners exist only while it is open.

## Verification (verbatim summaries)

```
npm run typecheck                -> tsc --project tsconfig.json --noEmit --pretty false (exit 0)
npm run test:frontend            -> ℹ tests 687 / ℹ pass 687 / ℹ fail 0
npm run lint:a11y                -> svelte accessibility gate passed
npm run lint:critical            -> critical anti-pattern gate passed
npx eslint <10 changed src files> --max-warnings 0   -> exit 0
TRACEABILITY_BASE_REF=codex/pantograph-rpc-workflow-2026-09-26 ./scripts/check-decision-traceability.sh
                                 -> No changed files detected for decision traceability check. (exit 0; range mode sees no uncommitted work, see WR-003 and WR-M5-8)
npx vite build --outDir <scratch>/m5/dist  -> ✓ built (no warnings)
grep !important in the M5 files  -> 0 in every file
```

The fixture file is not under the repo's lint scope (the base `fixture.js` fails eslint the
same way: browser globals).

Headless walkthroughs (`cdp-init.cjs`, `CDP_PORT=9345`, built app served on port 8795,
fixtures `fixture.js` + `fixture-graph.js`; step files `scratchpad/m5/steps*.json`):

- No run: edit mode, palette tree, "Submit unavailable: Workflow validation summary unavailable" under the toolbar.
- Run of the open workflow (`ingest-docs-rag`, running): mode switch `["Edit workflow:false","As run f90ae7:true"]`; live wires in accent; running node selected by default; panel shows settings, ports, "Running since …", snapshot facts.
- Explicit `Edit workflow`, then Scheduler and back: `["Edit workflow:true","As run f90ae7:false"]` (choice kept).
- Run of another workflow (`image-gen-sdxl`, failed): edit mode with "The selected run is of image-gen-sdxl, not ingest-docs-rag. Show image-gen-sdxl as run or keep editing ingest-docs-rag."; the button switches to run mode; failed node and dashed danger wire; `Edit current version` loads `image-gen-sdxl` and switches to edit (selector `image-gen-sdxl`, no note).
- Run with no captured graph (`summarize-daily`): designed empty state with an `Edit workflow` action.
- `As a run…` with no run opens the header picker.
- System graph: selector "App Architecture system", chip "read-only system graph", `Nodes` `aria-disabled`, palette hidden, Save disabled, "Submit unavailable: Cannot submit a read-only graph".
- Selector popover: Escape closes it and focus returns to the toggle (`[false,"workflow-graph-selector-toggle"]`).
- Assistant panel open (page 1056px wide): toolbar stays on one row; the dimension line drops its words.

Screenshots (all in `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m5/`):
`graph-edit.png`, `graph-run.png`, `graph-run-node.png`, `graph-run-legend.png`,
`graph-edit-run-selected.png`, `graph-mismatch.png`, `graph-run-failed.png`,
`graph-selector.png`, `graph-run-missing.png`, `graph-edit-current.png`, `graph-system.png`,
`graph-run-assistant.png`, `graph-edit-assistant.png`, and the board for comparison
`board-graph.png`. The server was stopped.

Not proven here: keyboard activation of the run-graph node buttons (the CDP tool sends
`keyDown` without `text`, so Enter does not activate any button in it; the nodes are native
buttons) and HTML5 drag from the palette in the Tauri webview (A5).

## Ledger entry text

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

## Issue rows

| ID | Severity | Evidence | Relationship | Owner / boundary | Disposition | Verification | Revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WR-M5-1 | low | No service creates a workflow from a run snapshot. `save_workflow(name, graph)` replaces an existing file's graph when the identity matches and stores no provenance; the snapshot keeps settings in `graph_settings`, apart from the graph. `Save as new workflow` renders `aria-disabled` with the title "Creating a workflow from a run snapshot is not available yet: no service saves a run graph as a new workflow." | Graph run-mode actions (A2) | workflow service (persistence) | defer | button disabled in `graph-run.png` | a "create workflow from run" command, or a create-only save that refuses existing identities |
| WR-M5-2 | low | No duplicate-node action exists in the workflow stores or backend; the edit properties footer shows `Duplicate` `aria-disabled` with a title | Graph edit properties (A2) | graph stores / backend | defer | `graph-edit-run-selected.png` | a duplicate (copy with new id) mutation |
| WR-M5-3 | low | No projection exposes a workflow's current saved semantic version (`WorkflowMetadata` has none). `Edit current vX` uses the toolbar's per-machine last submitted version (`pantograph.workflowSemanticVersion.<id>`), or reads "Edit current version" when none is stored | `Edit current vX` label | workflow registry | defer | `graph-run.png` (v1.4.1 stored), `graph-run-failed.png` (none) | the workflow list or a version query returns the current version |
| WR-M5-4 | low | Node status has no progress or ETA (`NodeStatusProjectionRecord`); the board's "61%, about 2 min left" card note is not shown. Run-mode settings show scalar values only; structured values in `graph_settings` are left out | Run-mode node cards and panel | node status projection | defer | presenter tests `run node facts …`, `setting rows …` | progress fields in the node status projection |
| WR-M5-5 | medium | Literal colours remain on the Graph canvas outside the M5 write set: `src/components/nodes/workflow/*.svelte` (GenericNode category borders and `animate-pulse` dot, coloured headers/icons in NumberInput, TextInput, LLMInference, PumaLib, ImageOutput, VectorOutput nodes), `src/components/edges/ReconnectableEdge.svelte` (type gradient and glow, cyan-ish insert preview `rgba(125,211,252,.45)`, drift red; main path neutralised from `WorkflowGraph.css`), `src/components/workflowMiniMap.ts` (category colours via inline `style:fill`, so CSS cannot reach them), package constants `WORKFLOW_GRAPH_MINIMAP_MASK_COLOR` and the `style` in `WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS` | Token-only palette, no cyan, no pulsing (spec) | integration owner (node components are no page's write set; package for the constants) | defer to a follow-up slice | `graph-edit.png`, `graph-edit-current.png` | node components restyled |
| WR-M5-6 | low | The board's `Snap` toggle has no counterpart: `WorkflowGraphCanvas.svelte` mounts xyflow `Controls` (zoom in/out, fit, lock) and has no snap-to-grid setting; Controls were restyled as the pill group | Edit canvas controls | `WorkflowGraphCanvas.svelte` owner | defer | `graph-edit.png` | snap-to-grid becomes an editor setting |
| WR-M5-7 | medium | `scripts/check-decision-traceability.sh` needs `README.md` with the fixed headings in `src/components/` and `src/components/nodes/` (M5 changes files there; neither exists) and an update to `src/components/workbench/README.md` in the same commit | Pre-commit for M5 | integration owner | fix at commit | hook passes on the staged M5 files | hook retired |
| WR-M5-8 | low | Palette items stay `div role="button" tabindex="0"` (Enter/Space add) rather than `<button draggable>`: HTML5 drag from a button is not reliable in every webview and drag could not be checked in WebKitGTK here | Native-elements rule vs. palette drag | integration owner | verify in A5 | drag a palette item onto the canvas in the desktop app; then try `<button draggable>` | A5 run |
| WR-M5-9 | low | Keyboard activation of run-graph node buttons and the selector rows was not driven: the CDP tool's key steps send no `text`, so Enter activates no button in it | A2 keyboard evidence | integration owner | verify in A5 (or add `text` to `cdp-init.cjs` key steps) | Tab to a run node, Enter selects it and fills the panel | tool update |
| WR-M5-10 | low | Picking a workflow in the selector (and Sessions' `Open graph`, WR-044) replaces the edit graph without asking about unsaved edits, as `GraphSelector` did; `Edit current vX` does ask. The explicit Graph mode lives in `GraphPage.svelte` module state, so another page cannot request `Edit workflow` | WR-044 | integration owner | defer: if Sessions must open Graph in edit mode, move the mode request (`{ runId, mode }`) into `workbenchStore` | — | Sessions integration |

## Requests to the integration owner

1. **WR-008, safe to remove now.** Delete the `!important` block in `src/styles.css`
   (`.svelte-flow, .base-node, .node-palette, .workflow-toolbar, .navigation-breadcrumb`).
   `.node-palette` and `.base-node` set the same panel background themselves,
   `.workflow-toolbar` is no longer used, `.svelte-flow` gets its ground from
   `WorkflowGraph.css`, and `.navigation-breadcrumb` is only in `UnifiedGraphView`, which is
   not mounted anywhere. After removal the canvas ground becomes `--pg-panel-head` as on the
   board (today the block paints it `--pg-panel`).
2. **Package constants** (read-only for M5), exact change: in
   `packages/svelte-graph/src/workflowGraphViewport.ts` set
   `WORKFLOW_GRAPH_MINIMAP_MASK_COLOR` to `'rgba(35, 39, 45, 0.6)'` (or stop passing
   `maskColor` in `WorkflowGraphCanvas.svelte` so `--xy-minimap-mask-background-color`
   applies), then drop the `.svelte-flow__minimap-mask` rule from `WorkflowGraph.css`; in
   `packages/svelte-graph/src/workflowGraphEdgeOptions.ts` remove `style: 'stroke: #525252;
   stroke-width: 2px;'` from `WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS` (the app's edge ignores it;
   other consumers would then follow `--xy-edge-stroke`).
3. `src/components/workflowMiniMap.ts`: return token colours (for example
   `'var(--pg-border-idle)'` for nodes, `'var(--pg-text-soft)'` for groups); its colours are
   inline `style:fill`, so only that file can change them (WR-M5-5).
4. `src/components/GraphSelector.svelte` is now unused (the selector lives in
   `WorkflowPersistenceControls.svelte`); delete it.
5. Dead exports in `runGraphPresenters.ts` now that I/O Inspector no longer uses them and the
   old snapshot tables are gone: `buildRunGraphNodeRows`, `buildRunGraphEdgeRows`,
   `resolveRunGraphCounts`, `formatRunGraphCountLabel`, `runGraphNodeErrorClass`,
   `runGraphStaleDiagnosticClass`, `runGraphNodeStatusClass`,
   `buildRunGraphNodeArtifactSummaries`, `formatRunGraphArtifactSummary`,
   `formatRunGraphArtifactDetail` (still covered by their old tests). I left them to avoid
   churn while M3 integrates; remove them with their tests when M3 is merged.
6. Traceability READMEs (WR-M5-7). Suggested `src/components/workbench/README.md` Contents row:
   "`GraphPage.svelte`, `RunGraphSnapshot.svelte`, `runGraphPresenters.ts`,
   `graphInspectionPresenters.ts` | Graph tab: edit mode over the editable graph stores and a
   read-only run mode over the run graph and node status projections (ADR-014)." and an
   Invariant: "Graph run mode never writes the editable graph store."
7. `reports/tools/README.md`: add a line for `fixture-graph.js`.
8. Shell (not M5): with the Assistant panel open the header's "Local engine" and "Settings"
   overlap (`graph-edit-assistant.png`, top right).
