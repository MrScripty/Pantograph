# Plan: Replace the workbench GUI with the approved calm redesign

**Plan status:** `Verifying`

**Current phase:** Objective verification. M0–M9 are implemented and gated; A1–A4 are satisfied in the representative environment; A5 (desktop with the local engine) awaits the owner.

**Next slice:** A5 by the owner (`npm run dev:desktop`, walk every tab with the local engine, then run the two WebKit harness commands in `reports/m9-e2e-harnesses.md`), followed by the commit of the branch.

**Acceptance status:** `partial` (A1–A4 satisfied; A5 pending the owner's desktop run)

**Decision owner:** MrScripty. **Integration owner:** the Claude Code primary session (Fable 5.1); implementation slices are delegated to Opus 5.5 agents.

**Execution ledger:** [execution-ledger.md](execution-ledger.md)

**Issues:** [issues.md](issues.md)

**Design authority:** [workbench redesign spec](../../design/workbench-redesign-spec.md) and the mockups in `docs/design/mockups/`.

**Standards baseline:** Coding-Standards checkout `5bce3110` at
`repos/owned/developer-tooling/Coding-Standards`, routed through the Standards
Engine (application purpose, interface 48, environment
`/home/jeremy/.local/share/standards-engine/venv`) on 2026-11-03. Route
snapshot `snapshot:v1:58a8cb6f-8bc1-4f8b-9f84-caa364b3df2b` from facts:
activities planning, implementation, verification, documentation, commit;
applications frontend; boundaries known-absent; details topic.code-design,
workflow.verification.gui; frameworks known-absent; languages typescript,
typescript-async; topics accessibility, architecture, concurrency,
dependencies; workflow-profiles known-absent. Selected closure (18, all
normative, whole-artifact): core, router, workflow.planning,
workflow.implementation, workflow.verification, workflow.verification.gui,
workflow.development-proportionality, workflow.documentation, workflow.commit,
profile.application.frontend, profile.language.typescript,
profile.language.typescript.async, topic.contracts, topic.accessibility,
topic.concurrency, topic.code-design, topic.architecture, topic.dependencies.
Re-route at each milestone boundary if the task facts change (for example a
page that needs a new IPC contract adds the `ipc` boundary).

## Objective

A Pantograph user opens the desktop app and sees the redesigned workbench:
a top tab bar, one run-context picker shared by every tab, and the Scheduler,
Diagnostics, I/O Inspector, Network, Graph, Node Editor, Library and new
Sessions tabs laid out as in the approved mockups, with the Calm Tech
behaviours (grouped lists, progressive filters, in-place errors, two-step
destructive actions, themed scrollbars). Existing backend projections remain
the only data authority (ADR-014); no page gains a second source of truth.

## Objective Acceptance

| ID | Observable criterion | Kind | Environment | Mode | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| A1 | Shell shows the tab bar and run chip; selecting a run in Scheduler, in the picker, or by Ctrl+V over the chip changes the run every tab shows; `×` clears it and pages show their empty state | `user-workflow` | `representative` (headless Chromium over the built app with mocked services) | `automated` | `satisfied` | ledger 2026-11-03 closure; `reports/evidence/01…14*.png`; combined walkthrough log in the ledger |
| A2 | Each of the eight tabs renders its mockup layout from real projection types, including loading, empty, error and engine-unavailable states | `user-workflow` | `representative` | `automated` | `satisfied` | per-page reports `reports/m1…m8*.md` with their screenshot sets; `reports/evidence/*.png`; WebKitGTK harness pass `reports/evidence/webkit-io-inspector-harness.log` |
| A3 | Presenter and store logic for the picker, grouping, filters, glyph mapping and Graph run/edit mode is covered by focused tests | `focused` | `not-applicable` | `automated` | `satisfied` | `npm run test:frontend` 688 passing on the final tree (ledger closure entry) |
| A4 | Static gates: typecheck, eslint on changed files, lint:a11y, lint:critical, vite build | supporting | `not-applicable` | `automated` | `satisfied` | ledger closure entry |
| A5 | The desktop app (`npm run dev:desktop`) with the local engine shows every tab working against live data | `user-workflow` | `required-real` (owner's machine, local engine) | `manual` | `pending` | owner procedure: open each tab, select runs, check clipboard shortcuts (WR-010), palette drag (WR-052), Enter on run-graph nodes (WR-053), then run the harness commands in `reports/m9-e2e-harnesses.md` |

## Scope

### In Scope

- `src/styles.css`, `tailwind.config.cjs`, `index.html`, font packages.
- `src/components/workbench/**` (shell, shared parts, all pages) and the
  presenters/tests beside them.
- `src/stores/workbenchStore.ts` (sessions page id, picker helpers) and a
  run-context picker store.
- A Sessions page over existing services only.
- Settings restyled to the new tokens without redesign.
- `docs/design/**`, this plan package, `docs/plans/README.md`.

### Out Of Scope

- New backend commands, projections or persistence (sessions, node authoring,
  pairing, "open folder"). Where the mockup shows such an action, the control
  is rendered disabled with a title and the gap is recorded in `issues.md`.
- The retired drawing/canvas feature and `packages/svelte-graph` internals
  beyond styling hooks the Graph page needs.
- The Pumas RPC work in `docs/plans/domain-architecture-and-multimodal/`.

## Constraints And Assumptions

### Constraints

- ADR-014: active run is transient shared frontend state; pages consume typed
  projection services only; historic graphs render in an isolated read-only
  surface and never touch the editable graph store.
- Svelte 5 runes and snippets, Tailwind 3, lucide-svelte, Node test runner for
  `*.test.ts` (`npm run test:frontend`), `scripts/check-svelte-a11y.mjs`,
  `scripts/check-critical-antipatterns.mjs`.
- `scripts/check-decision-traceability.sh` still enforces a README with fixed
  headers per changed source directory at pre-commit. `src/components/workbench`
  has none today. New directories under `src/` need one; see M0 tasks.
- Commits are made by the owner or on explicit request; agents leave the
  working tree with their slice complete and verified, uncommitted.
- Branch `feat/workbench-redesign` (private, based on
  `codex/pantograph-rpc-workflow-2026-09-26` at `019caf4aa`), target: the
  owner's integration branch; terminal disposition: integrated or abandoned
  by the owner.

### Assumptions

- `@fontsource/newsreader`, `@fontsource/atkinson-hyperlegible`,
  `@fontsource/jetbrains-mono` can be installed (decision: bundled fonts vs
  system stacks; validate by `npm install` in M0; stop condition: install
  fails, record in issues, use system stacks).
- Device memory facts for the lanes exist in the local Network status
  projection (decision: show lanes vs hide strip; validate in M0 by reading
  `networkPagePresenters.ts` and the status type; stop condition: absent,
  strip hidden with a muted sentence).
- The run list projection exposes a client session id per run (decision:
  Sessions "Active" grouping; validate in M8; stop condition: absent, Active
  shows only the current window's runs).

## Binding Decisions

| Decision | Owner | Evidence | Supersedes |
| --- | --- | --- | --- |
| Direction E (dark drafting paper, inbox layout) is the target for every tab | MrScripty | Mockup review 2026-11-02/03 | icon-rail shell |
| One header run-context picker is the only selection control; Scheduler rows set the same state | MrScripty | "lets add the picker… selected run in the scheduler automatically set" | per-page active-run labels |
| Ctrl+C over a run copies its id; Ctrl+V over the chip views it | MrScripty | same | none |
| Graph has `Edit workflow` and `As run` modes; run mode is the isolated run graph projection | MrScripty, ADR-014 | "view the workflow by a run and by a workflow" | run graph strip inside I/O Inspector |
| Unavailable capabilities render disabled at the owning boundary, never as working controls | Coding-Standards implementation workflow | Disabled And Incomplete Behavior | none |

## Design Observations And Remaining Evidence

| Assumption or claim | Decision affected | Owner | Executable procedure | Required environment | Status/evidence |
| --- | --- | --- | --- | --- | --- |
| Fonts installable offline-safe | token bundle | M0 agent | `npm install @fontsource/...` then `npx vite build` | npm registry access | satisfied: three packages at 5.3.0, OFL 1.1, 18 woff2 files in the build (ledger 2026-11-03) |
| Device memory in local status projection | lanes strip | M0 agent | read `src/components/workbench/networkPagePresenters.ts`, `src/services/diagnostics/types.ts` | none | absent: `WorkflowLocalGpuMetrics` is `{available, reason}` only (WR-006); strip hidden with a muted sentence until the projection exposes per-device memory |
| Headless Chromium walkthrough can mock services | A1, A2 | integration owner | the M0 scratch stand-in answers `workflow_run_list_query` with fixture runs (see ledger); pages extend it per milestone | Chromium headless shell at `~/.cache/ms-playwright` | satisfied for A1 (picker walkthrough recorded in the ledger); A2 pending per page |

## Simplicity And Ownership Review

**Applicability:** `applicable`

- Independent concepts and dimensions: visual tokens (styles), shell/navigation, run context (one store), per-page projection (presenters), shared presentation parts.
- State, identity, value, time, policy, and mechanism:
  - Canonical authority scope: run selection in `workbenchStore`; run list in `schedulerRunListStore`; page data in backend projections.
  - Version roles and owned promises: none new; no persisted contract changes.
  - Supported compatibility overlaps: none; the old shell is replaced, not kept.
  - Material identity-invalidation effects: none.
- Caller and composition-root knowledge: pages know only the shared parts' props and their own presenters; the shell knows the page registry.
- Representative change paths and forced owners: a new status value changes `StatusGlyph` mapping once; a new page adds a registry entry and one component.
- Stable Interfaces versus hidden knowledge: shared parts expose props, not DOM; the picker exposes `selectRun`, `clearRun`, `openPicker`.
- Independent evolution, testing, failure, and replacement: presenters stay pure and tested; pages can be replaced one at a time.
- Necessary complexity and containment: clipboard shortcuts are one small module with a documented fallback.
- Deletion and cumulative machinery result: the `!important` neutral overrides, the icon rail, the per-page "Active Run" labels, the I/O Inspector graph strip and the 14-select filter wall are deleted.

## Milestones

### Milestone 0: Foundation

**Goal:** The new shell, tokens, run-context picker and shared parts exist and the existing pages still render inside them.

**Allowed write set:**

- `src/styles.css`, `tailwind.config.cjs`, `index.html`, `package.json`, `package-lock.json` (fonts only)
- `src/components/workbench/WorkbenchShell.svelte`
- `src/components/workbench/shell/**` (new: `RunContextPicker.svelte`, `runContextPicker.ts`, `runContextClipboard.ts`, tests, `README.md`)
- `src/components/workbench/ui/**` (new shared parts, `README.md`)
- `src/components/workbench/README.md` (new, traceability headers)
- `src/stores/workbenchStore.ts`, `src/stores/workbenchStore.test.ts`
- `src/README.md`, `src/stores/README.md` (traceability hook, WR-003)
- `docs/plans/workbench-redesign/**`

**Tasks:**

- [x] Tokens, fonts, scrollbar, focus styles; `!important` neutral overrides removed (one block for the graph package remains, WR-008, removed in M5/M9).
- [x] Shell with top tabs, groups, Settings right, engine line, Assistant toggle; `sessions` page id with a placeholder page.
- [x] Run-context picker store + component + clipboard shortcuts + empty-state component (deviations WR-009 accepted by the integration owner: a "Finished earlier" group, "recent history" wording, outside click leaves focus where it is).
- [x] Shared parts listed in the spec.
- [x] READMEs satisfying `scripts/check-decision-traceability.sh` for every touched/new source directory, including `src/README.md` and `src/stores/README.md` (WR-003, added to the write set).
- [x] Tests: picker grouping/search/paste resolution, glyph mapping, store page id, listener lifecycle and stale-result exclusion (30 new tests, 574 total passing).

**Acceptance gate:**

- `npm run typecheck`, `npm run test:frontend`, `npm run lint:a11y`, `npm run lint:critical`, `npx eslint <changed> --max-warnings 0`, `npx vite build` pass; a headless screenshot of the built app shows the new shell. Evidence in the ledger.

**Status:** `Implemented` (gate passed on 2026-11-03 except `lint:a11y`/`lint:critical`, which fail only on pre-existing findings in M3's files, WR-007; see ledger)

### Milestones 1 to 8: Pages (parallel, after M0)

One milestone per page, each with the goal "the page matches its board using real projection types" and the write set limited to its own `*Page.svelte`, its presenters and tests, plus `issues.md` entries. Shared parts are read-only for page agents; a needed change to a shared part is reported to the integration owner, not made in place.

| Milestone | Page | Primary write set |
| --- | --- | --- |
| M1 | Scheduler | `SchedulerPage.svelte`, `schedulerPagePresenters.ts` + test, `src/stores/schedulerRunListStore.ts` + test |
| M2 | Diagnostics | `DiagnosticsPage.svelte`, `diagnosticsPagePresenters.ts` + test |
| M3 | I/O Inspector | `IoInspectorPage.svelte`, `ioInspectorPresenters.ts` + test, `SavedGraphInspectionSnapshot.svelte` (saved-graph inspection stays reachable from the page's empty state); stops importing `RunGraphSnapshot.svelte` |
| M4 | Network | `NetworkPage.svelte`, `networkPagePresenters.ts` + test |
| M5 | Graph | `GraphPage.svelte`, `RunGraphSnapshot.svelte` (run mode surface), `runGraphPresenters.ts`, `graphInspectionPresenters.ts` + tests, `src/components/NodePalette.svelte`, `src/components/WorkflowToolbar.svelte`, `WorkflowPersistenceControls.svelte`, `src/components/nodes/BaseNode.svelte`, `WorkflowGraph.css` |
| M6 | Node Editor | `NodeLabPage.svelte`, `nodeLabPresenters.ts` + test |
| M7 | Library | `LibraryPage.svelte`, `libraryUsagePresenters.ts` + test |
| M8 | Sessions | `SessionsPage.svelte` (new), `sessionsPresenters.ts` + test |

**Acceptance gate per page:** the static gates above on the changed files, the page's tests, and a headless screenshot of the page in the built app recorded in the ledger.

**Status:** M1–M8 `Implemented` (each page's gates passed and its screenshots reviewed by the integration owner on 2026-11-03; reports in `reports/m<N>-<page>.md`, ledger entries and issue rows merged).

### Milestone 9: Settings restyle and closure

**Goal:** Settings looks right under the new tokens; old style overrides are gone; objective evidence A1–A5 recorded.

**Allowed write set:** `SettingsPage.svelte`, `settingsPagePresenters.ts` (+ test), `tests/e2e/io-inspector-image-preview/**` and `tests/e2e/workflow-editor-image-generation/**` (harness selectors only, WR-023), `src/styles.css` (removal of the graph `!important` block, WR-008), the shared parts in `src/components/workbench/{shell,ui}/**` for the additive changes the page reports requested, `src/services/diagnostics/types.ts` (WR-037 type correction), this plan package.

**Status:** `Implemented` (M9a Settings, M9b harnesses, M9c embedded panels, M9d graph components, plus the integration slice; see ledger)

## Blockers

- M6 Node Editor: superseded premise (WR-066). PR #95 adds node authoring and rewrites the same three files; M6b must redo the page on top of it once #95 is on this branch.

## Re-Plan Triggers

- Triggered 2026-11-03 for M6 by PR #95 (WR-066): the Node Editor is re-planned as M6b over the authoring API.
- A page needs a projection field the backend does not expose and the mockup
  depends on it materially (record in issues; if it blocks the page's main
  layout, replan that milestone).
- The traceability hook cannot be satisfied without rewriting unrelated
  directories.
- The Graph editor package (`packages/svelte-graph`) needs internal changes
  beyond styling hooks.

## Concurrent Work

| Owner | Primary write set | Adjacent write set | Forbidden/shared | Output/report | Integration order |
| --- | --- | --- | --- | --- | --- |
| M0 agent | see Milestone 0 | none | all page files | ledger entry + screenshot | first, serial |
| M1–M8 agents | their page files (table above) | `reports/m<N>-<page>.md` (their own report, merged into ledger/issues by the integration owner) | `src/styles.css`, `tailwind.config.cjs`, `WorkbenchShell.svelte`, `shell/**`, `ui/**`, `workbenchStore.ts`, other pages | ledger entry + screenshot per page | after M0, parallel, integrated by the integration owner in milestone order |
| Integration owner | plan package, shared parts fixes reported by page agents | none | none | this plan | serial |

## Repository Isolation

Branch `feat/workbench-redesign`, private, created 2026-11-03 from
`codex/pantograph-rpc-workflow-2026-09-26` at `019caf4aa`. Owner MrScripty;
integration owner as above; no worktrees. Terminal disposition decided by the
owner after A5.

## Final Acceptance

- Acceptance status: `partial` (A5 pending)
- Deferred follow-ups: the `defer` rows in `issues.md` (projection fields the mockups assumed, session and node-authoring services, package CSS scoping WR-063, Scheduler server-side filtering WR-011)
- Final status: `Verifying`
