import test from 'node:test';
import assert from 'node:assert/strict';
import {
  subscribeGraphValidationLifecycleEvents,
  WORKFLOW_GRAPH_VALIDATION_LIFECYCLE_EVENT,
  type WorkflowGraphValidationLifecycleListener,
} from './WorkflowGraphValidationLifecycleSubscriptionService.ts';
import type {
  WorkflowGraphValidationLifecycleEvent,
  WorkflowGraphValidationLifecycleTransportEvent,
} from './types.ts';

function lifecycleEvent(
  graphSessionId: string,
  sequence: number,
): WorkflowGraphValidationLifecycleEvent {
  return {
    graph_session_id: graphSessionId,
    graph_revision: 'graph-revision-a',
    validation_session_id: 'validation-session-a',
    sequence,
    kind: { kind: 'validation_pending' },
  };
}

function createListenerHarness() {
  let handler:
    | ((event: { payload: WorkflowGraphValidationLifecycleTransportEvent }) => void)
    | null = null;
  const calls: string[] = [];
  const listenEvent: WorkflowGraphValidationLifecycleListener = async (eventName, nextHandler) => {
    calls.push(eventName);
    handler = nextHandler as (event: {
      payload: WorkflowGraphValidationLifecycleTransportEvent;
    }) => void;
    return () => {
      calls.push('unlisten');
      handler = null;
    };
  };
  return {
    calls,
    listenEvent,
    emit(payload: WorkflowGraphValidationLifecycleTransportEvent) {
      assert.ok(handler, 'listener must be installed before emitting');
      handler({ payload });
    },
  };
}

async function flushMicrotasks(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

test('subscribeGraphValidationLifecycleEvents filters active sessions and stale sequences', async () => {
  const harness = createListenerHarness();
  const delivered: WorkflowGraphValidationLifecycleEvent[] = [];
  const unlisten = await subscribeGraphValidationLifecycleEvents(
    {
      getActiveGraphSessionId: () => 'graph-session-a',
      handleEvent: (event) => {
        delivered.push(event);
      },
    },
    harness.listenEvent,
  );

  assert.deepEqual(harness.calls, [WORKFLOW_GRAPH_VALIDATION_LIFECYCLE_EVENT]);
  harness.emit({ event: lifecycleEvent('graph-session-a', 1) });
  harness.emit({ event: lifecycleEvent('graph-session-a', 1) });
  harness.emit({ event: lifecycleEvent('graph-session-b', 2) });
  harness.emit({ event: lifecycleEvent('graph-session-a', 3) });
  await flushMicrotasks();

  assert.deepEqual(
    delivered.map((event) => event.sequence),
    [1, 3],
  );

  unlisten();
  assert.deepEqual(harness.calls, [WORKFLOW_GRAPH_VALIDATION_LIFECYCLE_EVENT, 'unlisten']);
});

test('subscribeGraphValidationLifecycleEvents drops queued events after unsubscribe', async () => {
  const harness = createListenerHarness();
  const delivered: WorkflowGraphValidationLifecycleEvent[] = [];
  const unlisten = await subscribeGraphValidationLifecycleEvents(
    {
      handleEvent: (event) => {
        delivered.push(event);
      },
    },
    harness.listenEvent,
  );

  harness.emit({ event: lifecycleEvent('graph-session-a', 1) });
  unlisten();
  await flushMicrotasks();

  assert.deepEqual(delivered, []);
});

test('subscribeGraphValidationLifecycleEvents reports handler errors', async () => {
  const harness = createListenerHarness();
  const error = new Error('lifecycle handler failed');
  const errors: unknown[] = [];
  await subscribeGraphValidationLifecycleEvents(
    {
      handleEvent: () => {
        throw error;
      },
      onEventError: (reported) => {
        errors.push(reported);
      },
    },
    harness.listenEvent,
  );

  harness.emit({ event: lifecycleEvent('graph-session-a', 1) });
  await flushMicrotasks();

  assert.deepEqual(errors, [error]);
});
