import { Channel } from '@tauri-apps/api/core';
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
  WorkflowArtifactFormatCapabilities,
  WorkflowArtifactFormatSettings,
  WorkflowArtifactFormatSettingsQueryRequest,
  WorkflowArtifactFormatSettingsQueryResponse,
  WorkflowArtifactFormatSettingsUpdateRequest,
  WorkflowArtifactFormatSettingsUpdateResponse,
  WorkflowArtifactPolicy,
  WorkflowArtifactReadRequest,
  WorkflowArtifactStreamBodyRead,
  WorkflowArtifactStreamReadRequest,
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
  WorkflowExecutableValidationSnapshotRecord,
  WorkflowEvent,
  WorkflowGraphCurrentValidationRefreshRequest,
  WorkflowGraphCurrentValidationRefreshResponse,
  WorkflowGraphCurrentValidationSummaryRequest,
  WorkflowGraphCurrentValidationSummaryResponse,
  WorkflowGraphValidationLifecycleEventSnapshot,
  WorkflowGraphSessionExecutableValidationSnapshotPublishRequest,
  WorkflowManagedMediaDependencyId,
  WorkflowManagedMediaDependencyInstallFromStagingRequest,
  WorkflowManagedMediaDependencyStatus,
  WorkflowManagedMediaDependencyVersionActionRequest,
  WorkflowManagedMediaDependencyVersionSelectionRequest,
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
  protected emitWorkflowEvent(_event: WorkflowEvent): void {}

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

    const channel = new Channel<WorkflowEvent>();
    channel.onmessage = (event: WorkflowEvent) => {
      this.emitWorkflowEvent(event);
    };

    return invokeWorkflowCommand<WorkflowRunResponse>('workflow_run_execution_session', {
      request,
      channel,
    });
  }

  async publishGraphSessionExecutableValidationSnapshot(
    request: WorkflowGraphSessionExecutableValidationSnapshotPublishRequest,
  ): Promise<WorkflowExecutableValidationSnapshotRecord> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        schema_version: 2,
        validation_snapshot_id: request.validation_snapshot_id ?? 'mock-validation-snapshot',
        workflow_id: request.workflow_id,
        workflow_version_id: `${request.workflow_id}@${request.workflow_semantic_version}`,
        workflow_semantic_version: request.workflow_semantic_version,
        workflow_execution_fingerprint: 'mock-workflow-execution-fingerprint',
        descriptor_contract_version: 1,
        graph_revision: 'mock-graph-revision',
        validation_session_id: request.validation_session_id ?? 'mock-validation-session',
        validation_summary: null,
        nodes: [],
      };
    }

    return invokeWorkflowCommand<WorkflowExecutableValidationSnapshotRecord>(
      'publish_graph_session_executable_validation_snapshot',
      { request },
    );
  }

  async currentGraphValidationSummary(
    request: WorkflowGraphCurrentValidationSummaryRequest,
  ): Promise<WorkflowGraphCurrentValidationSummaryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        graph_session_id: request.graph_session_id,
        requested_graph_revision: request.graph_revision,
        current_graph_revision: request.graph_revision,
        validation_session_id: 'mock-validation-session',
        state: 'current',
        summary: {
          status: 'executable',
          executable: true,
          enqueue_disabled_reasons: [],
          diagnostics_count: 0,
          blocking_diagnostics_count: 0,
        },
        submit_gate: {
          allowed: true,
        },
        diagnostics: [],
      };
    }

    return invokeWorkflowCommand<WorkflowGraphCurrentValidationSummaryResponse>(
      'current_graph_validation_summary',
      { request },
    );
  }

  async refreshCurrentGraphValidationSummary(
    request: WorkflowGraphCurrentValidationRefreshRequest,
  ): Promise<WorkflowGraphCurrentValidationRefreshResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        summary: {
          graph_session_id: request.graph_session_id,
          requested_graph_revision: request.graph_revision,
          current_graph_revision: request.graph_revision,
          validation_session_id: 'mock-validation-session',
          state: 'current',
          summary: {
            status: 'executable',
            executable: true,
            enqueue_disabled_reasons: [],
            diagnostics_count: 0,
            blocking_diagnostics_count: 0,
          },
          submit_gate: {
            allowed: true,
          },
          diagnostics: [],
        },
        node_projections: [],
      };
    }

    return invokeWorkflowCommand<WorkflowGraphCurrentValidationRefreshResponse>(
      'refresh_current_graph_validation_summary',
      { request },
    );
  }

  async graphValidationLifecycleEventSnapshot(
    graphSessionId: string,
  ): Promise<WorkflowGraphValidationLifecycleEventSnapshot> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        events: [],
        dropped_event_count: 0,
      };
    }

    return invokeWorkflowCommand<WorkflowGraphValidationLifecycleEventSnapshot>(
      'graph_validation_lifecycle_event_snapshot',
      { graphSessionId },
    );
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

  async readArtifactStream(
    request: WorkflowArtifactStreamReadRequest,
  ): Promise<WorkflowArtifactStreamBodyRead> {
    if (USE_WORKFLOW_MOCKS) {
      const body = mockArtifactBodyBytes();
      return {
        response: {
          artifact_id: request.artifact_id,
          stream_handle: `artifact-stream://${request.artifact_id}`,
          media_type: 'text/plain',
          body_transport: 'binary_body',
          byte_length: body.length,
          available_byte_length: body.length,
          lifecycle_state: 'streaming',
          complete: false,
        },
        body,
      };
    }

    return invokeWorkflowCommand<WorkflowArtifactStreamBodyRead>('workflow_read_artifact_stream', {
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

  async artifactFormatSettings(
    request: WorkflowArtifactFormatSettingsQueryRequest = {},
  ): Promise<WorkflowArtifactFormatSettingsQueryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        settings: mockArtifactFormatSettings(),
      };
    }

    return invokeWorkflowCommand<WorkflowArtifactFormatSettingsQueryResponse>(
      'workflow_artifact_format_settings',
      { request },
    );
  }

  async updateArtifactFormatSettings(
    request: WorkflowArtifactFormatSettingsUpdateRequest,
  ): Promise<WorkflowArtifactFormatSettingsUpdateResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        settings: request.settings,
      };
    }

    return invokeWorkflowCommand<WorkflowArtifactFormatSettingsUpdateResponse>(
      'workflow_update_artifact_format_settings',
      { request },
    );
  }

  async artifactFormatCapabilities(): Promise<WorkflowArtifactFormatCapabilities> {
    if (USE_WORKFLOW_MOCKS) {
      return mockArtifactFormatCapabilities();
    }

    return invokeWorkflowCommand<WorkflowArtifactFormatCapabilities>(
      'workflow_artifact_format_capabilities',
    );
  }

  async listManagedMediaDependencies(): Promise<WorkflowManagedMediaDependencyStatus[]> {
    if (USE_WORKFLOW_MOCKS) {
      return mockManagedMediaDependencies();
    }

    return invokeWorkflowCommand<WorkflowManagedMediaDependencyStatus[]>(
      'workflow_list_managed_media_dependencies',
    );
  }

  async managedMediaDependencyStatus(
    id: WorkflowManagedMediaDependencyId,
  ): Promise<WorkflowManagedMediaDependencyStatus> {
    if (USE_WORKFLOW_MOCKS) {
      return mockManagedMediaDependencyStatus(id);
    }

    return invokeWorkflowCommand<WorkflowManagedMediaDependencyStatus>(
      'workflow_managed_media_dependency_status',
      { id },
    );
  }

  async installManagedMediaDependencyFromStaging(
    request: WorkflowManagedMediaDependencyInstallFromStagingRequest,
  ): Promise<WorkflowManagedMediaDependencyStatus> {
    if (USE_WORKFLOW_MOCKS) {
      return mockManagedMediaDependencyStatus(request.id, request.version);
    }

    return invokeWorkflowCommand<WorkflowManagedMediaDependencyStatus>(
      'workflow_install_managed_media_dependency_from_staging',
      {
        id: request.id,
        version: request.version,
        staging_dir: request.staging_dir,
      },
    );
  }

  async selectManagedMediaDependencyVersion(
    request: WorkflowManagedMediaDependencyVersionSelectionRequest,
  ): Promise<WorkflowManagedMediaDependencyStatus> {
    if (USE_WORKFLOW_MOCKS) {
      return mockManagedMediaDependencyStatus(request.id, request.version ?? undefined);
    }

    return invokeWorkflowCommand<WorkflowManagedMediaDependencyStatus>(
      'workflow_select_managed_media_dependency_version',
      {
        id: request.id,
        version: request.version,
      },
    );
  }

  async setDefaultManagedMediaDependencyVersion(
    request: WorkflowManagedMediaDependencyVersionSelectionRequest,
  ): Promise<WorkflowManagedMediaDependencyStatus> {
    if (USE_WORKFLOW_MOCKS) {
      return mockManagedMediaDependencyStatus(request.id, request.version ?? undefined);
    }

    return invokeWorkflowCommand<WorkflowManagedMediaDependencyStatus>(
      'workflow_set_default_managed_media_dependency_version',
      {
        id: request.id,
        version: request.version,
      },
    );
  }

  async activateManagedMediaDependencyVersion(
    request: WorkflowManagedMediaDependencyVersionActionRequest,
  ): Promise<WorkflowManagedMediaDependencyStatus> {
    if (USE_WORKFLOW_MOCKS) {
      return mockManagedMediaDependencyStatus(request.id, request.version);
    }

    return invokeWorkflowCommand<WorkflowManagedMediaDependencyStatus>(
      'workflow_activate_managed_media_dependency_version',
      {
        id: request.id,
        version: request.version,
      },
    );
  }

  async removeManagedMediaDependencyVersion(
    request: WorkflowManagedMediaDependencyVersionActionRequest,
  ): Promise<WorkflowManagedMediaDependencyStatus> {
    if (USE_WORKFLOW_MOCKS) {
      return mockManagedMediaDependencyStatus(request.id);
    }

    return invokeWorkflowCommand<WorkflowManagedMediaDependencyStatus>(
      'workflow_remove_managed_media_dependency_version',
      {
        id: request.id,
        version: request.version,
      },
    );
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

function mockArtifactFormatSettings(): WorkflowArtifactFormatSettings {
  return {
    image: {
      format_id: 'jpg',
      quality_percent: 75,
      color_profile_id: 'srgb',
    },
    audio: {
      container_id: 'ogg',
      codec_id: 'opus',
      bitrate_kbps: 96,
    },
    video: {
      container_id: 'ivf',
      codec_id: 'svt_av1',
      crf: 32,
      bit_depth: '8bit',
    },
    three_d: {
      format_id: 'glb',
    },
  };
}

function mockArtifactFormatCapabilities(): WorkflowArtifactFormatCapabilities {
  return {
    image_formats: [
      mockMediaFormatOption('jpg', 'JPEG', 'image/jpeg', {
        quality_min_percent: 1,
        quality_max_percent: 100,
        color_profile_ids: ['srgb'],
        provided_by_dependency_id: 'oiiotool',
      }),
      mockMediaFormatOption('png', 'PNG', 'image/png', {
        color_profile_ids: ['srgb'],
        provided_by_dependency_id: 'oiiotool',
      }),
    ],
    audio_formats: [
      mockMediaFormatOption('ogg', 'Ogg', 'audio/ogg', {
        codec_ids: ['opus', 'vorbis'],
        bitrate_min_kbps: 32,
        bitrate_max_kbps: 512,
        provided_by_dependency_id: 'ffmpeg',
      }),
      mockMediaFormatOption('wav', 'WAV', 'audio/wav', {
        codec_ids: ['pcm'],
        provided_by_dependency_id: 'ffmpeg',
      }),
    ],
    video_formats: [
      mockMediaFormatOption('ivf', 'AV1 IVF', 'video/av1', {
        codec_ids: ['svt_av1'],
        crf_min: 0,
        crf_max: 63,
        bit_depths: ['8bit', '10bit'],
        provided_by_dependency_id: 'ffmpeg',
      }),
    ],
    three_d_formats: [
      mockMediaFormatOption('glb', 'GLB', 'model/gltf-binary', {
        provided_by_dependency_id: 'pantograph-3d',
      }),
      mockMediaFormatOption('gltf', 'glTF', 'model/gltf+json', {
        provided_by_dependency_id: 'pantograph-3d',
      }),
    ],
  };
}

function mockMediaFormatOption(
  formatId: string,
  displayName: string,
  mediaType: string,
  options: Partial<WorkflowArtifactFormatCapabilities['image_formats'][number]>,
): WorkflowArtifactFormatCapabilities['image_formats'][number] {
  return {
    format_id: formatId,
    display_name: displayName,
    media_type: mediaType,
    codec_ids: options.codec_ids ?? [],
    quality_min_percent: options.quality_min_percent ?? null,
    quality_max_percent: options.quality_max_percent ?? null,
    bitrate_min_kbps: options.bitrate_min_kbps ?? null,
    bitrate_max_kbps: options.bitrate_max_kbps ?? null,
    crf_min: options.crf_min ?? null,
    crf_max: options.crf_max ?? null,
    bit_depths: options.bit_depths ?? [],
    color_profile_ids: options.color_profile_ids ?? [],
    provided_by_dependency_id: options.provided_by_dependency_id ?? 'mock',
    provided_by_version: options.provided_by_version ?? null,
  };
}

function mockManagedMediaDependencies(): WorkflowManagedMediaDependencyStatus[] {
  return [
    mockManagedMediaDependencyStatus('ffmpeg', '7.1'),
    mockManagedMediaDependencyStatus('ocioconvert'),
    mockManagedMediaDependencyStatus('oiiotool'),
    mockManagedMediaDependencyStatus('open_color_io'),
  ];
}

function mockManagedMediaDependencyStatus(
  id: WorkflowManagedMediaDependencyId,
  installedVersion?: string,
): WorkflowManagedMediaDependencyStatus {
  const catalogVersion = managedMediaCatalogVersion(id);
  const version = installedVersion ?? catalogVersion;
  const installed = installedVersion !== undefined;
  const missingFiles = installed ? [] : managedMediaExpectedFiles(id);
  const category = id === 'open_color_io' ? 'native_library_artifact' : 'tool_binary';

  return {
    id,
    display_name: managedMediaDisplayName(id),
    category,
    install_state: installed ? 'installed' : 'missing',
    readiness: installed ? 'ready' : 'missing',
    available: installed,
    missing_files: missingFiles,
    catalog: {
      id,
      display_name: managedMediaDisplayName(id),
      category,
      source: managedMediaSource(id),
      license_redistribution: id === 'ffmpeg'
        ? 'LGPL-2.1-or-later/GPL-2.0-or-later depending on enabled codecs'
        : 'BSD-3-Clause',
      platform_key: 'mock-linux-x86_64',
      version: catalogVersion,
      package_kind: id === 'open_color_io' ? 'native_package' : 'archive',
      archive_kind: id === 'open_color_io' ? null : 'tar_gz',
      archive_name: null,
      download_url: null,
      expected_files: managedMediaExpectedFiles(id),
      checksum_sha256: null,
      signature: null,
    },
    selection: {
      selected_version: installed ? version : null,
      active_version: installed ? version : null,
      default_version: installed ? version : null,
    },
    versions: installed
      ? [
          {
            version,
            platform_key: 'mock-linux-x86_64',
            install_root: `/mock/pantograph/managed-media/${id}/${version}`,
            expected_files: managedMediaExpectedFiles(id),
            missing_files: [],
            install_state: 'installed',
            readiness: 'ready',
            selected: true,
            active: true,
          },
        ]
      : [],
  };
}

function managedMediaDisplayName(id: WorkflowManagedMediaDependencyId): string {
  switch (id) {
    case 'ffmpeg':
      return 'FFmpeg';
    case 'ocioconvert':
      return 'ocioconvert';
    case 'oiiotool':
      return 'oiiotool';
    case 'open_color_io':
      return 'OpenColorIO';
  }
}

function managedMediaCatalogVersion(id: WorkflowManagedMediaDependencyId): string {
  switch (id) {
    case 'ffmpeg':
      return '7.1';
    case 'ocioconvert':
    case 'open_color_io':
      return '2.4.2';
    case 'oiiotool':
      return '3.0.0';
  }
}

function managedMediaExpectedFiles(id: WorkflowManagedMediaDependencyId): string[] {
  switch (id) {
    case 'ffmpeg':
      return ['bin/ffmpeg'];
    case 'ocioconvert':
      return ['bin/ocioconvert'];
    case 'oiiotool':
      return ['bin/oiiotool'];
    case 'open_color_io':
      return ['lib/OpenColorIO'];
  }
}

function managedMediaSource(
  id: WorkflowManagedMediaDependencyId,
): { owner: string; project: string } {
  switch (id) {
    case 'ffmpeg':
      return { owner: 'FFmpeg', project: 'FFmpeg' };
    case 'oiiotool':
      return { owner: 'AcademySoftwareFoundation', project: 'OpenImageIO' };
    case 'ocioconvert':
    case 'open_color_io':
      return { owner: 'AcademySoftwareFoundation', project: 'OpenColorIO' };
  }
}
