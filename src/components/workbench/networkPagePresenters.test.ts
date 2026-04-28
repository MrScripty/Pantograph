import test from 'node:test';
import assert from 'node:assert/strict';

import type { WorkflowLocalNetworkNodeStatus } from '../../services/workflow/types.ts';
import {
  buildNetworkFactRows,
  formatCpuUsage,
  formatGpuAvailability,
  formatNetworkBytes,
  formatSchedulerLoad,
  formatSelectedRunLocalState,
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
});

test('buildNetworkFactRows summarizes local node capabilities', () => {
  const rows = buildNetworkFactRows(createNode());

  assert.equal(rows.find((row) => row.label === 'Node ID')?.value, 'local-node');
  assert.equal(rows.find((row) => row.label === 'OS')?.value, 'Linux 6');
  assert.equal(rows.find((row) => row.label === 'Network Interfaces')?.value, '1');
});
