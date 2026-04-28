import test from 'node:test';
import assert from 'node:assert/strict';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { WorkflowCommandService } from './WorkflowCommandService.ts';
import type {
  DiagnosticsRetentionPolicySettings,
  WorkflowRetentionCleanupResponse,
  WorkflowRetentionPolicyUpdateResponse,
} from '../diagnostics/types.ts';
import type {
  WorkflowAdminQueueCancelResponse,
  WorkflowAdminQueuePushFrontResponse,
  WorkflowAdminQueueReprioritizeResponse,
  WorkflowExecutionSessionCloseResponse,
  WorkflowExecutionSessionCreateResponse,
  WorkflowRunResponse,
  WorkflowSessionQueueCancelResponse,
  WorkflowSessionQueuePushFrontResponse,
  WorkflowSessionQueueReprioritizeResponse,
} from './types.ts';

function installWindowMock(): void {
  const target = globalThis as unknown as Record<string, unknown>;
  target.window = globalThis;
}

function standardRetentionSettings(retentionDays: number): DiagnosticsRetentionPolicySettings {
  const scope = {
    retention_days: retentionDays,
    payload_mode: 'retain_payload_reference' as const,
  };
  return {
    final_outputs: scope,
    workflow_inputs: scope,
    intermediate_node_io: scope,
    failed_run_data: scope,
    max_artifact_bytes: null,
    max_total_storage_bytes: null,
    media_behavior: 'metadata_and_reference_only',
    compression_behavior: 'not_configured',
    cleanup_trigger: 'manual_or_maintenance',
  };
}

test('updateRetentionPolicy returns backend policy state without client-side optimistic replacement', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const response: WorkflowRetentionPolicyUpdateResponse = {
    retention_policy: {
      policy_id: 'standard-local-v2',
      policy_version: 2,
      retention_class: 'standard',
      retention_days: 14,
      settings: standardRetentionSettings(14),
      applied_at_ms: 123,
      explanation: 'backend normalized policy',
    },
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });

  try {
    const service = new WorkflowCommandService();
    const result = await service.updateRetentionPolicy({
      retention_days: 7,
      explanation: 'client requested policy',
      reason: 'test',
    });

    assert.deepEqual(result, response);
    assert.equal(result.retention_policy.retention_days, 14);
    assert.equal(result.retention_policy.explanation, 'backend normalized policy');
    assert.deepEqual(calls, [
      {
        cmd: 'workflow_retention_policy_update',
        args: {
          request: {
            retention_days: 7,
            explanation: 'client requested policy',
            reason: 'test',
          },
        },
      },
    ]);
  } finally {
    clearMocks();
  }
});

test('queue control methods return backend command results exactly', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const cancelResponse: WorkflowSessionQueueCancelResponse = { ok: true };
  const adminCancelResponse: WorkflowAdminQueueCancelResponse = {
    ok: true,
    session_id: 'session-b',
  };
  const adminReprioritizeResponse: WorkflowAdminQueueReprioritizeResponse = {
    ok: true,
    session_id: 'session-c',
  };
  const adminPushFrontResponse: WorkflowAdminQueuePushFrontResponse = {
    ok: true,
    session_id: 'session-d',
    priority: 33,
  };
  const reprioritizeResponse: WorkflowSessionQueueReprioritizeResponse = { ok: true };
  const pushFrontResponse: WorkflowSessionQueuePushFrontResponse = { ok: true, priority: 11 };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    if (cmd === 'workflow_cancel_execution_session_queue_item') {
      return cancelResponse;
    }
    if (cmd === 'workflow_admin_cancel_queue_item') {
      return adminCancelResponse;
    }
    if (cmd === 'workflow_admin_reprioritize_queue_item') {
      return adminReprioritizeResponse;
    }
    if (cmd === 'workflow_admin_push_queue_item_to_front') {
      return adminPushFrontResponse;
    }
    if (cmd === 'workflow_reprioritize_execution_session_queue_item') {
      return reprioritizeResponse;
    }
    return pushFrontResponse;
  });

  try {
    const service = new WorkflowCommandService();
    const cancel = await service.cancelSessionQueueItem({
      session_id: 'session-a',
      workflow_run_id: 'run-a',
    });
    const adminCancel = await service.adminCancelQueueItem({
      workflow_run_id: 'run-admin',
    });
    const adminReprioritize = await service.adminReprioritizeQueueItem({
      workflow_run_id: 'run-admin-priority',
      priority: 22,
    });
    const adminPushFront = await service.adminPushQueueItemToFront({
      workflow_run_id: 'run-admin-front',
    });
    const reprioritize = await service.reprioritizeSessionQueueItem({
      session_id: 'session-a',
      workflow_run_id: 'run-b',
      priority: 10,
    });
    const pushFront = await service.pushSessionQueueItemToFront({
      session_id: 'session-a',
      workflow_run_id: 'run-c',
    });

    assert.deepEqual(cancel, cancelResponse);
    assert.deepEqual(adminCancel, adminCancelResponse);
    assert.deepEqual(adminReprioritize, adminReprioritizeResponse);
    assert.deepEqual(adminPushFront, adminPushFrontResponse);
    assert.deepEqual(reprioritize, reprioritizeResponse);
    assert.deepEqual(pushFront, pushFrontResponse);
    assert.deepEqual(calls, [
      {
        cmd: 'workflow_cancel_execution_session_queue_item',
        args: {
          request: {
            session_id: 'session-a',
            workflow_run_id: 'run-a',
          },
        },
      },
      {
        cmd: 'workflow_admin_cancel_queue_item',
        args: {
          request: {
            workflow_run_id: 'run-admin',
          },
        },
      },
      {
        cmd: 'workflow_admin_reprioritize_queue_item',
        args: {
          request: {
            workflow_run_id: 'run-admin-priority',
            priority: 22,
          },
        },
      },
      {
        cmd: 'workflow_admin_push_queue_item_to_front',
        args: {
          request: {
            workflow_run_id: 'run-admin-front',
          },
        },
      },
      {
        cmd: 'workflow_reprioritize_execution_session_queue_item',
        args: {
          request: {
            session_id: 'session-a',
            workflow_run_id: 'run-b',
            priority: 10,
          },
        },
      },
      {
        cmd: 'workflow_push_execution_session_queue_item_to_front',
        args: {
          request: {
            session_id: 'session-a',
            workflow_run_id: 'run-c',
          },
        },
      },
    ]);
  } finally {
    clearMocks();
  }
});

