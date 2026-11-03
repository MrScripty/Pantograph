/**
 * Shared run-context state for the shell and pages.
 *
 * The active run itself lives in `workbenchStore` (ADR-014: transient frontend
 * state shared across pages). This module adds what the header picker needs
 * around it: whether the popover is open and what is typed in it, the run-list
 * projection the picker shows, and the actions pages call (`openRunContextPicker`,
 * `selectRunContext`, `clearRunContext`).
 */
import { writable, type Readable } from 'svelte/store';
import type { RunListProjectionRecord } from '../../../services/diagnostics/types';
import { subscribeDiagnosticsProjectionInvalidations } from '../../../services/workflow/WorkflowProjectionSubscriptionService';
import { workflowService } from '../../../services/workflow/WorkflowService';
import { normalizeWorkflowServiceError } from '../../../services/workflow/workflowServiceErrors';
import { clearActiveWorkflowRun, selectActiveWorkflowRun } from '../../../stores/workbenchStore';
import { RUN_CONTEXT_RUN_LIMIT, toActiveRunContext } from './runContextPicker.ts';
import {
  createRunContextRunListSource,
  type RunContextRunListState,
  type RunListQueryFailure,
} from './runContextRunList.ts';

export interface RunContextPickerState {
  open: boolean;
  query: string;
}

const pickerStore = writable<RunContextPickerState>({ open: false, query: '' });

export const runContextPicker: Readable<RunContextPickerState> = {
  subscribe: pickerStore.subscribe,
};

/** Open the header popover, optionally with text already in its search field. */
export function openRunContextPicker(query = ''): void {
  pickerStore.set({ open: true, query });
}

export function closeRunContextPicker(): void {
  pickerStore.set({ open: false, query: '' });
}

export function setRunContextPickerQuery(query: string): void {
  pickerStore.update((state) => ({ ...state, query }));
}

/** Make `run` the active run for every tab and close the picker. */
export function selectRunContext(run: RunListProjectionRecord): void {
  selectActiveWorkflowRun(toActiveRunContext(run));
  closeRunContextPicker();
}

/** Clear the active run; pages fall back to their empty state. */
export function clearRunContext(): void {
  clearActiveWorkflowRun();
}

export function classifyRunListFailure(error: unknown): RunListQueryFailure {
  const normalized = normalizeWorkflowServiceError(error);
  return {
    transport: normalized.code === 'transport_error',
    message: normalized.message,
  };
}

const runListSource = createRunContextRunListSource({
  queryRunList: (request) => workflowService.queryRunList(request),
  classifyFailure: classifyRunListFailure,
  now: () => Date.now(),
  limit: RUN_CONTEXT_RUN_LIMIT,
});

/** The run-list projection as the picker last received it. */
export const runContextRunList: Readable<RunContextRunListState> = runListSource.state;

/**
 * Whether `run_list` invalidation events reach the picker. `unavailable` means
 * the event channel could not be subscribed (for example, no Tauri host); the
 * list then updates only on open and on Retry, and the popover says so.
 */
export type RunContextLiveUpdates = 'stopped' | 'connecting' | 'live' | 'unavailable';

const liveUpdatesStore = writable<RunContextLiveUpdates>('stopped');

export const runContextLiveUpdates: Readable<RunContextLiveUpdates> = {
  subscribe: liveUpdatesStore.subscribe,
};

/** Re-query the run list (Retry in the engine line, popover open). */
export function refreshRunContextRunList(): Promise<unknown> {
  return runListSource.refresh();
}

/**
 * Start keeping the run list current. Owned by the shell: call once on mount
 * and call the returned function on destroy. It attaches the source, loads
 * once, and refreshes on `run_list` projection invalidations. A failed
 * subscription is published as `runContextLiveUpdates = 'unavailable'`.
 */
export function startRunContextSync(): () => void {
  runListSource.attach();
  let stopped = false;
  let unlisten: (() => void) | null = null;

  liveUpdatesStore.set('connecting');
  void runListSource.refresh();
  subscribeDiagnosticsProjectionInvalidations({
    projections: ['run_list'],
    refresh: async () => {
      await runListSource.refresh();
    },
  })
    .then((nextUnlisten) => {
      if (stopped) {
        nextUnlisten();
        return;
      }
      unlisten = nextUnlisten;
      liveUpdatesStore.set('live');
    })
    .catch(() => {
      if (!stopped) liveUpdatesStore.set('unavailable');
    });

  return () => {
    stopped = true;
    unlisten?.();
    unlisten = null;
    runListSource.detach();
    liveUpdatesStore.set('stopped');
    closeRunContextPicker();
  };
}
