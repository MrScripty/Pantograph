# Plan: Graph Editor Decomposition

## Objective

Finish the deferred standards work from graph synchronization hardening by
reducing graph editor/store duplication and oversized modules without changing
graph editing behavior or backend-owned graph state contracts.

## Scope

### In Scope

- Split remaining mixed responsibilities in `createWorkflowStores.ts` into
  focused store/action modules.
- Centralize duplicated graph backend action behavior where package and app
  graph editor paths can share contracts safely.
- Extract graph editor interaction controllers from both active
  `WorkflowGraph.svelte` paths.
- Update source-directory README files for changed ownership boundaries.
- Preserve current public facades unless a plan update explicitly records an
  API-breaking rewrite decision.

### Out of Scope

- Redesigning workflow scheduler or diagnostics identity.
- Changing backend graph mutation semantics.
- Removing either graph editor path unless implementation proves one inactive
  and a re-plan records that decision.
- Visual redesign of the graph editor.
- Changing saved workflow JSON shape beyond preserving the existing
  structural/runtime separation.

## Inputs

### Problem

The previous synchronization hardening plan corrected stale responses, batch
delete, runtime overlays, and backend-derived graph metadata, but deferred some
standards work. The active graph editor still has large Svelte components,
duplicated backend action helpers, and a large store assembler. These files are
hard to review and make graph interaction behavior drift-prone.

### Constraints

- Backend graph responses remain the only source for persistent graph mutation.
- No frontend optimistic updates for structural graph data.
- Runtime overlays remain transient and must not enter `$workflowGraph` or save
  payloads.
- New stateful flows must have one owner for lifecycle and cleanup.
- No new polling or timers unless lifecycle ownership and cleanup tests are
  added.
- Source changes must follow the standards in
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.
- Existing unrelated dirty workflow/assets/SQLite artifacts must remain
  untouched.

### Assumptions

- Both `packages/svelte-graph/src/components/WorkflowGraph.svelte` and
  `src/components/WorkflowGraph.svelte` are active until proven otherwise.
- Public package exports should remain compatible during refactor unless a
  re-plan records an intentional breaking change.
- Colocated tests remain the expected frontend testing style in this repo.

### Dependencies

- `packages/svelte-graph/src/stores/createWorkflowStores.ts`
- `packages/svelte-graph/src/stores/workflowStoreGraphState.ts`
- `packages/svelte-graph/src/components/WorkflowGraph.svelte`
- `packages/svelte-graph/src/components/workflowGraphBackendActions.ts`
- `src/components/WorkflowGraph.svelte`
- `src/components/workflowGraphBackendActions.ts`
- `packages/svelte-graph/src/stores/README.md`
- `packages/svelte-graph/src/components/README.md`
- `src/components/README.md`
- `src/stores/README.md`

### Affected Structured Contracts

- `WorkflowStores` public facade should keep existing action/store names while
  implementation moves to focused helper modules.
- Shared graph action helpers must remain session-scoped and must not apply
  backend responses without active-session validation.
- Graph component controllers may own UI interaction state, but persistent
  graph structure must still update only through accepted backend responses.

### Affected Persisted Artifacts

- No saved workflow JSON, SQLite diagnostics, or build artifact contract change
  is expected.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Component extraction changes drag/connect behavior | High | Extract pure/controller helpers first, keep Svelte event bindings thin, and run full frontend tests after each slice. |
| Shared action helper overfits one graph editor path | Medium | Use dependency injection for app-only service/session behavior and keep package helper transport-agnostic. |
| Store split breaks public package facade | Medium | Preserve `WorkflowStores` surface and move internals only. |
| Broad line-count cleanup expands beyond behavior safety | Medium | Commit focused slices and re-plan if a split requires visual behavior changes. |
| Existing unrelated dirty files obscure commit scope | Low | Stage only plan/source files owned by the current slice and inspect status before each commit. |

## Definition of Done

- `createWorkflowStores.ts` is primarily an assembler and delegates graph state,
  mutation dispatch, execution state, and group actions to focused owners.
- Both graph editor Svelte components delegate selected interaction behavior to
  tested controller/helper modules.
- Shared graph mutation behavior has one tested owner where practical.
- Any remaining duplicated active paths have a documented reason, risk, and
  follow-up owner.
- README docs reflect changed ownership boundaries.
- Final verification and release build pass.
- Only unrelated pre-existing dirty artifacts remain uncommitted.

## Milestones

### Milestone 1: Preflight And Baseline

**Goal:** Establish a standards-compliant plan and baseline before source
refactoring.

**Tasks:**
- [x] Read the applicable plan, commit, coding, frontend, testing, and
  documentation standards.
- [x] Inspect current git status and record unrelated dirty files.
- [x] Run baseline `npm run test:frontend`.
- [x] Run baseline `npm run typecheck`.
- [x] Commit this plan artifact.

**Verification:**
- `npm run test:frontend`
- `npm run typecheck`

**Status:** Complete.

### Milestone 2: Store Responsibility Split

**Goal:** Make `createWorkflowStores.ts` an assembler with focused store/action
owners.

**Tasks:**
- [x] Extract backend mutation dispatch and stale-session result handling.
- [x] Extract node execution-state ownership.
- [x] Extract group mutation actions.
- [x] Preserve the existing `WorkflowStores` public facade.
- [x] Update `packages/svelte-graph/src/stores/README.md`.

**Verification:**
- `node --experimental-strip-types --test packages/svelte-graph/src/stores/createWorkflowStores.test.ts`
- `npm run typecheck`

**Status:** Complete.

### Milestone 3: Shared Backend Action Boundary

**Goal:** Give duplicated graph backend action behavior one tested owner where
package and app paths can share it safely.

