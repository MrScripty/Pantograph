import type {
  WorkflowLocalNetworkNodeStatus,
  WorkflowNetworkTransportState,
} from '../../services/workflow/types';

export interface NetworkFactRow {
  label: string;
  value: string;
}

export function formatNetworkBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) {
    return 'Unavailable';
  }
  if (bytes >= 1_073_741_824) {
    return `${(bytes / 1_073_741_824).toFixed(1)} GiB`;
  }
  if (bytes >= 1_048_576) {
    return `${(bytes / 1_048_576).toFixed(1)} MiB`;
  }
  if (bytes >= 1_024) {
    return `${(bytes / 1_024).toFixed(1)} KiB`;
  }
  return `${bytes} B`;
}

export function formatNetworkTimestamp(value: number): string {
  return new Date(value).toLocaleString();
}

export function formatTransportState(state: WorkflowNetworkTransportState): string {
  switch (state) {
    case 'local_only':
      return 'Local only';
    case 'peer_networking_unavailable':
      return 'Peer networking unavailable';
    case 'pairing_required':
      return 'Pairing required';
    case 'connected':
      return 'Connected';
    case 'degraded':
      return 'Degraded';
  }
}

export function formatCpuUsage(value: number | null | undefined): string {
  if (value === null || value === undefined) {
    return 'Usage unavailable';
  }
  return `${value.toFixed(1)}% average`;
}

export function formatGpuAvailability(node: WorkflowLocalNetworkNodeStatus): string {
  if (node.system.gpu.available) {
    return 'Available';
  }
  return node.system.gpu.reason ?? 'Unavailable';
}

export function formatSchedulerLoad(node: WorkflowLocalNetworkNodeStatus): string {
  const load = node.scheduler_load;
  return `${load.active_run_count} active / ${load.queued_run_count} queued`;
}

export function formatSessionLoad(node: WorkflowLocalNetworkNodeStatus): string {
  const load = node.scheduler_load;
  return `${load.active_session_count}/${load.max_sessions} sessions, ${load.loaded_session_count}/${load.max_loaded_sessions} loaded`;
}

export function buildNetworkFactRows(node: WorkflowLocalNetworkNodeStatus): NetworkFactRow[] {
  const memory = node.system.memory;
  return [
    { label: 'Node ID', value: node.node_id },
    { label: 'Hostname', value: node.system.hostname ?? 'Unavailable' },
    { label: 'OS', value: formatOsLabel(node) },
    { label: 'Kernel', value: node.system.kernel_version ?? 'Unavailable' },
    { label: 'CPU', value: `${node.system.cpu.logical_core_count} logical cores` },
    {
      label: 'Memory',
      value: `${formatNetworkBytes(memory.used_bytes)} used / ${formatNetworkBytes(memory.total_bytes)} total`,
    },
    { label: 'GPU', value: formatGpuAvailability(node) },
    { label: 'Disks', value: String(node.system.disks.length) },
    { label: 'Network Interfaces', value: String(node.system.network_interfaces.length) },
  ];
}

function formatOsLabel(node: WorkflowLocalNetworkNodeStatus): string {
  const name = node.system.os_name;
  const version = node.system.os_version;
  if (name && version) {
    return `${name} ${version}`;
  }
  return name ?? 'Unavailable';
}
