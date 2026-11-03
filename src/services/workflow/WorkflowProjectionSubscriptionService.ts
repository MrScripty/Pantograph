import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getWorkflowBrowserHost } from './workflowBrowserTransport.ts';
import type {
  DiagnosticsProjectionInvalidation,
  DiagnosticsProjectionInvalidationEvent,
  DiagnosticsProjectionKind,
} from '../diagnostics/types.ts';

export const WORKFLOW_DIAGNOSTICS_PROJECTION_INVALIDATED_EVENT =
  'workflow://diagnostics/projection-invalidated';

export type WorkflowProjectionInvalidationListener = <Payload>(
  eventName: string,
  handler: (event: { payload: Payload }) => void,
) => Promise<UnlistenFn>;

export interface DiagnosticsProjectionSubscriptionOptions {
  projections: DiagnosticsProjectionKind[];
  getActiveRunId?: () => string | null;
  refresh: (event: DiagnosticsProjectionInvalidation) => Promise<void> | void;
  onRefreshError?: (error: unknown) => void;
}

export async function subscribeDiagnosticsProjectionInvalidations(
  options: DiagnosticsProjectionSubscriptionOptions,
  listenEvent?: WorkflowProjectionInvalidationListener,
): Promise<UnlistenFn> {
  if (!listenEvent && getWorkflowBrowserHost()) return pollDiagnosticsProjections(options);
  const listener = listenEvent ?? listen;
  const projections = new Set(options.projections);
  const pending = new Map<string, DiagnosticsProjectionInvalidation>();
  let scheduled = false;
  let closed = false;

  const flush = async () => {
    scheduled = false;
    const events = [...pending.values()];
    pending.clear();
    for (const event of events) {
      if (closed || !matchesActiveRun(event, options.getActiveRunId)) continue;
      try {
        await options.refresh(event);
      } catch (error) {
        options.onRefreshError?.(error);
      }
    }
  };

  const scheduleFlush = () => {
    if (scheduled) return;
    scheduled = true;
    queueMicrotask(() => {
      void flush();
    });
  };

  const unlisten = await listener<DiagnosticsProjectionInvalidationEvent>(
    WORKFLOW_DIAGNOSTICS_PROJECTION_INVALIDATED_EVENT,
    (event) => {
      for (const invalidation of normalizeInvalidations(event.payload)) {
        if (!projections.has(invalidation.projection_kind)) continue;
        if (!matchesActiveRun(invalidation, options.getActiveRunId)) continue;
        pending.set(invalidationKey(invalidation), invalidation);
      }
      if (pending.size > 0) scheduleFlush();
    },
  );

  return () => {
    closed = true;
    pending.clear();
    unlisten();
  };
}

function normalizeInvalidations(
  payload: DiagnosticsProjectionInvalidationEvent,
): DiagnosticsProjectionInvalidation[] {
  if (!payload || !Array.isArray(payload.invalidations)) return [];
  return payload.invalidations.filter((event) => typeof event?.projection_kind === 'string');
}

function matchesActiveRun(
  invalidation: DiagnosticsProjectionInvalidation,
  getActiveRunId: DiagnosticsProjectionSubscriptionOptions['getActiveRunId'],
): boolean {
  const activeRunId = getActiveRunId?.() ?? null;
  return !activeRunId || !invalidation.workflow_run_id || invalidation.workflow_run_id === activeRunId;
}

function invalidationKey(invalidation: DiagnosticsProjectionInvalidation): string {
  return [
    invalidation.projection_kind,
    invalidation.workflow_run_id ?? '',
    invalidation.workflow_id ?? '',
  ].join('\u{1f}');
}

/** Browser hosts refresh projections through their real query APIs, without native IPC. */
export function pollDiagnosticsProjections(
  options: DiagnosticsProjectionSubscriptionOptions,
  intervalMs = 2000,
): UnlistenFn {
  let closed = false;
  let timer: ReturnType<typeof setTimeout>;
  const refresh = async () => {
    for (const projection of new Set(options.projections)) {
      if (closed) return;
      try {
        await options.refresh({
          projection_kind: projection,
          workflow_run_id: options.getActiveRunId?.() ?? null,
          // Explicit refresh carries no fabricated ledger sequence or backend event.
          last_event_seq: 0,
          reason: 'explicit_refresh',
          updated_at_ms: Date.now(),
        });
      } catch (error) {
        if (!closed) options.onRefreshError?.(error);
      }
    }
    if (!closed) timer = setTimeout(() => { void refresh(); }, intervalMs);
  };
  timer = setTimeout(() => { void refresh(); }, intervalMs);
  return () => {
    closed = true;
    clearTimeout(timer);
  };
}