test('execution session commands preserve scheduler-backed request boundaries', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const createResponse: WorkflowExecutionSessionCreateResponse = {
    session_id: 'execution-session-a',
    attribution: null,
    runtime_capabilities: [],
  };
  const runResponse: WorkflowRunResponse = {
    workflow_run_id: 'run-a',
    outputs: [],
    timing_ms: 45,
  };
  const closeResponse: WorkflowExecutionSessionCloseResponse = { ok: true };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    if (cmd === 'workflow_create_execution_session') {
      return createResponse;
    }
    if (cmd === 'workflow_run_execution_session') {
      return runResponse;
    }
    return closeResponse;
  });

  try {
    const service = new WorkflowCommandService();
    const created = await service.createWorkflowExecutionSession({
      workflow_id: 'workflow-a',
      usage_profile: null,
      keep_alive: false,
    });
    const run = await service.runWorkflowExecutionSession({
      session_id: created.session_id,
      workflow_semantic_version: '0.1.0',
      inputs: [],
      output_targets: null,
      override_selection: null,
      timeout_ms: null,
      priority: null,
    });
    const closed = await service.closeWorkflowExecutionSession({
      session_id: created.session_id,
    });

    assert.deepEqual(created, createResponse);
    assert.deepEqual(run, runResponse);
    assert.deepEqual(closed, closeResponse);
    assert.deepEqual(calls, [
      {
        cmd: 'workflow_create_execution_session',
        args: {
          request: {
            workflow_id: 'workflow-a',
            usage_profile: null,
            keep_alive: false,
          },
        },
      },
      {
        cmd: 'workflow_run_execution_session',
        args: {
          request: {
            session_id: 'execution-session-a',
            workflow_semantic_version: '0.1.0',
            inputs: [],
            output_targets: null,
            override_selection: null,
            timeout_ms: null,
            priority: null,
          },
        },
      },
      {
        cmd: 'workflow_close_execution_session',
        args: {
          request: {
            session_id: 'execution-session-a',
          },
        },
      },
    ]);
  } finally {
    clearMocks();
  }
});

test('applyRetentionCleanup returns backend cleanup result without optimistic mutation', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const response: WorkflowRetentionCleanupResponse = {
    cleanup: {
      policy_id: 'standard-local-v1',
      policy_version: 3,
      retention_class: 'standard',
      cutoff_occurred_before_ms: 1700,
      expired_artifact_count: 2,
      last_event_seq: 44,
    },
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });

  try {
    const service = new WorkflowCommandService();
    const result = await service.applyRetentionCleanup({
      limit: 25,
      reason: 'GUI cleanup request',
    });

    assert.deepEqual(result, response);
    assert.deepEqual(calls, [
      {
        cmd: 'workflow_retention_cleanup_apply',
        args: {
          request: {
            limit: 25,
            reason: 'GUI cleanup request',
          },
        },
      },
    ]);
  } finally {
    clearMocks();
  }
});

test('deletePumasModelWithAudit returns backend delete audit result exactly', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const response = {
    success: true,
    error: null,
    auditEventSeq: 77,
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });

  try {
    const service = new WorkflowCommandService();
    const result = await service.deletePumasModelWithAudit('org/model-a');

    assert.deepEqual(result, response);
    assert.deepEqual(calls, [
      {
        cmd: 'delete_pumas_model_with_audit',
        args: {
          modelId: 'org/model-a',
        },
      },
    ]);
  } finally {
    clearMocks();
  }
});

test('searchHfModelsWithAudit forwards bounded search parameters and preserves result', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const response = {
    models: [{ id: 'org/model-a' }],
    auditEventSeq: 88,
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });

  try {
    const service = new WorkflowCommandService();
    const result = await service.searchHfModelsWithAudit({
      query: 'diffusion',
      kind: 'text-to-image',
      limit: 25,
      hydrateLimit: 5,
    });

    assert.deepEqual(result, response);
    assert.deepEqual(calls, [
      {
        cmd: 'search_hf_models_with_audit',
        args: {
          query: 'diffusion',
          kind: 'text-to-image',
          limit: 25,
          hydrateLimit: 5,
        },
      },
    ]);
  } finally {
    clearMocks();
  }
});

test('startHfDownloadWithAudit forwards download request and preserves result', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const response = {
    downloadId: 'download-1',
    auditEventSeq: 89,
  };
  const request = {
    repo_id: 'org/model-a',
    family: 'diffusion',
    official_name: 'Model A',
    model_type: 'diffusion',
    quant: null,
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });

  try {
    const service = new WorkflowCommandService();
    const result = await service.startHfDownloadWithAudit(request);

    assert.deepEqual(result, response);
    assert.deepEqual(calls, [
      {
        cmd: 'start_hf_download_with_audit',
        args: {
          request,
        },
      },
    ]);
  } finally {
    clearMocks();
  }
});
