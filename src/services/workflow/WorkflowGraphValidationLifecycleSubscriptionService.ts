import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  WorkflowGraphValidationLifecycleEvent,
  WorkflowGraphValidationLifecycleTransportEvent,
} from './types.ts';

export const WORKFLOW_GRAPH_VALIDATION_LIFECYCLE_EVENT =
  'workflow://graph-validation/lifecycle-event';

export type WorkflowGraphValidationLifecycleListener = <Payload>(
  eventName: string,
  handler: (event: { payload: Payload }) => void,
) => Promise<UnlistenFn>;

export interface WorkflowGraphValidationLifecycleSubscriptionOptions {
  getActiveGraphSessionId?: () => string | null;
  handleEvent: (event: WorkflowGraphValidationLifecycleEvent) => Promise<void> | void;
  onEventError?: (error: unknown) => void;
}

export async function subscribeGraphValidationLifecycleEvents(
  options: WorkflowGraphValidationLifecycleSubscriptionOptions,
  listenEvent: WorkflowGraphValidationLifecycleListener = listen,
): Promise<UnlistenFn> {
  const lastSequenceByGraphSession = new Map<string, number>();
  let closed = false;

  const unlisten = await listenEvent<WorkflowGraphValidationLifecycleTransportEvent>(
    WORKFLOW_GRAPH_VALIDATION_LIFECYCLE_EVENT,
    (event) => {
      const lifecycleEvent = normalizeLifecycleEvent(event.payload);
      if (!lifecycleEvent) return;
      if (!matchesActiveGraphSession(lifecycleEvent, options.getActiveGraphSessionId)) return;
      const lastSequence = lastSequenceByGraphSession.get(lifecycleEvent.graph_session_id);
      if (lastSequence !== undefined && lifecycleEvent.sequence <= lastSequence) return;
      lastSequenceByGraphSession.set(lifecycleEvent.graph_session_id, lifecycleEvent.sequence);
      queueMicrotask(async () => {
        if (closed) return;
        try {
          await options.handleEvent(lifecycleEvent);
        } catch (error) {
          options.onEventError?.(error);
        }
      });
    },
  );

  return () => {
    closed = true;
    lastSequenceByGraphSession.clear();
    unlisten();
  };
}

function normalizeLifecycleEvent(
  payload: WorkflowGraphValidationLifecycleTransportEvent,
): WorkflowGraphValidationLifecycleEvent | null {
  const event = payload?.event;
  if (!event || typeof event.graph_session_id !== 'string') return null;
  if (typeof event.graph_revision !== 'string') return null;
  if (typeof event.validation_session_id !== 'string') return null;
  if (typeof event.sequence !== 'number') return null;
  if (!event.kind || typeof event.kind.kind !== 'string') return null;
  return event;
}

function matchesActiveGraphSession(
  event: WorkflowGraphValidationLifecycleEvent,
  getActiveGraphSessionId: WorkflowGraphValidationLifecycleSubscriptionOptions['getActiveGraphSessionId'],
): boolean {
  const activeGraphSessionId = getActiveGraphSessionId?.() ?? null;
  return !activeGraphSessionId || event.graph_session_id === activeGraphSessionId;
}
