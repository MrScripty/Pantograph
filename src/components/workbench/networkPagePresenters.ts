import type {
  LibraryUsageProjectionRecord,
  NodeStatusProjectionRecord,
  ProjectionStateRecord,
  SchedulerTimelineProjectionRecord,
} from '../../services/diagnostics/types';
import type {
  WorkflowLocalNetworkNodeStatus,
  WorkflowLocalRunPlacementRecord,
  WorkflowNetworkTransportState,
  WorkflowPeerNetworkNodeStatus,
  WorkflowSchedulerModelCacheState,
} from '../../services/workflow/types';
import {
  formatLibraryAssetCategory,
  formatLibraryBytes,
} from './libraryUsagePresenters.ts';
import { formatProjectionFreshnessState } from './projectionFreshness.ts';
import { glyphStateForStatus, type GlyphState, type LogLevel } from './ui/statusGlyph.ts';
import type { KeyValueRow } from './ui/types.ts';

/*
 * Network page projection. Everything here is derived from the local network
 * status query, the selected run's node status, library usage and scheduler
 * timeline projections. Nothing is invented: when the projection does not
 * report a fact (peer metrics, per-device memory, latency), the presenters say
 * so in a sentence instead of filling a number.
 */

// ---------------------------------------------------------------------------
// Formatting

