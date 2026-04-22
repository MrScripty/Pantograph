import type {
  ConnectionAnchor,
  ConnectionCandidatesResponse,
  ConnectionCommitResponse,
  GraphEdge,
  NodeDefinition,
  PortDataType,
} from '../services/workflow/types';
import type { ConnectionIntentState } from '@pantograph/svelte-graph';

interface WorkflowConnectionLike {
  source?: string | null;
  sourceHandle?: string | null;
  target?: string | null;
  targetHandle?: string | null;
}

export interface WorkflowConnectionAnchors {
  sourceAnchor: ConnectionAnchor;
  targetAnchor: ConnectionAnchor;
}

interface WorkflowEdgeLike {
  id: string;
  source: string;
  sourceHandle?: string | null;
  target: string;
  targetHandle?: string | null;
}

interface WorkflowNodeLike {
  id: string;
  data?: Record<string, unknown> | null;
}

type PortCompatibilityCheck = (source: PortDataType, target: PortDataType) => boolean;

export function edgeToGraphEdge(edge: WorkflowEdgeLike): GraphEdge {
  return {
    id: edge.id,
    source: edge.source,
    source_handle: edge.sourceHandle || 'output',
    target: edge.target,
    target_handle: edge.targetHandle || 'input',
  };
}

export function buildConnectionIntentState(
  candidates: ConnectionCandidatesResponse,
  rejection?: ConnectionCommitResponse['rejection'],
): ConnectionIntentState {
  return {
    sourceAnchor: candidates.source_anchor,
    graphRevision: candidates.graph_revision,
    compatibleNodeIds: candidates.compatible_nodes.map((node) => node.node_id),
    compatibleTargetKeys: candidates.compatible_nodes.flatMap((node) =>
      node.anchors.map((anchor) => `${node.node_id}:${anchor.port_id}`),
    ),
    insertableNodeTypes: candidates.insertable_node_types,
    rejection,
  };
}

export function resolveWorkflowConnectionAnchors(
  connection: WorkflowConnectionLike,
): WorkflowConnectionAnchors | null {
  if (
    !connection.source ||
    !connection.sourceHandle ||
    !connection.target ||
    !connection.targetHandle
  ) {
    return null;
  }

  return {
    sourceAnchor: {
      node_id: connection.source,
      port_id: connection.sourceHandle,
    },
    targetAnchor: {
      node_id: connection.target,
      port_id: connection.targetHandle,
    },
  };
}

export function resolveConnectionCommitGraphRevision(params: {
  sourceAnchor: ConnectionAnchor;
  currentIntent: ConnectionIntentState | null;
  currentGraphRevision: string;
}): string {
  return params.currentIntent &&
    params.currentIntent.sourceAnchor.node_id === params.sourceAnchor.node_id &&
    params.currentIntent.sourceAnchor.port_id === params.sourceAnchor.port_id
    ? params.currentIntent.graphRevision
    : params.currentGraphRevision;
}

export function isWorkflowConnectionValid(
  connection: WorkflowConnectionLike,
  graphNodes: WorkflowNodeLike[],
  connectionIntent: ConnectionIntentState | null,
  isPortCompatible: PortCompatibilityCheck,
): boolean {
  if (
    connectionIntent &&
    connection.source === connectionIntent.sourceAnchor.node_id &&
    connection.sourceHandle === connectionIntent.sourceAnchor.port_id &&
    connection.target &&
    connection.targetHandle
  ) {
    return connectionIntent.compatibleTargetKeys.includes(
      `${connection.target}:${connection.targetHandle}`,
    );
  }

  const sourceNode = graphNodes.find((node) => node.id === connection.source);
  const targetNode = graphNodes.find((node) => node.id === connection.target);
  const sourceDef = sourceNode?.data?.definition as NodeDefinition | undefined;
  const targetDef = targetNode?.data?.definition as NodeDefinition | undefined;
  const sourcePort = sourceDef?.outputs?.find((port) => port.id === connection.sourceHandle);
  const targetPort = targetDef?.inputs?.find((port) => port.id === connection.targetHandle);

  if (!sourcePort || !targetPort) return true;
  return isPortCompatible(sourcePort.data_type, targetPort.data_type);
}
