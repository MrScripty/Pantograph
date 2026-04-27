import test from 'node:test';
import assert from 'node:assert/strict';

import type { RunDetailProjectionRecord } from '../../services/diagnostics/types.ts';
import {
  buildDiagnosticsFactRows,
  diagnosticsStatusClass,
  formatDiagnosticEventKind,
  formatDiagnosticSourceComponent,
  formatDiagnosticsDuration,
  formatDiagnosticsProjectionFreshness,
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
    last_event_seq: 9,
    last_updated_at_ms: 10,
    client_id: 'client-a',
    client_session_id: 'session-a',
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

test('formatDiagnosticsDuration exposes pending and running states', () => {
  assert.equal(formatDiagnosticsDuration(null, 'running'), 'Running');
  assert.equal(formatDiagnosticsDuration(null, 'queued'), 'Pending');
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
  assert.match(diagnosticsStatusClass('queued'), /amber/);
  assert.match(diagnosticsStatusClass('failed'), /red/);
  assert.match(diagnosticsStatusClass('cancelled'), /neutral/);
});

test('buildDiagnosticsFactRows uses projection fields without ledger parsing', () => {
  const rows = buildDiagnosticsFactRows(createRunDetail());

  assert.equal(rows.find((row) => row.label === 'Workflow')?.value, 'workflow-a');
  assert.equal(rows.find((row) => row.label === 'Workflow Version')?.value, '1.2.3');
  assert.equal(rows.find((row) => row.label === 'Timeline Events')?.value, '4');
});

test('timeline label helpers format typed projection enums and payload presence', () => {
  assert.equal(formatDiagnosticEventKind('scheduler_queue_placement'), 'Scheduler Queue Placement');
  assert.equal(formatDiagnosticSourceComponent('node_execution'), 'Node Execution');
  assert.equal(hasTimelinePayload({ payload_json: '{}' }), false);
  assert.equal(hasTimelinePayload({ payload_json: '{"decision":"delay"}' }), true);
});