export function formatNetworkBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) {
    return 'Unavailable';
  }
  if (bytes >= 1_099_511_627_776) {
    return `${(bytes / 1_099_511_627_776).toFixed(1)} TiB`;
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

function pad(value: number, width = 2): string {
  return String(value).padStart(width, '0');
}

/** Local wall-clock time, `HH:MM:SS`, or `HH:MM:SS.mmm` with `millis`. */
export function formatNetworkClock(valueMs: number, options: { millis?: boolean; seconds?: boolean } = {}): string {
  const date = new Date(valueMs);
  const base = `${pad(date.getHours())}:${pad(date.getMinutes())}`;
  if (options.seconds === false) {
    return base;
  }
  const withSeconds = `${base}:${pad(date.getSeconds())}`;
  return options.millis ? `${withSeconds}.${pad(date.getMilliseconds(), 3)}` : withSeconds;
}

export function formatNetworkProjectionFreshness(state: ProjectionStateRecord | null): string {
  return formatProjectionFreshnessState(state);
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

/**
 * Glyph for a node's transport state: connected → succeeded, degraded →
 * cancelled, pairing required → queued, local → running. A peer whose host has
 * no peer networking has no live link to show and renders idle.
 */
export function glyphStateForTransport(state: WorkflowNetworkTransportState): GlyphState {
  switch (state) {
    case 'connected':
      return 'succeeded';
    case 'degraded':
      return 'cancelled';
    case 'pairing_required':
      return 'queued';
    case 'local_only':
      return 'running';
    case 'peer_networking_unavailable':
      return 'idle';
  }
}

/** The local node is the one this engine runs on; it always reads as local. */
export const LOCAL_NODE_GLYPH: GlyphState = 'running';

function formatOsLabel(node: WorkflowLocalNetworkNodeStatus): string {
  const name = node.system.os_name;
  const version = node.system.os_version;
  if (name && version) {
    return `${name} ${version}`;
  }
  return name ?? 'OS not reported';
}

function plural(count: number, singular: string, pluralForm = `${singular}s`): string {
  return `${count} ${count === 1 ? singular : pluralForm}`;
}

// ---------------------------------------------------------------------------
// Left list: local node first, then peers in projection order

export type NetworkNodeKind = 'local' | 'peer';

export interface NetworkNodeEntry {
  nodeId: string;
  kind: NetworkNodeKind;
  name: string;
  glyph: GlyphState;
  transportLabel: string;
  /** First sub-line. */
  line1: string;
  /** Second, smaller sub-line. */
  line2: string;
  /** Mono text on the right of the row; empty when nothing is reported. */
  time: string;
  timeTitle?: string;
}

export interface NetworkNodeGroups {
  local: NetworkNodeEntry;
  peers: NetworkNodeEntry[];
}

export function buildLocalNodeEntry(node: WorkflowLocalNetworkNodeStatus): NetworkNodeEntry {
  const load = node.scheduler_load;
  const memory = node.system.memory;
  return {
    nodeId: node.node_id,
    kind: 'local',
    name: node.display_name,
    glyph: LOCAL_NODE_GLYPH,
    transportLabel: formatTransportState(node.transport_state),
    line1: `${load.active_run_count} running, ${load.queued_run_count} queued`,
    line2: `${plural(node.system.cpu.logical_core_count, 'core')}, ${formatNetworkBytes(memory.used_bytes)} of ${formatNetworkBytes(memory.total_bytes)} memory`,
    time: 'local',
    timeTitle: 'This node',
  };
}

export function buildPeerNodeEntry(peer: WorkflowPeerNetworkNodeStatus): NetworkNodeEntry {
  const lastSeen = peer.last_seen_at_ms ?? null;
  return {
    nodeId: peer.node_id,
    kind: 'peer',
    name: peer.display_name,
    glyph: glyphStateForTransport(peer.transport_state),
    transportLabel: formatTransportState(peer.transport_state),
    line1: formatTransportState(peer.transport_state),
    line2:
      peer.transport_state === 'pairing_required'
        ? 'Details hidden until paired'
        : 'Metrics not reported for peers',
    time: lastSeen === null ? '' : formatNetworkClock(lastSeen, { seconds: false }),
    timeTitle: lastSeen === null ? undefined : `Last seen ${formatNetworkClock(lastSeen)}`,
  };
}

export function buildNetworkNodeGroups(
  local: WorkflowLocalNetworkNodeStatus,
  peers: WorkflowPeerNetworkNodeStatus[],
): NetworkNodeGroups {
  return {
    local: buildLocalNodeEntry(local),
    peers: peers.map(buildPeerNodeEntry),
  };
}

/** The requested node when it is still listed, otherwise the local node. */
export function resolveSelectedNodeId(groups: NetworkNodeGroups, requestedId: string | null): string {
  if (requestedId && groups.peers.some((peer) => peer.nodeId === requestedId)) {
    return requestedId;
  }
  return groups.local.nodeId;
}

// ---------------------------------------------------------------------------
// Dimension line

type TransportClass = 'connected' | 'degraded' | 'pairing' | 'unreachable';

function classifyTransport(state: WorkflowNetworkTransportState, isLocal: boolean): TransportClass {
  switch (state) {
    case 'connected':
    case 'local_only':
      return 'connected';
    case 'degraded':
      return 'degraded';
    case 'pairing_required':
      return 'pairing';
    case 'peer_networking_unavailable':
      // The local node answered this query, so it is reachable even without peers.
      return isLocal ? 'connected' : 'unreachable';
  }
}

/**
 * "N nodes, c connected, d degraded, p waiting to be paired." GPU count and
 * VRAM are not in the projection, so that part of the line is omitted.
 */
export function buildNetworkDimensionLine(
  local: WorkflowLocalNetworkNodeStatus | null,
  peers: WorkflowPeerNetworkNodeStatus[],
  loading: boolean,
): string {
  if (!local) {
    return loading ? 'Reading local status' : 'Local status unavailable';
  }
  const counts: Record<TransportClass, number> = { connected: 0, degraded: 0, pairing: 0, unreachable: 0 };
  counts[classifyTransport(local.transport_state, true)] += 1;
  for (const peer of peers) {
    counts[classifyTransport(peer.transport_state, false)] += 1;
  }
  const parts = [plural(1 + peers.length, 'node'), `${counts.connected} connected`];
  if (counts.degraded > 0) parts.push(`${counts.degraded} degraded`);
  if (counts.pairing > 0) parts.push(`${counts.pairing} waiting to be paired`);
  if (counts.unreachable > 0) parts.push(`${counts.unreachable} without peer networking`);
  return `${parts.join(', ')}.`;
}

// ---------------------------------------------------------------------------
// Degraded metrics, routed to the section they affect

export type NetworkMetricSection = 'cpu' | 'memory' | 'devices' | 'disks' | 'interfaces' | 'node';

export type NetworkDegradations = Record<NetworkMetricSection, string[]>;

const DEGRADATION_ROUTES: Array<[RegExp, NetworkMetricSection]> = [
  [/\b(gpu|cuda|vram|rocm|metal|device)/i, 'devices'],
  [/\b(peer|transport|pairing)/i, 'node'],
  [/\bcpu/i, 'cpu'],
  [/\b(memory|ram|swap)\b/i, 'memory'],
  [/\b(disk|mount|storage|volume)/i, 'disks'],
  [/\b(network|interface|nic)/i, 'interfaces'],
];

/**
 * Routes the projection's free-text degradation warnings to the section whose
 * numbers they qualify. Unmatched warnings belong to the node as a whole. The
 * GPU reason is already shown in Devices, so a warning repeating it is dropped.
 */
export function groupDegradationWarnings(node: WorkflowLocalNetworkNodeStatus): NetworkDegradations {
  const groups: NetworkDegradations = { cpu: [], memory: [], devices: [], disks: [], interfaces: [], node: [] };
  const gpuReason = node.system.gpu.reason?.trim() ?? '';
  for (const raw of node.degradation_warnings) {
    const warning = raw.trim();
    if (warning.length === 0 || warning === gpuReason) continue;
    const section = DEGRADATION_ROUTES.find(([pattern]) => pattern.test(warning))?.[1] ?? 'node';
    if (!groups[section].includes(warning)) groups[section].push(warning);
  }
  return groups;
}

// ---------------------------------------------------------------------------
// Local node detail

export interface NetworkNodeHeader {
  title: string;
  nodeId: string;
  subtitle: string;
  glyph: GlyphState;
  transportLabel: string;
  chips: string[];
}

export function buildLocalNodeHeader(
  node: WorkflowLocalNetworkNodeStatus,
  peerCount: number,
): NetworkNodeHeader {
  const chips = [
    plural(node.system.cpu.logical_core_count, 'core'),
    node.system.gpu.available ? 'GPU available' : 'No GPU reported',
  ];
  if (node.transport_state !== 'local_only') {
    chips.push(plural(peerCount, 'peer'));
  }
  return {
    title: node.display_name,
    nodeId: node.node_id,
    subtitle: `${formatOsLabel(node)}, captured ${formatNetworkClock(node.captured_at_ms)}`,
    glyph: LOCAL_NODE_GLYPH,
    transportLabel: formatTransportState(node.transport_state),
    chips,
  };
}

export interface NetworkStatCell {
  id: 'cpu' | 'memory' | 'scheduler' | 'sessions';
  value: string;
  label: string;
  /** 0–100 when a meter is meaningful, null when the share is not reported. */
  meterPercent: number | null;
}

function clampPercent(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(100, Math.max(0, value));
}

export function buildLocalStatCells(node: WorkflowLocalNetworkNodeStatus): NetworkStatCell[] {
  const cpu = node.system.cpu;
  const memory = node.system.memory;
  const load = node.scheduler_load;
  const usage = cpu.average_usage_percent ?? null;
  const cores = plural(cpu.logical_core_count, 'core');
  return [
    {
      id: 'cpu',
      value: usage === null ? 'Not reported' : `${Math.round(usage)}%`,
      label: usage === null ? `${cores}, usage not reported` : `${cores}`,
      meterPercent: usage === null ? null : clampPercent(usage),
    },
    {
      id: 'memory',
      value: formatNetworkBytes(memory.used_bytes),
      label: `of ${formatNetworkBytes(memory.total_bytes)} memory in use`,
      meterPercent: memory.total_bytes > 0 ? clampPercent((memory.used_bytes / memory.total_bytes) * 100) : null,
    },
    {
      id: 'scheduler',
      value: `${load.active_run_count} / ${load.queued_run_count}`,
      label: 'running / queued here',
      meterPercent: null,
    },
    {
      id: 'sessions',
      value: `${load.loaded_session_count} of ${load.max_loaded_sessions}`,
      label: `runtime sessions loaded, ${load.active_session_count} of ${load.max_sessions} active`,
      meterPercent: null,
    },
  ];
}

/** Shown under the GPU facts until the projection reports per-device memory (WR-006). */
export const DEVICE_MEMORY_NOT_REPORTED =
  'Per-device memory is not reported by this engine build.';

export function buildLocalDeviceRows(node: WorkflowLocalNetworkNodeStatus): KeyValueRow[] {
  const gpu = node.system.gpu;
  const rows: KeyValueRow[] = [{ key: 'GPU', value: gpu.available ? 'Available' : 'Not available' }];
  const reason = gpu.reason?.trim();
  if (reason) {
    rows.push({ key: 'Reason', value: reason });
  }
  return rows;
}

export interface NetworkDiskRow {
  key: string;
  mount: string;
  name: string;
  free: string;
  total: string;
}

export function buildLocalDiskRows(node: WorkflowLocalNetworkNodeStatus): NetworkDiskRow[] {
  return node.system.disks.map((disk) => ({
    key: `${disk.name}\u{1f}${disk.mount_point}`,
    mount: disk.mount_point,
    name: disk.name,
    free: formatNetworkBytes(disk.available_bytes),
    total: formatNetworkBytes(disk.total_bytes),
  }));
}

export interface NetworkInterfaceRow {
  name: string;
  received: string;
  sent: string;
}

export function buildLocalInterfaceRows(node: WorkflowLocalNetworkNodeStatus): NetworkInterfaceRow[] {
  return node.system.network_interfaces.map((networkInterface) => ({
    name: networkInterface.name,
    received: formatNetworkBytes(networkInterface.total_received_bytes),
    sent: formatNetworkBytes(networkInterface.total_transmitted_bytes),
  }));
}

export function buildLocalIdentityRows(node: WorkflowLocalNetworkNodeStatus): KeyValueRow[] {
  return [
    { key: 'Node id', value: node.node_id, mono: true },
    { key: 'Hostname', value: node.system.hostname ?? 'Not reported', mono: Boolean(node.system.hostname) },
    { key: 'OS', value: formatOsLabel(node) },
    { key: 'Kernel', value: node.system.kernel_version ?? 'Not reported' },
  ];
}

// ---------------------------------------------------------------------------
// Peer detail

export interface NetworkPeerDetail {
  header: NetworkNodeHeader;
  /** Why the metric sections are absent for this peer. */
  note: string;
  identityRows: KeyValueRow[];
}

export function buildPeerDetail(peer: WorkflowPeerNetworkNodeStatus): NetworkPeerDetail {
  const lastSeen = peer.last_seen_at_ms ?? null;
  const lastSeenText = lastSeen === null ? 'Not seen yet' : formatNetworkClock(lastSeen);
  const pairing = peer.transport_state === 'pairing_required';
  return {
    header: {
      title: peer.display_name,
      nodeId: peer.node_id,
      subtitle: lastSeen === null ? 'Not seen yet' : `Last seen ${lastSeenText}`,
      glyph: glyphStateForTransport(peer.transport_state),
      transportLabel: formatTransportState(peer.transport_state),
      chips: [],
    },
    note: pairing
      ? 'This node was found on the network but is not trusted yet. Its details stay hidden until it is paired, and pairing is not available in this build.'
      : 'This engine build reports only the transport state and last-seen time of peers. CPU, memory, devices, disks and interfaces appear here once peer metrics are reported.',
    identityRows: [
      { key: 'Node id', value: peer.node_id, mono: true },
      { key: 'Transport', value: formatTransportState(peer.transport_state) },
      { key: 'Last seen', value: lastSeenText, mono: lastSeen !== null },
    ],
  };
}

// ---------------------------------------------------------------------------
// Selected run

export interface NetworkSelectedRunRef {
  workflow_run_id: string;
  workflow_id?: string | null;
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
  return 'Not running or queued here';
}

export function formatSelectedRunRuntimePosture(placement: WorkflowLocalRunPlacementRecord | null): string {
  if (!placement) {
    return 'Unavailable';
  }
  return placement.runtime_loaded ? 'Runtime session loaded' : 'Runtime session not loaded';
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

export function formatSelectedRunRequirementList(values: string[], emptyLabel: string): string {
  return values.length > 0 ? values.join(', ') : emptyLabel;
}

export function selectedRunWorkflowLabel(
  run: NetworkSelectedRunRef,
  placement: WorkflowLocalRunPlacementRecord | null,
): string {
  const workflowId = run.workflow_id?.trim() || placement?.workflow_id?.trim();
  return workflowId && workflowId.length > 0 ? workflowId : 'Selected run';
}

export interface NetworkSelectedRunOnNode {
  /** "<workflow> on this node". */
  heading: string;
  runId: string;
  state: string;
  placed: boolean;
  /** Placement facts after the run id and state; empty when not placed here. */
  rows: KeyValueRow[];
  /** Sentence shown when the run is not running or queued on this node. */
  note: string | null;
}

/**
 * The selected run's placement on the local node. The local status lists
 * only running and queued runs, so a finished run is reported as "not running
 * or queued here" rather than guessed.
 */
export function buildSelectedRunOnLocalNode(
  node: WorkflowLocalNetworkNodeStatus,
  run: NetworkSelectedRunRef,
): NetworkSelectedRunOnNode {
  const placement = findSelectedRunPlacement(node, run.workflow_run_id);
  const workflow = selectedRunWorkflowLabel(run, placement);
  const state = formatSelectedRunLocalState(node, run.workflow_run_id);
  if (!placement) {
    return {
      heading: `${workflow} on this node`,
      runId: run.workflow_run_id,
      state,
      placed: false,
      rows: [],
      note:
        state === 'Not running or queued here'
          ? 'The local status lists only runs that are running or queued, so this run has no placement here right now. Its execution and resources below come from the run projections.'
          : 'The scheduler lists this run here, but its placement record is not in the local status yet.',
    };
  }
  return {
    heading: `${workflow} on this node`,
    runId: run.workflow_run_id,
    state,
    placed: true,
    rows: [
      { key: 'Runtime session', value: `${formatSelectedRunRuntimePosture(placement)}, ${formatSelectedRunModelCacheState(placement.model_cache_state).toLowerCase()}` },
      { key: 'Session id', value: placement.workflow_execution_session_id, mono: true },
      { key: 'Backends', value: formatSelectedRunRequirementList(placement.required_backends, 'No backend requirements') },
      { key: 'Models', value: formatSelectedRunRequirementList(placement.required_models, 'No model requirements') },
    ],
    note: null,
  };
}

export interface NetworkSelectedRunOnPeer {
  heading: string;
  sentence: string;
}

/** The selected run while a peer is open: placement on peers is not reported. */
export function describeSelectedRunForPeer(
  local: WorkflowLocalNetworkNodeStatus,
  run: NetworkSelectedRunRef,
  peerName: string,
): NetworkSelectedRunOnPeer {
  const placement = findSelectedRunPlacement(local, run.workflow_run_id);
  const workflow = selectedRunWorkflowLabel(run, placement);
  if (placement) {
    const verb = placement.state === 'running' ? 'running' : 'queued';
    return { heading: workflow, sentence: `${workflow} is ${verb} on ${local.display_name}, not on ${peerName}.` };
  }
  return {
    heading: workflow,
    sentence: `This engine build does not report run placement on peers, so it cannot say whether ${workflow} used ${peerName}.`,
  };
}

export interface NetworkSelectedRunExecutionRow {
  nodeId: string;
  status: string;
  glyph: GlyphState;
  runtime: string;
  model: string;
}

export function formatSelectedRunNodeStatus(value: NodeStatusProjectionRecord['status']): string {
  const text = value.replaceAll('_', ' ');
  return text.charAt(0).toUpperCase() + text.slice(1);
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

export function buildSelectedRunExecutionRows(
  nodes: NodeStatusProjectionRecord[],
): NetworkSelectedRunExecutionRow[] {
  return nodes.map((node) => ({
    nodeId: node.node_id,
    status: formatSelectedRunNodeStatus(node.status),
    glyph: glyphStateForStatus(node.status),
    runtime: formatSelectedRunVersionedLabel(node.runtime_id, node.runtime_version, 'Runtime unavailable'),
    model: formatSelectedRunVersionedLabel(node.model_id, node.model_version, 'Model unavailable'),
  }));
}

export interface NetworkSelectedRunResourceRow {
  assetId: string;
  category: string;
  cacheStatus: string;
  networkBytes: string;
  accessCount: string;
}

export function formatSelectedRunResourceCacheStatus(value: string | null | undefined): string {
  if (!value || value.trim().length === 0) {
    return 'Not reported';
  }
  const text = value.trim().replaceAll('_', ' ');
  return text.charAt(0).toUpperCase() + text.slice(1);
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

export interface NetworkEventLine {
  id: string;
  time: string;
  level: LogLevel;
  kind: string;
  message: string;
  source: string;
}

export function logLevelForTimelineEvent(
  severity: SchedulerTimelineProjectionRecord['error_severity'],
): LogLevel {
  switch (severity) {
    case 'warning':
      return 'warn';
    case 'error':
    case 'fatal':
      return 'error';
    default:
      return 'info';
  }
}

export function buildSelectedRunEventLines(events: SchedulerTimelineProjectionRecord[]): NetworkEventLine[] {
  return events.map((event) => ({
    id: event.event_id,
    time: formatNetworkClock(event.occurred_at_ms, { millis: true }),
    level: logLevelForTimelineEvent(event.error_severity),
    kind: event.event_kind,
    message: event.detail ? `${event.summary}. ${event.detail}` : event.summary,
    source: event.source_component,
  }));
}

// ---------------------------------------------------------------------------
// Query failures, as sentences for the panel they affect

function asSentence(text: string): string {
  const trimmed = text.trim();
  return /[.!?]$/.test(trimmed) ? trimmed : `${trimmed}.`;
}

export function describeLocalStatusFailure(
  errorText: string,
  lastCapturedAtMs: number | null,
): string {
  if (lastCapturedAtMs === null) {
    return `Pantograph could not read the local status. ${asSentence(errorText)}`;
  }
  return `The last refresh failed, so this is the status captured at ${formatNetworkClock(lastCapturedAtMs)}. ${asSentence(errorText)}`;
}

/** Sentence for a failed selected-run projection query, shown in its section. */
export function describeRunProjectionFailure(what: string, errorText: string): string {
  return `Could not read ${what}. ${asSentence(errorText)}`;
}
