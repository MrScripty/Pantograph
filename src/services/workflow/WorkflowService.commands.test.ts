import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { WorkflowCommandService } from './WorkflowCommandService.ts';
import { MOCK_NODE_DEFINITIONS } from './mocks.ts';
import type {
  DiagnosticsRetentionPolicySettings,
  WorkflowRetentionCleanupResponse,
  WorkflowRetentionPolicyUpdateResponse,
} from '../diagnostics/types.ts';
import type {
  WorkflowArtifactBodyRead,
  WorkflowArtifactDescriptorQueryResponse,
  WorkflowArtifactFormatCapabilities,
  WorkflowArtifactFormatSettingsQueryResponse,
  WorkflowArtifactFormatSettingsUpdateResponse,
  WorkflowArtifactPolicy,
  WorkflowArtifactStreamBodyRead,
  WorkflowArtifactStoreStats,
  WorkflowAdminQueueCancelResponse,
  WorkflowAdminQueuePushFrontResponse,
  WorkflowAdminQueueReprioritizeResponse,
  WorkflowGraphCurrentValidationSummaryResponse,
  WorkflowExecutionSessionCloseResponse,
  WorkflowExecutionSessionCreateResponse,
  WorkflowExecutableValidationSnapshotRecord,
  InferencePortPayloadContract,
  WorkflowBackendTaskCapability,
  WorkflowManagedMediaDependencyStatus,
  WorkflowRunResponse,
  WorkflowSessionQueueCancelResponse,
  WorkflowSessionQueuePushFrontResponse,
  WorkflowSessionQueueReprioritizeResponse,
  WorkflowTechnicalFitDecision,
  WorkflowTechnicalFitRequest,
} from './types.ts';

interface WorkbenchSettingsNetworkContractFixture {
  artifact_policy: WorkflowArtifactPolicy;
  artifact_format_settings_response: WorkflowArtifactFormatSettingsQueryResponse;
  artifact_format_capabilities: WorkflowArtifactFormatCapabilities;
}

interface TechnicalFitContractFixture {
  technical_fit_request: WorkflowTechnicalFitRequest;
  technical_fit_decision: WorkflowTechnicalFitDecision;
}

function installWindowMock(): void {
  const target = globalThis as unknown as Record<string, unknown>;
  target.window = globalThis;
}

function loadWorkbenchSettingsNetworkContractFixture(): WorkbenchSettingsNetworkContractFixture {
  const fixtureUrl = new URL(
    '../../../crates/pantograph-workflow-service/tests/fixtures/workbench_settings_network_contract.json',
    import.meta.url,
  );
  return JSON.parse(readFileSync(fixtureUrl, 'utf8')) as WorkbenchSettingsNetworkContractFixture;
}

