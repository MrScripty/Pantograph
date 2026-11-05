# Frontend stores

## Purpose

Svelte stores for state the frontend owns: which workbench page is shown, the
active run shared across pages, page-local filters, panel state, and the
editor's graph session. Backend facts are not stored here as authority.

## Contents

| File | Responsibility |
| --- | --- |
| `workbenchStore.ts` | Workbench page registry and order (tab groups), selected page, active run context, Diagnostics and Settings focus targets. |
| `schedulerRunListStore.ts` | Scheduler filters (search, status segment, the 14 disclosure filters, sort) and view preferences (`showLanes`); the run rows come from the shell's `runContextRunList`. |
| `panelStore.ts` | Assistant side panel open state and width. |
| `graphSessionStore.ts`, `workflowStore.ts`, `storeInstances.ts` | Editable graph session and the store instances shared with `@pantograph/svelte-graph`. |
| `*.test.ts` | Node test runner tests for the pure transition functions. |

## Problem

Several pages need the same transient selection (the active run) and the same
navigation state, while each page keeps its own filters. Without one owner per
piece of state, pages would copy it and drift.

## Constraints

- ADR-014: the active run is transient frontend state shared across pages and
  is not persisted across restarts.
- Store modules export pure `with*` transition functions so behaviour can be
  tested without a DOM; writable stores are wrapped in read-only exports plus
  named actions.

## Decision

Each store owns one concern and exposes read-only stores plus actions.
`workbenchStore.ts` owns the page registry: `WORKBENCH_PAGES` lists every page
with its tab group (`sessions`, `runs`, `workflows`, `settings`), and the shell
renders tabs from it. The heading set of this README is required by
`scripts/check-decision-traceability.sh` (issue WR-001 in
`docs/plans/workbench-redesign/issues.md`).

## Alternatives Rejected

- Persisting the active run to local storage: ADR-014 keeps it transient so a
  restart never opens on a stale run.
- Hard-coding the tab bar in the shell: the page list would exist twice.

## Invariants

- Page order is Sessions, Scheduler, Diagnostics, I/O Inspector, Network,
  Graph, Node Editor, Library, Settings; Scheduler is the default page.
- Unknown page ids normalise to Scheduler.
- Changing the active run clears a Diagnostics focus that belongs to another
  run.

## Revisit Triggers

- A page is added, removed or regrouped in the tab bar.
- Run selection needs to survive a restart (would require superseding
  ADR-014).

## Dependencies

**Internal:** `src/services/diagnostics/types.ts` (run-list status types).

**External:** `svelte/store`.

## Related ADRs

- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`

## Usage Examples

```ts
import {
  selectActiveWorkflowRun,
  setWorkbenchPage,
  workbenchPagesInGroup,
} from './workbenchStore';

setWorkbenchPage('diagnostics');
selectActiveWorkflowRun({ workflow_run_id: 'run_7c9e…', workflow_id: 'ingest-docs-rag', status: 'running' });
const runTabs = workbenchPagesInGroup('runs').map((page) => page.label);
```
