import test from 'node:test';
import assert from 'node:assert/strict';

import {
  claimWorkflowExecutionIdFromEvent,
  getWorkflowEventExecutionId,
  isWorkflowEventRelevantToExecution,
  projectWorkflowEventOwnership,
} from './workflowEventOwnership.ts';

test('getWorkflowEventExecutionId returns the event execution id when present', () => {
  assert.equal(
    getWorkflowEventExecutionId({
      data: {
        execution_id: 'run-1',
      },
    }),
    'run-1',
  );
});

test('projectWorkflowEventOwnership exposes event identity active identity and relevance', () => {
  assert.deepEqual(
    projectWorkflowEventOwnership(
      {
        data: {
          execution_id: 'run-2',
        },
      },
      'run-1',
    ),
    {
      eventExecutionId: 'run-2',
      activeExecutionId: 'run-1',
      relevant: false,
    },
  );
});

test('projectWorkflowEventOwnership claims the backend event id when no run is pinned', () => {
  assert.deepEqual(
    projectWorkflowEventOwnership(
      {
        data: {
          execution_id: 'run-1',
        },
      },
      null,
    ),
    {
      eventExecutionId: 'run-1',
      activeExecutionId: 'run-1',
      relevant: true,
    },
  );
});

test('isWorkflowEventRelevantToExecution accepts all events when no execution is pinned', () => {
  assert.equal(
    isWorkflowEventRelevantToExecution(
      {
        data: {
          execution_id: 'run-1',
        },
      },
      null,
    ),
    true,
  );
});

test('isWorkflowEventRelevantToExecution accepts matching execution ids', () => {
  assert.equal(
    isWorkflowEventRelevantToExecution(
      {
        data: {
          execution_id: 'session-1',
        },
      },
      'session-1',
    ),
    true,
  );
});

test('isWorkflowEventRelevantToExecution rejects mismatched execution ids', () => {
  assert.equal(
    isWorkflowEventRelevantToExecution(
      {
        data: {
          execution_id: 'session-2',
        },
      },
      'session-1',
    ),
    false,
  );
});

test('isWorkflowEventRelevantToExecution keeps events without execution ids', () => {
  assert.equal(
    isWorkflowEventRelevantToExecution(
      {
        data: {},
      },
      null,
    ),
    true,
  );
});

test('isWorkflowEventRelevantToExecution rejects events without execution ids after pinning', () => {
  assert.equal(
    isWorkflowEventRelevantToExecution(
      {
        data: {},
      },
      'session-1',
    ),
    false,
  );
});

test('claimWorkflowExecutionIdFromEvent pins the started execution id for transient runs', () => {
  assert.equal(
    claimWorkflowExecutionIdFromEvent(
      {
        type: 'Started',
        data: {
          execution_id: 'run-9',
        },
      },
      null,
    ),
    'run-9',
  );
});

test('claimWorkflowExecutionIdFromEvent pins the first execution-scoped event id', () => {
  assert.equal(
    claimWorkflowExecutionIdFromEvent(
      {
        type: 'GraphModified',
        data: {
          execution_id: 'run-graph-1',
        },
      },
      null,
    ),
    'run-graph-1',
  );
});

test('claimWorkflowExecutionIdFromEvent ignores events without an execution id', () => {
  assert.equal(
    claimWorkflowExecutionIdFromEvent(
      {
        type: 'SchedulerSnapshot',
        data: {},
      },
      null,
    ),
    null,
  );
});

test('claimWorkflowExecutionIdFromEvent does not replace an existing execution id', () => {
  assert.equal(
    claimWorkflowExecutionIdFromEvent(
      {
        type: 'Started',
        data: {
          execution_id: 'run-9',
        },
      },
      'run-8',
    ),
    'run-8',
  );
});
