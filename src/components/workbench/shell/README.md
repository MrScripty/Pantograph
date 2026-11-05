# Workbench shell: run context

## Purpose

Owns the single control that chooses which run every workbench tab shows: the
header run chip and its popover, the run list it offers, and the Ctrl/Cmd+C,
Ctrl/Cmd+V shortcuts for run ids.

## Contents

| File | Responsibility |
| --- | --- |
| `RunContextPicker.svelte` | Chip, popover (search, grouped rows), copy toast. Mounted once by `WorkbenchShell.svelte`; owns the lifecycle of the sync and clipboard listeners. |
| `runContextPicker.ts` | Pure helpers: status to group, search and paste resolution, row decoration, chip content, engine line text. |
| `runContextRunList.ts` | Run-list source with current-invocation authority (generation token) and typed refresh outcomes. |
| `runContextClipboard.ts` | Delegated hover tracking over `data-run-id` elements and the document keydown handler for copy and paste. |
| `runContextStore.ts` | Shared picker state and the actions pages call; wires the run-list source to `workflowService.queryRunList`. |

## Problem

The redesign makes the selected run the context of every tab, shown once in
the header. Selecting, clearing, pasting and copying run ids must behave the
same from any tab, and the run list behind the picker must come from the
scheduler projection rather than ledger rows.

## Constraints

- ADR-014: the active run is transient state in `workbenchStore`; the picker
  only calls `selectActiveWorkflowRun` and `clearActiveWorkflowRun`.
- The run list comes from `workflowService.queryRunList` (the scheduler
  run-list projection), never from the diagnostics ledger.
- Listeners and timers that outlive one call have one owner and are removed on
  destroy (Coding-Standards `topic.concurrency`, TypeScript async profile).
- Keystrokes typed into inputs, selects, textareas and editable elements are
  never intercepted.

## Decision

The picker component is the owner: on mount it calls `startRunContextSync()`
(attach the run-list source, load once, subscribe to `run_list` projection
invalidations) and attaches one clipboard controller to `document`; on destroy
it detaches both and clears the toast timer. Overlapping run-list refreshes are
resolved by a generation token so only the newest result is applied; paste
reads use the same mechanism so a read that resolves after the popover closed
is not applied. The heading set of this README is required by
`scripts/check-decision-traceability.sh` (issue WR-001).

## Alternatives Rejected

- Per-element `mouseenter` handlers for copy hover: every page would have to
  wire handlers. One delegated `pointerover` listener plus a `data-run-id`
  attribute keeps pages declarative.
- Reading the run list from the Scheduler page's local state: the picker must
  work on every tab, including before Scheduler has mounted.
- `localStorage` for the copy fallback (as in the mockup): it would persist a
  run id across restarts, which ADR-014 rules out for run selection.

## Invariants

- At most one attachment of the run-list source and of the clipboard
  controller; a second `attach()` throws.
- A refresh or paste that is superseded, or completes after detach, never
  writes state.
- Transport failures keep the last projection rows and record the first
  unreachable time; backend errors keep the engine marked online.
- Selecting a run closes the popover; Escape and selection return focus to the
  chip; an outside click leaves focus where the pointer put it.

## Revisit Triggers

- The run-list projection gains a memory-wait status (add the "Waiting for
  memory" group; issue WR-004).
- The Scheduler filters client-side over the 250 newest runs this store holds (WR-M1-1). Revisit when a page needs server-side filtering over older runs: either widen this store's query or give the Scheduler a filtered query while keeping one owner of refresh.
- The desktop host grants or denies clipboard permissions differently from the
  browser fallback described here.

## Dependencies

**Internal:** `src/stores/workbenchStore.ts`,
`src/services/workflow/WorkflowService.ts`,
`src/services/workflow/WorkflowProjectionSubscriptionService.ts`,
`src/services/workflow/workflowServiceErrors.ts`, `../ui/` (StatusGlyph, Toast).

**External:** `svelte` (stores, lifecycle).

## Related ADRs

- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`

## Usage Examples

```svelte
<script lang="ts">
  import { openRunContextPicker, selectRunContext } from './shell/runContextStore';
</script>

<!-- Ctrl+C over this element copies the id; nothing else to wire. -->
<span class="pg-mono" data-run-id={run.workflow_run_id}>{run.workflow_run_id}</span>

<button class="pg-btn pg-btn-primary" type="button" onclick={() => openRunContextPicker()}>Pick a run</button>
<button class="pg-btn" type="button" onclick={() => selectRunContext(run)}>View this run</button>
```
