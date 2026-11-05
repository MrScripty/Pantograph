# M6b Node Editor: report

Agent: Opus 5.5 page agent, 2026-11-04. Board:
`docs/design/mockups/boards/NodeEditor.dc.html`. Resolves WR-066.

Status: implemented, verified and committed. There are two commits:

- Merge commit `ec332b170`: merges PR #95 (`origin/feat/nodelab-trusted-catalog`) into this branch.
- This commit: the M6b page rebuild.

## Step 1: merge of PR #95

`git merge --no-ff origin/feat/nodelab-trusted-catalog` brought in the six
#95 commits on top of `46aad5841`. Three files conflicted. Both sides had
rewritten each of them.

| File | Resolution |
| --- | --- |
| `src/components/workbench/NodeLabPage.svelte` | Took #95's version. It drives the real authoring flow through `nodeLabController.ts` and `customNodeAuthoring.ts`. The M6 page only showed authoring as disabled controls. |
| `src/components/workbench/nodeLabPresenters.ts` | Took #95's version, which backs its page. |
| `src/components/workbench/nodeLabPresenters.test.ts` | Took #95's version, so its test runs against its presenters. |

These auto-merged with no conflict:
`docs/plans/domain-architecture-and-multimodal/{plan,execution-ledger,issues}.md`,
`docs/custom-nodes.md`, `src-tauri/src/app_setup.rs` and the other #95
files. `docs/plans/README.md` was not touched. No ledger entry was lost:
#95's ledger additions are appended text and auto-merged.

Results on the merge commit, before M6b:

- `npm run typecheck`: passed with no errors.
- `npm run test:frontend`: `ℹ tests 871`, `ℹ pass 871`, `ℹ fail 0`. This
  includes #95's `nodeLabPresenters.test.ts` and `customNodeAuthoring.test.ts`.

## Step 2: changed files (this commit)

- `src/components/workbench/NodeLabPage.svelte`: rebuilt in the redesign layout over #95's controller.
- `src/components/workbench/nodeLabPresenters.ts`: #95's two presenters unchanged, plus new pure projections.
- `src/components/workbench/nodeLabPresenters.test.ts`: #95's test unchanged, plus 9 tests for the new projections (10 in all).
- `docs/plans/workbench-redesign/reports/tools/fixture-node-authoring.js`: new walkthrough fixture.
- `docs/plans/workbench-redesign/reports/m6b-node-editor.md`: this report.
- `docs/plans/workbench-redesign/issues.md`: WR-066 disposition changed to "resolved by M6b".
- `docs/plans/workbench-redesign/plan.md`: the Blockers line.

`nodeLabController.ts` and `customNodeAuthoring.ts` are untouched. The new
layout did not need a controller change.

## How #95's behaviour maps to the board

The page adds only view state: the search query, the selected row, whether
the group is expanded, and which control acted last. Every service call
still goes through `NodeLabController`, as in #95. The page adds no backend
call, timer, listener or subscription. The controller is still created on
mount and disposed on destroy, as in #95.

