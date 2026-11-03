# M9 Settings restyle report

Agent: Opus 5.5 page agent, 2026-11-03. Spec section "Settings": not redesigned;
the page must look right under the new tokens and shared parts. No board.

## Changed files

| File | Change |
| --- | --- |
| `src/components/workbench/SettingsPage.svelte` | Markup restyled; the script logic is unchanged apart from imports and the capability count moved to the presenters. The page is now `pg-main` with two `Panel`s: "Settings" (fixed top with the serif title, the subtitle, Refresh and the page error; scrolling body) and a 320px "Settings summary" panel. Sections are `pg-sec` with sentence-case headings. Fields are `pg-fl` labels wrapping `pg-field` controls, cards are `pg-card`, row lists are `KeyValue`, readiness and category are `Chip`s (the readiness chip has a status glyph), buttons are `pg-btn` (saves use `pg-btn-primary`, Remove version uses `pg-btn-danger`). Error and status text is a muted sentence in the section it belongs to (`role="alert"` / `role="status"` kept). Removed: every `bg-neutral-*`, `border-neutral-*`, `text-neutral-*`, cyan, emerald, amber and red class; uppercase labels; lucide icons and the `animate-spin` refresh icon (the busy state is now the button text: "Refreshing", "Applying cleanup"). Button copy is sentence case ("Save Policy" became "Save policy", and so on). |
| `src/components/workbench/settingsPagePresenters.ts` | Rows are the shared `KeyValueRow` (`key` replaces `label`; `SettingsPolicyRow` is now an alias). Row keys and snake-case values are sentence case ("Disk budget", "Retain payload reference", "Tool binary"). `statusClass` (Tailwind emerald/neutral/amber strings) became `statusGlyph: GlyphState` through `managedMediaDependencyGlyph` (ready and installed → succeeded, unsupported → idle, otherwise warn). New `formatCapabilityCount`, `formatCapabilitySummary`, `buildCapabilityRows` (moved out of the page). |
| `src/components/workbench/settingsPagePresenters.test.ts` | Updated for `key` and the sentence-case copy; two new tests: the dependency glyph mapping and the capability count, summary and rows (loaded and unloaded). 10 tests. |
| `docs/plans/workbench-redesign/reports/tools/fixture-settings.js` | New walkthrough fixture, injected after `fixture.js`. It answers the Settings commands (`workflow_artifact_policy`, `workflow_update_artifact_policy`, `workflow_retention_policy_query`, `workflow_retention_policy_update`, `workflow_retention_cleanup_apply`, `workflow_artifact_format_settings`, `workflow_update_artifact_format_settings`, `workflow_artifact_format_capabilities`, `workflow_list_managed_media_dependencies`, `workflow_managed_media_dependency_status`) and gives the embedded runtime panels empty lists (`list_managed_runtimes`, `refresh_managed_runtime_catalogs`, `list_managed_dependencies`). Everything else falls through to the base host. |
| `docs/plans/workbench-redesign/reports/m9-settings.md` | This report. |

`src/styles.css` was not touched (outside the write set I was given); see
"Shared-part changes".

## What stays exactly as before

- The same service calls on mount and on Refresh. The same handlers, drafts,
  validation messages, status messages, `disabled` conditions and `title`s. The
  same element ids, including every field id.
- Settings, controls and their order: Runtime and app settings (the five
  embedded app panels), ArtifactStore policy, Diagnostics retention policy with
  Retention cleanup, Artifact format defaults (Image, Audio, Video, 3D), Managed
  inference runtimes (the two embedded runtime panels), Managed media
  dependencies (per dependency: Refresh, Version, Staging directory, Installed
  version, Install from staging, Select, Set default, Activate, Clear selection,
  Remove version). Nothing added, removed or renamed. The page has no confirm
  flows of its own; Remove version and Apply cleanup stay single-click as before.
- The `settingsFocus` deep link: the same `$effect`, the same element id
  `settings-diagnostics-retention-policy` on the same form, the same
  `scrollIntoView` and `clearSettingsFocus`. Because the page now scrolls inside
  the Settings panel, `scrollIntoView` scrolls that panel, not the document.
  Note that the id sits on the ArtifactStore policy form (pre-existing; see
  WR-M9-1).

Small markup changes, none functional: field labels wrap their controls
(native association) instead of using `for`; the per-dependency Refresh button
has `aria-label="Refresh <name>"` so the four buttons have distinct names; the
dependency rows render as two `KeyValue` columns; the "Missing: ..." line is a
muted sentence instead of an amber box.

## Verification

Run from the repo root on 2026-11-03, final state.

```
$ npm run typecheck
> tsc --project tsconfig.json --noEmit --pretty false
(exit 0, no diagnostics)

$ npm run test:frontend
ℹ tests 687
ℹ pass 687
ℹ fail 0

$ node --experimental-strip-types --test src/components/workbench/settingsPagePresenters.test.ts
ℹ tests 10
ℹ pass 10
ℹ fail 0

$ npm run lint:a11y
svelte accessibility gate passed

$ npm run lint:critical
critical anti-pattern gate passed

$ npx eslint src/components/workbench/SettingsPage.svelte src/components/workbench/settingsPagePresenters.ts src/components/workbench/settingsPagePresenters.test.ts --max-warnings 0
(exit 0)

$ TRACEABILITY_BASE_REF=codex/pantograph-rpc-workflow-2026-09-26 ./scripts/check-decision-traceability.sh
No changed files detected for decision traceability check.
(exit 0; per WR-003 range mode proves nothing for uncommitted work. No new
directory was added.)

$ npx vite build --outDir <scratch>/m9/dist
✓ built in 9.98s
```

