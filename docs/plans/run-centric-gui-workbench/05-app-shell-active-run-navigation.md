# 05: App Shell And Active Run Navigation

## Status

In progress. Milestone 1 has a frontend workbench-state contract for page ids,
navigation order, and transient active-run context. The app shell and page
bodies are not yet migrated.

## Objective

Refactor the Svelte application shell from the current canvas/workflow toggle
into a Scheduler-first workbench with top-level pages and transient active-run
context shared across Diagnostics, Graph, I/O Inspector, Library, and Network.

## Scope

### In Scope

- Top-level workbench shell.
- Default route/page set to Scheduler.
- Toolbar or rail navigation for Scheduler, Diagnostics, Graph, I/O Inspector,
  Library, Network, and Node Lab.
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
- Node Lab can start as a reserved page/route with an explicit future/disabled
  state.
- The drawing-to-Svelte tool can be moved to a secondary page, converted into a
  Library/Node Lab tool, or retired if it no longer fits the workbench.

### Dependencies

- Stage `04` frontend services and active-run DTOs.
- Existing `src/App.svelte`, `src/stores/viewModeStore.ts`,
  `WorkflowGraph.svelte`, `DiagnosticsPanel.svelte`, drawing feature modules,
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
  Library, Network, and Node Lab.
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
- [ ] Define no-active-run behavior per page.
- [ ] Decide whether drawing-to-Svelte becomes a workbench page/tool or is
  retired.

**Verification:**

- Store unit tests cover page switching and active-run persistence during the
  current session only.
- Typecheck passes for shell contracts.

**Status:** In progress. `src/stores/workbenchStore.ts` defines the page ids,
navigation order, selected page store, and active-run summary context used by
later shell work. No-active-run page behavior and legacy surface placement are
still open.

### Milestone 2: App Shell Refactor

**Goal:** Replace the root canvas/workflow toggle as the organizing layout with
the workbench shell.

**Tasks:**

- [ ] Extract current canvas/workflow surfaces into route/page components if
  needed.
- [ ] Add workbench layout with toolbar/rail, top bar, main content region, and
  optional contextual drawer/bottom events area.
- [ ] Set Scheduler as default page.
- [ ] Add accessible navigation controls with labels and selected state.
- [ ] Preserve keyboard behavior that still applies, and remove or remap
  obsolete global shortcuts.

**Verification:**

- Frontend tests cover default page and navigation.
- Accessibility lint/checks cover toolbar controls.
- Existing graph interaction tests still pass or are updated for new shell
  ownership.

**Status:** Not started.

### Milestone 3: Active Run Wiring

**Goal:** Make page context follow the selected run without persisting it across
restart.

**Tasks:**

- [ ] Wire Scheduler row selection to active-run store.
- [ ] Add active-run summary in top bar.
- [ ] Pass active-run id/context to page shells.
- [ ] Add clear active run action if supported by UX.
- [ ] Ensure page refresh/startup has no selected active run.

**Verification:**

- Tests cover selecting a run, switching pages, and clearing/no restart
  persistence.
- Tests cover pages receiving no-active-run versus active-run props/state.

**Status:** Not started.

### Milestone 4: Legacy Surface Migration

**Goal:** Remove duplicate shell semantics by relocating or retiring old
surfaces.

**Tasks:**

- [ ] Move drawing-to-Svelte surface under an explicit workbench page/tool if
  it remains in the app, otherwise retire it.
- [ ] Move existing workflow graph view into Graph page shell.
- [ ] Move existing diagnostics panel into Diagnostics page shell or embed it
  as an interim page body.
- [ ] Update README documentation for new frontend shell ownership.

**Verification:**

- Existing frontend tests for drawing, graph, and diagnostics pass or have
  targeted updates.
- README updates satisfy documentation standards for changed directories.

**Status:** Not started.

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

### Deviations

- None.

### Follow-Ups

- Decide final name for Node Lab before implementation.
- Decide whether drawing-to-Svelte becomes a page, a Library/Node Lab tool, or
  is retired.
- Define each page's no-active-run empty state before wiring page shells.
- Refactor `src/App.svelte` to consume the workbench store and retire the
  current canvas/workflow mode switch.

### Verification Summary

- `node --experimental-strip-types --test src/stores/workbenchStore.test.ts`
  passed.
- `npm run test:frontend` passed.
- `npm run typecheck` passed.
- `git diff --check` passed for the staged slice.

### Traceability Links

- Requirement sections: Top-Level GUI Pages, Default Landing Page, Active Run
  Navigation Model.
