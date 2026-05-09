import { listen, type UnlistenFn } from '@tauri-apps/api/event';
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
  listenEvent: WorkflowProjectionInvalidationListener = listen,
): Promise<UnlistenFn> {
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

  const unlisten = await listenEvent<DiagnosticsProjectionInvalidationEvent>(
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
