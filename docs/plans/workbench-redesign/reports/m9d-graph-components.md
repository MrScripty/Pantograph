# M9d Graph canvas components: report

Agent: Opus 5.5 follow-up agent, 2026-11-03. Scope: WR-M5-5 and M5 report requests 2, 3, 4 and 5,
plus the traceability READMEs for the directories touched. Board: `docs/design/mockups/boards/Graph.dc.html`
(`.gnode`, `.wire`, `.minimap` in `boards/shell.css`). No git state changed. A forked helper restyled the
media and dependency-environment cards under the same conventions; everything below was verified together.

## Changed files

| File | Change |
| --- | --- |
| `src/components/nodes/workflow/NodeHeaderTitle.svelte` | New. The title every typed card puts in `BaseNode`'s `header` snippet: token type (12.5px/700, `--pg-text-bright`), one 20px line so the 37px header and the `52 + 20·i` handle offsets hold. |
| `src/components/nodes/workflow/GenericNode.svelte` | Category border colours and the pulsing status row removed; it is `BaseNode` alone. BaseNode's header glyph (title + sr-only text) carries the state. The old row also read `nodeExecutionStates.get(id)` as a string although the store holds `{ state, errorMessage }`, so it never showed the running/error words. |
| `AgentToolsNode`, `BooleanInputNode`, `EmbeddingNode`, `LinkedInputNode`, `LLMInferenceNode`, `MaskedTextInputNode`, `ModelProviderNode`, `NumberInputNode`, `PumaLibNode`, `SelectionInputNode`, `TextInputNode`, `TextOutputNode`, `VectorInputNode`, `VectorOutputNode` (`.svelte`) | Coloured header tiles replaced by `NodeHeaderTitle`; wrapper rules that overrode `.base-node` border and `.node-header` background removed (a scoped wrapper rule outranked `.base-node.selected`, so the selection border was lost on these cards); colour constants (`NODE_COLOR`, `nodeColor`, TextInput's `typeColors`) removed; fields `bg-pg-panel-sunken border-pg-border text-pg-text`; buttons raised panels; errors `text-pg-danger`; amber warnings soft text; `focus:outline-none` / coloured focus borders removed so the global `:focus-visible` accent ring applies; header status dots and `animate-pulse` removed (TextOutput's streaming caret is a static accent `▍`, live state). LLMInference validation tones moved to tokens (info soft, warning accent border, error danger, success ok). MaskedTextInput chips: anchored = selected panel, masked = dashed outline (not colour-only). PumaLib: `$state` + `$effect` mirror of `data.model_id` became a writable `$derived` (same behaviour; fixes the pre-existing `svelte/prefer-writable-derived` eslint error that failed `--max-warnings 0` on HEAD as well). |
| `AudioInputNode`, `AudioOutputNode`, `ImageOutputNode`, `PointCloudOutputNode` (`.svelte`) | Same conventions. Pink/violet/teal accents to tokens (`accent-pg-text-soft` sliders, "Streaming" in accent as live state); pulsing dots removed; image preview dialog in panel tokens; PointCloud canvas `var(--pg-panel-sunken)`. |
| `DependencyEnvironmentNode`, `DependencyEnvironmentNodeHeader`, `DependencyEnvironmentActivityLog`, `DependencyEnvironmentBindingsPanel`, `DependencyEnvironmentModeControls`, `DependencyEnvironmentRefPanel`, `DependencyEnvironmentStatusPanel` (`.svelte`) | Cyan wrapper/header/panels to tokens; the header is `NodeHeaderTitle`; Auto/Manual mode buttons gained `aria-pressed` (selected panel + text, not hue); the status badge box is neutral (state is in its label text). |
| `src/components/nodes/workflow/NodeGroupNode.svelte` | Purple/indigo gradient, glow and type-coloured inline handle styles removed. Neutral card, 2px dashed `--pg-border-strong` (solid when expanded; width kept so geometry is unchanged), selected = `--pg-text` border + 1px ring, neutral handles like `BaseNode`. Header still 37px; handles still `52 + 20·i`. |
| `src/components/edges/ReconnectableEdge.svelte` | Type gradient, `#fff` mid-stops and glow filter removed, and with them the per-edge `$nodes` subscription. Main path `--pg-border-strong` 1.5px, selected `--pg-text` 2px (scoped CSS, so the node-group editor mount follows too); insert preview (live drag) accent at 45%; inference drift dashed `--pg-danger` at 70% with its title. Edge id, `react-flow__edge-path` / `react-flow__edge-interaction` classes, interaction width and reconnect anchors unchanged. |
| `src/components/workflowMiniMap.ts` + `.test.ts` | Request 3: returns `var(--pg-border-idle)` for nodes and `var(--pg-text-muted)` for groups (exported as `WORKFLOW_MINIMAP_NODE_COLOR` / `WORKFLOW_MINIMAP_GROUP_COLOR`); categories no longer colour-coded. Tests rewritten for the new contract (3 tests). |
| `packages/svelte-graph/src/workflowGraphViewport.ts` + `.test.ts` | Request 2: `WORKFLOW_GRAPH_MINIMAP_MASK_COLOR = 'rgba(35, 39, 45, 0.6)'`; the test that pins the constant updated to match. |
| `packages/svelte-graph/src/workflowGraphEdgeOptions.ts` + `.test.ts` | Request 2: `style: 'stroke: #525252; stroke-width: 2px;'` removed from `WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS`; the pinning test updated. |
| `src/components/GraphSelector.svelte` | Request 4: deleted. `grep -rn GraphSelector src packages tests` found no importer (only its own log string); the e2e uses the `workflow-graph-selector-*` test ids, which live in `WorkflowPersistenceControls.svelte`. |
| `src/components/workbench/runGraphPresenters.ts` + `.test.ts` | Request 5: the ten exports are gone. `grep` showed no importer outside the presenter and its test. Two of them, `runGraphNodeStatusClass` and `formatRunGraphArtifactSummary`, are still called by `buildRunGraphCanvasModel` (used by `graphInspectionPresenters.ts`), so they stay as module-private helpers (with `formatCount`); the other eight, the three row/count interfaces they returned and the helpers only they used (`formatGraphEdgeEndpoint`, `formatNodePosition`) are deleted, plus the now-unused `IoArtifactProjectionRecord` import. Tests: five tests of removed functions deleted; three mixed tests keep their `buildRunGraphCanvasModel` assertions; the artifact test builds its summary literal and now also asserts `artifactSummaryLabel` on the canvas nodes (covers the private helper). |
| `src/components/nodes/README.md`, `src/components/nodes/workflow/README.md`, `src/components/edges/README.md`, `packages/svelte-graph/src/README.md` | New, with the 11 headings `scripts/check-decision-traceability.sh` requires and real boundary content (handle geometry contract, token-only rule, selection invariant, what the shared constants mean). None of the four existed. |
| `docs/plans/workbench-redesign/reports/m9d-graph-components.md` | This report. |

