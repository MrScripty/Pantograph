import test from 'node:test';
import assert from 'node:assert/strict';

import {
  claimWorkflowRunIdFromEvent,
  getWorkflowEventWorkflowRunId,
  isWorkflowEventRelevantToWorkflowRun,
  projectWorkflowEventOwnership,
} from './workflowEventOwnership.ts';

test('getWorkflowEventWorkflowRunId returns the event workflow run id when present', () => {
  assert.equal(
    getWorkflowEventWorkflowRunId({
      data: {
        workflow_run_id: 'run-1',
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
          workflow_run_id: 'run-2',
        },
      },
      'run-1',
    ),
    {
      eventWorkflowRunId: 'run-2',
      activeWorkflowRunId: 'run-1',
      relevant: false,
    },
  );
});

test('projectWorkflowEventOwnership claims the backend event id when no run is pinned', () => {
  assert.deepEqual(
    projectWorkflowEventOwnership(
      {
        data: {
          workflow_run_id: 'run-1',
        },
      },
      null,
    ),
    {
      eventWorkflowRunId: 'run-1',
      activeWorkflowRunId: 'run-1',
      relevant: true,
    },
  );
});

test('projectWorkflowEventOwnership prefers backend-authored ownership', () => {
  assert.deepEqual(
    projectWorkflowEventOwnership(
      {
        data: {
          workflow_run_id: 'legacy-run',
          ownership: {
            eventWorkflowRunId: 'backend-run',
            activeWorkflowRunId: 'backend-run',
            relevant: true,
          },
        },
      },
      null,
    ),
    {
      eventWorkflowRunId: 'backend-run',
      activeWorkflowRunId: 'backend-run',
      relevant: true,
    },
  );
});

test('projectWorkflowEventOwnership trusts backend ownership relevance', () => {
  assert.deepEqual(
    projectWorkflowEventOwnership(
      {
        data: {
          ownership: {
            eventWorkflowRunId: 'run-2',
            activeWorkflowRunId: 'run-2',
            relevant: true,
          },
        },
      },
      'run-1',
    ),
    {
      eventWorkflowRunId: 'run-2',
      activeWorkflowRunId: 'run-2',
      relevant: true,
    },
  );
});

test('isWorkflowEventRelevantToWorkflowRun accepts all events when no run is pinned', () => {
  assert.equal(
    isWorkflowEventRelevantToWorkflowRun(
      {
        data: {
          workflow_run_id: 'run-1',
        },
      },
      null,
    ),
    true,
  );
});

test('isWorkflowEventRelevantToWorkflowRun accepts matching workflow run ids', () => {
  assert.equal(
    isWorkflowEventRelevantToWorkflowRun(
      {
        data: {
          workflow_run_id: 'run-1',
        },
      },
      'run-1',
    ),
    true,
  );
});

test('isWorkflowEventRelevantToWorkflowRun rejects mismatched workflow run ids', () => {
  assert.equal(
    isWorkflowEventRelevantToWorkflowRun(
      {
        data: {
          workflow_run_id: 'run-2',
        },
      },
      'run-1',
    ),
    false,
  );
});

test('isWorkflowEventRelevantToWorkflowRun keeps events without workflow run ids', () => {
  assert.equal(
    isWorkflowEventRelevantToWorkflowRun(
      {
        data: {},
      },
      null,
    ),
    true,
  );
});

test('isWorkflowEventRelevantToWorkflowRun rejects events without workflow run ids after pinning', () => {
  assert.equal(
    isWorkflowEventRelevantToWorkflowRun(
      {
        data: {},
      },
      'run-1',
    ),
    false,
  );
});

test('claimWorkflowRunIdFromEvent pins the started workflow run id for transient runs', () => {
  assert.equal(
    claimWorkflowRunIdFromEvent(
      {
        type: 'Started',
        data: {
          workflow_run_id: 'run-9',
        },
      },
      null,
    ),
    'run-9',
  );
});

test('claimWorkflowRunIdFromEvent pins the first run-scoped event id', () => {
  assert.equal(
    claimWorkflowRunIdFromEvent(
      {
        type: 'GraphModified',
        data: {
          workflow_run_id: 'run-graph-1',
        },
      },
      null,
    ),
    'run-graph-1',
  );
});

test('claimWorkflowRunIdFromEvent ignores events without a workflow run id', () => {
  assert.equal(
    claimWorkflowRunIdFromEvent(
      {
        type: 'SchedulerSnapshot',
        data: {},
      },
      null,
    ),
    null,
  );
});

test('claimWorkflowRunIdFromEvent does not replace an existing workflow run id', () => {
  assert.equal(
    claimWorkflowRunIdFromEvent(
      {
        type: 'Started',
        data: {
          workflow_run_id: 'run-9',
        },
      },
      'run-8',
    ),
    'run-8',
  );
});
