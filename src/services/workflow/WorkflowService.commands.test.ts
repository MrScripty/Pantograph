import test from 'node:test';
import assert from 'node:assert/strict';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { WorkflowCommandService } from './WorkflowCommandService.ts';
import type { WorkflowRetentionPolicyUpdateResponse } from '../diagnostics/types.ts';
import type {
  WorkflowSessionQueueCancelResponse,
  WorkflowSessionQueueReprioritizeResponse,
} from './types.ts';

function installWindowMock(): void {
  const target = globalThis as unknown as Record<string, unknown>;
  target.window = globalThis;
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

test('queue cancel and reprioritize methods return backend command results exactly', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const cancelResponse: WorkflowSessionQueueCancelResponse = { ok: true };
  const reprioritizeResponse: WorkflowSessionQueueReprioritizeResponse = { ok: true };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return cmd === 'workflow_cancel_execution_session_queue_item'
      ? cancelResponse
      : reprioritizeResponse;
  });

  try {
    const service = new WorkflowCommandService();
    const cancel = await service.cancelSessionQueueItem({
      session_id: 'session-a',
      workflow_run_id: 'run-a',
    });
    const reprioritize = await service.reprioritizeSessionQueueItem({
      session_id: 'session-a',
      workflow_run_id: 'run-b',
      priority: 10,
    });

    assert.deepEqual(cancel, cancelResponse);
    assert.deepEqual(reprioritize, reprioritizeResponse);
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
        cmd: 'workflow_reprioritize_execution_session_queue_item',
        args: {
          request: {
            session_id: 'session-a',
            workflow_run_id: 'run-b',
            priority: 10,
          },
        },
      },
    ]);
  } finally {
    clearMocks();
  }
});
