# 05: App Shell And Active Run Navigation

## Status

In progress. The frontend now has a Scheduler-first workbench shell, transient
active-run navigation, graph and diagnostics page wrappers, and reserved pages
for I/O Inspector, Library, Network, and Node Editor. Rich I/O and Library page
bodies remain later-stage work.

## Objective

Refactor the Svelte application shell from the current canvas/workflow toggle
into a Scheduler-first workbench with top-level pages and transient active-run
context shared across Diagnostics, Graph, I/O Inspector, Library, and Network.

## Scope

### In Scope

- Top-level workbench shell.
- Default route/page set to Scheduler.
- Toolbar or rail navigation for Scheduler, Diagnostics, Graph, I/O Inspector,
  Library, Network, and Node Editor.
- Active-run store and top-bar context display.
- No-active-run states per page.
- Explicit relocation or retirement path for existing drawing-to-Svelte and
  workflow graph surfaces.
- Accessibility and keyboard navigation for the page toolbar.

### Out of Scope

- Full implementation of every page body.
- Backend API implementation.
- Full visual redesign beyond layout and navigation required for the
  workbench.

## Inputs

### Problem

`src/App.svelte` currently switches between `canvas` and `workflow` view modes.
The target product needs distinct pages and a persistent active-run context.
The shell must change before page implementations can feel coherent.

### Constraints

- Scheduler is the default landing page.
- Active-run selection does not persist across restart.
- The frontend may own active-run UI selection, page route, filters, sorting,
  panel widths, and other transient UI state.
- Backend-owned run state must come from Stage `04` services.
- Scheduler timelines, diagnostics, I/O state, and Library activity shown in
  the shell must come from ledger-derived projections, not raw diagnostic
  event rows.
- Existing graph and drawing functionality may be relocated or retired as part
  of the workbench shell cutover.
- Toolbar and page controls must meet frontend and accessibility standards.

### Assumptions

- A lightweight in-app route/page store is sufficient unless the repo already
  adopts a router during implementation.
- Node Editor can start as a reserved page/route with an explicit future/disabled
  state.
- The drawing-to-Svelte tool can be moved to a secondary page, converted into a
  Library/Node Editor tool, or retired if it no longer fits the workbench.

### Dependencies

- Stage `04` frontend services and active-run DTOs.
- Existing `src/App.svelte`, `WorkflowGraph.svelte`,
  `DiagnosticsPage.svelte`, drawing feature modules,
  and design-system components.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Shell refactor leaves duplicate old/new navigation models. | Medium | Choose one workbench shell and explicitly relocate or retire old surfaces. |
| Active-run state leaks into backend-owned data. | Medium | Keep active-run as selected id/context only; fetch details from backend services. |
| Navigation becomes mode-like and confusing. | Medium | Model pages as top-level workspaces with no hidden mode semantics. |
| Toolbar lacks accessible names/keyboard behavior. | Medium | Use semantic buttons/links and role/name-based tests. |

## Definition of Done

- App opens to Scheduler page.
- Top-level navigation includes Scheduler, Diagnostics, Graph, I/O Inspector,
  Library, Network, and Node Editor.
- Selecting a run updates transient active-run context.
- Page switching preserves active-run context for the current GUI session.
- No-active-run states are available.
- Existing graph/drawing entry points are either relocated into the workbench
  or intentionally retired with tests/docs updated.
- Old canvas/workflow mode toggles, conflicting shortcuts, and lifecycle hooks
  are removed or re-owned by the new shell.
- Frontend lint, typecheck, focused tests, and accessibility checks pass.

## Milestones

### Milestone 1: Shell Contract And Route Model

**Goal:** Define page/navigation state without changing page bodies.

**Tasks:**

- [x] Define page ids and navigation order.
- [x] Add transient active-run store and selected page store.
- [x] Define active-run top-bar summary model.
- [x] Define no-active-run behavior per page.
- [x] Decide whether drawing-to-Svelte becomes a workbench page/tool or is
  retired.

**Verification:**

- Store unit tests cover page switching and active-run persistence during the
  current session only.
- Typecheck passes for shell contracts.

**Status:** Complete. `src/stores/workbenchStore.ts` defines the page ids,
navigation order, selected page store, and active-run summary context used by
the shell. Reserved pages provide explicit no-active-run states, and the
drawing-to-Svelte startup surface is retired from root navigation.

### Milestone 2: App Shell Refactor

**Goal:** Replace the root canvas/workflow toggle as the organizing layout with
the workbench shell.

**Tasks:**

- [x] Extract current canvas/workflow surfaces into route/page components if
  needed.
- [x] Add workbench layout with toolbar/rail, top bar, main content region, and
  optional contextual drawer/bottom events area.
- [x] Set Scheduler as default page.
- [x] Add accessible navigation controls with labels and selected state.
- [x] Preserve keyboard behavior that still applies, and remove or remap
  obsolete global shortcuts.

**Verification:**

