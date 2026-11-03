import type {
  GraphEdge,
  GraphNode,
  WorkflowGraph,
  WorkflowGraphDiagnostic,
  WorkflowRunGraphProjection,
} from '../../services/workflow/types';
import type {
  DiagnosticErrorSeverity,
  NodeExecutionProjectionStatus,
  NodeStatusProjectionRecord,
} from '../../services/diagnostics/types';
import { glyphStateForStatus, type GlyphState } from './ui/statusGlyph.ts';
import type { KeyValueRow } from './ui/types';

export interface RunGraphCanvasNode {
  id: string;
  nodeType: string;
  x: number;
  y: number;
  width: number;
  height: number;
  inputCount: number;
  outputCount: number;
  artifactCount: number;
  artifactSummaryLabel: string;
  hasOutputArtifacts: boolean;
  statusLabel: string;
  statusClass: string;
  cacheStatusLabel: string | null;
  errorEventId: string | null;
  errorSeverity: DiagnosticErrorSeverity | null;
  errorPhase: string | null;
  errorCode: string | null;
  errorBadgeLabel: string | null;
  staleDiagnosticCount: number;
  staleSeverity: WorkflowGraphDiagnostic['severity'] | null;
  staleBadgeLabel: string | null;
  staleDiagnosticMessages: string[];
}

export interface RunGraphCanvasEdge {
  id: string;
  sourceX: number;
  sourceY: number;
  targetX: number;
  targetY: number;
  staleDiagnosticCount: number;
  staleSeverity: WorkflowGraphDiagnostic['severity'] | null;
  staleDiagnosticMessages: string[];
}

export interface RunGraphCanvasModel {
  viewBox: string;
  nodes: RunGraphCanvasNode[];
  edges: RunGraphCanvasEdge[];
}

export interface RunGraphNodeArtifactSummary {
  nodeId: string;
  inputCount: number;
  outputCount: number;
  artifactCount: number;
  payloadRefCount: number;
  latestEventSeq: number;
  mediaTypes: string[];
}

export type RunGraphNodeArtifactSummaryByNodeId = Record<string, RunGraphNodeArtifactSummary>;
export type RunGraphNodeStatusByNodeId = Record<string, NodeStatusProjectionRecord>;

interface RunGraphDiagnosticIndex {
  byNodeId: Map<string, WorkflowGraphDiagnostic[]>;
  byEdgeId: Map<string, WorkflowGraphDiagnostic[]>;
}

const NODE_WIDTH = 190;
const NODE_HEIGHT = 104;
const CANVAS_PADDING = 96;
const EMPTY_CANVAS_VIEWBOX = '0 0 640 360';

export function formatRunGraphTimestamp(value: number): string {
  return new Date(value).toLocaleString();
}

export function resolveRunGraphPresentationLabel(runGraph: WorkflowRunGraphProjection): string {
  const presentedNodeIds = new Set(runGraph.presentation_metadata.nodes.map((node) => node.node_id));
  const presentedEdgeIds = new Set(runGraph.presentation_metadata.edges.map((edge) => edge.edge_id));
  const allNodesPresented = runGraph.graph.nodes.every((node) => presentedNodeIds.has(node.id));
  const allEdgesPresented = runGraph.graph.edges.every((edge) => presentedEdgeIds.has(edge.id));

  if (allNodesPresented && allEdgesPresented) {
    return 'Presentation revision';
  }
  return 'Generated layout fallback';
}

