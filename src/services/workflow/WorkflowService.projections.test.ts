import test from 'node:test';
import assert from 'node:assert/strict';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { WorkflowProjectionService } from './WorkflowProjectionService.ts';
import { WorkflowServiceError } from './workflowServiceErrors.ts';
import type {
  WorkflowLibraryUsageQueryResponse,
  WorkflowRunDetailQueryResponse,
  WorkflowRunListQueryResponse,
  WorkflowSchedulerTimelineQueryResponse,
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
        client_id: 'client-a',
        client_session_id: 'session-a',
        bucket_id: 'bucket-a',
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
      projection_version: 2,
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
    const service = new WorkflowProjectionService();
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

test('queryRunDetail normalizes backend error envelopes', async () => {
  installWindowMock();
  mockIPC(() => {
    throw JSON.stringify({
      code: 'invalid_request',
      message: 'workflow_run_id must be non-empty',
    });
  });

  try {
    const service = new WorkflowProjectionService();
    await assert.rejects(
      service.queryRunDetail({ workflow_run_id: '' }),
      (error) =>
        error instanceof WorkflowServiceError &&
        error.code === 'invalid_request' &&
        error.message === 'workflow_run_id must be non-empty',
    );
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
    const service = new WorkflowProjectionService();
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

test('querySchedulerTimeline preserves typed event projection fields', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const response: WorkflowSchedulerTimelineQueryResponse = {
    events: [
      {
        event_seq: 8,
        event_id: 'event-scheduler-queue-a',
        event_kind: 'scheduler_queue_placement',
        source_component: 'scheduler',
        occurred_at_ms: 120,
        recorded_at_ms: 121,
        workflow_run_id: 'run-a',
        workflow_id: 'workflow-a',
        workflow_version_id: 'wfver-a',
        workflow_semantic_version: '1.2.3',
        scheduler_policy_id: 'priority_then_fifo',
        retention_policy_id: 'ephemeral',
        summary: 'queued at position 0',
        detail: 'priority 7',
        payload_json: '{"queue_position":0,"priority":7}',
      },
    ],
    projection_state: {
      projection_name: 'scheduler_timeline',
      projection_version: 1,
      last_applied_event_seq: 8,
      status: 'current',
      rebuilt_at_ms: null,
      updated_at_ms: 125,
    },
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });

  try {
    const service = new WorkflowProjectionService();
    const result = await service.querySchedulerTimeline({
      workflow_run_id: 'run-a',
      limit: 50,
    });

    assert.deepEqual(result, response);
    assert.equal(calls.length, 1);
    assert.equal(calls[0].cmd, 'workflow_scheduler_timeline_query');
    assert.deepEqual(calls[0].args, {
      request: {
        workflow_run_id: 'run-a',
        limit: 50,
      },
    });
    assert.equal(result.events[0].event_kind, 'scheduler_queue_placement');
    assert.equal(result.events[0].payload_json, '{"queue_position":0,"priority":7}');
    assert.equal(result.projection_state.last_applied_event_seq, 8);
  } finally {
    clearMocks();
  }
});

test('queryLibraryUsage preserves warm projection catching-up state', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const response: WorkflowLibraryUsageQueryResponse = {
    assets: [
      {
        asset_id: 'model-a',
        total_access_count: 1,
        run_access_count: 1,
        total_network_bytes: 128,
        last_accessed_at_ms: 200,
        last_operation: 'download',
        last_cache_status: 'miss',
        last_workflow_run_id: 'run-a',
        last_workflow_id: 'workflow-a',
        last_workflow_version_id: 'wfver-a',
        last_workflow_semantic_version: '1.2.3',
        last_client_id: 'client-a',
        last_client_session_id: 'session-a',
        last_bucket_id: 'bucket-a',
        last_event_seq: 9,
        last_updated_at_ms: 205,
      },
    ],
    projection_state: {
      projection_name: 'library_usage',
      projection_version: 1,
      last_applied_event_seq: 9,
      status: 'rebuilding',
      rebuilt_at_ms: null,
      updated_at_ms: 206,
    },
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });

  try {
    const service = new WorkflowProjectionService();
    const result = await service.queryLibraryUsage({
      asset_id: 'model-a',
      workflow_run_id: 'run-a',
      projection_batch_size: 1,
      limit: 10,
    });

    assert.deepEqual(result, response);
    assert.equal(calls.length, 1);
    assert.equal(calls[0].cmd, 'workflow_library_usage_query');
    assert.deepEqual(calls[0].args, {
      request: {
        asset_id: 'model-a',
        workflow_run_id: 'run-a',
        projection_batch_size: 1,
        limit: 10,
      },
    });
    assert.equal(result.projection_state.status, 'rebuilding');
    assert.equal(result.projection_state.last_applied_event_seq, 9);
    assert.equal(result.assets[0].last_event_seq, 9);
  } finally {
    clearMocks();
  }
});
