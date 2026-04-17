import test from 'node:test';
import assert from 'node:assert/strict';

import {
  claimWorkflowExecutionIdFromEvent,
  getWorkflowEventExecutionId,
  isWorkflowEventRelevantToExecution,
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
