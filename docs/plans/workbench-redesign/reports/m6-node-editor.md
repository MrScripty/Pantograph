# M6 Node Editor: report

Agent: Opus 5.5 page agent, 2026-11-03. Board: `docs/design/mockups/boards/NodeEditor.dc.html`.
Status: implemented and verified, not committed.

## Changed files

- `src/components/workbench/NodeLabPage.svelte`: rewritten to the board's
  three-column layout (list, definition form, canvas preview and checks).
- `src/components/workbench/nodeLabPresenters.ts`: rewritten as a pure
  projection of `NodeDefinition[]`, plus the authoring notice and the reason
  for each disabled control.
- `src/components/workbench/nodeLabPresenters.test.ts`: 12 tests (replaces 1).

No other file was edited. The walkthrough fixture is in scratch (see Evidence).

## What the page does

- **Data authority.** The page reads the `nodeDefinitions` store from
  `src/stores/workflowStore.ts`, which is the store the Graph palette
  (`NodePalette.svelte`, through `nodeDefinitionsByCategory`) lists. The graph
  session fills it at startup (`App.svelte` → `loadLastGraph` →
  `backend.getNodeDefinitions()` → `get_node_definitions`), and Graph fills it
  again when it mounts. The page adds no backend call, ledger read, timer or
  listener. Its only subscription is the `$nodeDefinitions` auto-subscription,
  which Svelte opens on mount and closes on destroy.
  `src/registry/pantographNodeTypes.ts` maps the same definitions to Svelte
  components; it holds no definition data of its own.
- **Notice.** The page opens with an in-place notice, not a banner: "Node
  authoring is not available in this build. You can read the built-in node
  definitions, but the Authoring API, the Local Agent and Runtime Publishing are
  not included, so creating, saving, publishing and trying nodes are off. They
  would be enabled in Settings; this build has no setting for them yet."
  Settings is named but not linked, because Settings has no such controls and a
  link would lead nowhere.
- **Left panel.**
  - A search field filters by label, type id, category and description.
  - `New node` is disabled.
  - One collapsible "Built-in" group, whose header shows "3 of 17" while a
    search is active. Rows show the label, the mono type id, and
    "Category, N inputs, M outputs".
  - Rows are sorted in palette category order (input, processing, tool,
    output, control, then any unknown category), then by label. The selected
    row has `aria-current`.
  - "Your nodes" is omitted.
- **Centre panel.**
  - Header: serif title and mono type id, with `Save draft` and
    `Publish to runtime` disabled.
  - Read-only fields: Label, Category, Execution (`execution_mode`),
    I/O binding (`io_binding_origin`) and Description.
  - Inputs table: Name, Type, Default, Required, plus a disabled remove
    button. Outputs table: Name, Type, plus a disabled remove button.
  - Under a port's name: its id when it reads differently from the label,
    and its description when it has one. `Add input` and `Add output` are
    disabled.
  - The Implementation block reads "built-in, source not editable".
- **Right panel.**
  - "On the canvas" preview card in the board's `.gnode` / BaseNode style: a
    header with the idle glyph, the label and the mono type, then inputs on
    the left edge and outputs on the right edge, with port dots.
  - Checks: "Built-in node, always valid".
  - Try it: one sentence and a disabled `Try with sample inputs`.
- **Disabled controls.**
  - The six action buttons (`New node`, `Save draft`, `Publish to runtime`,
    `Add input`, `Add output`, `Try with sample inputs`) use
    `aria-disabled="true"` with no handler. As the spec asks, they stay
    focusable, so keyboard and screen-reader users reach their `title`.
  - The form fields and per-port remove buttons (112 on LLM Inference) are
    natively `disabled`, so they are not all tab stops. Each has a `title`
    giving the reason.
  - Disabled fields keep readable text (soft text on the panel-head
    background) because they show the content of the definition.
- **Empty state.** When the store is empty (plain browser, or engine
  unreachable at startup), the left panel says "No built-in node definitions
  are loaded." The centre and right panels are replaced by the shared
  `EmptyState`: "No node definitions are loaded yet. Pantograph reads the
  built-in nodes from the engine when the app starts and again when Graph
  opens. They appear here once the engine answers." The dimension line reads
  "No node definitions loaded".
