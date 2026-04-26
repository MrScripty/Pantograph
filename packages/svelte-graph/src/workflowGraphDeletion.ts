import type { Edge, Node } from '@xyflow/svelte';

export interface WorkflowDeleteSelectionRequest {
  edgeIds: string[];
  nodeIds: string[];
}

export interface WorkflowEdgeRemovalRequest {
  edgeIds: string[];
  sessionId: string;
}

export function resolveWorkflowDeleteSelectionRequest(params: {
  canEdit: boolean;
  edges: Edge[];
  nodes: Node[];
}): WorkflowDeleteSelectionRequest | null {
  if (!params.canEdit) {
    return null;
  }

  return {
    edgeIds: params.edges.map((edge) => edge.id),
    nodeIds: params.nodes.map((node) => node.id),
  };
}

export function resolveWorkflowEdgeRemovalRequest(params: {
  edgeIds: string[];
  sessionId: string | null;
}): WorkflowEdgeRemovalRequest | null {
  if (!params.sessionId || params.edgeIds.length === 0) {
    return null;
  }

  return {
    edgeIds: params.edgeIds,
    sessionId: params.sessionId,
  };
}
