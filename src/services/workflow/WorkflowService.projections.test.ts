import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { WorkflowProjectionService } from './WorkflowProjectionService.ts';
import { WorkflowServiceError } from './workflowServiceErrors.ts';
import type { WorkflowLocalNetworkStatusQueryResponse } from './types.ts';
import type {
  WorkflowLibraryUsageQueryResponse,
  WorkflowIoArtifactQueryResponse,
  WorkflowRunDetailQueryResponse,
  WorkflowRunListQueryResponse,
  WorkflowSchedulerEstimateQueryResponse,
  WorkflowSchedulerTimelineQueryResponse,
} from '../diagnostics/types.ts';

interface RunProjectionContractFixture {
  run_list_response: WorkflowRunListQueryResponse;
  run_detail_response: WorkflowRunDetailQueryResponse;
}

function installWindowMock(): void {
  const target = globalThis as unknown as Record<string, unknown>;
  target.window = globalThis;
}

function loadRunProjectionContractFixture(): RunProjectionContractFixture {
  const fixtureUrl = new URL(
    '../../../crates/pantograph-workflow-service/tests/fixtures/run_projection_contract.json',
    import.meta.url,
  );
  return JSON.parse(readFileSync(fixtureUrl, 'utf8')) as RunProjectionContractFixture;
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
        workflow_execution_session_id: 'exec-session-a',
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
      projection_version: 3,
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
      client_id: 'client-a',
      client_session_id: 'session-a',
      bucket_id: 'bucket-a',
      accepted_at_from_ms: 1,
      accepted_at_to_ms: 100,
      limit: 25,
    });

    assert.deepEqual(result, response);
    assert.equal(calls.length, 1);
    assert.equal(calls[0].cmd, 'workflow_run_list_query');
    assert.deepEqual(calls[0].args, {
      request: {
        workflow_id: 'workflow-a',
        client_id: 'client-a',
        client_session_id: 'session-a',
        bucket_id: 'bucket-a',
        accepted_at_from_ms: 1,
        accepted_at_to_ms: 100,
        limit: 25,
      },
    });
  } finally {
    clearMocks();
  }
});

