import type {
  DiagnosticsRetentionPolicy,
  PumasHfDownloadRequest,
  PumasHfDownloadStartAuditResponse,
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
  WorkflowArtifactBodyRead,
  WorkflowArtifactConsumeAcknowledgementRequest,
  WorkflowArtifactConsumeAcknowledgementResponse,
  WorkflowArtifactDescriptorQueryRequest,
  WorkflowArtifactDescriptorQueryResponse,
  WorkflowArtifactPolicy,
  WorkflowArtifactReadRequest,
  WorkflowArtifactStoreStats,
  WorkflowAdminQueueCancelRequest,
  WorkflowAdminQueueCancelResponse,
  WorkflowAdminQueuePushFrontRequest,
  WorkflowAdminQueuePushFrontResponse,
  WorkflowAdminQueueReprioritizeRequest,
  WorkflowAdminQueueReprioritizeResponse,
  WorkflowExecutionSessionCloseRequest,
  WorkflowExecutionSessionCloseResponse,
  WorkflowExecutionSessionCreateRequest,
  WorkflowExecutionSessionCreateResponse,
  WorkflowExecutionSessionRunRequest,
  WorkflowRunResponse,
  WorkflowSessionQueueCancelRequest,
  WorkflowSessionQueueCancelResponse,
  WorkflowSessionQueuePushFrontRequest,
  WorkflowSessionQueuePushFrontResponse,
  WorkflowSessionQueueReprioritizeRequest,
  WorkflowSessionQueueReprioritizeResponse,
} from './types.ts';
import { WorkflowProjectionService } from './WorkflowProjectionService.ts';
import { USE_WORKFLOW_MOCKS } from './workflowServiceConfig.ts';
import { invokeWorkflowCommand } from './workflowServiceErrors.ts';

