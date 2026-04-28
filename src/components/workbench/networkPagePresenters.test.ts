import test from 'node:test';
import assert from 'node:assert/strict';

import type { NodeStatusProjectionRecord } from '../../services/diagnostics/types.ts';
import type { WorkflowLocalNetworkNodeStatus } from '../../services/workflow/types.ts';
import {
  buildNetworkFactRows,
  buildSelectedRunExecutionRows,
  buildSelectedRunResourceRows,
  buildSelectedRunPlacementRows,
  formatCpuUsage,
  formatGpuAvailability,
  formatNetworkBytes,
  formatNetworkProjectionFreshness,
  formatSchedulerLoad,
  findSelectedRunPlacement,
  formatSelectedRunRequirementList,
  formatSelectedRunModelCacheState,
  formatSelectedRunResourceCacheStatus,
  formatSelectedRunNodeStatus,
  formatSelectedRunLocalState,
  formatSelectedRunPlacementState,
  formatSelectedRunRuntimePosture,
  selectedRunNodeStatusClass,
  selectedRunResourceCacheClass,
  formatSessionLoad,
  formatTransportState,
} from './networkPagePresenters.ts';

function createNode(): WorkflowLocalNetworkNodeStatus {
  return {
    node_id: 'local-node',
    display_name: 'Local Node',
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
      disks: [
        {
          name: 'disk-a',
          mount_point: '/',
          total_bytes: 1_073_741_824,
          available_bytes: 536_870_912,
        },
      ],
      network_interfaces: [
        {
          name: 'eth0',
          total_received_bytes: 2_048,
          total_transmitted_bytes: 4_096,
        },
      ],
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
      active_run_count: 2,
      queued_run_count: 3,
      active_workflow_run_ids: ['run-active'],
      queued_workflow_run_ids: ['run-queued'],
      run_placements: [
        {
          workflow_run_id: 'run-active',
          workflow_execution_session_id: 'session-active',
          workflow_id: 'workflow-a',
          state: 'running',
          runtime_loaded: true,
          model_cache_state: 'unknown',
          required_backends: ['python'],
          required_models: ['model-a'],
        },
        {
          workflow_run_id: 'run-queued',
          workflow_execution_session_id: 'session-queued',
          workflow_id: 'workflow-a',
          state: 'queued',
          runtime_loaded: false,
          model_cache_state: 'not_required',
          required_backends: [],
          required_models: [],
        },
      ],
    },
    degradation_warnings: ['GPU probe unavailable'],
  };
}

function createNodeStatus(overrides: Partial<NodeStatusProjectionRecord>): NodeStatusProjectionRecord {
  return {
    workflow_run_id: 'run-active',
    workflow_id: 'workflow-a',
    workflow_version_id: 'wfver-a',
    workflow_semantic_version: '1.0.0',
    node_id: 'node-a',
    node_type: 'image.generate',
    node_version: '1.0.0',
    runtime_id: 'runtime-a',
    runtime_version: '2.0.0',
    model_id: 'model-a',
    model_version: '3.0.0',
    status: 'running',
    started_at_ms: 1,
    completed_at_ms: null,
    duration_ms: null,
    error: null,
    last_event_seq: 9,
    last_updated_at_ms: 10,
    ...overrides,
  };
}

test('network byte presenter keeps compact storage and traffic labels', () => {
  assert.equal(formatNetworkBytes(null), 'Unavailable');
  assert.equal(formatNetworkBytes(512), '512 B');
  assert.equal(formatNetworkBytes(2_048), '2.0 KiB');
  assert.equal(formatNetworkBytes(2_097_152), '2.0 MiB');
});

test('network state presenters avoid fake degraded metric values', () => {
  const node = createNode();

  assert.equal(formatTransportState('local_only'), 'Local only');
  assert.equal(formatTransportState('pairing_required'), 'Pairing required');
  assert.equal(formatCpuUsage(node.system.cpu.average_usage_percent), 'Usage unavailable');
  assert.equal(formatGpuAvailability(node), 'GPU probe unavailable');
});

test('network load presenters expose scheduler capacity', () => {
  const node = createNode();

  assert.equal(formatSchedulerLoad(node), '2 active / 3 queued');
  assert.equal(formatSessionLoad(node), '1/4 sessions, 1/2 loaded');
  assert.equal(formatSelectedRunLocalState(node, 'run-active'), 'Running locally');
  assert.equal(formatSelectedRunLocalState(node, 'run-queued'), 'Queued locally');
  assert.equal(formatSelectedRunLocalState(node, 'run-missing'), 'Not scheduled locally');
  assert.equal(formatSelectedRunLocalState(node, null), 'No selected run');

  const placement = findSelectedRunPlacement(node, 'run-active');
  assert.equal(placement?.workflow_execution_session_id, 'session-active');
  assert.equal(formatSelectedRunRuntimePosture(placement), 'Runtime session loaded');
  assert.equal(formatSelectedRunRuntimePosture(findSelectedRunPlacement(node, 'run-queued')), 'Runtime session not loaded');
  assert.equal(formatSelectedRunRuntimePosture(null), 'Unavailable');
  assert.equal(formatSelectedRunModelCacheState(placement?.model_cache_state ?? 'unknown'), 'Cache state unknown');
  assert.equal(
    formatSelectedRunModelCacheState(findSelectedRunPlacement(node, 'run-queued')?.model_cache_state ?? 'unknown'),
    'Model not required',
  );
  assert.equal(formatSelectedRunPlacementState(placement), 'Running locally');
  assert.equal(formatSelectedRunPlacementState(findSelectedRunPlacement(node, 'run-queued')), 'Queued locally');
  assert.equal(formatSelectedRunPlacementState(null), 'Unavailable');
  assert.equal(formatSelectedRunRequirementList(placement?.required_backends ?? [], 'No backends'), 'python');
  assert.equal(formatSelectedRunRequirementList([], 'No models'), 'No models');
});

