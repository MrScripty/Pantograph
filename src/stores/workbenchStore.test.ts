import test from 'node:test';
import assert from 'node:assert/strict';

import {
  DEFAULT_WORKBENCH_STATE,
  WORKBENCH_PAGE_IDS,
  normalizeWorkbenchPageId,
  withActiveWorkflowRun,
  withSelectedWorkbenchPage,
} from './workbenchStore.ts';

test('workbench page order starts at Scheduler and includes reserved pages', () => {
  assert.deepEqual(WORKBENCH_PAGE_IDS, [
    'scheduler',
    'diagnostics',
    'graph',
    'io_inspector',
    'library',
    'network',
    'node_lab',
  ]);
});

test('normalizeWorkbenchPageId falls back to Scheduler for unknown values', () => {
  assert.equal(normalizeWorkbenchPageId('graph'), 'graph');
  assert.equal(normalizeWorkbenchPageId('canvas'), 'scheduler');
  assert.equal(normalizeWorkbenchPageId(null), 'scheduler');
});

test('withSelectedWorkbenchPage changes pages without clearing active run', () => {
  const selected = withActiveWorkflowRun(
    DEFAULT_WORKBENCH_STATE,
    {
      workflow_run_id: 'run-a',
      workflow_id: 'workflow-a',
      workflow_version_id: 'wfver-a',
      workflow_semantic_version: '1.0.0',
      status: 'running',
    },
    100,
  );

  const next = withSelectedWorkbenchPage(selected, 'io_inspector');

  assert.equal(next.selected_page_id, 'io_inspector');
  assert.equal(next.active_run?.workflow_run_id, 'run-a');
});

test('withActiveWorkflowRun stores transient selected run context', () => {
  const next = withActiveWorkflowRun(
    DEFAULT_WORKBENCH_STATE,
    {
      workflow_run_id: 'run-b',
      workflow_id: null,
      workflow_version_id: 'wfver-b',
      workflow_semantic_version: '2.1.0',
      status: 'completed',
    },
    200,
  );

  assert.deepEqual(next.active_run, {
    workflow_run_id: 'run-b',
    workflow_id: null,
    workflow_version_id: 'wfver-b',
    workflow_semantic_version: '2.1.0',
    status: 'completed',
    selected_at_ms: 200,
  });

  const cleared = withActiveWorkflowRun(next, null, 300);
  assert.equal(cleared.active_run, null);
  assert.equal(cleared.selected_page_id, 'scheduler');
});