export function buildRunGraphCanvasModel(
  graph: WorkflowGraph,
  artifactSummaries: RunGraphNodeArtifactSummaryByNodeId = {},
  nodeStatuses: RunGraphNodeStatusByNodeId = {},
  graphDiagnostics: WorkflowGraphDiagnostic[] = [],
): RunGraphCanvasModel {
  if (graph.nodes.length === 0) {
    return {
      viewBox: EMPTY_CANVAS_VIEWBOX,
      nodes: [],
      edges: [],
    };
  }

  const diagnostics = indexRunGraphDiagnostics(graphDiagnostics);
  const nodes = graph.nodes.map((node) => {
    const nodeDiagnostics = diagnostics.byNodeId.get(node.id) ?? [];
    return {
      id: node.id,
      nodeType: node.node_type,
      x: node.position.x,
      y: node.position.y,
      width: NODE_WIDTH,
      height: NODE_HEIGHT,
      inputCount: artifactSummaries[node.id]?.inputCount ?? 0,
      outputCount: artifactSummaries[node.id]?.outputCount ?? 0,
      artifactCount: artifactSummaries[node.id]?.artifactCount ?? 0,
      artifactSummaryLabel: formatRunGraphArtifactSummary(artifactSummaries[node.id]),
      hasOutputArtifacts: (artifactSummaries[node.id]?.outputCount ?? 0) > 0,
      statusLabel: formatRunGraphNodeStatusLabel(nodeStatuses[node.id]?.status),
      statusClass: runGraphNodeStatusClass(nodeStatuses[node.id]?.status),
      cacheStatusLabel: formatRunGraphNodeCacheStatusLabel(
        nodeStatuses[node.id]?.execution_cache_status,
      ),
      errorEventId:
        nodeStatuses[node.id]?.canonical_error_event_id ?? nodeStatuses[node.id]?.error_event_id ?? null,
      errorSeverity: nodeStatuses[node.id]?.error_severity ?? null,
      errorPhase: nodeStatuses[node.id]?.error_phase ?? null,
      errorCode: nodeStatuses[node.id]?.error_code ?? null,
      errorBadgeLabel: formatRunGraphNodeErrorBadge(nodeStatuses[node.id]),
      staleDiagnosticCount: nodeDiagnostics.length,
      staleSeverity: strongestGraphDiagnosticSeverity(nodeDiagnostics),
      staleBadgeLabel: formatRunGraphStaleBadge(nodeDiagnostics),
      staleDiagnosticMessages: nodeDiagnostics.map((diagnostic) => diagnostic.message),
    };
  });
  const nodeById = new Map(nodes.map((node) => [node.id, node]));
  const edges = graph.edges
    .map((edge) => buildRunGraphCanvasEdge(edge, nodeById, diagnostics.byEdgeId))
    .filter((edge): edge is RunGraphCanvasEdge => edge !== null);

  const minX = Math.min(...nodes.map((node) => node.x)) - CANVAS_PADDING;
  const minY = Math.min(...nodes.map((node) => node.y)) - CANVAS_PADDING;
  const maxX = Math.max(...nodes.map((node) => node.x + node.width)) + CANVAS_PADDING;
  const maxY = Math.max(...nodes.map((node) => node.y + node.height)) + CANVAS_PADDING;

  return {
    viewBox: `${minX} ${minY} ${maxX - minX} ${maxY - minY}`,
    nodes,
    edges,
  };
}

export function formatRunGraphNodeErrorBadge(
  status: NodeStatusProjectionRecord | null | undefined,
): string | null {
  if (
    !status?.error_severity &&
    !status?.canonical_error_event_id &&
    !status?.error_event_id &&
    !status?.error_code
  ) {
    return null;
  }
  const severity = status.error_severity ? formatRunGraphErrorSeverity(status.error_severity) : 'Error';
  const code = status.error_code?.trim() || 'node_error';
  return `${severity}: ${code}`;
}

function formatRunGraphErrorSeverity(severity: DiagnosticErrorSeverity): string {
  switch (severity) {
    case 'fatal':
      return 'Fatal';
    case 'error':
      return 'Error';
    case 'warning':
      return 'Warning';
  }
}

export function buildRunGraphNodeStatusMap(
  statuses: NodeStatusProjectionRecord[],
): RunGraphNodeStatusByNodeId {
  const byNodeId: RunGraphNodeStatusByNodeId = {};
  for (const status of statuses) {
    const previous = byNodeId[status.node_id];
    if (!previous || status.last_event_seq >= previous.last_event_seq) {
      byNodeId[status.node_id] = status;
    }
  }
  return byNodeId;
}

