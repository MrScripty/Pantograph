# M7 Library: report

Agent: Opus 5.5 page agent, 2026-11-03. Board: `docs/design/mockups/boards/Library.dc.html`.
No git state changed.

## Changed files

| File | Change |
| --- | --- |
| `src/components/workbench/LibraryPage.svelte` | Rewritten to the board: dimension line, 400px `Models` panel (search, `Find on Hugging Face` toggle, kind segment, grouped rows, footer with Refresh, usage freshness and status sentence), `Model detail` panel (title, Pumas id, chips, disabled `Use in a new graph` / `Open folder`, two-step `Delete from disk`, sections Where it lives / How it is used / Workflows that use it / Source). Search mode replaces the list with Hugging Face results. |
| `src/components/workbench/libraryUsagePresenters.ts` | Existing exports unchanged (Network page imports them). Added the Pumas-models x usage join, kind classification, grouping, dimension line, sub-lines, detail rows, workflow-use rows, delete-block reason, HF search/download request builders, HF result projection, status sentences, and a per-owner request gate. |
| `src/components/workbench/libraryUsagePresenters.test.ts` | 15 new tests (20 total in the file). |
| `docs/plans/workbench-redesign/reports/tools/fixture-library.js` | New, as the brief asked: wraps `fixture.js` and answers `query_port_options`, `workflow_library_usage_query`, `search_hf_models_with_audit`, `start_hf_download_with_audit`, `delete_pumas_model_with_audit`, `plugin:event|listen/unlisten`. Records calls in `window.__libraryFixtureCalls`. |
| `docs/plans/workbench-redesign/reports/m7-library.md` | This report. |

Walkthrough step files and an error-injection fixture live in the scratch directory only
(`/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m7/steps*.json`, `fixture-library-errors.js`).

## Data authority (unchanged)

Same service calls as before, no new ones: `loadPumasModelOptions` (`query_port_options`
for `puma-lib`/`pumas_model_ref`, with `invalidatePumasModelOptionsCache` on Refresh),
`workflowService.queryLibraryUsage({ limit: 250 })`, `subscribeDiagnosticsProjectionInvalidations(['library_usage'])`,
`searchHfModelsWithAudit`, `startHfDownloadWithAudit`, `deletePumasModelWithAudit`.
The only behavioural difference: after a successful delete the page re-reads both the usage
projection and the Pumas model list (forced), so the deleted model leaves the list from the
authority rather than by local removal. Before, only usage was re-read.

How the projection maps to the board:

- Rows: Pumas selector rows (`PortOption`, metadata from `puma_lib.rs::selector_row_option_metadata`)
  joined with `LibraryUsageProjectionRecord` by `asset_id === "pumas://models/" + model_id`
  (the id the host writes in `session_execution_api.rs::pumas_model_asset_id` and
  `puma_lib_commands.rs::record_pumas_model_delete_audit`). Rows without usage say
  "No usage recorded". The `hf://models/<repo_id>` download audit row feeds "Download recorded" in Source.
- Groups: device residency is not knowable from either projection, so there is no
  "Loaded on a device" group and no cache squares. Groups are "Used this month",
  "Unused for 30 days" (usage older than 30 days) and "No usage recorded" (no usage row);
  when no row has usage at all, one "Models" group. I added the third group rather than put
  never-used models under "Unused for 30 days", because absence from the ledger does not prove
  30 days of non-use (retention, a fresh ledger).
- Dimension line: "N models, U used this month, V unused for 30 days, K with no usage recorded"
  (only "N models" when no usage exists; "Pumas model list unavailable" when the list failed).
  No GB, no loaded count.
- Kind segment (All, Text, Image, Audio): from Pumas `model_type` (llm, embedding, reranker -> Text;
  diffusion, vision -> Image; audio -> Audio), else `task_type_primary`, else `pipeline_tag`
  (fixed table in the presenter). `vlm` and unknown models appear only under All. The segment is
  hidden when no row has a known kind.
- Right column of a row: last-used time from usage; size omitted (not in any projection).
- Detail: omitted fields where facts are missing (format, size, licence, device). "Workflows that use it"
  comes from the usage record for the asset; the row whose `last_workflow_run_id` equals the active run
  gets the selected treatment (panel-selected background, 3px inset bar, `aria-current`) and the
  "Last used by active run" line; the run id cell carries `data-run-id`.
- Delete: `TwoStepButton` keyed by model id (an armed confirm never carries over to another
  selection; found and fixed during the walkthrough), disabled with a title while the active run is
  the last user of the model, or while any action (search, download, delete) is busy.