Not touched: `BaseNode.svelte` (M5's restyle already matches the board; the cards now inherit it), `WorkflowGraph.css`, `WorkflowGraphCanvas.svelte`, `src/components/README.md` (M9c), any `.ts` helper under `nodes/workflow/`.

## Saved-graph compatibility

Presentation only. Handle ids, handle types/positions, the `port-connected` / `intent-*` /
`inference-drift-affected` classes, `nodrag nopan nowheel` and `copyable-output` classes, stored node data
keys, event handlers and wrapper class names are unchanged. Measured in the built app (headless, both fixture
workflows, every node):

```
ingest-docs-rag: read_docs header=37 handles=53 | chunk_size header=37 handles=53 | split header=37 handles=53,73,53
                 embed_chunks header=37 handles=53,73,53,73 | index header=37 handles=53 | manifest header=37 handles=53,73
image-gen-sdxl:  prompt header=37 handles=53 | load_model header=37 handles=53,73
                 sample header=37 handles=53,73,93,113,53,73,93 | decode header=37 handles=53
handle ids: prompt:text | load_model:model_path,model_id | sample:prompt,system,model,tools,response,stream,kv | decode:image
```

(Handle centres are measured from the card's outer edge, so `52 + 20·i` reads as 53, 73, ... with the 1px
border.) Selection: clicking `sample` gives border `rgb(228, 230, 234)` and box-shadow
`rgb(228, 230, 234) 0px 0px 0px 1px`, i.e. a `--pg-text` border plus outline ring, no blue glow.
Rendered edge stroke `rgb(138, 144, 154)` (`--pg-border-strong`), minimap node fill `rgb(107, 113, 123)`
(`--pg-border-idle`), mask fill `rgba(35, 39, 45, 0.6)`, `[class*=animate-]` elements on the canvas: 0.

Literal colours left in the write set, both because a token cannot be used there:
`PointCloudOutputNode.svelte:168` `new THREE.Color(0x23272d)`. three.js cannot read CSS variables, so this
is the `--pg-panel-sunken` value, with a comment. `ImageOutputNode.svelte:464`
`rgba(31, 34, 39, 0.82)` is `--pg-ink` at 82% for the preview backdrop, because the tokens are hex and take no alpha.
`NodeGroupNode`'s `box-shadow: 0 1px 0 rgba(0,0,0,.25)` matches BaseNode and the board.

## Svelte Flow attribution

`@xyflow/svelte` is MIT (`node_modules/@xyflow/svelte/LICENSE`). Its `proOptions` doc says "You are free to
remove this attribution", only asking that people read the removal guide first. `Attribution.svelte` renders
nothing when `proOptions.hideAttribution` is true. So hiding it is allowed. The switch is a `SvelteFlow` prop
in `src/components/WorkflowGraphCanvas.svelte`, which is outside this write set (request 1 below). Until that
change lands, the attribution stays as M5 left it: muted grey text on a transparent background
(`--xy-attribution-background-color: transparent`).

## Verification (verbatim summaries)

```
npm run typecheck                -> tsc --project tsconfig.json --noEmit --pretty false (exit 0)
npm run test:frontend            -> ℹ tests 688 / ℹ pass 688 / ℹ fail 0
npm run lint:a11y                -> svelte accessibility gate passed
npm run lint:critical            -> critical anti-pattern gate passed
npx eslint <all 28 nodes/workflow .svelte, ReconnectableEdge.svelte, workflowMiniMap(.test).ts,
            workflowGraphViewport(.test).ts, workflowGraphEdgeOptions(.test).ts,
            runGraphPresenters(.test).ts> --max-warnings 0     -> exit 0
TRACEABILITY_BASE_REF=codex/pantograph-rpc-workflow-2026-09-26 ./scripts/check-decision-traceability.sh
                                 -> No changed files detected for decision traceability check. (exit 0; range mode, WR-003)
Simulation (script copy whose changed-file list is this milestone's uncommitted files):
  all M9d files                  -> Changed directory without decision traceability update: src/components/workbench
                                    Missing README.md for changed directory: src/components   (exit 1)
  without the src/components and src/components/workbench files
                                 -> Decision traceability check passed.  (nodes/, nodes/workflow/, edges/, packages/svelte-graph/src/)
npx vite build --outDir scratchpad/m9d/dist   -> ✓ built in 9.85s (no warnings)
```

The two remaining traceability failures belong to directories whose README this agent may not edit:
`src/components/README.md` is M9c's and does not exist yet. `src/components/workbench/README.md` is untracked
on the branch, so committing it with these files satisfies the check. Its Contents and Invariants text is
still accurate after request 5.

On HEAD, eslint already failed `PumaLibNode.svelte` (`34:7 svelte/prefer-writable-derived`), checked by
piping `git show HEAD:` into `eslint --stdin`. That error is now fixed.

Headless screenshots: built app served on port 8801, `CDP_PORT=9351`, fixtures `fixture.js` +
`fixture-graph.js` (+ a scratch `sdxl.js` that sets `pantograph.lastGraph` to `image-gen-sdxl` for the typed
cards). The step files are `scratchpad/m9d/steps-{a,b,c}.json`. Everything is in
`/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m9d/`:

- Before: `before-graph-edit.png` (M5 `graph-edit.png`), `before-graph-edit-typed.png` (M5 `graph-edit-current.png`).
  These show blue, purple, green and orange category borders and header tiles, "Idle" rows with dots, and minimap squares in category colours.
- After: `after-graph-edit.png` (ingest-docs-rag), `after-graph-edit-typed.png` (image-gen-sdxl: text input,
  Puma-Lib, LLM inference, image output) and `after-graph-edit-selected.png` (`sample` selected, properties panel filled).
  These show neutral cards with the glyph header, grey wires, neutral handles that fill when connected, and grey minimap nodes.

The server and the headless shell were stopped.

Not shown in a screenshot: the cards the fixture graphs do not contain (audio, point cloud, dependency
environment, masked text, linked input, agent tools, node group). They were checked by eslint, the Svelte
compile in `vite build`, and a grep for literal colours, `animate-*` and non-token Tailwind hues over the
write set. Running, streaming and error states were not driven, because the fixture has no execution events.

## Ledger entry text

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

## Issue rows

| ID | Severity | Evidence | Relationship | Owner / boundary | Disposition | Verification | Revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WR-M5-5 | medium | Node cards, edge, minimap node colours and package constants restyled; measured edge stroke `--pg-border-strong`, minimap fill `--pg-border-idle`, 0 `animate-*` on the canvas | Token-only canvas | M9d | fixed (app canvas); remainder split into WR-M9d-1..3 | `after-graph-edit*.png`, geometry log above | — |
| WR-M9d-1 | medium | `packages/svelte-graph/src/components/WorkflowGraph.css` is bundled into the app (imported by the package's `WorkflowGraph.svelte`) and its rules are unscoped: `.svelte-flow__handle.connecting{background:#60a5fa!important;box-shadow:0 0 10px …}` overrides the app's accent handle while connecting, `.svelte-flow__minimap{background-color:#171717}` paints the minimap ground (measured `rgb(23,23,23)` instead of `--pg-panel-sunken`), and it also sets blue glowing `.svelte-flow__edge-path` / `.svelte-flow__connection-line` (the app's `WorkflowGraph.css` happens to outrank the edge rule). `.group-editor-container .svelte-flow__minimap` is likewise `#171717` with a `#7c3a…` border | Token-only canvas, no glow | integration owner (`packages/svelte-graph/src/components/`) | defer: scope those rules under the package's own container class, or move them to `--xy-*` variables | minimap ground in `after-graph-edit.png` is near-black; computed minimap background `rgb(23, 23, 23)` | package CSS scoped |
| WR-M9d-2 | low | The package's own copies, `packages/svelte-graph/src/components/edges/ReconnectableEdge.svelte`, `components/nodes/*.svelte` and `packages/svelte-graph/src/workflowMiniMap.ts`, keep the old category and type colours. The app mounts its `src/components` copies, so the Graph tab is unaffected | Package consumers | integration owner (package) | defer | — | the package is consumed outside the app, or the app switches to the package components |
| WR-M9d-3 | low | `src/components/nodes/workflow/dependencyEnvironmentDisplay.ts:51-72` still returns Tailwind hue `className`s (emerald, amber, cyan, sky, violet, orange, red) for the dependency badge. `DependencyEnvironmentStatusPanel.svelte` no longer applies them; the badge is neutral and its label carries the state | Dead presentation data | integration owner (`.ts` outside the `.svelte` write set) | defer: drop `className` from the display model and its test | `grep -n className src/components/nodes/workflow/dependencyEnvironmentDisplay.ts` | next edit of the dependency display model |

## Requests to the integration owner

1. **Hide the Svelte Flow attribution** (allowed, MIT; see above): in `src/components/WorkflowGraphCanvas.svelte`
   add `proOptions={{ hideAttribution: true }}` to `<SvelteFlow …>` (and in `src/components/NodeGroupEditor.svelte`
   if that canvas should match).
2. **Drop the now-redundant mask rule** (M5 request 2, second half): with `WORKFLOW_GRAPH_MINIMAP_MASK_COLOR`
   equal to the token value, the `.workflow-graph-container .svelte-flow__minimap-mask { fill: … }` block in
   `src/components/WorkflowGraph.css` can go (keep its `stroke` lines or rely on
   `--xy-minimap-mask-stroke-color`). Its `filter: none` on edge paths can also go once WR-M9d-1 is fixed.
3. **WR-M9d-1**: scope or remove the unscoped package `WorkflowGraph.css` rules (minimap `#171717`,
   `.svelte-flow__handle.connecting` blue `!important` glow, blue edge and connection-line glow).
4. **Traceability at commit**: commit `src/components/workbench/README.md` (untracked) with the presenter change,
   and the M9c `src/components/README.md` with `workflowMiniMap.ts` / the `GraphSelector.svelte` deletion; the four
   M9d READMEs cover the other directories.
5. **Ledger and issues**: the M5 report's request 4 (`GraphSelector`) and request 5 (dead exports) are done;
   WR-M5-5 can be marked fixed with WR-M9d-1..3 as its remainder.