export function formatRunGraphNodeStatusLabel(
  status: NodeExecutionProjectionStatus | null | undefined,
): string {
  switch (status) {
    case 'queued':
      return 'Queued';
    case 'running':
      return 'Running';
    case 'waiting':
      return 'Waiting';
    case 'completed':
      return 'Completed';
    case 'failed':
      return 'Failed';
    case 'cancelled':
      return 'Cancelled';
    default:
      return 'No status';
  }
}

export function formatRunGraphNodeCacheStatusLabel(
  status: NodeStatusProjectionRecord['execution_cache_status'] | null | undefined,
): string | null {
  switch (status) {
    case 'fresh_execution':
      return 'Fresh execution';
    case 'cache_hit':
      return 'Cache hit';
    case 'cache_invalidated':
      return 'Cache invalidated';
    default:
      return null;
  }
}

function indexRunGraphDiagnostics(diagnostics: WorkflowGraphDiagnostic[]): RunGraphDiagnosticIndex {
  const byNodeId = new Map<string, WorkflowGraphDiagnostic[]>();
  const byEdgeId = new Map<string, WorkflowGraphDiagnostic[]>();

  for (const diagnostic of diagnostics) {
    if (diagnostic.scope === 'node' && diagnostic.node_id) {
      appendDiagnostic(byNodeId, diagnostic.node_id, diagnostic);
    }
    if (diagnostic.scope === 'edge') {
      const edgeId = diagnostic.details?.edge_id;
      if (edgeId) {
        appendDiagnostic(byEdgeId, edgeId, diagnostic);
      }
    }
  }

  return { byNodeId, byEdgeId };
}

function appendDiagnostic(
  target: Map<string, WorkflowGraphDiagnostic[]>,
  key: string,
  diagnostic: WorkflowGraphDiagnostic,
): void {
  const existing = target.get(key);
  if (existing) {
    existing.push(diagnostic);
  } else {
    target.set(key, [diagnostic]);
  }
}

// Used only by buildRunGraphCanvasModel; no longer exported (M9d).
function runGraphNodeStatusClass(
  status: NodeExecutionProjectionStatus | null | undefined,
): string {
  switch (status) {
    case 'running':
      return 'running';
    case 'waiting':
      return 'waiting';
    case 'completed':
      return 'completed';
    case 'failed':
      return 'failed';
    case 'cancelled':
      return 'cancelled';
    case 'queued':
      return 'queued';
    default:
      return 'unknown';
  }
}

function formatRunGraphArtifactSummary(
  summary: RunGraphNodeArtifactSummary | null | undefined,
): string {
  if (!summary || summary.artifactCount === 0) {
    return 'No retained I/O';
  }
  return `${formatCount(summary.outputCount, 'output')} / ${formatCount(summary.inputCount, 'input')}`;
}

function formatCount(count: number, noun: string): string {
  return count === 1 ? `1 ${noun}` : `${count} ${noun}s`;
}

function strongestGraphDiagnosticSeverity(
  diagnostics: WorkflowGraphDiagnostic[],
): WorkflowGraphDiagnostic['severity'] | null {
  if (diagnostics.some((diagnostic) => diagnostic.severity === 'error')) {
    return 'error';
  }
  if (diagnostics.some((diagnostic) => diagnostic.severity === 'warning')) {
    return 'warning';
  }
  if (diagnostics.some((diagnostic) => diagnostic.severity === 'info')) {
    return 'info';
  }
  return null;
}

function formatRunGraphStaleBadge(diagnostics: WorkflowGraphDiagnostic[]): string | null {
  if (diagnostics.length === 0) {
    return null;
  }
  return diagnostics.length === 1 ? '1 stale graph fact' : `${diagnostics.length} stale graph facts`;
}

function buildRunGraphCanvasEdge(
  edge: GraphEdge,
  nodeById: Map<string, RunGraphCanvasNode>,
  diagnosticsByEdgeId: Map<string, WorkflowGraphDiagnostic[]>,
): RunGraphCanvasEdge | null {
  const source = nodeById.get(edge.source);
  const target = nodeById.get(edge.target);
  if (!source || !target) {
    return null;
  }

  const diagnostics = diagnosticsByEdgeId.get(edge.id) ?? [];
  return {
    id: edge.id,
    sourceX: source.x + source.width,
    sourceY: source.y + source.height / 2,
    targetX: target.x,
    targetY: target.y + target.height / 2,
    staleDiagnosticCount: diagnostics.length,
    staleSeverity: strongestGraphDiagnosticSeverity(diagnostics),
    staleDiagnosticMessages: diagnostics.map((diagnostic) => diagnostic.message),
  };
}