`grep -E "neutral-|cyan|uppercase|red-|emerald|amber|animate-spin|lucide"` over
`SettingsPage.svelte` and `settingsPagePresenters.ts`: 0 matches each.

Walkthrough: `python3 -m http.server 8799 --directory <scratch>/m9/dist`,
`CDP_PORT=9349 node reports/tools/cdp-init.cjs <scratch>/m9/steps.json` with
`fixture.js` then `fixture-settings.js`; a second run with no fixture for the
plain-browser state; server stopped afterwards. Logged checks:

- Scrolling: document height 900 for a 900px viewport; the Settings panel body
  has `scrollHeight` 2985 against `clientHeight` 756 (scrolls inside).
- Headings in order: Runtime and app settings, ArtifactStore policy,
  Diagnostics retention policy, Retention cleanup, Artifact format defaults,
  Image, Audio, Video, 3D, Managed inference runtimes, (embedded "Managed
  Dependency State"), Managed media dependencies, FFmpeg, ocioconvert, oiiotool,
  OpenColorIO.
- Save policy → status sentence "ArtifactStore policy saved" in that section;
  summary panel shows version 5.
- Retention days emptied, Save retention → alert sentence "Retention days must
  be at least 1" in the retention section. (Typing 0 is stopped by the native
  `min="1"` validation before submit, as before.)
- Apply cleanup → status "12 artifacts expired" and the Last cleanup rows.
- FFmpeg Refresh → status "FFmpeg status refreshed" in the media section.
- Deep link: pick a run, I/O Inspector, click "Retention settings" → Settings
  opens, panel `scrollTop` 239, target form at the top of the panel
  (offset 0), document `scrollTop` 0.
- Plain browser (no host): both page alerts are muted sentences ("Transport
  error: Cannot read properties of undefined (reading 'invoke')" in the panel
  top); the summary panel shows "Unavailable" and "0 formats"; saves disabled.
- 34 elements inside the page still carry old `neutral`/`cyan`/`uppercase`
  classes; all of them belong to the embedded app panels (WR-M9-2).

## Screenshots

All under `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m9/`:

- `settings-top.png`: populated page at load (fixture): header, embedded app panels, ArtifactStore policy, Diagnostics retention, summary panel.
- `settings-policy.png`: ArtifactStore policy after the deep-link target scroll.
- `settings-retention.png`: Diagnostics retention with the cleanup status and Last cleanup rows.
- `settings-formats.png`: Artifact format defaults cards and the embedded runtime panels (empty state).
- `settings-media.png`: Managed media dependencies (ready FFmpeg with two versions, missing ocioconvert) with the refresh status sentence.
- `settings-bottom.png`: panel scrolled to the end (oiiotool, OpenColorIO).
- `settings-deeplink.png`: Settings opened from I/O Inspector "Retention settings".
- `settings-plain-browser.png`, `settings-plain-browser-scrolled.png`: no host; engine unreachable.

## Ledger entry text

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

## Issue rows

IDs are placeholders for the integration owner to number.

| ID | Severity | Evidence | Relationship | Owner / boundary | Disposition | Verification | Revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WR-M9-1 | low | `id="settings-diagnostics-retention-policy"` is on the ArtifactStore policy form, not the Diagnostics retention policy form; it has been there since `d7e397fd2`, whose message says the focus target navigates "to the retention section". The I/O Inspector "Retention settings" link therefore lands one section early (retention is visible just below it at 1440x900) | `settingsFocus` deep link (`diagnostics_retention`) | Settings page | keep: the brief said the deep link must behave exactly as today. Fix is one line: move the `id` to the Diagnostics retention `<form>` | walkthrough log "deep link result" (target heading "ArtifactStore policy"), `settings-deeplink.png` | integration owner approves the behaviour change |
| WR-M9-2 | medium | The embedded app panels `ServerStatus`, `ModelConfig`, `DeviceConfig`, `RagStatus`, `SandboxSettings`, `runtime-manager/ManagedRuntimePanel`, `runtime-manager/ManagedBinaryOverviewPanel` still use `neutral-*`, uppercase tracked headings, coloured dots and red error boxes (34 elements inside the page in the walkthrough; red boxes when their commands fail) | Settings "looks right under the new tokens" (A-criteria for M9) | shared app components outside the M9 write set | defer: Settings wraps them in `pg-sec` sections; restyle them in a follow-up that owns those files | `settings-top.png`, `settings-formats.png`, walkthrough log | a milestone takes ownership of those components |
| WR-M9-3 | low | Remove version and Apply cleanup are destructive single-click buttons; the shared `TwoStepButton` exists | Destructive actions on Settings | Settings page | keep: adding a confirm step changes behaviour, which this milestone forbids. Remove version is styled `pg-btn-danger` | `settings-media.png` | Settings gets its own redesign pass |

## Shared-part changes requested from the integration owner

None were made.

1. `src/styles.css`: `.pg-fl .pg-field` forces a 28px height, which collapses a
   `<textarea class="pg-field">`. Settings carries a scoped
   `.pg-fl textarea.set-textarea { height: auto; ... }`; an equal-specificity
   scoped rule lost to the shared one, so the override needs the extra element
   selector. A shared `.pg-fl textarea.pg-field { height: auto; padding: 6px 10px; }`
   (or a `pg-textarea` class) would let pages drop this.
2. `src/styles.css` (WR-008, M9 removal): the `!important` neutral override block
   for the graph package is still present; it was not in my write set. M9's plan
   row allows its removal once M5 restyles the graph package.
3. Optional: a `columns` option on `KeyValue` (or a `pg-kv-cols` wrapper) for long
   row lists. Settings splits the dependency rows into two `KeyValue`s in a grid.