- Frontend tests cover default page and navigation.
- Accessibility lint/checks cover toolbar controls.
- Existing graph interaction tests still pass or are updated for new shell
  ownership.

**Status:** Complete. `src/App.svelte` now mounts `WorkbenchShell.svelte`
instead of switching between canvas and workflow modes. Graph editing is a
workbench page, Scheduler is the default page, and obsolete root mode shortcuts
were removed.

### Milestone 3: Active Run Wiring

**Goal:** Make page context follow the selected run without persisting it across
restart.

**Tasks:**

- [x] Wire Scheduler row selection to active-run store.
- [x] Add active-run summary in top bar.
- [x] Pass active-run id/context to page shells.
- [x] Add clear active run action if supported by UX.
- [x] Ensure page refresh/startup has no selected active run.

**Verification:**

- Tests cover selecting a run, switching pages, and clearing/no restart
  persistence.
- Tests cover pages receiving no-active-run versus active-run props/state.

**Status:** Complete for the shell slice. Scheduler rows set active-run
context, page switching preserves it during the current GUI session, and
startup still initializes with no selected run.

### Milestone 4: Legacy Surface Migration

**Goal:** Remove duplicate shell semantics by relocating or retiring old
surfaces.

**Tasks:**

- [x] Move drawing-to-Svelte surface under an explicit workbench page/tool if
  it remains in the app, otherwise retire it.
- [x] Move existing workflow graph view into Graph page shell.
- [x] Move diagnostics ownership into the projection-backed Diagnostics page
  and retire legacy panel startup from the active shell lifecycle.
- [x] Update README documentation for new frontend shell ownership.

**Verification:**

- Existing frontend tests for drawing, graph, and diagnostics pass or have
  targeted updates.
- README updates satisfy documentation standards for changed directories.

**Status:** Complete for root-shell ownership. The old drawing-to-Svelte
startup surface is retired from app-root navigation, and the app no longer
starts the legacy diagnostics panel store during root mount. The retired
drawing implementation files and legacy diagnostics panel files remain for
separate cleanup or future reuse under standards-compliant tool surfaces.

## Ownership And Lifecycle Note

The app shell owns navigation state and active-run selection only. Run details,
queue state, scheduler timelines, graph versions, retention state, and audit
summaries remain owned by backend projections and frontend service caches. The
shell must not consume or interpret raw diagnostic ledger events.

## Re-Plan Triggers

- Existing app shortcuts conflict with top-level page navigation.
- Drawing-to-Svelte cannot be cleanly relocated or retired inside the selected
  implementation slice.
- A routing library becomes necessary instead of a lightweight page store.
- Existing global stores cannot be cleanly separated from App.svelte.

## Completion Summary

### Completed

- Added `src/stores/workbenchStore.ts` as the transient frontend owner for
  selected workbench page and active workflow run context.
- Added store tests covering reserved page order, page normalization,
  page-switch persistence of the current active run, and explicit active-run
  clearing.
- Documented the workbench store boundary in `src/stores/README.md`.
- Added `src/components/workbench/WorkbenchShell.svelte` as the Scheduler-first
  root shell with accessible page navigation and active-run summary.
- Added Scheduler, Graph, Diagnostics, I/O Inspector, Library, Network, and
  Node Editor workbench page wrappers.
- Wired Scheduler run-list projection rows to the active-run store and page
  handoff actions.
- Mounted the existing workflow graph under the Graph page and moved
  diagnostics rendering to the projection-backed Diagnostics page.
- Removed the root diagnostics-store startup hook and graph-toolbar diagnostics
  panel toggle so diagnostics are no longer a duplicate shell lifecycle.
- Removed `src/stores/viewModeStore.ts` and the root canvas/workflow mode
  shortcut path from `src/App.svelte`.
- Updated source READMEs and architecture metadata for workbench ownership.

### Deviations

- The Network page received a small local-status projection view during the
  shell slice because the local network status service already exists and gives
  the reserved page a typed backend-owned data path.
- The drawing-to-Svelte startup surface was retired from root navigation rather
  than relocated into Node Editor. Its files are intentionally left in place
  until a later cleanup or reuse decision can remove them without mixing that
  deletion into the shell cutover.

### Follow-Ups

- Fill in the I/O Inspector gallery and retention controls in the dedicated
  I/O page stage.
- Fill in Library usage and Pumas-backed Library actions in the Library page
  stage.
- Decide whether retired drawing-to-Svelte implementation files should be
  deleted or reused under a future Node Editor tool.

### Verification Summary

- `node --experimental-strip-types --test src/stores/workbenchStore.test.ts`
  passed.
- `npm run test:frontend` passed.
- `npm run typecheck` passed.
- `npm run lint:critical` passed after the workbench shell cutover.
- `npm run lint:a11y` passed after the workbench shell cutover.
- `npm run build` passed after the workbench shell cutover.
- `git diff --check` passed for the staged slices.

### Traceability Links

- Requirement sections: Top-Level GUI Pages, Default Landing Page, Active Run
  Navigation Model.