- **Selection.** Selection is resolved against all definitions, not the
  filtered list. A search that hides the selected row keeps the detail, and a
  type the engine stops reporting falls back to the first row. This is tested.

### Omitted because `NodeDefinition` does not carry it

`NodeDefinition` has only `node_type`, `category`, `label`, `description`,
`io_binding_origin`, `inputs`, `outputs` and `execution_mode`. Ports have `id`,
`label`, `data_type`, `required`, `multiple`, `description`, `default_value`
and `constraints`. The following mockup items are therefore left out, not
filled with invented values:

- the version suffix (`v2.4.0`) in the list and the header;
- the `Runtime` select (runtime binding) and the `Device` select;
- the outputs `Retained` column;
- the right-hand time column in list rows (the board's "built-in");
- the word "live" on the preview label (nothing edits it, so it reads
  "preview").

Execution mode and I/O binding are shown in place of Runtime and Device,
because they are real fields of the definition.

## Verification (verbatim results, repo root)

- `npm run typecheck`: pass, no errors. An earlier run during the work
  failed only in other agents' in-progress files
  (`diagnosticsPagePresenters.test.ts`, `networkPagePresenters.test.ts`,
  `schedulerPagePresenters.test.ts`).
- `npm run test:frontend`: `ℹ tests 660`, `ℹ pass 660`, `ℹ fail 0`. This
  includes the 12 node lab tests (`node --experimental-strip-types --test
  src/components/workbench/nodeLabPresenters.test.ts`: tests 12, pass 12).
- `npm run lint:a11y`: "svelte accessibility gate failed". It lists only the
  pre-existing WR-007 items `IoInspectorPage.svelte:1024` and
  `RunGraphSnapshot.svelte:156`; there is no finding in M6 files.
- `npm run lint:critical`: "critical anti-pattern gate failed". It lists only
  the pre-existing WR-007 item `IoInspectorPage.svelte:508:20
  [no-append-remove-child]`; there is no finding in M6 files.
- `npx eslint src/components/workbench/NodeLabPage.svelte
  src/components/workbench/nodeLabPresenters.ts
  src/components/workbench/nodeLabPresenters.test.ts --max-warnings 0`: pass
  (no output).
- `TRACEABILITY_BASE_REF=codex/pantograph-rpc-workflow-2026-09-26 ./scripts/check-decision-traceability.sh`:
  "No changed files detected for decision traceability check.", exit 0. This
  proves nothing about uncommitted work (WR-003).
  `src/components/workbench/README.md` already exists and covers this
  directory.
- `npx vite build --outDir /tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m6/dist-tree`:
  "✓ built in 9.54s" (working tree, final run).
  - Earlier runs failed outside M6: `DiagnosticsPage.svelte` imported
    `filterDiagnosticsTimelineEvents`, which M2's in-progress
    `diagnosticsPagePresenters.ts` did not export yet.
  - The screenshot build was therefore made from a scratch snapshot of the
    tree with only `DiagnosticsPage.svelte` and `diagnosticsPagePresenters.ts`
    taken from `HEAD`: `.../scratchpad/m6/dist`, "✓ built in 9.41s". The M6
    files in it are identical to the working tree.
- Headless walkthrough of the built app at 1440x900, run as
  `CDP_PORT=9346 node docs/plans/workbench-redesign/reports/tools/cdp-init.cjs .../scratchpad/m6/steps.json`
  against `python3 -m http.server 8796` (stopped afterwards).
  - Fixture: `reports/tools/fixture.js` plus a scratch
    `fixture-node-editor.js`. It answers `get_node_definitions` with the
    frontend's `MOCK_NODE_DEFINITIONS` (`src/services/workflow/mocks.ts`,
    17 definitions), not the Rust registry. The Rust registry is checked
    only in A5.
  - Logged results:
    - `rows: 17`; `initially selected row: "Boolean Input"`; `page does not scroll: true`
    - `selected row, detail title: ["LLM Inference","LLM Inference"]`
    - aria-disabled buttons, each with its title: New node, Save draft,
      Publish to runtime, Add input, Add output, Try with sample inputs
    - `natively disabled fields (count, all titled, reasons): [112,true,[edit_field reason, remove_port reason]]`
    - `enabled authoring controls in Definition: 0`
    - `implementation | checks: "built-in, source not editable | Built-in node, always valid"`
    - notice text as quoted above
    - `search 'output' (rows, header, detail kept): [["Component Preview","Text Output","Vector Output"],"Built-in 3 of 17 ▾","LLM Inference"]`
    - `no match: "No built-in node matches “zebra”."`
    - `collapsed (aria-expanded, hidden): ["false",true]`; `expanded (aria-expanded, hidden): ["true",false]`
    - `after clicking New node and Publish (nothing changes): ["LLM Inference",17]`