- Search mode: query, kind and limit (clamped 1..100, trimmed, as the host validates) on a form;
  results as rows (mono repo id, kind/format/downloads sub-line, size when `totalSizeBytes` exists,
  Download button). The `Download options` disclosure (`aria-expanded`) above the results exposes
  official name, family and model type with today's defaults (family "model", name = repo id).
  Status sentences ("Download dl-51 of Org/Model recorded at event 51", "Search recorded at event N",
  "<model>: Delete recorded at event N") render muted in the panel footer; failures likewise.
- Errors: Pumas list and usage failures render as muted sentences at the top of the list panel and in the
  affected detail sections; an invalidation-subscription failure shows as "Usage current at seq N, not
  updating live" in the footer with the error in its title. No banners.
- Async: one generation gate per request kind (usage, models, actions), owned by the page instance,
  disposed on destroy; superseded or post-unmount completions never write state. Covered by
  `createLibraryRequestGate applies only the newest invocation and nothing after dispose`.

Deviations from the board/spec, for the integration owner:

- Toggle keeps the label `Find on Hugging Face` with `aria-pressed` and a pressed style instead of
  swapping to "Back to library" (a pressed state plus a changing label reads twice to screen readers).
- Requested downloads read `Requested` (with the download id in the title) rather than `Queued`:
  the host returns a started download id, not a queue position.
- No footer cache legend and no free-disk line (no facts).

## Verification (verbatim, from the repo root)

```
$ npx eslint src/components/workbench/LibraryPage.svelte src/components/workbench/libraryUsagePresenters.ts src/components/workbench/libraryUsagePresenters.test.ts --max-warnings 0 && echo ESLINT_OK
ESLINT_OK

$ npm run typecheck
> Pantograph@0.0.0 typecheck
> tsc --project tsconfig.json --noEmit --pretty false
(no output; an earlier run failed only on diagnosticsPagePresenters.test.ts, M2's in-progress files)

$ npm run test:frontend
ℹ tests 685
ℹ pass 685
ℹ fail 0

$ npm run lint:a11y
svelte accessibility gate passed

$ npm run lint:critical
critical anti-pattern gate passed

$ TRACEABILITY_BASE_REF=codex/pantograph-rpc-workflow-2026-09-26 ./scripts/check-decision-traceability.sh
No changed files detected for decision traceability check.
(range mode proves nothing for uncommitted work, WR-003; src/components/workbench/README.md exists from M0)

$ npx vite build --outDir /tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m7/dist --emptyOutDir
✓ built in 21.38s
```

`npx svelte-check` cannot run in this repo ("No Svelte configuration found in vite config"), so the
Svelte template is type-checked only through the vite build.

Headless walkthrough: `python3 -m http.server 8797` over the build, `CDP_PORT=9347 node cdp-init.cjs <steps>`
with `fixture.js` + `fixture-library.js` (server stopped afterwards). Logged results:

```
groups: ["Used this month 3 ▾","Unused for 30 days 1 ▾","No usage recorded 3 ▾"]
dim + selected: ["7 models, 3 used this month, 1 unused for 30 days, 3 with no usage recorded","bge-m3"]
detail buttons: [["Use in a new graph","true","Not available yet: no service starts a new graph from a library model"],["Open folder","true","Not available yet: no service opens the model folder"],["Delete from disk","true","Cannot delete while the active run is the last one that used this model"]]
after switching selection while armed: "Use in a new graph|Open folder|Delete from disk"
delete status, calls: ["sdxl-base-1.0: Delete recorded at event 49","[[\"delete_pumas_model_with_audit\",{\"modelId\":\"diffusion/stabilityai/sdxl-base-1.0\"}]]"]
toggle pressed: "true"
status + buttons: ["Download dl-51 of Qwen/Qwen3-14B-Instruct-GGUF recorded at event 51","Requested|Download|Download|Download|Download"]
hf calls: [["search_hf_models_with_audit",{"query":"qwen","kind":null,"limit":25,"hydrateLimit":10}],["start_hf_download_with_audit",{"request":{"repo_id":"Qwen/Qwen3-14B-Instruct-GGUF","family":"model","official_name":"Qwen/Qwen3-14B-Instruct-GGUF","model_type":null}}]]
no host -> notes: ["The Pumas model list could not be read. Transport error: ...","Usage could not be read. Transport error: ...","Models appear here once the Pumas library lists them."]
injected errors -> notes: ["The Pumas model list could not be read. Transport error: Pumas model selector access not available","Usage could not be read. Internal error: library_usage projection is rebuilding", ...]
```

## Screenshots

Directory `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m7/`:

