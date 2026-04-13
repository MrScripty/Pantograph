import test from 'node:test';
import assert from 'node:assert/strict';

import { isWorkflowEventRelevantToExecution } from './workflowEventOwnership.ts';

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
      'session-1',
    ),
    true,
  );
});
