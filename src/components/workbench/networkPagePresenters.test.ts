import test from 'node:test';
import assert from 'node:assert/strict';

import type { WorkflowLocalNetworkNodeStatus } from '../../services/workflow/types.ts';
import {
  buildNetworkFactRows,
  buildSelectedRunPlacementRows,
  formatCpuUsage,
  formatGpuAvailability,
  formatNetworkBytes,
  formatSchedulerLoad,
  findSelectedRunPlacement,
  formatSelectedRunRequirementList,
  formatSelectedRunLocalState,
  formatSelectedRunPlacementState,
  formatSelectedRunRuntimePosture,
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
          required_backends: ['python'],
          required_models: ['model-a'],
        },
        {
          workflow_run_id: 'run-queued',
          workflow_execution_session_id: 'session-queued',
          workflow_id: 'workflow-a',
          state: 'queued',
          runtime_loaded: false,
          required_backends: [],
          required_models: [],
        },
      ],
    },
    degradation_warnings: ['GPU probe unavailable'],
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
  assert.equal(formatSelectedRunPlacementState(placement), 'Running locally');
  assert.equal(formatSelectedRunPlacementState(findSelectedRunPlacement(node, 'run-queued')), 'Queued locally');
  assert.equal(formatSelectedRunPlacementState(null), 'Unavailable');
  assert.equal(formatSelectedRunRequirementList(placement?.required_backends ?? [], 'No backends'), 'python');
  assert.equal(formatSelectedRunRequirementList([], 'No models'), 'No models');
});

test('buildSelectedRunPlacementRows exposes selected-run local relevance facts', () => {
  const placement = findSelectedRunPlacement(createNode(), 'run-active');
  const rows = buildSelectedRunPlacementRows(placement);

  assert.equal(rows.find((row) => row.label === 'State')?.value, 'Running locally');
  assert.equal(rows.find((row) => row.label === 'Session')?.value, 'session-active');
  assert.equal(rows.find((row) => row.label === 'Workflow')?.value, 'workflow-a');
  assert.equal(rows.find((row) => row.label === 'Runtime')?.value, 'Runtime session loaded');
  assert.equal(rows.find((row) => row.label === 'Backends')?.value, 'python');
  assert.equal(rows.find((row) => row.label === 'Models')?.value, 'model-a');
  assert.equal(buildSelectedRunPlacementRows(null).find((row) => row.label === 'State')?.value, 'Not scheduled locally');
});

test('buildNetworkFactRows summarizes local node capabilities', () => {
  const rows = buildNetworkFactRows(createNode());

  assert.equal(rows.find((row) => row.label === 'Node ID')?.value, 'local-node');
  assert.equal(rows.find((row) => row.label === 'OS')?.value, 'Linux 6');
  assert.equal(rows.find((row) => row.label === 'Network Interfaces')?.value, '1');
});
