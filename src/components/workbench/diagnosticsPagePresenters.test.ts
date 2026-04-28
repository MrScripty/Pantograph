import test from 'node:test';
import assert from 'node:assert/strict';

import type { RunDetailProjectionRecord, RunListProjectionRecord } from '../../services/diagnostics/types.ts';
import {
  DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
  buildDiagnosticsFacetSummary,
  buildDiagnosticsFactRows,
  buildDiagnosticsComparisonFilterOptions,
  diagnosticsStatusClass,
  filterDiagnosticsComparisonRuns,
  formatDiagnosticEventKind,
  formatDiagnosticSourceComponent,
  formatDiagnosticsDuration,
  formatDiagnosticsProjectionFreshness,
  formatDiagnosticsStatusLabel,
  hasActiveDiagnosticsComparisonFilters,
  hasTimelinePayload,
} from './diagnosticsPagePresenters.ts';

function createRunDetail(): RunDetailProjectionRecord {
  return {
    workflow_run_id: 'run-1',
    workflow_id: 'workflow-a',
    workflow_version_id: 'wfver-1',
    workflow_semantic_version: '1.2.3',
    status: 'running',
    accepted_at_ms: 1,
    enqueued_at_ms: 2,
    started_at_ms: 3,
    completed_at_ms: null,
    duration_ms: null,
    scheduler_policy_id: 'policy-a',
    retention_policy_id: 'retention-a',
    scheduler_queue_position: 1,
    scheduler_priority: 5,
    estimate_confidence: 'low',
    estimated_queue_wait_ms: 1_500,
    estimated_duration_ms: 2_500,
    scheduler_reason: 'warm_session_reused',
    last_event_seq: 9,
    last_updated_at_ms: 10,
    client_id: 'client-a',
    client_session_id: 'session-a',
    workflow_execution_session_id: 'exec-session-a',
    bucket_id: 'bucket-a',
    workflow_run_snapshot_id: 'snapshot-a',
    workflow_presentation_revision_id: 'presentation-a',
    latest_estimate_json: null,
    latest_queue_placement_json: null,
    started_payload_json: null,
    terminal_payload_json: null,
    terminal_error: null,
    timeline_event_count: 4,
  };
}

function createRunListPeer(overrides: Partial<RunListProjectionRecord>): RunListProjectionRecord {
  return {
    workflow_run_id: 'run-peer',
    workflow_id: 'workflow-a',
    workflow_version_id: 'wfver-1',
    workflow_semantic_version: '1.2.3',
    status: 'completed',
    scheduler_policy_id: 'policy-a',
    retention_policy_id: 'retention-a',
    accepted_at_ms: 86_400_000,
    client_id: 'client-a',
    client_session_id: 'session-a',
    bucket_id: 'bucket-a',
    last_event_seq: 10,
    last_updated_at_ms: 20,
    ...overrides,
  };
}

test('formatDiagnosticsDuration exposes pending and running states', () => {
  assert.equal(formatDiagnosticsDuration(null, 'running'), 'Running');
  assert.equal(formatDiagnosticsDuration(null, 'future'), 'Pending');
  assert.equal(formatDiagnosticsDuration(null, 'scheduled'), 'Pending');
  assert.equal(formatDiagnosticsDuration(null, 'queued'), 'Pending');
  assert.equal(formatDiagnosticsDuration(null, 'delayed'), 'Pending');
  assert.equal(formatDiagnosticsDuration(500, 'completed'), '500 ms');
  assert.equal(formatDiagnosticsDuration(2_500, 'completed'), '2.5 s');
});

test('formatDiagnosticsProjectionFreshness keeps projection cursor visible', () => {
  assert.equal(formatDiagnosticsProjectionFreshness(null), 'Projection unavailable');
  assert.equal(
    formatDiagnosticsProjectionFreshness({
      projection_name: 'run_detail',
      projection_version: 1,
      last_applied_event_seq: 42,
      status: 'needs_rebuild',
      rebuilt_at_ms: null,
      updated_at_ms: 5,
    }),
    'Needs rebuild at seq 42',
  );
});

test('diagnosticsStatusClass maps operational statuses to stable classes', () => {
  assert.match(diagnosticsStatusClass('completed'), /emerald/);
  assert.match(diagnosticsStatusClass('running'), /cyan/);
  assert.match(diagnosticsStatusClass('future'), /amber/);
  assert.match(diagnosticsStatusClass('scheduled'), /amber/);
  assert.match(diagnosticsStatusClass('queued'), /amber/);
  assert.match(diagnosticsStatusClass('failed'), /red/);
  assert.match(diagnosticsStatusClass('cancelled'), /neutral/);
  assert.equal(formatDiagnosticsStatusLabel('future'), 'Future');
  assert.equal(formatDiagnosticsStatusLabel('scheduled'), 'Scheduled');
  assert.equal(formatDiagnosticsStatusLabel('cancelled'), 'Cancelled');
});