// --- Graph page run mode (read-only run graph surface, ADR-014) ---

/** How a wire is drawn in the run snapshot. */
export type RunGraphWireKind = 'idle' | 'live' | 'failed' | 'stale';

export interface RunGraphSnapshotPort {
  id: string;
  label: string;
  dataType: string | null;
}

export interface RunGraphSnapshotNode {
  id: string;
  label: string;
  nodeType: string;
  x: number;
  y: number;
  width: number;
  height: number;
  glyph: GlyphState;
  stateLabel: string;
  inputs: RunGraphSnapshotPort[];
  outputs: RunGraphSnapshotPort[];
  /** Ports of the node definition that no captured edge uses. */
  hiddenPortCount: number;
  /** One short line under the ports: the error badge or the stale-fact badge. */
  note: string | null;
  noteDanger: boolean;
  errorEventId: string | null;
}

export interface RunGraphSnapshotWire {
  id: string;
  d: string;
  kind: RunGraphWireKind;
  sourceNodeId: string;
  targetNodeId: string;
}

export interface RunGraphSnapshotLayout {
  width: number;
  height: number;
  nodes: RunGraphSnapshotNode[];
  wires: RunGraphSnapshotWire[];
}

export interface RunGraphNodeGlyph {
  glyph: GlyphState;
  label: string;
}

export interface RunGraphNodePortFact {
  direction: 'in' | 'out';
  id: string;
  label: string;
  dataType: string | null;
}

export interface RunGraphNodeFacts {
  nodeId: string;
  label: string;
  nodeType: string;
  glyph: GlyphState;
  stateLabel: string;
  /** Captured run settings as read-only rows; empty when none were captured. */
  settings: KeyValueRow[];
  settingsNote: string | null;
  ports: RunGraphNodePortFact[];
  contractRows: KeyValueRow[];
  /** One sentence for the "In this run" section. */
  runSentence: string;
  runRows: KeyValueRow[];
  errorEventId: string | null;
  staleMessages: string[];
}

/** Card geometry, in canvas pixels. Port rows anchor the wires. */
export const RUN_GRAPH_CARD_WIDTH = 184;
export const RUN_GRAPH_CARD_HEADER = 32;
export const RUN_GRAPH_CARD_ROW = 22;
const RUN_GRAPH_CARD_PADDING = 6;
const RUN_GRAPH_LAYOUT_MARGIN = 48;
const RUN_GRAPH_SETTING_MAX_LENGTH = 120;

/**
 * Glyph and words for a node in a run. Without a status record the node was
 * not reached, unless the node-status projection itself was unavailable, in
 * which case nothing is claimed.
 */
export function runGraphNodeGlyph(
  status: NodeStatusProjectionRecord | null | undefined,
  statusesAvailable: boolean,
): RunGraphNodeGlyph {
  if (!status) {
    return statusesAvailable
      ? { glyph: 'queued', label: 'Not reached' }
      : { glyph: 'idle', label: 'No status' };
  }
  return {
    glyph: glyphStateForStatus(status.status),
    label: formatRunGraphNodeStatusLabel(status.status),
  };
}

/** Wire treatment from the status of the node it leaves. */
export function runGraphWireKind(
  sourceStatus: NodeExecutionProjectionStatus | null | undefined,
  staleDiagnosticCount: number,
): RunGraphWireKind {
  if (sourceStatus === 'running') {
    return 'live';
  }
  if (sourceStatus === 'failed') {
    return 'failed';
  }
  return staleDiagnosticCount > 0 ? 'stale' : 'idle';
}

/**
 * Lay out the captured run graph as cards and wires. Positions come from the
 * snapshot and are shifted so the top-left card sits at the margin; nothing
 * here reads or writes the editable graph.
 */
