import type { LibraryUsageProjectionRecord } from '../../services/diagnostics/types';
import type {
  WorkflowLocalNetworkNodeStatus,
  WorkflowLocalRunPlacementRecord,
  WorkflowNetworkTransportState,
} from '../../services/workflow/types';
import {
  formatLibraryAssetCategory,
  formatLibraryBytes,
} from './libraryUsagePresenters.ts';

export interface NetworkFactRow {
  label: string;
  value: string;
  mono?: boolean;
}

export type NetworkSelectedRunPlacementRow = NetworkFactRow;

export interface NetworkSelectedRunResourceRow {
  assetId: string;
  category: string;
  cacheStatus: string;
  networkBytes: string;
  accessCount: string;
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

export function formatSelectedRunLocalState(
  node: WorkflowLocalNetworkNodeStatus,
  workflowRunId: string | null | undefined,
): string {
  if (!workflowRunId) {
    return 'No selected run';
  }
  const placement = findSelectedRunPlacement(node, workflowRunId);
  if (placement?.state === 'running') {
    return 'Running locally';
  }
  if (placement?.state === 'queued') {
    return 'Queued locally';
  }
  if (node.scheduler_load.active_workflow_run_ids.includes(workflowRunId)) {
    return 'Running locally';
  }
  if (node.scheduler_load.queued_workflow_run_ids.includes(workflowRunId)) {
    return 'Queued locally';
  }
  return 'Not scheduled locally';
}

export function findSelectedRunPlacement(
  node: WorkflowLocalNetworkNodeStatus,
  workflowRunId: string | null | undefined,
): WorkflowLocalRunPlacementRecord | null {
  if (!workflowRunId) {
    return null;
  }
  return node.scheduler_load.run_placements.find((placement) => placement.workflow_run_id === workflowRunId) ?? null;
}

export function formatSelectedRunRuntimePosture(placement: WorkflowLocalRunPlacementRecord | null): string {
  if (!placement) {
    return 'Unavailable';
  }
  return placement.runtime_loaded ? 'Runtime session loaded' : 'Runtime session not loaded';
}

export function formatSelectedRunPlacementState(placement: WorkflowLocalRunPlacementRecord | null): string {
  if (!placement) {
    return 'Unavailable';
  }
  switch (placement.state) {
    case 'running':
      return 'Running locally';
    case 'queued':
      return 'Queued locally';
  }
}

export function formatSelectedRunRequirementList(values: string[], emptyLabel: string): string {
  return values.length > 0 ? values.join(', ') : emptyLabel;
}

export function buildSelectedRunPlacementRows(
  placement: WorkflowLocalRunPlacementRecord | null,
): NetworkSelectedRunPlacementRow[] {
  if (!placement) {
    return [
      { label: 'State', value: 'Not scheduled locally' },
      { label: 'Session', value: 'Unavailable', mono: true },
      { label: 'Workflow', value: 'Unavailable', mono: true },
      { label: 'Runtime', value: 'Unavailable' },
      { label: 'Backends', value: 'No backend requirements' },
      { label: 'Models', value: 'No model requirements' },
    ];
  }
  return [
    { label: 'State', value: formatSelectedRunPlacementState(placement) },
    { label: 'Session', value: placement.workflow_execution_session_id, mono: true },
    { label: 'Workflow', value: placement.workflow_id, mono: true },
    { label: 'Runtime', value: formatSelectedRunRuntimePosture(placement) },
    {
      label: 'Backends',
      value: formatSelectedRunRequirementList(placement.required_backends, 'No backend requirements'),
    },
    {
      label: 'Models',
      value: formatSelectedRunRequirementList(placement.required_models, 'No model requirements'),
    },
  ];
}

export function buildSelectedRunResourceRows(
  assets: LibraryUsageProjectionRecord[],
): NetworkSelectedRunResourceRow[] {
  return assets.map((asset) => ({
    assetId: asset.asset_id,
    category: formatLibraryAssetCategory(asset.asset_id),
    cacheStatus: formatSelectedRunResourceCacheStatus(asset.last_cache_status),
    networkBytes: formatLibraryBytes(asset.total_network_bytes),
    accessCount: String(asset.run_access_count),
  }));
}

export function formatSelectedRunResourceCacheStatus(value: string | null | undefined): string {
  if (!value || value.trim().length === 0) {
    return 'Cache status unavailable';
  }
  return value
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
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
