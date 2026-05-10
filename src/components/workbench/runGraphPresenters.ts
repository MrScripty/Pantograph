import type {
  GraphEdge,
  GraphNode,
  WorkflowGraph,
  WorkflowGraphDiagnostic,
  WorkflowRunGraphProjection,
} from '../../services/workflow/types';
import type {
  DiagnosticErrorSeverity,
  IoArtifactProjectionRecord,
  NodeExecutionProjectionStatus,
  NodeStatusProjectionRecord,
} from '../../services/diagnostics/types';

export interface RunGraphCounts {
  nodeCount: number;
  edgeCount: number;
}

export interface RunGraphNodeRow {
  nodeId: string;
  nodeType: string;
  contractVersion: string;
  behaviorDigest: string;
  positionLabel: string;
  settingsState: string;
  artifactSummaryLabel: string;
  artifactDetailLabel: string;
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

export interface RunGraphEdgeRow {
  edgeId: string;
  source: string;
  target: string;
  staleDiagnosticCount: number;
  staleSeverity: WorkflowGraphDiagnostic['severity'] | null;
  staleDiagnosticMessages: string[];
}

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

export function resolveRunGraphCounts(graph: WorkflowGraph | null | undefined): RunGraphCounts {
  return {
    nodeCount: graph?.nodes.length ?? 0,
    edgeCount: graph?.edges.length ?? 0,
  };
}

export function formatRunGraphCountLabel(counts: RunGraphCounts): string {
  return `${counts.nodeCount} nodes / ${counts.edgeCount} edges`;
}

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

export function buildRunGraphNodeRows(
  runGraph: WorkflowRunGraphProjection,
  artifactSummaries: RunGraphNodeArtifactSummaryByNodeId = {},
  nodeStatuses: RunGraphNodeStatusByNodeId = {},
): RunGraphNodeRow[] {
  const diagnostics = indexRunGraphDiagnostics(runGraph.graph_diagnostics ?? []);
  const topologyByNodeId = new Map(
    runGraph.executable_topology.nodes.map((node) => [node.node_id, node]),
  );
  const settingsByNodeId = new Map(
    runGraph.graph_settings.nodes.map((node) => [node.node_id, node]),
  );

  return runGraph.graph.nodes.map((node) => {
    const topology = topologyByNodeId.get(node.id);
    const settings = settingsByNodeId.get(node.id);
    const artifactSummary = artifactSummaries[node.id];
    const nodeStatus = nodeStatuses[node.id];
    const nodeDiagnostics = diagnostics.byNodeId.get(node.id) ?? [];
    return {
      nodeId: node.id,
      nodeType: topology?.node_type ?? node.node_type,
      contractVersion: topology?.contract_version || 'Unknown',
      behaviorDigest: topology?.behavior_digest || 'Unknown',
      positionLabel: formatNodePosition(node),
      settingsState: settings ? 'Run settings captured' : 'Run settings unavailable',
      artifactSummaryLabel: formatRunGraphArtifactSummary(artifactSummary),
      artifactDetailLabel: formatRunGraphArtifactDetail(artifactSummary),
      hasOutputArtifacts: (artifactSummary?.outputCount ?? 0) > 0,
      statusLabel: formatRunGraphNodeStatusLabel(nodeStatus?.status),
      statusClass: runGraphNodeStatusClass(nodeStatus?.status),
      cacheStatusLabel: formatRunGraphNodeCacheStatusLabel(nodeStatus?.execution_cache_status),
      errorEventId: nodeStatus?.canonical_error_event_id ?? nodeStatus?.error_event_id ?? null,
      errorSeverity: nodeStatus?.error_severity ?? null,
      errorPhase: nodeStatus?.error_phase ?? null,
      errorCode: nodeStatus?.error_code ?? null,
      errorBadgeLabel: formatRunGraphNodeErrorBadge(nodeStatus),
      staleDiagnosticCount: nodeDiagnostics.length,
      staleSeverity: strongestGraphDiagnosticSeverity(nodeDiagnostics),
      staleBadgeLabel: formatRunGraphStaleBadge(nodeDiagnostics),
      staleDiagnosticMessages: nodeDiagnostics.map((diagnostic) => diagnostic.message),
    };
  });
}

export function buildRunGraphEdgeRows(runGraph: WorkflowRunGraphProjection): RunGraphEdgeRow[] {
  const diagnostics = indexRunGraphDiagnostics(runGraph.graph_diagnostics ?? []);
  if (runGraph.graph.edges.length > 0) {
    return runGraph.graph.edges.map((edge) => {
      const edgeDiagnostics = diagnostics.byEdgeId.get(edge.id) ?? [];
      return {
        edgeId: edge.id,
        source: formatGraphEdgeEndpoint(edge.source, edge.source_handle),
        target: formatGraphEdgeEndpoint(edge.target, edge.target_handle),
        staleDiagnosticCount: edgeDiagnostics.length,
        staleSeverity: strongestGraphDiagnosticSeverity(edgeDiagnostics),
        staleDiagnosticMessages: edgeDiagnostics.map((diagnostic) => diagnostic.message),
      };
    });
  }

  return runGraph.executable_topology.edges.map((edge, index) => ({
    edgeId: `topology-edge-${index + 1}`,
    source: formatGraphEdgeEndpoint(edge.source_node_id, edge.source_port_id),
    target: formatGraphEdgeEndpoint(edge.target_node_id, edge.target_port_id),
    staleDiagnosticCount: 0,
    staleSeverity: null,
    staleDiagnosticMessages: [],
  }));
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

export function runGraphNodeErrorClass(
  severity: DiagnosticErrorSeverity | null | undefined,
): string {
  switch (severity) {
    case 'fatal':
      return 'fatal';
    case 'error':
      return 'error';
    case 'warning':
      return 'warning';
    case null:
    case undefined:
      return 'none';
  }
}

export function runGraphStaleDiagnosticClass(
  severity: WorkflowGraphDiagnostic['severity'] | null | undefined,
): string {
  switch (severity) {
    case 'error':
      return 'error';
    case 'warning':
      return 'warning';
    case 'info':
      return 'info';
    case null:
    case undefined:
      return 'none';
  }
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

export function runGraphNodeStatusClass(
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

export function buildRunGraphNodeArtifactSummaries(
  artifacts: Pick<
    IoArtifactProjectionRecord,
    'node_id' | 'artifact_role' | 'event_seq' | 'payload_ref' | 'media_type'
  >[],
): RunGraphNodeArtifactSummaryByNodeId {
  const summaries: RunGraphNodeArtifactSummaryByNodeId = {};
  const mediaTypesByNodeId = new Map<string, Set<string>>();

  for (const artifact of artifacts) {
    if (!artifact.node_id) {
      continue;
    }

    const summary = summaries[artifact.node_id] ?? {
      nodeId: artifact.node_id,
      inputCount: 0,
      outputCount: 0,
      artifactCount: 0,
      payloadRefCount: 0,
      latestEventSeq: artifact.event_seq,
      mediaTypes: [],
    };

    summary.artifactCount += 1;
    summary.latestEventSeq = Math.max(summary.latestEventSeq, artifact.event_seq);
    if (artifact.artifact_role === 'node_input') {
      summary.inputCount += 1;
    }
    if (artifact.artifact_role === 'node_output') {
      summary.outputCount += 1;
    }
    if (artifact.payload_ref?.trim()) {
      summary.payloadRefCount += 1;
    }
    if (artifact.media_type?.trim()) {
      const mediaTypes = mediaTypesByNodeId.get(artifact.node_id) ?? new Set<string>();
      mediaTypes.add(artifact.media_type);
      mediaTypesByNodeId.set(artifact.node_id, mediaTypes);
    }

    summaries[artifact.node_id] = summary;
  }

  for (const [nodeId, mediaTypes] of mediaTypesByNodeId.entries()) {
    summaries[nodeId].mediaTypes = [...mediaTypes].sort();
  }

  return summaries;
}

export function formatRunGraphArtifactSummary(
  summary: RunGraphNodeArtifactSummary | null | undefined,
): string {
  if (!summary || summary.artifactCount === 0) {
    return 'No retained I/O';
  }
  return `${formatCount(summary.outputCount, 'output')} / ${formatCount(summary.inputCount, 'input')}`;
}

export function formatRunGraphArtifactDetail(
  summary: RunGraphNodeArtifactSummary | null | undefined,
): string {
  if (!summary || summary.artifactCount === 0) {
    return 'No retained artifact metadata for this node';
  }

  const payloadLabel =
    summary.payloadRefCount === 1
      ? '1 payload reference'
      : `${summary.payloadRefCount} payload references`;
  const mediaLabel =
    summary.mediaTypes.length === 0 ? 'media unknown' : summary.mediaTypes.join(', ');
  return `${formatCount(summary.artifactCount, 'artifact')}, ${payloadLabel}, ${mediaLabel}`;
}

function formatGraphEdgeEndpoint(nodeId: string, portId: string): string {
  return `${nodeId}:${portId}`;
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

function formatCount(count: number, noun: string): string {
  return count === 1 ? `1 ${noun}` : `${count} ${noun}s`;
}

function formatNodePosition(node: GraphNode): string {
  return `${Math.round(node.position.x)}, ${Math.round(node.position.y)}`;
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
