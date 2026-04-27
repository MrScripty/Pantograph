import type {
  WorkflowRetentionPolicyQueryRequest,
  WorkflowRetentionPolicyQueryResponse,
  WorkflowRetentionPolicyUpdateRequest,
  WorkflowRetentionPolicyUpdateResponse,
} from '../diagnostics/types.ts';
import type {
  WorkflowSessionQueueCancelRequest,
  WorkflowSessionQueueCancelResponse,
  WorkflowSessionQueueReprioritizeRequest,
  WorkflowSessionQueueReprioritizeResponse,
} from './types.ts';
import { WorkflowProjectionService } from './WorkflowProjectionService.ts';
import { USE_WORKFLOW_MOCKS } from './workflowServiceConfig.ts';
import { invokeWorkflowCommand } from './workflowServiceErrors.ts';

export class WorkflowCommandService extends WorkflowProjectionService {
  async cancelSessionQueueItem(
    request: WorkflowSessionQueueCancelRequest,
  ): Promise<WorkflowSessionQueueCancelResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return { ok: true };
    }

    return invokeWorkflowCommand<WorkflowSessionQueueCancelResponse>(
      'workflow_cancel_execution_session_queue_item',
      { request },
    );
  }

  async reprioritizeSessionQueueItem(
    request: WorkflowSessionQueueReprioritizeRequest,
  ): Promise<WorkflowSessionQueueReprioritizeResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return { ok: true };
    }

    return invokeWorkflowCommand<WorkflowSessionQueueReprioritizeResponse>(
      'workflow_reprioritize_execution_session_queue_item',
      { request },
    );
  }

  async queryRetentionPolicy(
    request: WorkflowRetentionPolicyQueryRequest = {},
  ): Promise<WorkflowRetentionPolicyQueryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        retention_policy: {
          policy_id: 'standard-local-v1',
          policy_version: 1,
          retention_class: 'standard',
          retention_days: 365,
          applied_at_ms: Date.now(),
          explanation: 'Default local model/license usage retention policy',
        },
      };
    }

    return invokeWorkflowCommand<WorkflowRetentionPolicyQueryResponse>('workflow_retention_policy_query', {
      request,
    });
  }

  async updateRetentionPolicy(
    request: WorkflowRetentionPolicyUpdateRequest,
  ): Promise<WorkflowRetentionPolicyUpdateResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        retention_policy: {
          policy_id: 'standard-local-v1',
          policy_version: 2,
          retention_class: 'standard',
          retention_days: request.retention_days,
          applied_at_ms: Date.now(),
          explanation: request.explanation,
        },
      };
    }

    return invokeWorkflowCommand<WorkflowRetentionPolicyUpdateResponse>('workflow_retention_policy_update', {
      request,
    });
  }
}
