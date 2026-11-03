# M9c Settings embedded panels restyle report

Agent: Opus 5.5 follow-up agent, 2026-11-03. Task: issue WR-M9-2 from
`m9-settings.md`. The seven app panels the Settings page embeds are restyled
onto the `pg-*` classes, tokens and `workbench/ui` parts without changing what
they do. Settings is the only page that renders them (`SettingsPage.svelte`);
`ManagedRuntimePanel` is also rendered inside `ServerStatus` when
`showManagedRuntimePanel` is true (default; Settings passes false), and the
feature barrels in `src/features/{config,llm,rag}` re-export them with no
importers. Both paths keep working: no props, bindings or exports changed.

## Changed files

| File | Change |
| --- | --- |
| `src/components/ServerStatus.svelte` | "LLM server" `pg-card`: `h3` > disclosure button (`aria-expanded`, chevron `aria-hidden`), status glyph plus word (Ready, Unhealthy, Starting..., Not connected) always beside the title; collapsed summary as a muted line; body divided by a soft rule; red error box became a muted `role="alert"` sentence. `statusColor` (Tailwind dot classes with `animate-pulse`) became `statusGlyph` through `serverConnectionGlyph`. The ready footer line reads "Mode ..." plus the URL (the status word moved to the header). |
| `src/components/server-status/ConnectionModeTabs.svelte` | Two buttons became the shared `SegmentedControl` ("Connection mode" group, `aria-pressed`). Same bindable `connectionType`. |
| `src/components/server-status/ExternalConnectionPanel.svelte` | `pg-fl`/`pg-field` inputs with visible labels "Server URL" and "API key" (previously placeholder only); Connect is `pg-btn-primary`, Disconnect `pg-btn-danger`. Same disabled conditions. Uppercase hint became a muted sentence. |
| `src/components/server-status/BackendOptionList.svelte` | Backend buttons are `pg-tgl` with `aria-pressed` on the running backend; the spinner became the word "switching"; the amber unavailable summary became a muted line with a warn glyph. |
| `src/components/server-status/BackendCapabilityBadges.svelte` | Six colour pills became `Chip`s ("Device Select" → "Device select"). |
| `src/components/server-status/RuntimeSnapshotGrid.svelte` | Two lifecycle cards with `h4` titles and `KeyValue` rows (Runtime, Target, Instance, Warmup, Reused); same values and fallbacks; cards wrap to one column under 32rem. |
| `src/components/server-status/HealthStatusPanel.svelte` | Health disclosure button gains `aria-expanded`; status glyph via `healthStatusGlyph` instead of `HealthMonitorService.getStatusColor`; details as `KeyValue`; "Check Now" → "Check now" (`pg-btn-sm`); monitoring state as glyph plus words (was ●/○ characters in green/grey). |
| `src/components/server-status/serverStatusPresenters.ts` + `.test.ts` | New. `serverConnectionGlyph(ready, healthy, mode)` and `healthStatusGlyph(status)`; 2 tests. |
| `src/components/ModelConfig.svelte` | `pg-card` disclosure "Model configuration" with Configured / Not configured glyph and word; sub-headings `h4` sentence case; path fields `pg-field pg-mono` with `Browse` `pg-btn`s named per field (`aria-label="Browse for VLM model file"` etc.); Save is `pg-btn-primary` ("Save configuration"). Same ids, bindings, handlers. |
| `src/components/DeviceConfig.svelte` | `pg-card` disclosure "Device configuration" with glyph and words (Detecting devices / Detection failed / N devices). Refresh is a `pg-btn-sm`; error text a muted `role="alert"` sentence; selects `pg-field`; range `accent-pg-text`; VRAM bar is `pg-meter` (accent from 75%); the OOM flash is a static danger border plus a "Out of memory reported" line instead of a red pulsing box. Same timer and refresh scope ownership, ids, handlers. |
| `src/components/RagStatus.svelte` | `pg-card` disclosure "Documentation and RAG" with Indexed / Indexing / Not indexed. Docs and vector index rows use glyphs plus words; progress is `pg-meter`; the amber mode-switch box and red error box are muted sentences (warn glyph; `role="alert"`); the external-server toggle gains `aria-expanded` and a labelled URL field; "..." while testing → "Testing..."; the chunk preview icon is gone; buttons `pg-btn`/`pg-btn-primary` in sentence case ("Index now", "Clear index cache", "Preview chunks"). |
| `src/components/SandboxSettings.svelte` | `pg-card` disclosure "Sandbox settings" with Loading / Unavailable / Loaded; error as muted alert sentence; `pg-field` select and textarea; Save `pg-btn-primary`. Same `invoke` calls, ids, options. |
| `src/components/runtime-manager/ManagedRuntimePanel.svelte` | `section` with `h3` "Runtime manager"; error and loading as muted sentences (spinner removed); empty state a muted sentence. |
| `src/components/runtime-manager/ManagedRuntimeCard.svelte` | Card is `pg-card` (`aria-label` runtime name); catalog and activity stack in one column so the versions table fits; history events sentence case through `sentenceCaseState`. Same handlers. |
| `src/components/runtime-manager/ManagedRuntimeHeader.svelte` | `h4` name, readiness glyph plus "Readiness ..., active x, selected y"; amber unavailable box → muted sentence with warn glyph; red error box → muted `role="alert"` sentence. |
| `src/components/runtime-manager/ManagedRuntimeSummaryGrid.svelte` | `dl` with muted keys and mono versions; "Install State" → "Install state". |
| `src/components/runtime-manager/ManagedRuntimeJobPanel.svelte` | Blue box → `bg-pg-panel-head` box; uppercase labels → `h5` and muted words; state glyph via `managedRuntimeJobGlyph`; accent `pg-meter`; Pause `pg-btn`, Cancel/Discard `pg-btn-danger`. Same `role="status"` live region and conditions. |
| `src/components/runtime-manager/ManagedRuntimeCatalogPanel.svelte` | `pg-fl`/`pg-field` selects side by side; versions table is `pg-tbl` (`scope="col"`, sticky head) in a sunken scroll box; badges are `Chip`s (Active has a succeeded glyph); Uninstall `pg-btn-danger pg-btn-sm`, Install `pg-btn-sm`, each with the version in a `pg-sr-only` span so every row's button has a distinct name. |
| `src/components/runtime-manager/ManagedRuntimeActivityPanel.svelte` | History list on sunken rows with mono times; missing-files `details`; Install `pg-btn-primary`, Remove runtime `pg-btn-danger`. |
| `src/components/runtime-manager/ManagedBinaryOverviewPanel.svelte` | `h3` "Managed dependency state" (was uppercase); lucide `RefreshCw` with `animate-spin` removed; the button is now "Refresh dependency state" / "Refreshing dependency state" so it no longer shares the name "Refresh" with other buttons; category headers are `h4` on `panel-head`; rows are a `ul` with status glyph plus words via `managedDependencyGlyph`; the red error box is a muted `role="alert"` sentence. Category labels sentence case. |
| `src/components/runtime-manager/managedRuntimePresenters.ts` + `.test.ts` | New. `managedRuntimeReadinessGlyph`, `managedDependencyGlyph`, `managedRuntimeJobGlyph`, `sentenceCaseState`; 4 tests covering every readiness and job state. |
| `src/components/README.md`, `src/components/server-status/README.md`, `src/components/runtime-manager/README.md` | New. None of the three existed; each has the eleven headings `scripts/check-decision-traceability.sh` requires, with the directory's real boundary (ADR-003, ADR-007, ADR-014, ADR-002), no "None" sections and no banned placeholders. |
| `docs/plans/workbench-redesign/reports/m9c-settings-panels.md` | This report. |

