/**
 * The picker's view of the scheduler run-list projection.
 *
 * One instance per shell (created in runContextStore.ts). It owns:
 * - the last projection rows it received (kept while the engine is
 *   unreachable so pages and the chip keep showing the last projection);
 * - engine availability as observed through the run-list query;
 * - current-invocation authority: every refresh takes a generation number and
 *   only the newest refresh may write state. Older completions are classified
 *   `superseded`. While detached (before the shell mounts, after it is
 *   destroyed) refreshes do not run and in-flight completions are classified
 *   `detached` without writing state.
 *
 * It has no service imports so it can be tested with injected functions.
 */
import { writable, type Readable } from 'svelte/store';
import type {
  RunListProjectionRecord,
  WorkflowRunListQueryRequest,
  WorkflowRunListQueryResponse,
} from '../../../services/diagnostics/types';
import type { EngineAvailability } from './runContextPicker.ts';

export type RunListLoadStatus = 'idle' | 'loading' | 'ready' | 'failed';

export interface RunContextRunListState {
  runs: RunListProjectionRecord[];
  status: RunListLoadStatus;
  availability: EngineAvailability;
  /** When the rows in `runs` were received; null before the first success. */
  loadedAtMs: number | null;
  /** Backend failure that is not a transport failure (the engine answered). */
  errorMessage: string | null;
}

export type RunListRefreshOutcome = 'applied' | 'failed' | 'unreachable' | 'superseded' | 'detached';

export interface RunListQueryFailure {
  /** True when the transport could not reach the engine at all. */
  transport: boolean;
  message: string;
}

export interface RunContextRunListDeps {
  queryRunList: (request: WorkflowRunListQueryRequest) => Promise<WorkflowRunListQueryResponse>;
  classifyFailure: (error: unknown) => RunListQueryFailure;
  now: () => number;
  limit: number;
}

export interface RunContextRunListSource {
  state: Readable<RunContextRunListState>;
  /** Start accepting refreshes. Throws if already attached (one owner). */
  attach(): void;
  /** Stop accepting refreshes and invalidate in-flight ones. Idempotent. */
  detach(): void;
  isAttached(): boolean;
  refresh(): Promise<RunListRefreshOutcome>;
}

export const INITIAL_RUN_CONTEXT_RUN_LIST_STATE: RunContextRunListState = {
  runs: [],
  status: 'idle',
  availability: { kind: 'unknown' },
  loadedAtMs: null,
  errorMessage: null,
};

export function createRunContextRunListSource(deps: RunContextRunListDeps): RunContextRunListSource {
  const store = writable<RunContextRunListState>({ ...INITIAL_RUN_CONTEXT_RUN_LIST_STATE });
  let generation = 0;
  let attached = false;

  async function refresh(): Promise<RunListRefreshOutcome> {
    if (!attached) return 'detached';
    const invocation = ++generation;
    store.update((state) => ({ ...state, status: 'loading' }));

    let response: WorkflowRunListQueryResponse | null = null;
    let failure: RunListQueryFailure | null = null;
    try {
      response = await deps.queryRunList({ limit: deps.limit });
    } catch (error) {
      failure = deps.classifyFailure(error);
    }

    if (!attached) return 'detached';
    if (invocation !== generation) return 'superseded';

    const nowMs = deps.now();
    if (response) {
      store.set({
        runs: response.runs,
        status: 'ready',
        availability: { kind: 'online' },
        loadedAtMs: nowMs,
        errorMessage: null,
      });
      return 'applied';
    }

    const failed = failure ?? { transport: true, message: 'Run list query failed' };
    if (failed.transport) {
      store.update((state) => ({
        ...state,
        status: 'failed',
        availability: {
          kind: 'unreachable',
          sinceMs: state.availability.kind === 'unreachable' ? state.availability.sinceMs : nowMs,
          hasProjection: state.loadedAtMs !== null,
        },
        errorMessage: null,
      }));
      return 'unreachable';
    }
    store.update((state) => ({
      ...state,
      status: 'failed',
      availability: { kind: 'online' },
      errorMessage: failed.message,
    }));
    return 'failed';
  }

  return {
    state: { subscribe: store.subscribe },
    refresh,
    attach() {
      if (attached) {
        throw new Error('Run-context run list is already attached; the shell owns one attachment.');
      }
      attached = true;
    },
    detach() {
      if (!attached) return;
      attached = false;
      generation += 1;
      store.update((state) => (state.status === 'loading' ? { ...state, status: 'idle' } : state));
    },
    isAttached() {
      return attached;
    },
  };
}
