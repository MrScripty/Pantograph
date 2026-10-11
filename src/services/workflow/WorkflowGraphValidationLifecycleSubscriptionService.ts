import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getWorkflowBrowserHost } from './workflowBrowserTransport.ts';
import { invokeWorkflowCommand } from './workflowServiceErrors.ts';
import type {
  WorkflowGraphValidationLifecycleEvent,
  WorkflowGraphValidationLifecycleEventSnapshot,
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
  listenEvent?: WorkflowGraphValidationLifecycleListener,
): Promise<UnlistenFn> {
  const listener = listenEvent ?? (getWorkflowBrowserHost()
    ? browserValidationListener(options)
    : listen);
  const lastSequenceByGraphSession = new Map<string, number>();
  let closed = false;

  const unlisten = await listener<WorkflowGraphValidationLifecycleTransportEvent>(
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

function browserValidationListener(
  options: WorkflowGraphValidationLifecycleSubscriptionOptions,
): WorkflowGraphValidationLifecycleListener {
  return async <Payload>(_eventName: string, handler: (event: { payload: Payload }) => void) => {
    let closed = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      const graphSessionId = options.getActiveGraphSessionId?.();
      if (graphSessionId) {
        try {
          const snapshot = await invokeWorkflowCommand<WorkflowGraphValidationLifecycleEventSnapshot>(
            'graph_validation_lifecycle_event_snapshot', { graphSessionId },
          );
          if (!closed && options.getActiveGraphSessionId?.() === graphSessionId) {
            for (const event of snapshot.events) handler({ payload: { event } as Payload });
          }
        } catch (error) {
          if (!closed) options.onEventError?.(error);
        }
      }
      if (!closed) timer = setTimeout(() => { void poll(); }, 2000);
    };
    timer = setTimeout(() => { void poll(); }, 2000);
    return () => { closed = true; clearTimeout(timer); };
  };
}