## What stays exactly as before

- Every service call, subscription, `onMount`/`onDestroy` pair, timer and
  refresh scope; every handler; every `disabled` condition and `title`; every
  element id (`vlm-model-path`, `vlm-mmproj-path`, `embedding-model-path`,
  `candle-embedding-path`, `compute-device-select`, `gpu-layers-range`,
  `embedding-memory-mode-select`, `validation-mode-select`,
  `validation-timeout-range`, `allowed-packages-textarea`,
  `${runtime.id}-selected-version`, `${runtime.id}-default-version`); the
  shared one-open-at-a-time accordion (`accordionStore`); the error and status
  messages' text.
- Controls and their order. Nothing added or removed except the text
  equivalents listed above (status words beside glyphs, visible labels on the
  three previously unlabelled inputs, the OOM line).
- Copy changes are sentence case only ("Save Configuration" → "Save
  configuration", "Documentation & RAG" → "Documentation and RAG", "Clear
  Index Cache" → "Clear index cache", ...) plus three name fixes: "..." →
  "Testing...", the dependency "Refresh" → "Refresh dependency state", and
  hidden version names on per-row Install/Uninstall.
- Remove runtime, Uninstall and Cancel download stay single-click, as before.

`grep -rnE "neutral-|cyan|uppercase|tracking-|red-|emerald|amber|green-|yellow-|blue-|purple|pink-|animate-|lucide"` over the five
panel files, `server-status/` and `runtime-manager/`: 0 matches.

## Verification

Run from the repo root on 2026-11-03, final state.

```
$ npm run typecheck
> tsc --project tsconfig.json --noEmit --pretty false
(exit 0, no diagnostics)

$ npm run test:frontend
ℹ tests 688
ℹ pass 688
ℹ fail 0

$ node --experimental-strip-types --test src/components/server-status/serverStatusPresenters.test.ts
ℹ tests 2
ℹ pass 2
$ node --experimental-strip-types --test src/components/runtime-manager/managedRuntimePresenters.test.ts
ℹ tests 4
ℹ pass 4

$ npm run lint:a11y
svelte accessibility gate passed

$ npm run lint:critical
critical anti-pattern gate passed

$ npx eslint src/components/ServerStatus.svelte src/components/ModelConfig.svelte src/components/DeviceConfig.svelte src/components/RagStatus.svelte src/components/SandboxSettings.svelte src/components/server-status src/components/runtime-manager --max-warnings 0
(exit 0)

$ TRACEABILITY_BASE_REF=codex/pantograph-rpc-workflow-2026-09-26 ./scripts/check-decision-traceability.sh
No changed files detected for decision traceability check.
(exit 0; range mode sees only commits, WR-003. The three changed directories
now each have a README with all eleven required headings, checked with
`grep -xF` per heading; none of them is in the host-facing or structured
producer lists.)

$ npx vite build --outDir <scratch>/m9c/dist
✓ built in 9.32s
```

`npx svelte-check` is not usable in this repo (it reports "No Svelte
configuration found in vite config" for every file, plus pre-existing
`state`-as-store false positives in `ModelConfig` and `DeviceConfig`); it is
not one of the gates.

Walkthrough: `python3 -m http.server 8800 --directory <scratch>/m9c/dist`;
`CDP_PORT=9350 node docs/plans/workbench-redesign/reports/tools/cdp-init.cjs <scratch>/m9c/steps.json`
injecting `fixture.js`, `fixture-settings.js`, then a scratch fixture
`<scratch>/m9c/fixture-m9c.js` (answers `get_app_config`, `get_llm_status`,
`list_devices`, `get_embedding_memory_mode`, `get_sandbox_config`,
`get_rag_status`, `list_backends`, `list_managed_runtimes`,
`refresh_managed_runtime_catalogs`, `list_managed_dependencies`,
`get_health_status`; `?job=1` adds an active download); a second run with
`fixture.js` only for the no-host state; server stopped afterwards. Panels
scroll with the Settings panel's own `.pg-scroll` (no `scrollIntoView`).
Logged checks (`<scratch>/m9c/walkthrough.log`, `walkthrough-plain.log`):

- Document height 900 for a 900px viewport; document `scrollTop` 0 after
  every step; no element other than `.pg-scroll` scrolled.
- Panel headers (button id, name, `aria-expanded`, status word):
  LLM server / Not connected, Model configuration / Configured, Device
  configuration / 3 devices, Documentation and RAG / Indexed, Sandbox
  settings / Loaded; each toggles `aria-expanded` to true when opened, and
  opening one closes the others (shared store, as before).
- Server expanded: connection group "External Sidecar"; backend toggles
  llama.cpp `aria-pressed=true`, Ollama disabled, Candle; status Ready; the
  health disclosure toggles `aria-expanded`; External shows labelled "Server
  URL" and "API key".
- Runtime buttons: Refresh runtimes, Uninstall b5021 (cuda12), Uninstall
  b4980 (cuda12), Install b5102 (cuda12), Install latest available version,
  Remove runtime. Dependency rows: llama.cpp Ready b5021, FFmpeg Ready 7.1,
  ocioconvert Missing Unselected bin/ocioconvert, oiiotool System Unselected.
- Active job (`?job=1`): the `role="status"` region reads "Active transfer
  Downloading ... State Downloading 62% ... Pause download Cancel download".
- Old-class elements inside the page: 0 from the restyled files. The page
  query finds 4 outside the page body (the side-panel handle in
  `SidePanel.svelte`); with the server panel open it finds 3 more, all in
  `BackendSelector.svelte` (see WR-M9c-1).
- No host: headers read Not connected, Not configured, Detection failed, Not
  indexed, Unavailable; runtime and dependency failures are muted alert
  sentences ("fixture host: list_managed_runtimes is not mocked").

## Screenshots

All under `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m9c/`:

- `panels-collapsed.png`: "Runtime and app settings" with all five panels collapsed (glyph, status word, summary line).
- `panel-server.png`, `panel-server-health.png`, `panel-server-external.png`: LLM server open (sidecar with backend toggles, chips and runtime cards; health details; external form).
- `panel-model.png`, `panel-device.png`, `panel-rag.png` (external server toggle open), `panel-sandbox.png`.
- `runtimes.png`: Managed inference runtimes with a populated llama.cpp card (version policy, versions table, install history).
- `runtimes-job.png`: the same card with an active download (`?job=1`).
- `dependency-state.png`: Managed dependency state above the page's own media section.
- `plain-collapsed.png`, `plain-server.png`, `plain-sandbox.png`, `plain-runtimes.png`: no host.

## Ledger entry text

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

## Issue rows

IDs are placeholders for the integration owner to number.

| ID | Severity | Evidence | Relationship | Owner / boundary | Disposition | Verification | Revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WR-M9-2 | medium | see `m9-settings.md` | Settings embedded panels | M9c | fixed: the seven panels and their parts are restyled; 0 old classes in those files | walkthrough log "old-class elements", screenshots above | none |
| WR-M9c-1 | medium | `src/components/BackendSelector.svelte` renders inside the LLM server card (sidecar mode) and still uses an uppercase "Built-in inference engine" label, `text-[10px]` neutral text, a tiny "Refresh backends" link and a red bordered error box (3 old-class elements with the panel open; red box in `plain-server.png`) | Settings "looks right under the new tokens" | `BackendSelector.svelte`, outside the M9c write set | defer: its children (`server-status/BackendOptionList`, `BackendCapabilityBadges`) are restyled, so only its own wrapper markup remains | `panel-server.png`, `plain-server.png`, walkthrough log | a follow-up owns `BackendSelector.svelte` |
| WR-M9c-2 | low | `HealthMonitorService.getStatusColor` returns Tailwind colour classes; nothing calls it after this change | Service returns view classes | `src/services/HealthMonitorService.ts` | keep: services were out of scope; `healthStatusGlyph` replaces it in the view | `grep -rn getStatusColor src` | services get a cleanup pass |
| WR-M9c-3 | low | The catalog helper text still says "without expanding the side panel"; the panel now lives on the Settings page | Copy accuracy | `ManagedRuntimeCatalogPanel.svelte` | keep: the task fixed messages as-is | `runtimes.png` | copy pass on Settings |

## Shared-part changes requested from the integration owner

None were made.

1. A disclosure-card part (title button with `aria-expanded`, glyph plus
   status word, collapsed summary, body) would replace the header markup now
   repeated in the five app panels.
2. `KeyValue` has no compact size; `RuntimeSnapshotGrid` sets 12.5px through a
   wrapper selector (`[&_.pg-kv]:text-[12.5px]`).
3. Same as M9 item 1: a `pg-textarea` or `.pg-fl textarea.pg-field` rule;
   `SandboxSettings` sets `h-auto py-1.5` on its textarea.