test('formatNetworkProjectionFreshness keeps selected-run projection cursor visible', () => {
  assert.equal(formatNetworkProjectionFreshness(null), 'Projection unavailable');
  assert.equal(
    formatNetworkProjectionFreshness({
      projection_name: 'node_status',
      projection_version: 1,
      last_applied_event_seq: 42,
      status: 'rebuilding',
      rebuilt_at_ms: null,
      updated_at_ms: 5,
    }),
    'Rebuilding at seq 42',
  );
});

test('buildSelectedRunPlacementRows exposes selected-run local relevance facts', () => {
  const placement = findSelectedRunPlacement(createNode(), 'run-active');
  const rows = buildSelectedRunPlacementRows(placement);

  assert.equal(rows.find((row) => row.label === 'State')?.value, 'Running locally');
  assert.equal(rows.find((row) => row.label === 'Session')?.value, 'session-active');
  assert.equal(rows.find((row) => row.label === 'Workflow')?.value, 'workflow-a');
  assert.equal(rows.find((row) => row.label === 'Runtime')?.value, 'Runtime session loaded');
  assert.equal(rows.find((row) => row.label === 'Model Cache')?.value, 'Cache state unknown');
  assert.equal(rows.find((row) => row.label === 'Backends')?.value, 'python');
  assert.equal(rows.find((row) => row.label === 'Models')?.value, 'model-a');
  assert.equal(buildSelectedRunPlacementRows(null).find((row) => row.label === 'State')?.value, 'Not scheduled locally');
});

test('buildSelectedRunExecutionRows exposes selected-run node runtime and model facts', () => {
  const rows = buildSelectedRunExecutionRows([
    createNodeStatus({ node_id: 'node-a', status: 'running' }),
    createNodeStatus({
      node_id: 'node-b',
      status: 'waiting',
      runtime_id: null,
      runtime_version: null,
      model_id: '',
      model_version: null,
    }),
  ]);

  assert.deepEqual(rows, [
    {
      nodeId: 'node-a',
      status: 'Running',
      statusClass: 'border-cyan-800 bg-cyan-950/50 text-cyan-200',
      runtime: 'runtime-a@2.0.0',
      model: 'model-a@3.0.0',
    },
    {
      nodeId: 'node-b',
      status: 'Waiting',
      statusClass: 'border-amber-800 bg-amber-950/50 text-amber-200',
      runtime: 'Runtime unavailable',
      model: 'Model unavailable',
    },
  ]);
  assert.equal(formatSelectedRunNodeStatus('cancelled'), 'Cancelled');
  assert.match(selectedRunNodeStatusClass('completed'), /emerald/);
  assert.match(selectedRunNodeStatusClass('failed'), /red/);
});

test('buildSelectedRunResourceRows exposes selected-run Library usage facts', () => {
  const rows = buildSelectedRunResourceRows([
    {
      asset_id: 'model:llama',
      total_access_count: 4,
      run_access_count: 2,
      total_network_bytes: 2_048,
      last_accessed_at_ms: 1,
      last_operation: 'read',
      last_cache_status: 'cache_hit',
      last_event_seq: 10,
      last_updated_at_ms: 11,
    },
  ]);

  assert.deepEqual(rows, [
    {
      assetId: 'model:llama',
      category: 'Model',
      cacheStatus: 'Cache Hit',
      cacheClass: 'border-emerald-800 bg-emerald-950/50 text-emerald-200',
      networkBytes: '2.0 KiB',
      accessCount: '2',
    },
  ]);
  assert.equal(formatSelectedRunResourceCacheStatus(null), 'Cache status unavailable');
  assert.match(selectedRunResourceCacheClass('cache_miss'), /amber/);
  assert.match(selectedRunResourceCacheClass('failed'), /red/);
  assert.match(selectedRunResourceCacheClass(null), /neutral/);
});

test('buildNetworkFactRows summarizes local node capabilities', () => {
  const rows = buildNetworkFactRows(createNode());

  assert.equal(rows.find((row) => row.label === 'Node ID')?.value, 'local-node');
  assert.equal(rows.find((row) => row.label === 'OS')?.value, 'Linux 6');
  assert.equal(rows.find((row) => row.label === 'Network Interfaces')?.value, '1');
});