export function buildRunGraphSnapshotLayout(
  graph: WorkflowGraph,
  nodeStatuses: RunGraphNodeStatusByNodeId = {},
  graphDiagnostics: WorkflowGraphDiagnostic[] = [],
  statusesAvailable = true,
): RunGraphSnapshotLayout {
  if (graph.nodes.length === 0) {
    return { width: 0, height: 0, nodes: [], wires: [] };
  }

  const diagnostics = indexRunGraphDiagnostics(graphDiagnostics);
  const minX = Math.min(...graph.nodes.map((node) => node.position.x));
  const minY = Math.min(...graph.nodes.map((node) => node.position.y));
  const offsetX = RUN_GRAPH_LAYOUT_MARGIN - minX;
  const offsetY = RUN_GRAPH_LAYOUT_MARGIN - minY;

  const nodes = graph.nodes.map((node) => {
    const status = nodeStatuses[node.id];
    const { glyph, label: stateLabel } = runGraphNodeGlyph(status, statusesAvailable);
    const ports = resolveSnapshotPorts(node, graph.edges);
    const nodeDiagnostics = diagnostics.byNodeId.get(node.id) ?? [];
    const errorBadge = formatRunGraphNodeErrorBadge(status);
    const staleBadge = formatRunGraphStaleBadge(nodeDiagnostics);
    const note = errorBadge ?? staleBadge;
    const rowCount =
      ports.inputs.length + ports.outputs.length + (ports.hiddenPortCount > 0 ? 1 : 0) + (note ? 1 : 0);
    return {
      id: node.id,
      label: resolveNodeLabel(node),
      nodeType: node.node_type,
      x: Math.round(node.position.x + offsetX),
      y: Math.round(node.position.y + offsetY),
      width: RUN_GRAPH_CARD_WIDTH,
      height: RUN_GRAPH_CARD_HEADER + RUN_GRAPH_CARD_PADDING * 2 + rowCount * RUN_GRAPH_CARD_ROW,
      glyph,
      stateLabel,
      inputs: ports.inputs,
      outputs: ports.outputs,
      hiddenPortCount: ports.hiddenPortCount,
      note,
      noteDanger: errorBadge !== null,
      errorEventId: status?.canonical_error_event_id ?? status?.error_event_id ?? null,
    };
  });

  const nodeById = new Map(nodes.map((node) => [node.id, node]));
  const wires = graph.edges.flatMap((edge) => {
    const source = nodeById.get(edge.source);
    const target = nodeById.get(edge.target);
    if (!source || !target) {
      return [];
    }
    const sourceX = source.x + source.width;
    const sourceY = outputAnchorY(source, edge.source_handle);
    const targetX = target.x;
    const targetY = inputAnchorY(target, edge.target_handle);
    const bend = Math.max(40, Math.abs(targetX - sourceX) / 2);
    return [
      {
        id: edge.id,
        d: `M ${sourceX} ${sourceY} C ${sourceX + bend} ${sourceY}, ${targetX - bend} ${targetY}, ${targetX} ${targetY}`,
        kind: runGraphWireKind(
          nodeStatuses[edge.source]?.status,
          diagnostics.byEdgeId.get(edge.id)?.length ?? 0,
        ),
        sourceNodeId: edge.source,
        targetNodeId: edge.target,
      },
    ];
  });

  return {
    width: Math.max(...nodes.map((node) => node.x + node.width)) + RUN_GRAPH_LAYOUT_MARGIN,
    height: Math.max(...nodes.map((node) => node.y + node.height)) + RUN_GRAPH_LAYOUT_MARGIN,
    nodes,
    wires,
  };
}

/**
 * The node the properties panel shows when nothing valid is selected: the
 * running node, else the failed node, else the first node.
 */
export function resolveRunGraphSelectedNodeId(
  currentNodeId: string | null,
  graph: WorkflowGraph,
  nodeStatuses: RunGraphNodeStatusByNodeId = {},
): string | null {
  const nodeIds = graph.nodes.map((node) => node.id);
  if (currentNodeId && nodeIds.includes(currentNodeId)) {
    return currentNodeId;
  }
  return (
    nodeIds.find((nodeId) => nodeStatuses[nodeId]?.status === 'running') ??
    nodeIds.find((nodeId) => nodeStatuses[nodeId]?.status === 'failed') ??
    nodeIds[0] ??
    null
  );
}

