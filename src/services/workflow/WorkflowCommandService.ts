import type {
  PumasHfModelSearchAuditRequest,
  PumasHfModelSearchAuditResponse,
  PumasModelDeleteAuditResponse,
  WorkflowRetentionPolicyQueryRequest,
  WorkflowRetentionPolicyQueryResponse,
  WorkflowRetentionCleanupRequest,
  WorkflowRetentionCleanupResponse,
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

  async applyRetentionCleanup(
    request: WorkflowRetentionCleanupRequest,
  ): Promise<WorkflowRetentionCleanupResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        cleanup: {
          policy_id: 'standard-local-v1',
          policy_version: 1,
          retention_class: 'standard',
          cutoff_occurred_before_ms: Date.now() - 365 * 86_400_000,
          expired_artifact_count: 0,
          last_event_seq: null,
        },
      };
    }

    return invokeWorkflowCommand<WorkflowRetentionCleanupResponse>('workflow_retention_cleanup_apply', {
      request,
    });
  }

  async deletePumasModelWithAudit(modelId: string): Promise<PumasModelDeleteAuditResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        success: true,
        error: null,
        auditEventSeq: null,
      };
    }

    return invokeWorkflowCommand<PumasModelDeleteAuditResponse>('delete_pumas_model_with_audit', {
      modelId,
    });
  }

  async searchHfModelsWithAudit(
    request: PumasHfModelSearchAuditRequest,
  ): Promise<PumasHfModelSearchAuditResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        models: [],
        auditEventSeq: null,
      };
    }

    return invokeWorkflowCommand<PumasHfModelSearchAuditResponse>('search_hf_models_with_audit', {
      query: request.query,
      kind: request.kind,
      limit: request.limit,
      hydrateLimit: request.hydrateLimit,
    });
  }
}