test('buildDiagnosticsFactRows uses projection fields without ledger parsing', () => {
  const rows = buildDiagnosticsFactRows(createRunDetail());

  assert.equal(rows.find((row) => row.label === 'Workflow')?.value, 'workflow-a');
  assert.equal(rows.find((row) => row.label === 'Workflow Version')?.value, '1.2.3');
  assert.equal(rows.find((row) => row.label === 'Queue Position')?.value, '1');
  assert.equal(rows.find((row) => row.label === 'Execution Session')?.value, 'exec-session-a');
  assert.equal(rows.find((row) => row.label === 'Estimated Queue Wait')?.value, '1.5 s');
  assert.equal(rows.find((row) => row.label === 'Scheduler Reason')?.value, 'warm_session_reused');
  assert.equal(rows.find((row) => row.label === 'Timeline Events')?.value, '4');
});

test('buildDiagnosticsFacetSummary exposes comparison-ready run-list facets', () => {
  const activeRun = createRunDetail();
  const summary = buildDiagnosticsFacetSummary(activeRun, [
    activeRun,
    {
      workflow_run_id: 'run-2',
      workflow_id: 'workflow-a',
      workflow_version_id: 'wfver-1',
      workflow_semantic_version: '1.2.3',
      status: 'completed',
      scheduler_policy_id: 'policy-a',
      retention_policy_id: 'retention-b',
      last_event_seq: 10,
      last_updated_at_ms: 20,
    },
    {
      workflow_run_id: 'run-3',
      workflow_id: 'workflow-a',
      workflow_version_id: 'wfver-2',
      workflow_semantic_version: '2.0.0',
      status: 'completed',
      scheduler_policy_id: 'policy-b',
      retention_policy_id: 'retention-a',
      last_event_seq: 11,
      last_updated_at_ms: 30,
    },
    {
      workflow_run_id: 'run-4',
      workflow_id: 'workflow-b',
      workflow_version_id: 'wfver-other',
      workflow_semantic_version: '9.0.0',
      status: 'completed',
      last_event_seq: 12,
      last_updated_at_ms: 40,
    },
  ]);

  assert.equal(summary.rows.find((row) => row.label === 'Workflow Version')?.count, 2);
  assert.equal(summary.rows.find((row) => row.label === 'Workflow Version')?.total, 3);
  assert.equal(summary.rows.find((row) => row.label === 'Scheduler Policy')?.count, 2);
  assert.equal(summary.rows.find((row) => row.label === 'Retention Policy')?.count, 2);
  assert.match(summary.mixedVersionWarning ?? '', /2 workflow versions/);
});

test('buildDiagnosticsFacetSummary includes the active run when the run list is paged', () => {
  const activeRun = createRunDetail();
  const summary = buildDiagnosticsFacetSummary(activeRun, [
    {
      workflow_run_id: 'run-2',
      workflow_id: 'workflow-a',
      workflow_version_id: 'wfver-2',
      workflow_semantic_version: '2.0.0',
      status: 'completed',
      scheduler_policy_id: 'policy-b',
      retention_policy_id: 'retention-b',
      last_event_seq: 11,
      last_updated_at_ms: 30,
    },
  ]);

  assert.equal(summary.rows.find((row) => row.label === 'Workflow Version')?.count, 1);
  assert.equal(summary.rows.find((row) => row.label === 'Workflow Version')?.total, 2);
});

test('buildDiagnosticsFacetSummary prefers backend projection facets when provided', () => {
  const activeRun = createRunDetail();
  const summary = buildDiagnosticsFacetSummary(activeRun, [activeRun], [
    { facet_kind: 'workflow_version', facet_value: '1.2.3', run_count: 12 },
    { facet_kind: 'workflow_version', facet_value: '2.0.0', run_count: 3 },
    { facet_kind: 'status', facet_value: 'completed', run_count: 10 },
    { facet_kind: 'status', facet_value: 'failed', run_count: 5 },
    { facet_kind: 'scheduler_policy', facet_value: 'policy-a', run_count: 15 },
    { facet_kind: 'retention_policy', facet_value: 'retention-a', run_count: 15 },
  ]);

  assert.equal(summary.rows.find((row) => row.label === 'Workflow Version')?.count, 12);
  assert.equal(summary.rows.find((row) => row.label === 'Workflow Version')?.total, 15);
  assert.equal(summary.rows.find((row) => row.label === 'Status')?.total, 15);
  assert.match(summary.mixedVersionWarning ?? '', /2 workflow versions/);
});