export class WorkflowCommandService extends WorkflowProjectionService {
  async createWorkflowExecutionSession(
    request: WorkflowExecutionSessionCreateRequest,
  ): Promise<WorkflowExecutionSessionCreateResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        session_id: `mock-execution-session-${Date.now()}`,
        attribution: null,
        runtime_capabilities: [],
      };
    }

    return invokeWorkflowCommand<WorkflowExecutionSessionCreateResponse>(
      'workflow_create_execution_session',
      { request },
    );
  }

  async runWorkflowExecutionSession(
    request: WorkflowExecutionSessionRunRequest,
  ): Promise<WorkflowRunResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        workflow_run_id: `mock-run-${Date.now()}`,
        outputs: [],
        timing_ms: 0,
      };
    }

    return invokeWorkflowCommand<WorkflowRunResponse>('workflow_run_execution_session', {
      request,
    });
  }

  async closeWorkflowExecutionSession(
    request: WorkflowExecutionSessionCloseRequest,
  ): Promise<WorkflowExecutionSessionCloseResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return { ok: true };
    }

    return invokeWorkflowCommand<WorkflowExecutionSessionCloseResponse>(
      'workflow_close_execution_session',
      { request },
    );
  }

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

  async adminCancelQueueItem(
    request: WorkflowAdminQueueCancelRequest,
  ): Promise<WorkflowAdminQueueCancelResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return { ok: true, session_id: 'mock-session' };
    }

    return invokeWorkflowCommand<WorkflowAdminQueueCancelResponse>('workflow_admin_cancel_queue_item', {
      request,
    });
  }

  async adminReprioritizeQueueItem(
    request: WorkflowAdminQueueReprioritizeRequest,
  ): Promise<WorkflowAdminQueueReprioritizeResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return { ok: true, session_id: 'mock-session' };
    }

    return invokeWorkflowCommand<WorkflowAdminQueueReprioritizeResponse>(
      'workflow_admin_reprioritize_queue_item',
      { request },
    );
  }

  async adminPushQueueItemToFront(
    request: WorkflowAdminQueuePushFrontRequest,
  ): Promise<WorkflowAdminQueuePushFrontResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return { ok: true, session_id: 'mock-session', priority: 1 };
    }

    return invokeWorkflowCommand<WorkflowAdminQueuePushFrontResponse>(
      'workflow_admin_push_queue_item_to_front',
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

  async pushSessionQueueItemToFront(
    request: WorkflowSessionQueuePushFrontRequest,
  ): Promise<WorkflowSessionQueuePushFrontResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return { ok: true, priority: 1 };
    }

    return invokeWorkflowCommand<WorkflowSessionQueuePushFrontResponse>(
      'workflow_push_execution_session_queue_item_to_front',
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
          settings: standardRetentionPolicySettings(365),
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
          settings: standardRetentionPolicySettings(request.retention_days),
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

  async artifactDescriptor(
    request: WorkflowArtifactDescriptorQueryRequest,
  ): Promise<WorkflowArtifactDescriptorQueryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        artifact: {
          artifact_id: request.artifact_id,
          payload_kind: 'text',
          lifecycle_state: 'retained',
          retention_state: 'retained',
          byte_length: mockArtifactBodyBytes().length,
          content_hash: 'mock-artifact-sha256',
          format: {
            format_id: 'txt',
            media_type: 'text/plain',
          },
          attribution: {
            workflow_run_id: 'mock-run',
          },
          access_modes: ['read', 'download'],
          read_handle: `artifact-read://${request.artifact_id}`,
          stream_handle: null,
          retention_reason: 'Mock retained artifact',
        },
      };
    }

    return invokeWorkflowCommand<WorkflowArtifactDescriptorQueryResponse>('workflow_artifact_descriptor', {
      request,
    });
  }

  async readArtifactBody(request: WorkflowArtifactReadRequest): Promise<WorkflowArtifactBodyRead> {
    if (USE_WORKFLOW_MOCKS) {
      const body = mockArtifactBodyBytes();
      return {
        response: {
          artifact_id: request.artifact_id,
          media_type: 'text/plain',
          body_transport: 'binary_body',
          read_handle: `artifact-read://${request.artifact_id}`,
          byte_length: body.length,
          content_hash: 'mock-artifact-sha256',
          complete: true,
        },
        body,
      };
    }

    return invokeWorkflowCommand<WorkflowArtifactBodyRead>('workflow_read_artifact_body', {
      request,
    });
  }

  async acknowledgeArtifactConsumed(
    request: WorkflowArtifactConsumeAcknowledgementRequest,
  ): Promise<WorkflowArtifactConsumeAcknowledgementResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        artifact_id: request.artifact_id,
        retained_after_consume: true,
      };
    }

    return invokeWorkflowCommand<WorkflowArtifactConsumeAcknowledgementResponse>(
      'workflow_acknowledge_artifact_consumed',
      { request },
    );
  }

  async artifactPolicy(): Promise<WorkflowArtifactPolicy> {
    if (USE_WORKFLOW_MOCKS) {
      return mockArtifactPolicy();
    }

    return invokeWorkflowCommand<WorkflowArtifactPolicy>('workflow_artifact_policy');
  }

  async updateArtifactPolicy(policy: WorkflowArtifactPolicy): Promise<WorkflowArtifactPolicy> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        ...policy,
        policy_version: policy.policy_version + 1,
      };
    }

    return invokeWorkflowCommand<WorkflowArtifactPolicy>('workflow_update_artifact_policy', {
      policy,
    });
  }

  async artifactStoreStats(): Promise<WorkflowArtifactStoreStats> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        artifact_count: 1,
        retained_body_count: 1,
        retained_body_bytes: mockArtifactBodyBytes().length,
        memory_cache_body_count: 0,
        memory_cache_body_bytes: 0,
        streaming_body_count: 0,
        streaming_body_bytes: 0,
        metadata_only_count: 0,
      };
    }

    return invokeWorkflowCommand<WorkflowArtifactStoreStats>('workflow_artifact_store_stats');
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

  async startHfDownloadWithAudit(
    request: PumasHfDownloadRequest,
  ): Promise<PumasHfDownloadStartAuditResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        downloadId: 'mock-download',
        auditEventSeq: null,
      };
    }

    return invokeWorkflowCommand<PumasHfDownloadStartAuditResponse>('start_hf_download_with_audit', {
      request,
    });
  }
}

function standardRetentionPolicySettings(retentionDays: number): DiagnosticsRetentionPolicy['settings'] {
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

function mockArtifactBodyBytes(): number[] {
  return Array.from(new TextEncoder().encode('Mock retained artifact body\n'));
}

function mockArtifactPolicy(): WorkflowArtifactPolicy {
  return {
    policy_id: 'artifact-mock-v1',
    policy_version: 1,
    ttl_seconds: 604_800,
    max_disk_bytes: null,
    max_memory_bytes: null,
    max_single_artifact_bytes: null,
    spill_threshold_bytes: null,
    delete_on_consume: false,
  };
}