/** Facts for the run-mode properties panel, all from the run snapshot and node status projection. */
export function buildRunGraphNodeFacts(
  runGraph: WorkflowRunGraphProjection,
  nodeId: string,
  nodeStatuses: RunGraphNodeStatusByNodeId = {},
  statusesAvailable = true,
): RunGraphNodeFacts | null {
  const node = runGraph.graph.nodes.find((candidate) => candidate.id === nodeId);
  if (!node) {
    return null;
  }

  const status = nodeStatuses[nodeId];
  const { glyph, label: stateLabel } = runGraphNodeGlyph(status, statusesAvailable);
  const topology = runGraph.executable_topology.nodes.find((candidate) => candidate.node_id === nodeId);
  const settingsRecord = runGraph.graph_settings.nodes.find((candidate) => candidate.node_id === nodeId);
  const settings = settingsRecord ? runGraphSettingRows(settingsRecord.data) : [];
  const ports = resolveSnapshotPorts(node, runGraph.graph.edges, true);
  const staleMessages = (runGraph.graph_diagnostics ?? [])
    .filter((diagnostic) => diagnostic.scope === 'node' && diagnostic.node_id === nodeId)
    .map((diagnostic) => diagnostic.message);

  return {
    nodeId,
    label: resolveNodeLabel(node),
    nodeType: topology?.node_type ?? node.node_type,
    glyph,
    stateLabel,
    settings,
    settingsNote: !settingsRecord
      ? 'This run did not capture settings for this node.'
      : settings.length === 0
        ? 'No plain settings were captured for this node.'
        : null,
    ports: [
      ...ports.inputs.map((port) => ({ direction: 'in' as const, ...port })),
      ...ports.outputs.map((port) => ({ direction: 'out' as const, ...port })),
    ],
    contractRows: [
      { key: 'Contract', value: topology?.contract_version || 'Unknown' },
      { key: 'Behavior', value: topology?.behavior_digest || 'Unknown', mono: true },
    ],
    runSentence: formatRunGraphNodeRunSentence(status, statusesAvailable),
    runRows: buildRunGraphNodeRunRows(status),
    errorEventId: status?.canonical_error_event_id ?? status?.error_event_id ?? null,
    staleMessages,
  };
}

export function formatRunGraphNodeRunSentence(
  status: NodeStatusProjectionRecord | null | undefined,
  statusesAvailable = true,
): string {
  if (!status) {
    return statusesAvailable
      ? 'Not reached in this run.'
      : 'Node status is unavailable for this run.';
  }
  switch (status.status) {
    case 'queued':
      return 'Queued, not started yet.';
    case 'waiting':
      return 'Waiting to start.';
    case 'running':
      return status.started_at_ms ? `Running since ${formatClockTime(status.started_at_ms)}.` : 'Running.';
    case 'completed':
      return status.duration_ms !== null && status.duration_ms !== undefined
        ? `Completed in ${formatRunGraphDuration(status.duration_ms)}.`
        : 'Completed.';
    case 'failed': {
      const reason = status.error?.trim() || formatRunGraphNodeErrorBadge(status);
      return reason ? `Failed: ${trimSentenceEnd(reason)}.` : 'Failed.';
    }
    case 'cancelled':
      return 'Cancelled.';
  }
}

export function formatRunGraphDuration(durationMs: number): string {
  if (durationMs < 1000) {
    return `${Math.max(0, Math.round(durationMs))} ms`;
  }
  const seconds = durationMs / 1000;
  if (seconds < 60) {
    return `${seconds.toFixed(1)} s`;
  }
  const minutes = Math.floor(seconds / 60);
  const rest = Math.round(seconds - minutes * 60);
  return `${minutes}m ${String(rest).padStart(2, '0')}s`;
}

