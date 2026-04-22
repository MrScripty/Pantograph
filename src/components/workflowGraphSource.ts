export interface WorkflowGraphSourceGraph<TNode, TEdge> {
  nodes: TNode[];
  edges: TEdge[];
}

export type WorkflowGraphSourceDecision<TNode, TEdge> =
  | {
      type: 'architecture';
      nodes: TNode[];
      edges: TEdge[];
    }
  | {
      type: 'architecture-pending';
    }
  | {
      type: 'workflow';
      nodes: TNode[];
      edges: TEdge[];
    };

export function resolveWorkflowGraphSource<TNode, TEdge>(params: {
  currentGraphType: string | null | undefined;
  currentGraphId: string | null | undefined;
  architectureGraph: WorkflowGraphSourceGraph<TNode, TEdge> | null | undefined;
  workflowNodes: TNode[];
  workflowEdges: TEdge[];
}): WorkflowGraphSourceDecision<TNode, TEdge> {
  if (params.currentGraphType === 'system' && params.currentGraphId === 'app-architecture') {
    if (!params.architectureGraph) {
      return { type: 'architecture-pending' };
    }

    return {
      type: 'architecture',
      nodes: params.architectureGraph.nodes,
      edges: params.architectureGraph.edges,
    };
  }

  return {
    type: 'workflow',
    nodes: params.workflowNodes,
    edges: params.workflowEdges,
  };
}
