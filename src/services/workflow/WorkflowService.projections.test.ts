import test from 'node:test';
import assert from 'node:assert/strict';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { WorkflowRunProjectionService } from './WorkflowRunProjectionService.ts';
import type {
  WorkflowRunDetailQueryResponse,
  WorkflowRunListQueryResponse,
} from '../diagnostics/types.ts';

function installWindowMock(): void {
  const target = globalThis as unknown as Record<string, unknown>;
  target.window = globalThis;
}

test('queryRunList preserves backend projection rows and facets', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const response: WorkflowRunListQueryResponse = {
    runs: [
      {
        workflow_run_id: 'run-a',
        workflow_id: 'workflow-a',
        workflow_version_id: 'wfver-a',
        workflow_semantic_version: '1.2.3',
        status: 'delayed',
        accepted_at_ms: 10,
        enqueued_at_ms: 20,
        started_at_ms: null,
        completed_at_ms: null,
        duration_ms: null,
        scheduler_policy_id: 'priority_then_fifo',
        retention_policy_id: 'ephemeral',
        scheduler_queue_position: 0,
        scheduler_priority: 7,
        estimate_confidence: 'low',
        estimated_queue_wait_ms: 10,
        estimated_duration_ms: 100,
        scheduler_reason: 'waiting_for_runtime_admission',
        last_event_seq: 5,
        last_updated_at_ms: 95,
      },
    ],
    facets: [
      { facet_kind: 'workflow_version', facet_value: '1.2.3', run_count: 1 },
      { facet_kind: 'status', facet_value: 'delayed', run_count: 1 },
    ],
    projection_state: {
      projection_name: 'run_list',
      projection_version: 1,
      last_applied_event_seq: 5,
      status: 'current',
      rebuilt_at_ms: null,
      updated_at_ms: 100,
    },
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });

  try {
    const service = new WorkflowRunProjectionService();
    const result = await service.queryRunList({
      workflow_id: 'workflow-a',
      limit: 25,
    });

    assert.deepEqual(result, response);
    assert.equal(calls.length, 1);
    assert.equal(calls[0].cmd, 'workflow_run_list_query');
    assert.deepEqual(calls[0].args, {
      request: {
        workflow_id: 'workflow-a',
        limit: 25,
      },
    });
  } finally {
    clearMocks();
  }
});

test('queryRunDetail preserves selected-run workflow version and estimate fields', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const response: WorkflowRunDetailQueryResponse = {
    run: {
      workflow_run_id: 'run-a',
      workflow_id: 'workflow-a',
      workflow_version_id: 'wfver-a',
      workflow_semantic_version: '1.2.3',
      status: 'delayed',
      accepted_at_ms: 10,
      enqueued_at_ms: 20,
      started_at_ms: null,
      completed_at_ms: null,
      duration_ms: null,
      scheduler_policy_id: 'priority_then_fifo',
      retention_policy_id: 'ephemeral',
      client_id: 'client-a',
      client_session_id: 'session-a',
      bucket_id: 'bucket-a',
      workflow_run_snapshot_id: 'snapshot-a',
      workflow_presentation_revision_id: 'presentation-a',
      latest_estimate_json: '{"confidence":"low"}',
      latest_queue_placement_json: '{"queue_position":0}',
      started_payload_json: null,
      terminal_payload_json: null,
      terminal_error: null,
      scheduler_queue_position: 0,
      scheduler_priority: 7,
      estimate_confidence: 'low',
      estimated_queue_wait_ms: 100,
      estimated_duration_ms: 500,
      scheduler_reason: 'waiting_for_runtime_admission',
      timeline_event_count: 3,
      last_event_seq: 6,
      last_updated_at_ms: 110,
    },
    projection_state: {
      projection_name: 'run_detail',
      projection_version: 1,
      last_applied_event_seq: 6,
      status: 'current',
      rebuilt_at_ms: null,
      updated_at_ms: 115,
    },
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });

  try {
    const service = new WorkflowRunProjectionService();
    const result = await service.queryRunDetail({ workflow_run_id: 'run-a' });

    assert.deepEqual(result, response);
    assert.equal(calls.length, 1);
    assert.equal(calls[0].cmd, 'workflow_run_detail_query');
    assert.deepEqual(calls[0].args, {
      request: {
        workflow_run_id: 'run-a',
      },
    });
  } finally {
    clearMocks();
  }
});