function loadTechnicalFitContractFixture(): TechnicalFitContractFixture {
  const fixtureUrl = new URL(
    '../../../crates/pantograph-workflow-service/tests/fixtures/technical_fit_contract.json',
    import.meta.url,
  );
  return JSON.parse(readFileSync(fixtureUrl, 'utf8')) as TechnicalFitContractFixture;
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

test('mock node definitions expose intent-only Pumas inference ports', () => {
  const pumaLib = MOCK_NODE_DEFINITIONS.find((definition) => definition.node_type === 'puma-lib');
  const llmInference = MOCK_NODE_DEFINITIONS.find(
    (definition) => definition.node_type === 'llm-inference',
  );

  assert.ok(pumaLib);
  assert.ok(llmInference);
  const pumasModelRef = pumaLib.outputs.find((port) => port.id === 'pumas_model_ref');
  assert.deepEqual(pumasModelRef?.options_provider, {
    node_type: 'puma-lib',
    port_id: 'pumas_model_ref',
  });
  assert.ok(pumaLib.outputs.some((port) => port.id === 'pumas_model_ref'));
  assert.equal(pumaLib.outputs.some((port) => port.id === 'model_path'), false);
  assert.equal(pumaLib.outputs.some((port) => port.id === 'resolved_model_package_facts'), false);
  assert.equal(pumaLib.outputs.some((port) => port.id === 'dependency_requirements'), false);
  assert.equal(pumaLib.outputs.some((port) => port.id === 'inference_settings'), false);
  assert.ok(llmInference.inputs.some((port) => port.id === 'task_kind'));
  assert.ok(llmInference.inputs.some((port) => port.id === 'runtime'));
  assert.ok(llmInference.inputs.some((port) => port.id === 'device'));
  assert.ok(!llmInference.inputs.some((port) => port.id === 'backend_key'));
  assert.ok(!llmInference.inputs.some((port) => port.id === 'runtime_hint'));
  assert.equal(llmInference.inputs.some((port) => port.id === 'resolved_model_source'), false);
  assert.equal(
    llmInference.inputs.some((port) => port.id === 'resolved_model_package_facts'),
    false,
  );
  assert.ok(llmInference.outputs.some((port) => port.id === 'diagnostics'));
  const prompt = llmInference.inputs.find((port) => port.id === 'prompt');
  const denoisingScheduler = llmInference.inputs.find((port) => port.id === 'denoising_scheduler');
  const audio = llmInference.inputs.find((port) => port.id === 'audio');
  const response = llmInference.outputs.find((port) => port.id === 'response');
  const results = llmInference.outputs.find((port) => port.id === 'results');
  const embedding = llmInference.outputs.find((port) => port.id === 'embedding');
  const scores = llmInference.outputs.find((port) => port.id === 'scores');
  const kvCacheOut = llmInference.outputs.find((port) => port.id === 'kv_cache_out');
  const diagnostics = llmInference.outputs.find((port) => port.id === 'diagnostics');
  assert.ok(
    prompt?.inference_payloads?.some(
      (payload) => payload.task_id === 'text_generation' && payload.role === 'task_input',
    ),
  );
  assert.ok(
    prompt?.inference_payloads?.some(
      (payload) =>
        payload.task_id === 'image_generation' && payload.input_kind === 'image_generation',
    ),
  );
  assert.deepEqual(denoisingScheduler?.inference_payloads, [
    { task_id: 'image_generation', role: 'options' },
  ]);
  assert.deepEqual(audio?.inference_payloads, [
    { task_id: 'audio_transcription', role: 'task_input', input_kind: 'audio_transcription' },
  ]);
  assert.ok(
    response?.inference_payloads?.some(
      (payload) => payload.task_id === 'chat_completion' && payload.role === 'task_output',
    ),
  );
  assert.ok(
    response?.inference_payloads?.some(
      (payload) =>
        payload.task_id === 'audio_transcription' &&
        payload.result_kind === 'audio_transcription',
    ),
  );
  assert.ok(
    results?.inference_payloads?.some(
      (payload) => payload.task_id === 'rerank' && payload.result_kind === 'rerank',
    ),
  );
  assert.ok(
    results?.inference_payloads?.some(
      (payload) =>
        payload.task_id === 'image_generation' && payload.result_kind === 'image_generation',
    ),
  );
  assert.deepEqual(embedding?.inference_payloads, [
    { task_id: 'embedding', role: 'task_output', result_kind: 'embedding' },
  ]);
  assert.deepEqual(scores?.inference_payloads, [
    { task_id: 'rerank', role: 'task_output', result_kind: 'rerank' },
  ]);
  assert.ok(
    kvCacheOut?.inference_payloads?.some(
      (payload) => payload.task_id === 'chat_completion' && payload.role === 'cache_handle',
    ),
  );
  assert.ok(
    kvCacheOut?.inference_payloads?.every((payload) => payload.role === 'cache_handle'),
  );
  assert.ok(
    diagnostics?.inference_payloads?.some(
      (payload) => payload.task_id === 'image_generation' && payload.role === 'diagnostics',
    ),
  );
  assert.ok(
    diagnostics?.inference_payloads?.some(
      (payload) => payload.task_id === 'audio_transcription' && payload.role === 'diagnostics',
    ),
  );
  assert.ok(!llmInference.inputs.some((port) => port.id === 'runtime_id'));

  const policyFields = [
    'backend_key',
    'runtime_id',
    'runtime_instance_id',
    'selected_backend_key',
    'selected_runtime_id',
    'scheduler_policy',
    'scheduler_policy_id',
    'admission',
    'reservation',
    'eviction',
    'priority',
  ];
  for (const port of [...llmInference.inputs, ...llmInference.outputs]) {
    for (const payload of port.inference_payloads ?? []) {
      for (const field of policyFields) {
        assert.equal(
          Object.prototype.hasOwnProperty.call(payload, field),
          false,
          `${port.id} inference payload unexpectedly exposes ${field}`,
        );
      }
    }
  }
});

test('technical-fit contract fixture preserves runtime variant and device facts', () => {
  const fixture = loadTechnicalFitContractFixture();
  const request: WorkflowTechnicalFitRequest = fixture.technical_fit_request;
  const decision: WorkflowTechnicalFitDecision = fixture.technical_fit_decision;

  assert.equal(request.override_selection?.runtime_id, 'pytorch');
  assert.equal(request.override_selection?.runtime_variant_id, 'pytorch.cuda');
  assert.equal(request.device_policy?.policy, 'explicit');
  if (request.device_policy?.policy === 'explicit') {
    assert.equal(request.device_policy.device_class, 'cuda');
    assert.equal(request.device_policy.device_id, 'cuda:0');
  }

  assert.equal(decision.selected_runtime_variant_id, 'pytorch.cuda');
  assert.equal(decision.selected_device_class, 'cuda');
  assert.equal(decision.selected_device_id, 'cuda:0');
  assert.equal(decision.device_diagnostics?.[0]?.code, 'candidate_unavailable');
  assert.equal(decision.device_diagnostics?.[0]?.severity, 'warning');
  assert.equal(decision.reasons?.[0]?.code, 'runtime_requirements');
  assert.equal(decision.reasons?.[2]?.code, 'automatic_ranking');
  assert.equal(decision.selection_policy_trace?.policy_version, 1);
  assert.equal(
    decision.selection_policy_trace?.candidate_set_summary?.eligible_candidate_count,
    1,
  );
  assert.equal(
    decision.selection_policy_trace?.ranking_reason,
    'readiness_history_preferred',
  );
  assert.equal(
    decision.selection_policy_trace?.seed_basis,
    'workflow:wf-image:pytorch',
  );
});

test('frontend inference payload contract accepts roadmap depth execution kinds', () => {
  const payloads: InferencePortPayloadContract[] = [
    {
      task_id: 'depth_estimation',
      role: 'task_input',
      input_kind: 'depth_estimation',
    },
    {
      task_id: 'depth_estimation',
      role: 'task_output',
      result_kind: 'depth_estimation',
    },
  ];

  assert.deepEqual(payloads, [
    {
      task_id: 'depth_estimation',
      role: 'task_input',
      input_kind: 'depth_estimation',
    },
    {
      task_id: 'depth_estimation',
      role: 'task_output',
      result_kind: 'depth_estimation',
    },
  ]);
});

test('frontend workflow capability facts accept roadmap depth task contracts', () => {
  const capability: WorkflowBackendTaskCapability = {
    task_id: 'depth_estimation',
    support_tier: 'roadmap',
    modality_signature: {
      inputs: ['image'],
      outputs: ['image', 'point_cloud'],
    },
    request_contract: {
      task_id: 'depth_estimation',
      input_kind: 'depth_estimation',
      result_kind: 'depth_estimation',
      execution_supported: false,
      streaming_support: 'unsupported',
      required_input_modalities: ['image'],
      output_modalities: ['image', 'point_cloud'],
    },
  };

  assert.deepEqual(capability, {
    task_id: 'depth_estimation',
    support_tier: 'roadmap',
    modality_signature: {
      inputs: ['image'],
      outputs: ['image', 'point_cloud'],
    },
    request_contract: {
      task_id: 'depth_estimation',
      input_kind: 'depth_estimation',
      result_kind: 'depth_estimation',
      execution_supported: false,
      streaming_support: 'unsupported',
      required_input_modalities: ['image'],
      output_modalities: ['image', 'point_cloud'],
    },
  });
});

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
  const snapshotResponse: WorkflowExecutableValidationSnapshotRecord = {
    schema_version: 2,
    validation_snapshot_id: 'wfvalsnap_test',
    workflow_id: 'workflow-a',
    workflow_version_id: 'workflow-version-a',
    workflow_semantic_version: '0.1.0',
    workflow_execution_fingerprint: 'workflow-fingerprint-a',
    descriptor_contract_version: 1,
    graph_revision: 'graph-revision-a',
    validation_session_id: 'validation-session-a',
    validation_summary: null,
    nodes: [],
  };
  const closeResponse: WorkflowExecutionSessionCloseResponse = { ok: true };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    if (cmd === 'workflow_create_execution_session') {
      return createResponse;
    }
    if (cmd === 'publish_graph_session_executable_validation_snapshot') {
      return snapshotResponse;
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
    const snapshot = await service.publishGraphSessionExecutableValidationSnapshot({
      workflow_id: 'workflow-a',
      workflow_semantic_version: '0.1.0',
      graph_session_id: 'graph-session-a',
      validation_session_id: 'validation-session-a',
      validation_snapshot_id: null,
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
    assert.deepEqual(snapshot, snapshotResponse);
    assert.deepEqual(run, runResponse);
    assert.deepEqual(closed, closeResponse);
    const runArgs = calls[2]?.args as { request?: unknown; channel?: unknown };
    assert.equal(typeof runArgs.channel, 'object');
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
        cmd: 'publish_graph_session_executable_validation_snapshot',
        args: {
          request: {
            workflow_id: 'workflow-a',
            workflow_semantic_version: '0.1.0',
            graph_session_id: 'graph-session-a',
            validation_session_id: 'validation-session-a',
            validation_snapshot_id: null,
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
          channel: runArgs.channel,
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

test('workflow command service forwards current validation summary requests', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const summaryResponse: WorkflowGraphCurrentValidationSummaryResponse = {
    graph_session_id: 'graph-session-a',
    requested_graph_revision: 'graph-revision-a',
    current_graph_revision: 'graph-revision-a',
    validation_session_id: 'validation-session-a',
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
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return summaryResponse;
  });

  try {
    const service = new WorkflowCommandService();
    const summary = await service.currentGraphValidationSummary({
      graph_session_id: 'graph-session-a',
      graph_revision: 'graph-revision-a',
    });

    assert.deepEqual(summary, summaryResponse);
    assert.deepEqual(calls, [
      {
        cmd: 'current_graph_validation_summary',
        args: {
          request: {
            graph_session_id: 'graph-session-a',
            graph_revision: 'graph-revision-a',
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

test('artifact store commands forward backend-owned descriptor body policy and consume contracts', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const descriptorResponse: WorkflowArtifactDescriptorQueryResponse = {
    artifact: {
      artifact_id: 'artifact-a',
      payload_kind: 'image',
      lifecycle_state: 'retained',
      retention_state: 'retained',
      byte_length: 3,
      content_hash: 'sha256:a',
      format: {
        format_id: 'png',
        media_type: 'image/png',
        conversion_id: 'conversion-a',
        conversion_status: 'converted',
        conversion_command_id: 'image_oiio_png_srgb',
        conversion_dependencies: [
          {
            dependency_id: 'oiiotool',
            active_version: '2.5.18',
            lease_id: 'lease-a',
            lease_holder: 'workflow_run:run-a/node:node-a/port:out/conversion:conversion-a',
          },
        ],
      },
      attribution: {
        workflow_run_id: 'run-a',
        node_id: 'node-a',
        port_id: 'out',
      },
      access_modes: ['read', 'download'],
      read_handle: 'artifact-read://artifact-a',
      stream_handle: null,
      retention_reason: 'retained for test',
    },
  };
  const bodyResponse: WorkflowArtifactBodyRead = {
    response: {
      artifact_id: 'artifact-a',
      media_type: 'image/png',
      body_transport: 'binary_body',
      read_handle: 'artifact-read://artifact-a',
      byte_length: 3,
      content_hash: 'sha256:a',
      complete: true,
    },
    body: [1, 2, 3],
  };
  const streamResponse: WorkflowArtifactStreamBodyRead = {
    response: {
      artifact_id: 'artifact-a',
      stream_handle: 'artifact-stream://artifact-a',
      media_type: 'image/png',
      body_transport: 'binary_body',
      byte_length: 2,
      available_byte_length: 3,
      lifecycle_state: 'streaming',
      complete: false,
    },
    body: [2, 3],
  };
  const policyResponse: WorkflowArtifactPolicy = {
    policy_id: 'artifact-policy-v1',
    policy_version: 1,
    ttl_seconds: 60,
    max_disk_bytes: null,
    max_memory_bytes: null,
    max_single_artifact_bytes: 1024,
    spill_threshold_bytes: null,
    delete_on_consume: false,
  };
  const updatedPolicyResponse: WorkflowArtifactPolicy = {
    ...policyResponse,
    policy_version: 2,
    delete_on_consume: true,
  };
  const statsResponse: WorkflowArtifactStoreStats = {
    artifact_count: 1,
    retained_body_count: 1,
    retained_body_bytes: 3,
    memory_cache_body_count: 0,
    memory_cache_body_bytes: 0,
    streaming_body_count: 0,
    streaming_body_bytes: 0,
    metadata_only_count: 0,
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    if (cmd === 'workflow_artifact_descriptor') {
      return descriptorResponse;
    }
    if (cmd === 'workflow_read_artifact_body') {
      return bodyResponse;
    }
    if (cmd === 'workflow_read_artifact_stream') {
      return streamResponse;
    }
    if (cmd === 'workflow_acknowledge_artifact_consumed') {
      return {
        artifact_id: 'artifact-a',
        retained_after_consume: false,
      };
    }
    if (cmd === 'workflow_artifact_policy') {
      return policyResponse;
    }
    if (cmd === 'workflow_update_artifact_policy') {
      return updatedPolicyResponse;
    }
    return statsResponse;
  });

  try {
    const service = new WorkflowCommandService();
    const descriptor = await service.artifactDescriptor({ artifact_id: 'artifact-a' });
    const body = await service.readArtifactBody({ artifact_id: 'artifact-a' });
    const stream = await service.readArtifactStream({
      artifact_id: 'artifact-a',
      byte_range_start: 1,
    });
    const acknowledgement = await service.acknowledgeArtifactConsumed({
      artifact_id: 'artifact-a',
      consumer_id: 'io-inspector',
    });
    const policy = await service.artifactPolicy();
    const updatedPolicy = await service.updateArtifactPolicy({
      ...policyResponse,
      delete_on_consume: true,
    });
    const stats = await service.artifactStoreStats();

    assert.deepEqual(descriptor, descriptorResponse);
    assert.equal(descriptor.artifact?.format?.conversion_status, 'converted');
    assert.equal(descriptor.artifact?.format?.conversion_dependencies?.[0]?.dependency_id, 'oiiotool');
    assert.deepEqual(body, bodyResponse);
    assert.deepEqual(stream, streamResponse);
    assert.deepEqual(acknowledgement, {
      artifact_id: 'artifact-a',
      retained_after_consume: false,
    });
    assert.deepEqual(policy, policyResponse);
    assert.deepEqual(updatedPolicy, updatedPolicyResponse);
    assert.deepEqual(stats, statsResponse);
    assert.deepEqual(calls, [
      {
        cmd: 'workflow_artifact_descriptor',
        args: {
          request: {
            artifact_id: 'artifact-a',
          },
        },
      },
      {
        cmd: 'workflow_read_artifact_body',
        args: {
          request: {
            artifact_id: 'artifact-a',
          },
        },
      },
      {
        cmd: 'workflow_read_artifact_stream',
        args: {
          request: {
            artifact_id: 'artifact-a',
            byte_range_start: 1,
          },
        },
      },
      {
        cmd: 'workflow_acknowledge_artifact_consumed',
        args: {
          request: {
            artifact_id: 'artifact-a',
            consumer_id: 'io-inspector',
          },
        },
      },
      {
        cmd: 'workflow_artifact_policy',
        args: {},
      },
      {
        cmd: 'workflow_update_artifact_policy',
        args: {
          policy: {
            ...policyResponse,
            delete_on_consume: true,
          },
        },
      },
      {
        cmd: 'workflow_artifact_store_stats',
        args: {},
      },
    ]);
  } finally {
    clearMocks();
  }
});

test('artifact format settings commands forward backend-owned settings and capabilities', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const settingsResponse: WorkflowArtifactFormatSettingsQueryResponse = {
    settings: {
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
    },
  };
  const updateResponse: WorkflowArtifactFormatSettingsUpdateResponse = {
    settings: {
      ...settingsResponse.settings,
      image: {
        ...settingsResponse.settings.image,
        quality_percent: 82,
      },
    },
  };
  const capabilitiesResponse: WorkflowArtifactFormatCapabilities = {
    image_formats: [
      {
        format_id: 'jpg',
        display_name: 'JPEG',
        media_type: 'image/jpeg',
        codec_ids: [],
        quality_min_percent: 1,
        quality_max_percent: 100,
        bitrate_min_kbps: null,
        bitrate_max_kbps: null,
        crf_min: null,
        crf_max: null,
        bit_depths: ['8bit', '16bit', 'float'],
        color_profile_ids: ['srgb'],
        provided_by_dependency_id: 'oiiotool',
        provided_by_version: null,
      },
    ],
    audio_formats: [
      {
        format_id: 'ogg',
        display_name: 'Ogg',
        media_type: 'audio/ogg',
        codec_ids: ['opus', 'vorbis'],
        quality_min_percent: null,
        quality_max_percent: null,
        bitrate_min_kbps: 32,
        bitrate_max_kbps: 512,
        crf_min: null,
        crf_max: null,
        bit_depths: [],
        color_profile_ids: [],
        provided_by_dependency_id: 'ffmpeg',
        provided_by_version: null,
      },
    ],
    video_formats: [],
    three_d_formats: [],
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    if (cmd === 'workflow_artifact_format_settings') {
      return settingsResponse;
    }
    if (cmd === 'workflow_update_artifact_format_settings') {
      return updateResponse;
    }
    return capabilitiesResponse;
  });

  try {
    const service = new WorkflowCommandService();
    const settings = await service.artifactFormatSettings();
    const updated = await service.updateArtifactFormatSettings({
      settings: updateResponse.settings,
      reason: 'test_settings_update',
    });
    const capabilities = await service.artifactFormatCapabilities();

    assert.deepEqual(settings, settingsResponse);
    assert.deepEqual(updated, updateResponse);
    assert.deepEqual(capabilities, capabilitiesResponse);
    assert.deepEqual(calls, [
      {
        cmd: 'workflow_artifact_format_settings',
        args: {
          request: {},
        },
      },
      {
        cmd: 'workflow_update_artifact_format_settings',
        args: {
          request: {
            settings: updateResponse.settings,
            reason: 'test_settings_update',
          },
        },
      },
      {
        cmd: 'workflow_artifact_format_capabilities',
        args: {},
      },
    ]);
  } finally {
    clearMocks();
  }
});

test('settings contract fixture crosses Rust and TypeScript service boundaries', async () => {
  installWindowMock();
  const fixture = loadWorkbenchSettingsNetworkContractFixture();
  mockIPC((cmd) => {
    if (cmd === 'workflow_artifact_policy') {
      return fixture.artifact_policy;
    }
    if (cmd === 'workflow_artifact_format_settings') {
      return fixture.artifact_format_settings_response;
    }
    if (cmd === 'workflow_artifact_format_capabilities') {
      return fixture.artifact_format_capabilities;
    }
    throw new Error(`Unexpected command ${cmd}`);
  });

  try {
    const service = new WorkflowCommandService();
    const policy = await service.artifactPolicy();
    const settings = await service.artifactFormatSettings();
    const capabilities = await service.artifactFormatCapabilities();

    assert.deepEqual(policy, fixture.artifact_policy);
    assert.deepEqual(settings, fixture.artifact_format_settings_response);
    assert.deepEqual(capabilities, fixture.artifact_format_capabilities);
    assert.equal(capabilities.image_formats[0].provided_by_dependency_id, 'oiiotool');
    assert.equal(settings.settings.video.codec_id, 'svt_av1');
  } finally {
    clearMocks();
  }
});

test('managed media dependency commands forward backend-owned status results', async () => {
  installWindowMock();
  const calls: Array<{ cmd: string; args: unknown }> = [];
  const missingStatus = managedMediaDependencyStatus('missing');
  const installedStatus = managedMediaDependencyStatus('installed');
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    if (cmd === 'workflow_list_managed_media_dependencies') {
      return [missingStatus];
    }
    if (cmd === 'workflow_managed_media_dependency_status') {
      return missingStatus;
    }
    return installedStatus;
  });

  try {
    const service = new WorkflowCommandService();
    const list = await service.listManagedMediaDependencies();
    const status = await service.managedMediaDependencyStatus('ffmpeg');
    const installed = await service.installManagedMediaDependencyFromStaging({
      id: 'ffmpeg',
      version: '7.1',
      staging_dir: '/tmp/staged-ffmpeg',
    });
    const selected = await service.selectManagedMediaDependencyVersion({
      id: 'ffmpeg',
      version: '7.1',
    });
    const defaulted = await service.setDefaultManagedMediaDependencyVersion({
      id: 'ffmpeg',
      version: '7.1',
    });
    const activated = await service.activateManagedMediaDependencyVersion({
      id: 'ffmpeg',
      version: '7.1',
    });
    const removed = await service.removeManagedMediaDependencyVersion({
      id: 'ffmpeg',
      version: '7.1',
    });

    assert.deepEqual(list, [missingStatus]);
    assert.deepEqual(status, missingStatus);
    assert.deepEqual(installed, installedStatus);
    assert.deepEqual(selected, installedStatus);
    assert.deepEqual(defaulted, installedStatus);
    assert.deepEqual(activated, installedStatus);
    assert.deepEqual(removed, installedStatus);
    assert.deepEqual(calls, [
      {
        cmd: 'workflow_list_managed_media_dependencies',
        args: {},
      },
      {
        cmd: 'workflow_managed_media_dependency_status',
        args: {
          id: 'ffmpeg',
        },
      },
      {
        cmd: 'workflow_install_managed_media_dependency_from_staging',
        args: {
          id: 'ffmpeg',
          version: '7.1',
          staging_dir: '/tmp/staged-ffmpeg',
        },
      },
      {
        cmd: 'workflow_select_managed_media_dependency_version',
        args: {
          id: 'ffmpeg',
          version: '7.1',
        },
      },
      {
        cmd: 'workflow_set_default_managed_media_dependency_version',
        args: {
          id: 'ffmpeg',
          version: '7.1',
        },
      },
      {
        cmd: 'workflow_activate_managed_media_dependency_version',
        args: {
          id: 'ffmpeg',
          version: '7.1',
        },
      },
      {
        cmd: 'workflow_remove_managed_media_dependency_version',
        args: {
          id: 'ffmpeg',
          version: '7.1',
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

function managedMediaDependencyStatus(
  state: 'missing' | 'installed',
): WorkflowManagedMediaDependencyStatus {
  const installed = state === 'installed';
  return {
    id: 'ffmpeg',
    display_name: 'FFmpeg',
    category: 'tool_binary',
    install_state: installed ? 'installed' : 'missing',
    readiness: installed ? 'ready' : 'missing',
    available: installed,
    missing_files: installed ? [] : ['bin/ffmpeg'],
    catalog: {
      id: 'ffmpeg',
      display_name: 'FFmpeg',
      category: 'tool_binary',
      source: {
        owner: 'FFmpeg',
        project: 'FFmpeg',
      },
      license_redistribution:
        'LGPL-2.1-or-later/GPL-2.0-or-later depending on enabled codecs',
      platform_key: 'linux-x86_64',
      version: '7.1',
      package_kind: 'archive',
      archive_kind: 'tar_gz',
      archive_name: null,
      download_url: null,
      expected_files: ['bin/ffmpeg'],
      checksum_sha256: null,
      signature: null,
    },
    selection: {
      selected_version: installed ? '7.1' : null,
      active_version: installed ? '7.1' : null,
      default_version: installed ? '7.1' : null,
    },
    versions: installed
      ? [
          {
            version: '7.1',
            platform_key: 'linux-x86_64',
            install_root: '/tmp/pantograph/managed/ffmpeg/7.1',
            expected_files: ['bin/ffmpeg'],
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
