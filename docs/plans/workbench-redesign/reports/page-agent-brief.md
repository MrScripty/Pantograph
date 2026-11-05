# Brief for page milestones (M1 to M8)

Each page milestone is delegated to one agent working in the shared working
tree on branch `feat/workbench-redesign`. This brief is the common part of
every page agent's instructions; the plan's milestone table gives the write set.

## Read before editing

1. `docs/plans/workbench-redesign/plan.md` (your milestone row and write set).
2. `docs/design/workbench-redesign-spec.md` (your page's section, the shared
   parts, the accessibility contract, the evidence list).
3. Your board under `docs/design/mockups/boards/` (the visual authority) and
   `boards/shell.css`. View it rendered with
   `bash docs/design/mockups/viewer/build.sh --serve` (http://127.0.0.1:8765/).
4. `src/components/workbench/ui/README.md` and `shell/README.md` (the shared
   parts and the run-context contract built in M0), then the parts' sources.
5. Your current page, its presenters and tests, and the service types it
   consumes (`src/services/diagnostics/types.ts`, `src/services/workflow/*`).
6. `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`.
7. Coding standards at
   `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`
   (the Standards Engine route for this plan selected these; read the ones
   marked * at minimum): `CORE-STANDARDS.md`*, `workflows/implementation.md`*,
   `workflows/verification.md`, `workflows/development-proportionality.md`,
   `profiles/applications/frontend.md`*, `profiles/languages/typescript.md`,
   `profiles/languages/typescript/async.md`*, `topics/accessibility.md`*,
   `topics/concurrency.md`*, `topics/code-design.md`*, `topics/contracts.md`,
   `topics/architecture.md`, `workflows/documentation.md`.

## Rules

- Write only inside your milestone's write set, plus your own report file
  `docs/plans/workbench-redesign/reports/m<N>-<page>.md` (ledger entry text,
  issue rows in the issues.md table format, verification results, screenshot
  paths). Do not edit `plan.md`, `execution-ledger.md` or `issues.md`; other
  agents run at the same time and the integration owner merges reports.
  Shared parts, the shell, the store, styles and other pages are read-only;
  if a shared part needs a change, describe it in your report instead of
  editing it.
- Several page agents share this working tree concurrently. Expect files
  outside your write set to change under you; never revert them. Use a
  distinct scratch directory and port for your build and server.
- Do not commit or alter git state.
- Keep the page's data authority exactly as today: the same service calls and
  projection types. Presenters stay pure; put new projection logic there with
  tests. No new backend calls, no ledger reads, no optimistic mutation.
- Capabilities the mockup shows but no service supports render as disabled
  native controls with a `title` explaining why, and get an `issues.md` row.
- Svelte 5 runes and snippets only. Native elements with names; `aria-current`
  on selected rows, `aria-pressed` on toggles, `aria-expanded` on disclosures,
  `aria-hidden` on decorative glyphs. No pulsing motion, uppercase tracked
  labels, cyan, or page-wide red banners.
- Elements that show a run id carry `data-run-id` so Ctrl+C copies it.
- Any subscription, timer, observer or listener you add has one owner, starts
  on mount, stops on destroy, never duplicates across re-renders, and discards
  stale async results (topic.concurrency, TypeScript async profile). Cover the
  deterministic part in a test.
- Do not add dependencies. If the page truly needs one, stop and report.
- The page never scrolls as a whole; panels scroll inside.

## Verify before reporting

From the repo root:

```
npm run typecheck
npm run test:frontend
npm run lint:a11y && npm run lint:critical
npx eslint <changed files> --max-warnings 0
TRACEABILITY_BASE_REF=codex/pantograph-rpc-workflow-2026-09-26 ./scripts/check-decision-traceability.sh
npx vite build --outDir <scratch>/<milestone>/dist
```

Then serve the build (`python3 -m http.server <free port> --directory <dist>`)
and screenshot your page with the headless shell at
`~/.cache/ms-playwright/chromium_headless_shell-1246/chrome-headless-shell-linux64/chrome-headless-shell --headless --disable-gpu --window-size=1440,900 --virtual-time-budget=5000 --screenshot=<png> <url>`.
In a plain browser the Tauri calls fail, so the page shows its unavailable or
empty state; that state must look designed, not broken. Where a screenshot of
the populated layout is needed, add a dev-only fixture path the page can
render from (behind `import.meta.env.DEV`-guarded code that production
cannot reach) or drive the page with `docs/design/mockups/viewer/cdp.js`.
Look at the screenshot and fix layout problems before reporting. Stop the
server you started.

## Report

Write `reports/m<N>-<page>.md` and return its content: changed files;
verification results verbatim; screenshot paths; ledger entry text; issue rows;
any shared-part change you need from the integration owner.
