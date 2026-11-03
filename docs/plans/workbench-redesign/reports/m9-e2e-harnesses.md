# M9 follow-up: I/O Inspector e2e harnesses (WR-023 / WR-M3-2)

Agent: integration follow-up (Opus 5.5), 2026-11-03. Branch `feat/workbench-redesign`, uncommitted.
Scope: the two WebKit e2e harnesses and this report. `IoInspectorPage.svelte` was not changed.

**The harnesses were not executed in this session.** Everything below is static checks only. The
acceptance claim for both harnesses (A5 evidence for artifact reads) stays open until the owner runs
the commands in "Commands to run on the desktop".

## Changed files

| File | Change |
| --- | --- |
| `tests/e2e/io-inspector-image-preview/run.mjs` | Selectors and flow ported to the M3 layout. Each card assertion now selects the artifact's tree row first (`selectArtifact`), the stream button and footer Refresh use the new selectors, error assertions read the card's `io-artifact-access-error` region, and "gone" checks cover both row and card. Added a `/* global */` header, and the empty `catch {}` now holds a comment (`no-empty`). |
| `tests/e2e/workflow-editor-image-generation/workflow-editor-image-generation.e2e.mjs` | After submit, `selectImageArtifactRow` clicks tree rows in order until the single card shows `data-artifact-media-family="image"` (polling up to 300 s while the run produces the image). Then it checks that row has `aria-current="true"`. Read button, preview and access-error lookups are scoped to that card's artifact id. Added a `/* global */` header. |
| `tests/e2e/workflow-editor-image-generation/wdio.conf.mjs` | Only a `/* global process, console */` header, so the directory lints clean. No behaviour change. |
| `tests/e2e/io-inspector-image-preview/fixture.ts` | Unchanged. It still mounts `IoInspectorPage` with a selected run, and its 8 output artifacts on node `image-output` become 8 tree rows. |

## Selector mapping, old to new

| Purpose | Old selector / flow | New selector / flow |
| --- | --- | --- |
| Pick an artifact | none (every artifact had its own card, all visible) | click `[data-testid="io-artifact-row"][data-artifact-id="<id>"]`, then wait for `aria-current="true"` on that row |
| Artifact card | `[data-testid="io-artifact-card"][data-artifact-id="<id>"]` | unchanged; only the selected artifact's card exists |
| Image card in the desktop smoke | first `[data-testid="io-artifact-card"][data-artifact-media-family="image"]` | the row whose card has `data-artifact-media-family="image"`, then `[data-testid="io-artifact-card"][data-artifact-id="<id>"][data-artifact-media-family="image"]` |
| Read body | `[data-testid="io-artifact-read-button"]` in the card | unchanged |
| Read stream | `button[aria-label^="Read artifact stream"]` | `[data-testid="io-artifact-stream-button"]` |
| Image preview | `[data-testid="io-artifact-image-preview"]` (with `data-preview-complete`, `data-preview-byte-length`) | unchanged |
| Read/decode error text | substring of the whole card's text | substring of the card's `[data-testid="io-artifact-access-error"]` text |
| Incomplete image error | card text contains `complete` | access error contains `complete image body is required` (from `assertCompleteIoArtifactImageRead`) |
| Bounded text body | `<card> pre` | unchanged (`pre.pg-code`); "text truncated" note is still inside the card |
| Page Refresh | `[data-testid="io-inspector-page"] button:nth-of-type(2)` | `[data-testid="io-inspector-page"] section[aria-label="Nodes and ports"] .pg-pfoot > button:first-child` (footer Refresh; waits for it to be enabled, because the button reads "Loading" and is disabled while loading) |
| Artifact removed after refresh | card gone | card **and** row gone |

The page gives Refresh no test id, so the footer selector depends on the panel's `aria-label` and the
`.pg-pfoot` class. A `data-testid="io-inspector-refresh"` on that button would be sturdier. That is a
page change, so it is left as a request.

## Flow notes (why each assertion keeps its intent)

- **Clicking the selected row does nothing.** `selectEntry` returns early when the row is already
  selected, so it does not invalidate pending reads. The detached/current image identity checks rely on
  this: they click `valid-image` again (it is the default selection, the first image output) while a
  read is pending.
- **Clicking a different row clears errors.** Selecting another row calls
  `invalidatePendingArtifactAccess()`, which clears access errors and in-flight reads. Previews are kept
  per artifact. Every error assertion still runs right after its own read, before any other row is
  selected.
- **Stale-read check is slightly stronger.** It now waits until `payload-stale-image` actually reaches
  `readArtifactBody` before removing the artifact and refreshing. Without that wait, the refresh could
  overtake the descriptor check, and the "stale read creates no URL" assertion would pass without ever
  being tested.