test('diagnostics comparison filters expose available projection values', () => {
  const activeRun = createRunDetail();
  const options = buildDiagnosticsComparisonFilterOptions(activeRun, [
    activeRun,
    createRunListPeer({ workflow_run_id: 'run-2', status: 'completed', scheduler_policy_id: 'policy-b' }),
    createRunListPeer({
      workflow_run_id: 'run-3',
      workflow_version_id: 'wfver-2',
      workflow_semantic_version: '2.0.0',
      bucket_id: null,
      client_id: null,
      client_session_id: null,
      retention_policy_id: null,
      accepted_at_ms: null,
    }),
    createRunListPeer({ workflow_run_id: 'run-4', workflow_id: 'workflow-b', status: 'failed' }),
  ]);

  assert.deepEqual(options.workflowVersions, ['1.2.3', '2.0.0']);
  assert.deepEqual(options.statuses, ['completed', 'running']);
  assert.deepEqual(options.schedulerPolicies, ['policy-a', 'policy-b']);
  assert.deepEqual(options.retentionPolicies, ['retention-a', 'Unassigned']);
  assert.deepEqual(options.clients, ['client-a', 'Unassigned']);
  assert.deepEqual(options.clientSessions, ['session-a', 'Unassigned']);
  assert.deepEqual(options.buckets, ['bucket-a', 'Unassigned']);
  assert.deepEqual(options.acceptedDates, ['1970-01-01', '1970-01-02', 'Unassigned']);
});

test('diagnostics comparison filters keep selected run and filter peer rows', () => {
  const activeRun = createRunDetail();
  const filteredRuns = filterDiagnosticsComparisonRuns(
    activeRun,
    [
      activeRun,
      createRunListPeer({ workflow_run_id: 'run-2', status: 'completed', scheduler_policy_id: 'policy-b' }),
      createRunListPeer({
        workflow_run_id: 'run-3',
        workflow_version_id: 'wfver-2',
        workflow_semantic_version: '2.0.0',
        status: 'completed',
        scheduler_policy_id: 'policy-b',
      }),
      createRunListPeer({ workflow_run_id: 'run-4', workflow_id: 'workflow-b', status: 'completed' }),
    ],
    {
      ...DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
      workflowVersion: '1.2.3',
      status: 'completed',
      schedulerPolicy: 'policy-b',
      acceptedDate: '1970-01-02',
    },
  );

  assert.deepEqual(
    filteredRuns.map((run) => run.workflow_run_id),
    ['run-1', 'run-2'],
  );
  assert.equal(hasActiveDiagnosticsComparisonFilters(DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS), false);
  assert.equal(
    hasActiveDiagnosticsComparisonFilters({
      ...DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
      status: 'completed',
    }),
    true,
  );
});

test('diagnostics comparison filters support accepted date ranges', () => {
  const activeRun = createRunDetail();
  const filteredRuns = filterDiagnosticsComparisonRuns(
    activeRun,
    [
      activeRun,
      createRunListPeer({ workflow_run_id: 'run-2', accepted_at_ms: Date.parse('2026-04-01T12:00:00.000Z') }),
      createRunListPeer({ workflow_run_id: 'run-3', accepted_at_ms: Date.parse('2026-04-10T12:00:00.000Z') }),
      createRunListPeer({ workflow_run_id: 'run-4', accepted_at_ms: null }),
    ],
    {
      ...DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
      acceptedFromDate: '2026-04-01',
      acceptedToDate: '2026-04-05',
    },
  );

  assert.deepEqual(
    filteredRuns.map((run) => run.workflow_run_id),
    ['run-1', 'run-2'],
  );
  assert.equal(
    hasActiveDiagnosticsComparisonFilters({
      ...DEFAULT_DIAGNOSTICS_COMPARISON_FILTERS,
      acceptedFromDate: '2026-04-01',
    }),
    true,
  );
});

test('timeline label helpers format typed projection enums and payload presence', () => {
  assert.equal(formatDiagnosticEventKind('scheduler_queue_placement'), 'Scheduler Queue Placement');
  assert.equal(formatDiagnosticSourceComponent('node_execution'), 'Node Execution');
  assert.equal(hasTimelinePayload({ payload_json: '{}' }), false);
  assert.equal(hasTimelinePayload({ payload_json: '{"decision":"delay"}' }), true);
});
