import test from 'node:test';
import assert from 'node:assert/strict';
import {
  subscribeDiagnosticsProjectionInvalidations,
  WORKFLOW_DIAGNOSTICS_PROJECTION_INVALIDATED_EVENT,
  type WorkflowProjectionInvalidationListener,
} from './WorkflowProjectionSubscriptionService.ts';
import type {
  DiagnosticsProjectionInvalidation,
  DiagnosticsProjectionInvalidationEvent,
} from '../diagnostics/types.ts';

function invalidation(
  projection_kind: DiagnosticsProjectionInvalidation['projection_kind'],
  workflow_run_id: string | null,
  last_event_seq: number,
): DiagnosticsProjectionInvalidation {
  return {
    projection_kind,
    workflow_run_id,
    workflow_id: 'wf-a',
    last_event_seq,
    reason: 'explicit_refresh',
    updated_at_ms: last_event_seq * 10,
  };
}

function createListenerHarness() {
  let handler: ((event: { payload: DiagnosticsProjectionInvalidationEvent }) => void) | null = null;
  const calls: string[] = [];
  const listenEvent: WorkflowProjectionInvalidationListener = async (eventName, nextHandler) => {
    calls.push(eventName);
    handler = nextHandler as (event: { payload: DiagnosticsProjectionInvalidationEvent }) => void;
    return () => {
      calls.push('unlisten');
      handler = null;
    };
  };
  return {
    calls,
    listenEvent,
    emit(payload: DiagnosticsProjectionInvalidationEvent) {
      assert.ok(handler, 'listener must be installed before emitting');
      handler({ payload });
    },
  };
}

async function flushMicrotasks(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

test('subscribeDiagnosticsProjectionInvalidations filters and coalesces events', async () => {
  const harness = createListenerHarness();
  const refreshed: DiagnosticsProjectionInvalidation[] = [];
  const unlisten = await subscribeDiagnosticsProjectionInvalidations(
    {
      projections: ['run_detail'],
      getActiveRunId: () => 'run-a',
      refresh: (event) => {
        refreshed.push(event);
      },
    },
    harness.listenEvent,
  );

  assert.deepEqual(harness.calls, [WORKFLOW_DIAGNOSTICS_PROJECTION_INVALIDATED_EVENT]);
  harness.emit({
    invalidations: [
      invalidation('run_detail', 'run-a', 1),
      invalidation('run_detail', 'run-a', 3),
      invalidation('run_detail', 'run-b', 4),
      invalidation('run_list', null, 5),
    ],
  });
  await flushMicrotasks();

  assert.equal(refreshed.length, 1);
  assert.equal(refreshed[0].projection_kind, 'run_detail');
  assert.equal(refreshed[0].workflow_run_id, 'run-a');
  assert.equal(refreshed[0].last_event_seq, 3);

  unlisten();
  assert.deepEqual(harness.calls, [
    WORKFLOW_DIAGNOSTICS_PROJECTION_INVALIDATED_EVENT,
    'unlisten',
  ]);
});

test('subscribeDiagnosticsProjectionInvalidations drops queued events after unsubscribe', async () => {
  const harness = createListenerHarness();
  const refreshed: DiagnosticsProjectionInvalidation[] = [];
  const unlisten = await subscribeDiagnosticsProjectionInvalidations(
    {
      projections: ['io_artifact'],
      refresh: (event) => {
        refreshed.push(event);
      },
    },
    harness.listenEvent,
  );

  harness.emit({
    invalidations: [invalidation('io_artifact', 'run-a', 1)],
  });
  unlisten();
  await flushMicrotasks();

  assert.deepEqual(refreshed, []);
});

test('subscribeDiagnosticsProjectionInvalidations reports refresh errors', async () => {
  const harness = createListenerHarness();
  const error = new Error('refresh failed');
  const errors: unknown[] = [];
  await subscribeDiagnosticsProjectionInvalidations(
    {
      projections: ['run_list'],
      refresh: () => {
        throw error;
      },
      onRefreshError: (reported) => {
        errors.push(reported);
      },
    },
    harness.listenEvent,
  );

  harness.emit({
    invalidations: [invalidation('run_list', null, 1)],
  });
  await flushMicrotasks();

  assert.deepEqual(errors, [error]);
});

test('subscribeDiagnosticsProjectionInvalidations leaves initial snapshots to page owners', async () => {
  const harness = createListenerHarness();
  const refreshed: DiagnosticsProjectionInvalidation[] = [];
  let snapshotRefreshes = 0;
  const refreshSnapshot = async () => {
    snapshotRefreshes += 1;
  };

  await refreshSnapshot();
  await subscribeDiagnosticsProjectionInvalidations(
    {
      projections: ['run_detail'],
      getActiveRunId: () => 'run-a',
      refresh: (event) => {
        refreshed.push(event);
      },
    },
    harness.listenEvent,
  );
  await flushMicrotasks();

  assert.equal(snapshotRefreshes, 1);
  assert.deepEqual(refreshed, []);

  await refreshSnapshot();
  assert.equal(snapshotRefreshes, 2);
});

test('subscribeDiagnosticsProjectionInvalidations lets manual refresh recover missed events', async () => {
  const harness = createListenerHarness();
  const refreshed: DiagnosticsProjectionInvalidation[] = [];
  let manualRefreshes = 0;
  const manualRefresh = async () => {
    manualRefreshes += 1;
  };

  await subscribeDiagnosticsProjectionInvalidations(
    {
      projections: ['run_detail'],
      getActiveRunId: () => 'run-a',
      refresh: (event) => {
        refreshed.push(event);
      },
    },
    harness.listenEvent,
  );

  await manualRefresh();
  await flushMicrotasks();

  assert.equal(manualRefreshes, 1);
  assert.deepEqual(refreshed, []);

  harness.emit({
    invalidations: [invalidation('run_detail', 'run-a', 8)],
  });
  await flushMicrotasks();

  assert.equal(manualRefreshes, 1);
  assert.equal(refreshed.length, 1);
  const delivered = refreshed[0] as DiagnosticsProjectionInvalidation | undefined;
  assert.ok(delivered);
  assert.equal(delivered.last_event_seq, 8);
});