- `library-no-run.png`: populated list and detail, no run selected.
- `library-active-run.png`: run `ingest-docs-rag` selected; bge-m3 shows "Last used by active run" in the row, chip and selected table row; Delete disabled.
- `library-delete-armed.png`, `library-armed-reset.png`, `library-deleted-status.png`: two-step delete, reset on selection change, footer status sentence.
- `library-not-ready.png`: a selector row the host marks not ready (readiness sentence, "Not ready" chip).
- `hf-empty.png`, `hf-results.png`: search mode before searching and with results, disclosure open, one download requested.
- `library-unavailable.png`: plain browser, no host: shell shows "Engine unreachable"; the page shows muted sentences and "Pumas model list unavailable".
- `library-errors.png`: host reachable but both reads fail.

## Ledger entry text

2026-11-03, M7 Library implemented (Opus 5.5 page agent). `LibraryPage.svelte` rebuilt on the shared
parts per the Library board: Pumas selector rows joined with the library usage projection by
`pumas://models/<model_id>`; groups by last access (Used this month / Unused for 30 days / No usage
recorded, or Models when no usage exists) because device residency is not projected; kind segment from
Pumas model type, task and pipeline tag; Hugging Face search mode over the existing search and download
calls with a download-options disclosure; two-step delete keyed by model and blocked while the active run
last used the model or an action is busy; `Use in a new graph` and `Open folder` disabled with titles.
Presenters gained the join, grouping, detail, search and request-gate logic with 15 new tests.
Gates: eslint (changed files), typecheck, test:frontend 685/685, lint:a11y, lint:critical, vite build pass;
traceability range mode reports no changed files (WR-003). Headless walkthrough with
`reports/tools/fixture-library.js` recorded populated, active-run, delete, search, unavailable and error
states (screenshots in the M7 scratch directory).

## Issue rows

| ID | Severity | Evidence | Relationship | Owner / boundary | Disposition | Verification | Revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WR-M7-1 | low | No service starts a new graph from a library model; `Use in a new graph` renders `aria-disabled` with title "Not available yet: no service starts a new graph from a library model" | Library actions (A2) | Graph/workflow services | defer | button stays disabled in `library-active-run.png` | a service creates a graph seeded with a Pumas model ref |
| WR-M7-2 | low | No service opens a model folder; `Open folder` renders `aria-disabled` with title "Not available yet: no service opens the model folder" | Library actions (A2) | desktop host (Tauri shell/opener) | defer | same screenshot | a host command reveals a Pumas model path |
| WR-M7-3 | medium | Neither the Pumas selector rows nor `LibraryUsageProjectionRecord` carry size on disk, device residency, free disk space, format or licence; the board's cache squares, "Loaded on a device" group, GB totals, free-disk footer and licence row are omitted | Library layout (A2) | Pumas selector snapshot / backend projections | defer: groups by last access, fields omitted | presenter tests `detail rows omit facts the projections lack`, `groupLibraryModelRows ...` | projections expose size, residency or licence |
| WR-M7-4 | medium | `library_usage_projection` keeps one row per asset with only the last workflow/run; the per-run rows in `library_usage_run_projection` are not returned by `workflow_library_usage_query`, and no node id is recorded. "Workflows that use it" therefore has at most one row and no Node column | Library detail (A2) | diagnostics ledger / workflow service projection | defer: one row plus a sentence saying so | `libraryWorkflowUseRows marks the active run ...` | the query returns per-workflow (or per-run) usage with node ids |
| WR-M7-5 | medium | `PumasHfModelSearchResult` in `src/services/diagnostics/types.ts` declares `id`, but the host returns pumas `HuggingFaceModel` serialised camelCase (`repoId`, `totalSizeBytes`, `downloads`, ...); the old page always showed "Unknown model". The presenter reads `repoId` then `id` through the index signature | HF search results | services/diagnostics types owner | fix in the type (add `repoId` and the other fields) outside M7's write set | `projectHfSearchResults reads the camelCase host fields ...` | type corrected; presenter can drop the `id` fallback |
| WR-M7-6 | low | The spec blocks delete "while a run uses the model"; no projection says which runs currently hold a model, so the block covers only "the active run is the last user" (plus busy). A model used by a running run that is not selected can be deleted | Delete safety | scheduler/runtime projection | defer | `libraryDeleteBlockReason` tests | a projection exposes live model holders per run |

## Shared-part changes requested

None required. Notes for the integration owner:

- `ListRow` requires a status glyph; library rows have no status, so the page renders its own
  `button.pg-item` with a `1fr auto` grid. If another page needs a glyph-less row, a `glyph` optional
  prop on `ListRow` would cover both.
- A pressed style for `pg-btn` toggles (`pg-btn[aria-pressed='true']`, as `pg-tgl` has) would let the
  page drop its local `.lib-hf-toggle` rule.