test('run projection contract fixture crosses Rust and TypeScript service boundaries', async () => {
  installWindowMock();
  const fixture = loadRunProjectionContractFixture();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    if (cmd === 'workflow_run_list_query') {
      return fixture.run_list_response;
    }
    if (cmd === 'workflow_run_detail_query') {
      return fixture.run_detail_response;
    }
    throw new Error(`unexpected command ${cmd}`);
  });

  try {
    const service = new WorkflowProjectionService();
    const runList = await service.queryRunList({ workflow_id: 'wf-1', limit: 25 });
    const runDetail = await service.queryRunDetail({ workflow_run_id: 'run-1' });

    assert.deepEqual(runList, fixture.run_list_response);
    assert.deepEqual(runDetail, fixture.run_detail_response);
    assert.equal(runList.runs[0].workflow_execution_session_id, 'exec-session-1');
    assert.equal(runDetail.run?.workflow_execution_session_id, 'exec-session-1');
    assert.deepEqual(calls, [
      {
        cmd: 'workflow_run_list_query',
        args: {
          request: {
            workflow_id: 'wf-1',
            limit: 25,
          },
        },
      },
      {
        cmd: 'workflow_run_detail_query',
        args: {
          request: {
            workflow_run_id: 'run-1',
          },
        },
      },
    ]);
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
      workflow_execution_session_id: 'exec-session-a',
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
      projection_version: 2,
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

test('querySchedulerEstimate preserves backend estimate projection shape', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const response: WorkflowSchedulerEstimateQueryResponse = {
    estimate: {
      workflow_run_id: 'run-estimate',
      workflow_id: 'workflow-a',
      workflow_version_id: 'wfver-a',
      workflow_semantic_version: '1.2.3',
      scheduler_policy_id: 'priority_then_fifo',
      latest_estimate_json: '{"confidence":"medium"}',
      estimate_confidence: 'medium',
      estimated_queue_wait_ms: 250,
      estimated_duration_ms: 1_500,
      last_event_seq: 12,
      last_updated_at_ms: 220,
    },
    projection_state: {
      projection_name: 'run_detail',
      projection_version: 2,
      last_applied_event_seq: 12,
      status: 'current',
      rebuilt_at_ms: null,
      updated_at_ms: 225,
    },
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });

  try {
    const service = new WorkflowProjectionService();
    const result = await service.querySchedulerEstimate({
      workflow_run_id: 'run-estimate',
      projection_batch_size: 25,
    });

    assert.deepEqual(result, response);
    assert.deepEqual(calls, [
      {
        cmd: 'workflow_scheduler_estimate_query',
        args: {
          request: {
            workflow_run_id: 'run-estimate',
            projection_batch_size: 25,
          },
        },
      },
    ]);
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

test('queryIoArtifacts preserves endpoint filters and retention summaries', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const response: WorkflowIoArtifactQueryResponse = {
    artifacts: [
      {
        event_seq: 13,
        event_id: 'event-io-a',
        occurred_at_ms: 300,
        recorded_at_ms: 301,
        workflow_run_id: 'run-a',
        workflow_id: 'workflow-a',
        workflow_version_id: 'wfver-a',
        workflow_semantic_version: '1.2.3',
        node_id: 'node-a',
        node_type: 'image',
        node_version: '1.0.0',
        runtime_id: 'runtime-a',
        runtime_version: '0.1.0',
        model_id: 'model-a',
        model_version: null,
        artifact_id: 'artifact-a',
        artifact_role: 'node_output',
        producer_node_id: 'node-a',
        producer_port_id: 'out',
        consumer_node_id: null,
        consumer_port_id: null,
        media_type: 'image/png',
        size_bytes: 128,
        content_hash: 'blake3:test',
        payload_ref: 'artifact://artifact-a',
        retention_state: 'retained',
        retention_reason: null,
        retention_policy_id: 'standard-local-v1',
      },
    ],
    retention_summary: [
      {
        retention_state: 'retained',
        artifact_count: 1,
      },
    ],
    projection_state: {
      projection_name: 'io_artifact',
      projection_version: 4,
      last_applied_event_seq: 13,
      status: 'current',
      rebuilt_at_ms: null,
      updated_at_ms: 305,
    },
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });

  try {
    const service = new WorkflowProjectionService();
    const result = await service.queryIoArtifacts({
      workflow_run_id: 'run-a',
      producer_node_id: 'node-a',
      consumer_node_id: null,
      retention_state: 'retained',
      limit: 25,
    });

    assert.deepEqual(result, response);
    assert.deepEqual(calls, [
      {
        cmd: 'workflow_io_artifact_query',
        args: {
          request: {
            workflow_run_id: 'run-a',
            producer_node_id: 'node-a',
            consumer_node_id: null,
            retention_state: 'retained',
            limit: 25,
          },
        },
      },
    ]);
    assert.equal(result.artifacts[0].producer_node_id, 'node-a');
    assert.equal(result.retention_summary[0].artifact_count, 1);
  } finally {
    clearMocks();
  }
});

test('queryLocalNetworkStatus preserves scheduler load and run placement facts', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const response: WorkflowLocalNetworkStatusQueryResponse = {
    local_node: {
      node_id: 'local-node',
      display_name: 'Local Pantograph',
      captured_at_ms: 1_000,
      transport_state: 'local_only',
      system: {
        hostname: 'host-a',
        os_name: 'Linux',
        os_version: '6',
        kernel_version: '6.1',
        cpu: {
          logical_core_count: 8,
          average_usage_percent: null,
        },
        memory: {
          total_bytes: 16_000,
          used_bytes: 8_000,
          available_bytes: 8_000,
        },
        disks: [],
        network_interfaces: [],
        gpu: {
          available: false,
          reason: 'GPU probe unavailable',
        },
      },
      scheduler_load: {
        max_sessions: 4,
        active_session_count: 1,
        max_loaded_sessions: 2,
        loaded_session_count: 1,
        active_run_count: 1,
        queued_run_count: 1,
        active_workflow_run_ids: ['run-active'],
        queued_workflow_run_ids: ['run-queued'],
        run_placements: [
          {
            workflow_run_id: 'run-queued',
            workflow_execution_session_id: 'exec-session-a',
            workflow_id: 'workflow-a',
            state: 'queued',
            runtime_loaded: false,
            required_backends: ['python'],
            required_models: ['model-a'],
          },
        ],
      },
      degradation_warnings: ['GPU probe unavailable'],
    },
    peer_nodes: [],
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });

  try {
    const service = new WorkflowProjectionService();
    const result = await service.queryLocalNetworkStatus({
      include_disks: true,
      include_network_interfaces: false,
    });

    assert.deepEqual(result, response);
    assert.deepEqual(calls, [
      {
        cmd: 'workflow_local_network_status_query',
        args: {
          request: {
            include_disks: true,
            include_network_interfaces: false,
          },
        },
      },
    ]);
    assert.equal(result.local_node.scheduler_load.run_placements[0].required_models[0], 'model-a');
  } finally {
    clearMocks();
  }
});