| #95 element | Board region | Rendering |
| --- | --- | --- |
| Catalog and draft state | Dimension line | `nodeLabSummaryLine`, for example "1 admitted node · draft of 3 nodes, contract valid, saved". Loading, failed and empty catalogs read differently. |
| `nodeLabTrustMessage()` | In-place notice under the dimension line (M6's pattern) | Idle glyph and the sentence, unchanged. This is where the page text "NodeLab" appears. |
| "Admitted nodes" heading, `Refresh catalog` | Left panel top | `pg-btn`, disabled while busy, as in #95. |
| Admitted node entries (label, description, "N string inputs · M string outputs · Trusted in-process") | Left panel list | `GroupHeader` "Admitted on this host" with a count, which shows "2 of 7" while a search is active. Each node is a `ListRow`: glyph, name, mono node type, ports on the first sub-line, description on the second. A full title gives ports, trust and description. The node the open draft pins gets the filled-muted glyph and "in draft" on the right. |
| Search | Left panel top | Search is new. It filters by label, type and description, and appears only for 6 or more admitted nodes (see WR-M6b-1). |
| `Create {label} draft` | Left panel footer | One primary button for the selected node. #95 had one button per entry. The text is still exactly `Create {label} draft` (see WR-M6b-3). |
| Empty catalog text | Centre panel `EmptyState` | Question "This host has no admitted custom nodes." The sentence keeps #95's wording ("A host maintainer must register a trusted implementation before creating a new draft.") and adds "You can still reopen a saved workflow above." The left list says "None admitted on this host." No Create button is rendered. |
| `Saved workflow path`, `Reopen saved workflow` | Centre panel top | `pg-field` and `pg-btn`. The button is disabled while busy or while the path is blank, as in #95. |
| Workflow draft: counts, the identity sentence, `Saved: <code>path</code>` | Centre header | Serif "Workflow draft". The mono line gives counts and the pinned node (label and type). Then the sentence "Saving preserves the selected implementation identity. Run saved workflows from Scheduler.", then the saved path. |
| Text-input sources, `Apply input` | Centre "Inputs" section | One form per `text-input` node. Each has a labelled `textarea name="text"` and `Apply input`. It reads `FormData` and calls `controller.update(node, text)`, as in #95. |
| (new, read-only) | Centre "Outputs" section | The draft's `text-output` labels with a not-reached glyph, and "Filled when the saved workflow runs from Scheduler." This is a projection of the draft graph only. |
| `Workflow name`, `Save workflow` | Centre panel footer | The name field and the primary button. |
| Busy and notice status (`role="status"`) | Centre panel footer, right | "Working…", "Input saved in draft" or "Workflow saved". The element is always mounted. |
| Graph diagnostics or "Graph contract valid" | Right "Checks" section (contract validity) | Glyph and text. A blocking diagnostic gets the danger diamond and the "Blocks execution: " prefix. A non-blocking one gets the warn diamond. The list is labelled "Graph diagnostics", as in #95. |
| Catalog entry (canvas look) | Right "On the canvas" | The preview card for the selected node: label, mono type, input and output port dots. The description is under the card. |
| Implementation identity `<details>` | Right "Implementation identity" | A native `<details open>` with a `KeyValue`. The rows are node type, contract version, contract digest, implementation, environment (all mono), adapter, and trust "Trusted in-process". #95 showed the same six identity fields as a JSON dump. |
| Pending cleanup alert, `Retry session cleanup` | Right "Sessions" section | A muted sentence in `role="alert"` with #95's wording, and the button. With nothing pending it reads "No earlier NodeLab sessions are waiting to close." |
| `buildNodeLabStatusRows()` | Right "Authoring on this host" | `KeyValue` rows: Graph authoring, Host-admitted nodes; Local Agent, Unavailable; Runtime Publishing, Unsupported. |
| `view.error` (`role="alert"`) | In place, in the panel of the control that raised it | `nodeLabErrorRegion`. Catalog load and Create errors go to the top of the left list. Reopen, Apply and Save errors go under the reopen row. Cleanup retry errors go in Sessions. Each is a muted line with a small danger diamond, not a banner. The alert's text is the bare message, as in #95. |

The section keeps `aria-label="NodeLab authoring"` and `aria-busy`. #95 had
no element ids or `data-testid`s, so there were none to keep. The visible
text that `scripts/check-nodelab-browser.mjs` relies on is all kept,
including the first `input`, `textarea`, `code` and `[role="alert"]` in the
DOM.

Layout: three `Panel`s in shrinkable column wrappers. At desktop size the
list is 300 px, the centre fills, and the right is 320 px. Narrower than
that, the columns shrink together (minimums 200, 240 and 220 px), and the
reopen row and the footer wrap instead of clipping. The page never scrolls;
the panels scroll inside.

## Omitted because #95 has no counterpart

No disabled stubs are rendered for these:

- `New node`, `Save draft` and `Publish to runtime`. #95 creates workflows
  from host-admitted nodes. It cannot author, install or publish node code
  (`nodeLabTrustMessage`, `docs/custom-nodes.md`).
- `Try with sample inputs`, the "Try it" section. #95 has no try or
  sample-run command; saved workflows run from Scheduler.
- The editable definition form: Label, Category, Runtime, Device,
  Description, the port tables with Default and Retained, `Add input`,
  `Add output` and the remove buttons. Contracts are fixed by the host. The
  page shows them read-only in the preview card and the identity.
- The "Built-in" and "Your nodes" groups, the version suffix, and the word
  "live" on the preview label. There is one "Admitted on this host" group,
  and the label reads "preview".

## Verification (verbatim, repo root, working tree before commit)

- `npm run typecheck`:
  `> tsc --project tsconfig.json --noEmit --pretty false` (no errors)
- `npm run test:frontend`: `ℹ tests 880`, `ℹ pass 880`, `ℹ fail 0`
- `node --experimental-strip-types --test src/components/workbench/nodeLabPresenters.test.ts src/services/workflow/customNodeAuthoring.test.ts`:
  `ℹ tests 19`, `ℹ pass 19`, `ℹ fail 0`
- `npm run lint:a11y`: `svelte accessibility gate passed`
- `npm run lint:critical`: `critical anti-pattern gate passed`
- `npx eslint src/components/workbench/NodeLabPage.svelte src/components/workbench/nodeLabPresenters.ts src/components/workbench/nodeLabPresenters.test.ts docs/plans/workbench-redesign/reports/tools/fixture-node-authoring.js --max-warnings 0`:
  no output, exit 0
- `npx vite build --outDir <scratch>/m6b/dist`: `✓ built in 10.12s`
- `TRACEABILITY_MODE=range TRACEABILITY_BASE_REF=origin/main TRACEABILITY_HEAD_REF=HEAD ./scripts/check-decision-traceability.sh`:
  - Run on the merge commit: `Decision traceability passed: 333 changed path(s), 0 mapped impact(s). Semantic contract review remains required.`
  - The run after this commit is recorded in the commit's hand-off.

### `scripts/check-nodelab-browser.mjs`

It could not run as #95 intends here, for two reasons:

- It needs `NODELAB_HOST_BINARY`, the built
  `crates/pantograph-workflow-service/examples/nodelab_host.rs`. Only
  check-build artefacts exist (`target/debug/examples/libnodelab_host-*.rmeta`,
  `.d`), with no executable, and this task may not run cargo.
- There is no `chromium` on PATH.

It was run instead against a scratch Node stand-in for the stdio host,
`<scratch>/m6b/stubhost/nodelab-stub-host.mjs`. The stand-in speaks the same
JSON-line protocol and builds drafts the way `create_custom_node_draft`
does. The browser was `CHROMIUM_BINARY=~/.cache/ms-playwright/chromium_headless_shell-1246/.../chrome-headless-shell`.

- Result: exit 0, `"ok": true`, with every claim listed: real empty catalog,
  transport failure distinct from empty, keyboard creation and visible
  focus, pointer edit/save/reopen, exact pin preserved, replaced session
  closed, missing implementation visible and blocking, failed catalog
  refresh hides stale choices and preserves draft.
- This proves the rebuilt page meets the script's DOM and interaction
  contract. It does not prove anything about the Rust host, so the
  "through Rust filesystem" wording does not hold for this run.
- Negative control: an earlier layout clipped the centre panel at the
  script's default 800×600 window. That run failed with
  `Missing UI text: Workflow saved`, because the pointer click on
  `Save workflow` landed on an overlapping status line. Making the columns
  shrinkable fixed it.

### Headless walkthrough

Built app, 1440×900, `python3 -m http.server 8806` (stopped afterwards),
`CDP_PORT=9356 node docs/plans/workbench-redesign/reports/tools/cdp-init.cjs <scratch>/m6b/steps.json`.
The fixtures are `reports/tools/fixture.js` and then `fixture-node-authoring.js`.

- `no page scroll, dimension, create buttons: [true,"1 admitted node · no draft open",["Create Host uppercase draft"]]`
- `status after apply: "Input saved in draft"`
- `after save (status, code, dimension, checks, first input label): ["Workflow saved",".pantograph/workflows/custom-node-workflow.json","1 admitted node · draft of 3 nodes, contract valid, saved","Graph contract valid","Saved workflow path"]`
- `after reopen (textarea, saved path, dimension): ["Saved Mixed case",".pantograph/workflows/custom-node-workflow.json","1 admitted node · draft of 3 nodes, contract valid, saved"]`
- `search text (rows, header, selected): [["Redact emails","Word count"],"Admitted on this host 2 of 7 ▾","Redact emails"]`
- `empty (text, create button, dimension): [true,false,"No admitted nodes on this host · no draft open"]`
- `error (alert text, region, empty text shown, dimension): ["The workflow host did not answer the catalog request.","Admitted nodes",false,"Admitted nodes unavailable · no draft open"]`
- Cleanup run (`steps-cleanup.json`, `?fixtureCatalog=many&fixtureClose=fail`):
  `cleanup (alerts by panel, dimension, retry shown): [[["Admitted nodes","Draft replacement and cleanup failed; previous draft retained"],["Preview and checks","1 previous NodeLab sessions could not be closed. Close them before creating or reopening a draft."]],"7 admitted nodes · draft of 3 nodes, contract valid",true]`

## Screenshots

All are under
`/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m6b/`:

- `node-editor-catalog.png`: one admitted node, no draft.
- `node-editor-draft.png`: draft created, input applied, workflow saved.
- `node-editor-many.png`: seven nodes, the fourth picked and drafted.
- `node-editor-search.png`: search "text", 2 of 7, selection kept.
- `node-editor-empty.png`: empty catalog and the `EmptyState`.
- `node-editor-error.png`: catalog transport failure, error in place.
- `node-editor-cleanup.png`: failed close, the Sessions alert and retry.

## Ledger entry text

- M6b Node Editor (resolves WR-066).
  - Merged PR #95 (`origin/feat/nodelab-trusted-catalog`) as `ec332b170`.
    The three Node Editor files conflicted and were resolved to #95's
    versions. typecheck passed; test:frontend 871/871.
  - The page was then rebuilt over `NodeLabController` in the board's three
    panels:
    - left: admitted nodes with search (6 or more nodes), glyphs, and
      Create for the selected node;
    - centre: reopen, the draft's inputs with Apply input, outputs, and
      Save workflow;
    - right: canvas preview, contract checks, implementation identity,
      session cleanup, and authoring on this host.
  - The dimension line summarises catalog and draft. Errors show in place
    in the panel of the control that raised them.
  - The controller and service are unchanged. Board items with no #95
    counterpart (New node, Save draft, Publish, Try, editable definition
    fields) are omitted, not stubbed.
  - Gates: typecheck pass; test:frontend 880/880; lint:a11y and
    lint:critical pass; eslint pass; vite build pass; traceability range
    pass.
  - `check-nodelab-browser.mjs` passes against a scratch stub host. The
    real Rust host is unbuilt and cargo was out of scope.

## Issue rows

The IDs are provisional.

| ID | Severity | Evidence | Relationship | Owner / boundary | Disposition | Verification | Revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WR-M6b-1 | low | `scripts/check-nodelab-browser.mjs` addresses controls by DOM position (`document.querySelector('input')`, first `textarea`, first `code`, first `[role="alert"]`). A search field above the reopen field would become the first `input` | Node Editor search (A2) | NodeLab check owner (`scripts/`) | accept: the search field appears only for 6 or more admitted nodes, so the reopen path is the first input for the check's one-node host. Better: the script selects by label | the check passes (stub run above) | the check selects by accessible name, or a host with 6 or more nodes is qualified |
| WR-M6b-2 | medium | `check-nodelab-browser.mjs` was not run against the real `nodelab_host`: there is no built binary and cargo was out of scope. It passed against a scratch stub host with the headless shell | #95 browser qualification on this branch | integration owner | verify: build `nodelab_host` and rerun with the real binary | `NODELAB_HOST_BINARY=target/debug/examples/nodelab_host node scripts/check-nodelab-browser.mjs` prints `"ok": true` | before merging this branch to main |
| WR-M6b-3 | low | #95 rendered one `Create {label} draft` button per catalog entry. M6b renders one, in the list footer, for the selected row, so another node takes select then Create | Interaction parity with #95 | MrScripty | defer: confirm or revert | walkthrough "create buttons" and the cleanup run | owner review |
| WR-M6b-4 | low | WR-028 to WR-032 describe M6's premise: no authoring backend, `nodeDefinitions` store, mock definitions. The page no longer reads `nodeDefinitions` | Issue hygiene | integration owner | re-disposition: WR-028 and WR-030 to WR-032 superseded by M6b. WR-029 (no runtime, device or retention fields) no longer applies, because the board's editable definition form is omitted | n/a | n/a |
| WR-M6b-5 | low | The spec section "Node Editor" still says "Node authoring has no backend in this build" and asks for disabled authoring controls and a Settings notice | Spec accuracy | integration owner (spec) | fix: rewrite the section to the #95 model (admitted-node catalog, workflow draft, no code publishing) | n/a | n/a |

## Requests for the integration owner (outside the write set, not edited)

- Add `fixture-node-authoring.js` to the page fixture list in
  `reports/tools/README.md`.
- Optional shared-part change: in `ListRow`, when the `time` column holds
  text, `.pg-ver` on the name line wraps under the name. The cleanup
  screenshot shows this on the "in draft" row. Consider letting `.pg-ver`
  shrink with an ellipsis on the same line.
- Update the spec section and WR-028 to WR-032 as above.