## Screenshots

- `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m6/node-editor-unavailable.png`
  (plain browser, engine unreachable: notice plus the designed empty state)
- `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m6/node-editor.png`
  (fixture, first definition selected)
- `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m6/node-editor-llm.png`
  (LLM Inference: 17 inputs and 13 outputs; the form and preview scroll inside
  their panels)
- `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m6/node-editor-search.png`
  (search "output", 3 of 17, detail kept)

## Ledger entry text

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

## Issue rows

The IDs are provisional; renumber them when merging.

| ID | Severity | Evidence | Relationship | Owner / boundary | Disposition | Verification | Revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WR-M6-1 | low | No node authoring backend: no Authoring API, Local Agent or Runtime Publishing command, no definition store for user nodes, and no Settings entry to enable them (`SettingsPage.svelte` has none) | Node Editor actions (A2) | backend node authoring; Settings (M9) | defer: the page shows the notice, renders `New node`, `Save draft`, `Publish to runtime`, `Add input`, `Add output` and `Try with sample inputs` as `aria-disabled` with reasons, makes fields and remove buttons natively disabled, and omits "Your nodes" | `nodeLabPresenters.test.ts` (notice and reasons); walkthrough "enabled authoring controls in Definition: 0" | an authoring API or a user definition store lands |
| WR-M6-2 | low | `NodeDefinition` has no version, runtime binding, device or per-output retention (`src/services/workflow/types.ts`); the board shows `v2.4.0`, Runtime and Device selects, and a Retained column | Node Editor fidelity (A2) | backend node registry (`get_node_definitions`) | defer: the page omits these and shows `execution_mode` and `io_binding_origin` instead | test "a definition projects to read-only form rows without invented fields" | the registry exposes version, runtime or retention per definition |
| WR-M6-3 | low | The `nodeDefinitions` store has no loading or failure state. `createSessionStores.loadLastGraph` either sets the definitions or throws, and `App.svelte` does not catch the error, so the page cannot tell "not loaded yet" from "engine failed" from "no definitions" | Typed unavailable outcome on Node Editor (frontend profile, Typed Outcomes) | graph session stores (`packages/svelte-graph` `createSessionStores`, `src/stores/graphSessionStore.ts`) | defer: one neutral empty state that says where definitions come from; the shell's engine line and dimming cover the unreachable case | screenshot `node-editor-unavailable.png` | the session store exposes a definitions load status or error |
| WR-M6-4 | low | The spec asks for `aria-disabled` on authoring controls; M6 uses `aria-disabled` for the six action buttons but native `disabled` for the fields and per-port remove buttons (112 on LLM Inference), to avoid one tab stop per field | Accessibility contract | MrScripty | defer: confirm or revert | walkthrough lists of aria-disabled and disabled controls | owner review of M6 |
| WR-M6-5 | low | The walkthrough fixture serves `MOCK_NODE_DEFINITIONS` (17 entries) from `src/services/workflow/mocks.ts`, not the Rust registry, so the real list, counts and port shapes are unchecked in a browser | A2 and A5 evidence | integration owner | verify in A5 on the desktop app | Node Editor in `npm run dev:desktop` lists the engine's definitions | A5 |

## Requests for the integration owner (shared parts, not edited)

- `ListRow`'s mono `version` wraps mid-word when it is long. For example
  "component-preview" breaks across two lines in `node-editor-search.png`.
  Consider `white-space: nowrap` with ellipsis on `.pg-ver` inside `.pg-item`.
- Optional: move `reports/tools/fixture-node-editor.js` into `reports/tools/`
  for reproducibility. The tools README invites per-page fixtures, but the
  folder is outside the M6 write set. The scratch copy is at
  `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m6/tools/fixture-node-editor.js`,
  and the steps are at `.../scratchpad/m6/steps.json`.
