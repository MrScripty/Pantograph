import type {
  LibraryUsageProjectionRecord,
  NodeStatusProjectionRecord,
  ProjectionStateRecord,
} from '../../services/diagnostics/types';
import type {
  WorkflowLocalNetworkNodeStatus,
  WorkflowLocalRunPlacementRecord,
  WorkflowSchedulerModelCacheState,
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
  cacheClass: string;
  networkBytes: string;
  accessCount: string;
}

export interface NetworkSelectedRunExecutionRow {
  nodeId: string;
  status: string;
  statusClass: string;
  runtime: string;
  model: string;
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

export function formatNetworkProjectionFreshness(state: ProjectionStateRecord | null): string {
  if (!state) {
    return 'Projection unavailable';
  }
  const cursor = `seq ${state.last_applied_event_seq}`;
  switch (state.status) {
    case 'current':
      return `Current at ${cursor}`;
    case 'rebuilding':
      return `Rebuilding at ${cursor}`;
    case 'needs_rebuild':
      return `Needs rebuild at ${cursor}`;
    case 'failed':
      return `Failed at ${cursor}`;
  }
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
      { label: 'Model Cache', value: 'Unavailable' },
      { label: 'Backends', value: 'No backend requirements' },
      { label: 'Models', value: 'No model requirements' },
    ];
  }
  return [
    { label: 'State', value: formatSelectedRunPlacementState(placement) },
    { label: 'Session', value: placement.workflow_execution_session_id, mono: true },
    { label: 'Workflow', value: placement.workflow_id, mono: true },
    { label: 'Runtime', value: formatSelectedRunRuntimePosture(placement) },
    { label: 'Model Cache', value: formatSelectedRunModelCacheState(placement.model_cache_state) },
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

export function formatSelectedRunModelCacheState(state: WorkflowSchedulerModelCacheState): string {
  switch (state) {
    case 'unknown':
      return 'Cache state unknown';
    case 'not_required':
      return 'Model not required';
    case 'cache_hit':
      return 'Model cache hit';
    case 'cache_miss':
      return 'Model cache miss';
    case 'load_requested':
      return 'Model load requested';
    case 'loaded':
      return 'Model loaded';
    case 'unload_requested':
      return 'Model unload requested';
    case 'unloaded':
      return 'Model unloaded';
    case 'failed':
      return 'Model cache failed';
  }
}

export function buildSelectedRunResourceRows(
  assets: LibraryUsageProjectionRecord[],
): NetworkSelectedRunResourceRow[] {
  return assets.map((asset) => ({
    assetId: asset.asset_id,
    category: formatLibraryAssetCategory(asset.asset_id),
    cacheStatus: formatSelectedRunResourceCacheStatus(asset.last_cache_status),
    cacheClass: selectedRunResourceCacheClass(asset.last_cache_status),
    networkBytes: formatLibraryBytes(asset.total_network_bytes),
    accessCount: String(asset.run_access_count),
  }));
}

export function buildSelectedRunExecutionRows(
  nodes: NodeStatusProjectionRecord[],
): NetworkSelectedRunExecutionRow[] {
  return nodes.map((node) => ({
    nodeId: node.node_id,
    status: formatSelectedRunNodeStatus(node.status),
    statusClass: selectedRunNodeStatusClass(node.status),
    runtime: formatSelectedRunVersionedLabel(node.runtime_id, node.runtime_version, 'Runtime unavailable'),
    model: formatSelectedRunVersionedLabel(node.model_id, node.model_version, 'Model unavailable'),
  }));
}

export function selectedRunResourceCacheClass(value: string | null | undefined): string {
  if (value === 'cache_hit' || value === 'loaded') {
    return 'border-emerald-800 bg-emerald-950/50 text-emerald-200';
  }
  if (value === 'cache_miss' || value === 'load_requested') {
    return 'border-amber-800 bg-amber-950/50 text-amber-200';
  }
  if (value === 'failed') {
    return 'border-red-800 bg-red-950/50 text-red-200';
  }
  return 'border-neutral-800 bg-neutral-950 text-neutral-400';
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

export function formatSelectedRunNodeStatus(value: NodeStatusProjectionRecord['status']): string {
  return value
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export function selectedRunNodeStatusClass(value: NodeStatusProjectionRecord['status']): string {
  switch (value) {
    case 'queued':
      return 'border-amber-800 bg-amber-950/50 text-amber-200';
    case 'completed':
      return 'border-emerald-800 bg-emerald-950/50 text-emerald-200';
    case 'running':
      return 'border-cyan-800 bg-cyan-950/50 text-cyan-200';
    case 'waiting':
      return 'border-amber-800 bg-amber-950/50 text-amber-200';
    case 'failed':
      return 'border-red-800 bg-red-950/50 text-red-200';
    case 'cancelled':
      return 'border-neutral-700 bg-neutral-900 text-neutral-300';
  }
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

function formatSelectedRunVersionedLabel(
  id: string | null | undefined,
  version: string | null | undefined,
  fallback: string,
): string {
  if (!id || id.trim().length === 0) {
    return fallback;
  }
  return version && version.trim().length > 0 ? `${id}@${version}` : id;
}

function formatOsLabel(node: WorkflowLocalNetworkNodeStatus): string {
  const name = node.system.os_name;
  const version = node.system.os_version;
  if (name && version) {
    return `${name} ${version}`;
  }
  return name ?? 'Unavailable';
}