/** Plain scalar settings as rows; structured values are left to I/O Inspector. */
export function runGraphSettingRows(data: unknown): KeyValueRow[] {
  if (!data || typeof data !== 'object' || Array.isArray(data)) {
    return [];
  }
  const seenKeys = new Set<string>();
  return Object.entries(data as Record<string, unknown>).flatMap(([key, value]) => {
    if (key === 'label' || key === 'definition') {
      return [];
    }
    // Rows are keyed by their visible key; keep the first of any that read the same.
    const visibleKey = humanizeSettingKey(key);
    if (seenKeys.has(visibleKey)) {
      return [];
    }
    seenKeys.add(visibleKey);
    if (typeof value === 'string') {
      if (value.trim().length === 0) {
        return [];
      }
      const text =
        value.length > RUN_GRAPH_SETTING_MAX_LENGTH
          ? `${value.slice(0, RUN_GRAPH_SETTING_MAX_LENGTH - 1)}…`
          : value;
      return [{ key: visibleKey, value: text }];
    }
    if (typeof value === 'number' || typeof value === 'boolean') {
      return [{ key: visibleKey, value: String(value), mono: typeof value === 'number' }];
    }
    return [];
  });
}

/**
 * Current-invocation authority for run graph loads: a load started for one run
 * must not apply after a newer load (another run, or a refresh) began, or
 * after the owner stopped.
 */
export interface RunGraphRequestGate {
  begin(): number;
  isCurrent(token: number): boolean;
  /** Supersede every started request, for example on unmount or leaving run mode. */
  invalidate(): void;
}

export function createRunGraphRequestGate(): RunGraphRequestGate {
  let generation = 0;
  return {
    begin() {
      generation += 1;
      return generation;
    },
    isCurrent(token) {
      return token === generation;
    },
    invalidate() {
      generation += 1;
    },
  };
}

function buildRunGraphNodeRunRows(status: NodeStatusProjectionRecord | null | undefined): KeyValueRow[] {
  if (!status) {
    return [];
  }
  const rows: KeyValueRow[] = [];
  const cache = formatRunGraphNodeCacheStatusLabel(status.execution_cache_status);
  if (cache) {
    rows.push({ key: 'Cache', value: cache });
  }
  if (status.runtime_id) {
    rows.push({
      key: 'Runtime',
      value: status.runtime_version ? `${status.runtime_id} ${status.runtime_version}` : status.runtime_id,
    });
  }
  if (status.selected_backend_key) {
    rows.push({ key: 'Backend', value: status.selected_backend_key });
  }
  if (status.model_id) {
    rows.push({ key: 'Model', value: status.model_id, mono: true });
  }
  // A running node's sentence already says when it started.
  if (status.started_at_ms && status.status !== 'running') {
    rows.push({ key: 'Started', value: formatClockTime(status.started_at_ms), mono: true });
  }
  const errorBadge = formatRunGraphNodeErrorBadge(status);
  if (errorBadge) {
    rows.push({
      key: 'Error',
      value: status.error_phase ? `${errorBadge} in ${status.error_phase}` : errorBadge,
    });
  }
  return rows;
}

function resolveNodeLabel(node: GraphNode): string {
  const data = node.data ?? {};
  const label = typeof data.label === 'string' ? data.label.trim() : '';
  if (label) {
    return label;
  }
  const definition = readDefinition(node);
  return definition?.label?.trim() || node.id;
}

interface SnapshotDefinition {
  label?: string;
  inputs: RunGraphSnapshotPort[];
  outputs: RunGraphSnapshotPort[];
}

function readDefinition(node: GraphNode): SnapshotDefinition | null {
  const definition = (node.data ?? {}).definition;
  if (!definition || typeof definition !== 'object') {
    return null;
  }
  const record = definition as Record<string, unknown>;
  return {
    label: typeof record.label === 'string' ? record.label : undefined,
    inputs: readPorts(record.inputs),
    outputs: readPorts(record.outputs),
  };
}

function readPorts(value: unknown): RunGraphSnapshotPort[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.flatMap((port) => {
    if (!port || typeof port !== 'object') {
      return [];
    }
    const record = port as Record<string, unknown>;
    if (typeof record.id !== 'string') {
      return [];
    }
    return [
      {
        id: record.id,
        label: typeof record.label === 'string' && record.label.trim() ? record.label : record.id,
        dataType: typeof record.data_type === 'string' ? record.data_type : null,
      },
    ];
  });
}

