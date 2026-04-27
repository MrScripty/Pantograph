import type {
  GraphEdge,
  GraphNode,
  WorkflowGraph,
  WorkflowRunGraphProjection,
} from '../../services/workflow/types';

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
}

export interface RunGraphEdgeRow {
  edgeId: string;
  source: string;
  target: string;
}

export interface RunGraphCanvasNode {
  id: string;
  nodeType: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface RunGraphCanvasEdge {
  id: string;
  sourceX: number;
  sourceY: number;
  targetX: number;
  targetY: number;
}

export interface RunGraphCanvasModel {
  viewBox: string;
  nodes: RunGraphCanvasNode[];
  edges: RunGraphCanvasEdge[];
}

const NODE_WIDTH = 190;
const NODE_HEIGHT = 64;
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

export function buildRunGraphNodeRows(runGraph: WorkflowRunGraphProjection): RunGraphNodeRow[] {
  const topologyByNodeId = new Map(
    runGraph.executable_topology.nodes.map((node) => [node.node_id, node]),
  );
  const settingsByNodeId = new Map(
    runGraph.graph_settings.nodes.map((node) => [node.node_id, node]),
  );

  return runGraph.graph.nodes.map((node) => {
    const topology = topologyByNodeId.get(node.id);
    const settings = settingsByNodeId.get(node.id);
    return {
      nodeId: node.id,
      nodeType: topology?.node_type ?? node.node_type,
      contractVersion: topology?.contract_version || 'Unknown',
      behaviorDigest: topology?.behavior_digest || 'Unknown',
      positionLabel: formatNodePosition(node),
      settingsState: settings ? 'Run settings captured' : 'Run settings unavailable',
    };
  });
}

export function buildRunGraphEdgeRows(runGraph: WorkflowRunGraphProjection): RunGraphEdgeRow[] {
  if (runGraph.graph.edges.length > 0) {
    return runGraph.graph.edges.map((edge) => ({
      edgeId: edge.id,
      source: formatGraphEdgeEndpoint(edge.source, edge.source_handle),
      target: formatGraphEdgeEndpoint(edge.target, edge.target_handle),
    }));
  }

  return runGraph.executable_topology.edges.map((edge, index) => ({
    edgeId: `topology-edge-${index + 1}`,
    source: formatGraphEdgeEndpoint(edge.source_node_id, edge.source_port_id),
    target: formatGraphEdgeEndpoint(edge.target_node_id, edge.target_port_id),
  }));
}

export function buildRunGraphCanvasModel(graph: WorkflowGraph): RunGraphCanvasModel {
  if (graph.nodes.length === 0) {
    return {
      viewBox: EMPTY_CANVAS_VIEWBOX,
      nodes: [],
      edges: [],
    };
  }

  const nodes = graph.nodes.map((node) => ({
    id: node.id,
    nodeType: node.node_type,
    x: node.position.x,
    y: node.position.y,
    width: NODE_WIDTH,
    height: NODE_HEIGHT,
  }));
  const nodeById = new Map(nodes.map((node) => [node.id, node]));
  const edges = graph.edges
    .map((edge) => buildRunGraphCanvasEdge(edge, nodeById))
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

function formatGraphEdgeEndpoint(nodeId: string, portId: string): string {
  return `${nodeId}:${portId}`;
}

function formatNodePosition(node: GraphNode): string {
  return `${Math.round(node.position.x)}, ${Math.round(node.position.y)}`;
}

function buildRunGraphCanvasEdge(
  edge: GraphEdge,
  nodeById: Map<string, RunGraphCanvasNode>,
): RunGraphCanvasEdge | null {
  const source = nodeById.get(edge.source);
  const target = nodeById.get(edge.target);
  if (!source || !target) {
    return null;
  }

  return {
    id: edge.id,
    sourceX: source.x + source.width,
    sourceY: source.y + source.height / 2,
    targetX: target.x,
    targetY: target.y + target.height / 2,
  };
}