- **Selection after the stale artifact is removed.** The page falls back to the default selection
  (`valid-image`). The next step selects `valid-image` explicitly anyway.
- **Selection after an inspection failure.** The tree empties and the card and rows disappear. The
  harness checks both, and also that the prior preview URL was revoked. The second Refresh rebuilds the
  tree, and the harness then selects `unmount-pending-image` by its row.
- **Assertions kept unchanged:**
  - full-body image read request `{ artifact_id: 'payload-valid-image' }`;
  - 320×320 decode;
  - URL create/revoke counts;
  - incomplete image read and stream rejected before any URL is created;
  - stream request bounded to `0..65 536`;
  - text preview bounded to 32 000 characters, with a `0..65 536` read request;
  - no URL from a stale read or from a read completing after unmount;
  - every live URL revoked on unmount;
  - desktop smoke: decoded image with `complete`, `data-preview-complete="true"`, positive byte length
    and dimensions, and no access error.

## Static checks (verbatim)

`tests/e2e/**` is not in `tsconfig.json`'s `include`, so `npm run typecheck` does not check these files.
It is run anyway to confirm the tree still compiles. On the unmodified harness files, the same ESLint
command reported 92 errors: `no-undef` for Node, WDIO and browser globals, because `eslint.config.mjs` has
no block for `tests/e2e`, plus one `no-empty`. The in-file `/* global */` headers clear them without
touching the shared config.

```
$ npx eslint tests/e2e/io-inspector-image-preview/run.mjs tests/e2e/io-inspector-image-preview/fixture.ts tests/e2e/workflow-editor-image-generation/workflow-editor-image-generation.e2e.mjs tests/e2e/workflow-editor-image-generation/wdio.conf.mjs --max-warnings 0
(no output)
eslint exit 0

$ node --check tests/e2e/io-inspector-image-preview/run.mjs
node --check tests/e2e/io-inspector-image-preview/run.mjs exit 0
$ node --check tests/e2e/workflow-editor-image-generation/workflow-editor-image-generation.e2e.mjs
node --check tests/e2e/workflow-editor-image-generation/workflow-editor-image-generation.e2e.mjs exit 0
$ node --check tests/e2e/workflow-editor-image-generation/wdio.conf.mjs
node --check tests/e2e/workflow-editor-image-generation/wdio.conf.mjs exit 0

$ npm run typecheck
> Pantograph@0.0.0 typecheck
> tsc --project tsconfig.json --noEmit --pretty false

typecheck exit 0
```

## Not executed

Neither harness was run. The brief for this follow-up ruled it out. Both harnesses drive a real WebKit
window: `run.mjs` opens MiniBrowser through WebKitWebDriver, and the smoke builds and launches the Tauri
app through `tauri-driver`.

A probe of this machine found the prerequisites present: `/usr/bin/WebKitWebDriver`, the MiniBrowser
binary at the path `run.mjs` hard-codes, `tauri-driver` on PATH, and an X display at `DISPLAY=:0.0` that
`xdpyinfo` can reach. The fixture harness can therefore probably run on this desktop as it is.

## Commands to run on the desktop (A5)

From the repo root
(`/media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pantograph`), with the desktop session
active.

1. **Fixture harness.** Needs no backend, model or Tauri build; about a minute.

   ```
   DISPLAY=:0.0 node tests/e2e/io-inspector-image-preview/run.mjs
   ```

   Pass looks like the final line
   `PASS real WebKitGTK: PNG <n> bytes decoded 320x320; full-read, malformed/partial/failure, text bound, stale read, and URL lifecycle checks passed`
   and exit code 0.

2. **Desktop image-generation smoke.** It builds `target/debug/pantograph` with `backend-pytorch`, then
   runs WDIO through `tauri-driver`.

   ```
   PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID=<pumas model id> \
   PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID=<pumas artifact id> \
   PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID=<saved workflow id under .pantograph/workflows/> \
   PANTOGRAPH_PYTHON_EXECUTABLE=<absolute path to the worker python> \
   npm run test:workflow-editor-image-gui
   ```

   `npm run test:workflow-editor-image-gui` is the same as `./scripts/check-workflow-editor-image-generation-gui-smoke.sh`.
   The script needs `npm ci` to have been run (`node_modules/.bin/wdio`), `tauri-driver`,
   `WebKitWebDriver` and `DISPLAY` or `WAYLAND_DISPLAY`. Pass means the spec reporter shows
   `✓ submits a configured workflow and reads the retained image artifact through I/O Inspector` and
   the exit code is 0.

One thing in the smoke spec was not changed. Its waits are up to 300 s for the I/O Inspector and the
image row, but `mochaOpts.timeout` in `wdio.conf.mjs` is 240 s. A slow cold model load could therefore
hit the mocha timeout first. This is outside the selector port, so it is noted here rather than changed.

## Ledger entry text

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