**Tasks:**
- [x] Create a shared package-level action helper or controller module with
  dependency injection for session lookup, graph sync, and transport calls.
- [x] Migrate common delete, edge removal, connect, insert, and reconnect
  behavior where contracts match.
- [x] Keep app-only workflow-service details app-local.
- [x] Add direct tests for the shared helper boundary.

**Verification:**
- `npm run test:frontend`
- `npm run typecheck`

**Status:** Complete.

### Milestone 4: Package Graph Component Decomposition

**Goal:** Move package graph interaction logic out of the oversized Svelte
component while preserving rendering behavior.

**Tasks:**
- [ ] Extract package delete/cut interaction controller code.
- [ ] Extract package reconnect interaction controller code.
- [ ] Extract package connection-intent or horseshoe lifecycle helpers where
  behavior can be isolated without visual changes.
- [ ] Keep the component responsible for rendering and event wiring.

**Verification:**
- `npm run test:frontend`
- `npm run typecheck`

**Status:** Not started.

### Milestone 5: App Graph Component Decomposition

**Goal:** Move app-only graph interaction/source-switching logic out of the
oversized app Svelte component while reusing package helpers where practical.

**Tasks:**
- [ ] Extract app architecture/workflow graph source interaction wiring.
- [ ] Extract app delete/cut/reconnect wiring that depends on singleton stores.
- [ ] Keep app-only orchestration and architecture behavior app-local.
- [ ] Update `src/components/README.md` if ownership changes.

**Verification:**
- `npm run test:frontend`
- `npm run typecheck`

**Status:** Not started.

### Milestone 6: Documentation And Compliance Pass

**Goal:** Confirm touched areas are standards-compliant or have explicit
traceable deferrals.

**Tasks:**
- [ ] Re-run line counts and responsibility review for touched oversized files.
- [ ] Search for unchecked backend graph response application.
- [ ] Search for duplicated reconnect/delete flow that should share ownership.
- [ ] Search for runtime fields entering structural graph projection.
- [ ] Update component/store READMEs for changed boundaries.
- [ ] Record any remaining deferrals with owner, risk, and follow-up.

**Verification:**
- `git diff --check`
- `npm run test:frontend`
- `npm run typecheck`

**Status:** Not started.

### Milestone 7: Release Verification

**Goal:** Prove the refactor is shippable.

**Tasks:**
- [ ] Run full frontend tests.
- [ ] Run TypeScript typecheck.
- [ ] Run workflow-service tests.
- [ ] Run Tauri cargo check.
- [ ] Run Pantograph release build.
- [ ] Update this plan with completion summary.
- [ ] Confirm final dirty worktree contains only unrelated pre-existing files.

**Verification:**
- `git diff --check`
- `npm run test:frontend`
- `npm run typecheck`
- `cargo test -p pantograph-workflow-service`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `bash launcher.sh --build-release`

**Status:** Not started.

## Execution Notes

- 2026-04-26: Plan created. Existing unrelated dirty files at preflight:
  deleted `.pantograph/workflows/tiny-sd-turbo-diffusion.json`, deleted tracked
  image assets under `assets/`, untracked `.pantograph/workflow-diagnostics.sqlite`,
  and untracked image/reject assets. These are outside the implementation scope
  and must remain untouched.
- 2026-04-26: Milestone 1 completed. Baseline `npm run test:frontend` and
  `npm run typecheck` passed before implementation refactors.
- 2026-04-26: Milestone 2 started. The first slice extracts backend mutation
  dispatch and stale-session result handling while preserving the existing
  `WorkflowStores` public facade.
- 2026-04-26: Milestone 2 completed. `createWorkflowStores.ts` now delegates
  active-session mutation dispatch, node execution-state overlays, and
  backend-backed group actions to focused store helpers while preserving the
  public `WorkflowStores` facade. Focused store test and typecheck passed.
- 2026-04-26: Milestone 3 started. Package and app graph backend action modules
  both own accepted mutation projection, connection commit rejection projection,
  edge removal, and reconnect rollback. The safest shared boundary is a
  package-level dependency-injected action core that keeps app-only
  `WorkflowService` session lookup local.
- 2026-04-26: Milestone 3 completed. Added
  `workflowGraphBackendActionCore.ts` with direct tests and adapted both package
  and app graph backend action modules to share accepted mutation projection,
  insert/connect rejection handling, edge removal, and reconnect rollback while
  keeping Pantograph `WorkflowService` lookup app-local. `npm run test:frontend`
  and `npm run typecheck` passed.

## Commit Cadence Notes

- Commit after each verified logical milestone.
- Stage only files owned by the completed milestone.
- Use conventional commit messages with detailed bodies and `Agent: Codex`.
- Do not include verification command output or logs in commit messages.

## Optional Subagent Assignment

No subagents are planned. The refactor crosses shared app/package frontend
contracts, so serial execution is safer unless the user explicitly authorizes
parallel work.

## Re-Plan Triggers

- A shared helper requires changing the public package API unexpectedly.
- Either graph component proves inactive and can be removed instead of
  refactored.
- Extracting controllers requires visual/UI behavior changes.
- Tests expose behavior differences between app and package graph editors.
- Store split requires changing backend graph ownership semantics.
- Any touched module grows in responsibility instead of shrinking.
- Verification failures point outside graph editor/store scope.

## Completion Summary

### Completed

- Not complete.

### Deviations

- None recorded.

### Follow-Ups

- None recorded.

### Verification Summary

- Not run.

### Traceability Links

- Module README updated: pending.
- ADR added/updated: N/A unless implementation changes graph/editor ownership
  architecture beyond helper extraction.
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A.