/**
 * Ports to draw on a card: the connected ones in definition order, plus any
 * port an edge names that the definition lacks. With `allPorts`, every
 * definition port is returned (properties panel).
 */
function resolveSnapshotPorts(
  node: GraphNode,
  edges: GraphEdge[],
  allPorts = false,
): { inputs: RunGraphSnapshotPort[]; outputs: RunGraphSnapshotPort[]; hiddenPortCount: number } {
  const definition = readDefinition(node);
  const connectedInputs = uniqueHandles(edges.filter((edge) => edge.target === node.id).map((edge) => edge.target_handle));
  const connectedOutputs = uniqueHandles(edges.filter((edge) => edge.source === node.id).map((edge) => edge.source_handle));
  const pick = (declared: RunGraphSnapshotPort[], connected: string[]) => {
    const declaredIds = new Set(declared.map((port) => port.id));
    const shown = allPorts ? declared : declared.filter((port) => connected.includes(port.id));
    const undeclared = connected
      .filter((id) => !declaredIds.has(id))
      .map((id) => ({ id, label: id, dataType: null }));
    return { shown: [...shown, ...undeclared], hidden: declared.length - (allPorts ? declared.length : shown.length) };
  };
  const inputs = pick(definition?.inputs ?? [], connectedInputs);
  const outputs = pick(definition?.outputs ?? [], connectedOutputs);
  return {
    inputs: inputs.shown,
    outputs: outputs.shown,
    hiddenPortCount: inputs.hidden + outputs.hidden,
  };
}

function uniqueHandles(handles: string[]): string[] {
  return [...new Set(handles)];
}

function rowCenterY(node: RunGraphSnapshotNode, rowIndex: number): number {
  return node.y + RUN_GRAPH_CARD_HEADER + RUN_GRAPH_CARD_PADDING + rowIndex * RUN_GRAPH_CARD_ROW + RUN_GRAPH_CARD_ROW / 2;
}

function inputAnchorY(node: RunGraphSnapshotNode, portId: string): number {
  const index = node.inputs.findIndex((port) => port.id === portId);
  return index < 0 ? node.y + RUN_GRAPH_CARD_HEADER / 2 : rowCenterY(node, index);
}

function outputAnchorY(node: RunGraphSnapshotNode, portId: string): number {
  const index = node.outputs.findIndex((port) => port.id === portId);
  return index < 0 ? node.y + RUN_GRAPH_CARD_HEADER / 2 : rowCenterY(node, node.inputs.length + index);
}

function humanizeSettingKey(key: string): string {
  const words = key.replaceAll('_', ' ').replaceAll('-', ' ').trim();
  return words.charAt(0).toUpperCase() + words.slice(1);
}

function trimSentenceEnd(text: string): string {
  return text.trim().replace(/[.\s]+$/, '');
}

function formatClockTime(ms: number): string {
  const date = new Date(ms);
  return [date.getHours(), date.getMinutes(), date.getSeconds()]
    .map((part) => String(part).padStart(2, '0'))
    .join(':');
}

export const RUN_GRAPH_MIN_SCALE = 0.35;
export const RUN_GRAPH_MAX_SCALE = 1.5;
const RUN_GRAPH_FIT_INSET = 24;

/** Scale that fits the whole snapshot in the viewport, never enlarging it. */
export function fitRunGraphScale(
  viewport: { width: number; height: number },
  layout: Pick<RunGraphSnapshotLayout, 'width' | 'height'>,
): number {
  if (viewport.width <= 0 || viewport.height <= 0 || layout.width <= 0 || layout.height <= 0) {
    return 1;
  }
  const fit = Math.min(
    1,
    (viewport.width - RUN_GRAPH_FIT_INSET) / layout.width,
    (viewport.height - RUN_GRAPH_FIT_INSET) / layout.height,
  );
  return clampRunGraphScale(fit);
}

export function clampRunGraphScale(scale: number): number {
  return Math.min(RUN_GRAPH_MAX_SCALE, Math.max(RUN_GRAPH_MIN_SCALE, Math.round(scale * 100) / 100));
}
