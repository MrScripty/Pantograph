import test from 'node:test';
import assert from 'node:assert/strict';

import {
  DEFAULT_WORKBENCH_STATE,
  WORKBENCH_PAGE_IDS,
  WORKBENCH_PAGES,
  WORKBENCH_TAB_GROUPS,
  normalizeWorkbenchPageId,
  workbenchPagesInGroup,
  withDiagnosticsFocus,
  withSettingsFocus,
  withActiveWorkflowRun,
  withSelectedWorkbenchPage,
} from './workbenchStore.ts';

test('workbench page order follows the tab bar with Sessions first', () => {
  assert.deepEqual(WORKBENCH_PAGE_IDS, [
    'sessions',
    'scheduler',
    'diagnostics',
    'io_inspector',
    'network',
    'graph',
    'node_lab',
    'library',
    'settings',
  ]);
  assert.deepEqual(
    WORKBENCH_PAGES.map((page) => page.id),
    [...WORKBENCH_PAGE_IDS],
  );
});

test('Scheduler stays the default page after Sessions is added', () => {
  assert.equal(DEFAULT_WORKBENCH_STATE.selected_page_id, 'scheduler');
  assert.equal(normalizeWorkbenchPageId('sessions'), 'sessions');
  assert.equal(
    WORKBENCH_PAGES.find((page) => page.id === 'sessions')?.label,
    'Sessions',
  );
});

test('tab groups partition every page except the pinned Settings tab', () => {
  assert.deepEqual(WORKBENCH_TAB_GROUPS, ['sessions', 'runs', 'workflows']);
  assert.deepEqual(
    workbenchPagesInGroup('sessions').map((page) => page.id),
    ['sessions'],
  );
  assert.deepEqual(
    workbenchPagesInGroup('runs').map((page) => page.id),
    ['scheduler', 'diagnostics', 'io_inspector', 'network'],
  );
  assert.deepEqual(
    workbenchPagesInGroup('workflows').map((page) => page.id),
    ['graph', 'node_lab', 'library'],
  );
  assert.deepEqual(
    workbenchPagesInGroup('settings').map((page) => page.id),
    ['settings'],
  );
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

test('clearing active run preserves the selected workbench page', () => {
  const selected = withSelectedWorkbenchPage(
    withActiveWorkflowRun(
      DEFAULT_WORKBENCH_STATE,
      {
        workflow_run_id: 'run-c',
        workflow_id: 'workflow-c',
        workflow_version_id: 'wfver-c',
        workflow_semantic_version: '3.0.0',
        status: 'running',
      },
      400,
    ),
    'diagnostics',
  );

  const cleared = withActiveWorkflowRun(selected, null, 500);

  assert.equal(cleared.active_run, null);
  assert.equal(cleared.selected_page_id, 'diagnostics');
});

test('withDiagnosticsFocus selects diagnostics and stores focused event context', () => {
  const selected = withActiveWorkflowRun(
    DEFAULT_WORKBENCH_STATE,
    {
      workflow_run_id: 'run-d',
      workflow_id: 'workflow-d',
      workflow_version_id: null,
      workflow_semantic_version: '1.0.0',
      status: 'failed',
    },
    600,
  );

  const focused = withDiagnosticsFocus(
    selected,
    {
      workflow_run_id: 'run-d',
      diagnostic_event_id: 'event-error-d',
      node_id: 'node-d',
    },
    700,
  );

  assert.equal(focused.selected_page_id, 'diagnostics');
  assert.deepEqual(focused.diagnostics_focus, {
    workflow_run_id: 'run-d',
    diagnostic_event_id: 'event-error-d',
    node_id: 'node-d',
    requested_at_ms: 700,
  });
});

test('withSettingsFocus selects settings and stores requested section context', () => {
  const selected = withSelectedWorkbenchPage(DEFAULT_WORKBENCH_STATE, 'io_inspector');

  const focused = withSettingsFocus(selected, 'diagnostics_retention', 750);

  assert.equal(focused.selected_page_id, 'settings');
  assert.deepEqual(focused.settings_focus, {
    target_id: 'diagnostics_retention',
    requested_at_ms: 750,
  });

  const cleared = withSettingsFocus(focused, null, 800);
  assert.equal(cleared.selected_page_id, 'settings');
  assert.equal(cleared.settings_focus, null);
});

test('changing active run clears stale diagnostics focus', () => {
  const focused = withDiagnosticsFocus(
    withActiveWorkflowRun(
      DEFAULT_WORKBENCH_STATE,
      {
        workflow_run_id: 'run-e',
        workflow_id: 'workflow-e',
        workflow_version_id: null,
        workflow_semantic_version: null,
        status: 'failed',
      },
      800,
    ),
    {
      workflow_run_id: 'run-e',
      diagnostic_event_id: 'event-error-e',
      node_id: null,
    },
    900,
  );

  const changed = withActiveWorkflowRun(
    focused,
    {
      workflow_run_id: 'run-f',
      workflow_id: 'workflow-f',
      workflow_version_id: null,
      workflow_semantic_version: null,
      status: 'running',
    },
    1_000,
  );

  assert.equal(changed.diagnostics_focus, null);
  assert.equal(changed.selected_page_id, 'diagnostics');
});
