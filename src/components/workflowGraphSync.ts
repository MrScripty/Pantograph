export interface WorkflowGraphSyncDecision<TNode, TEdge> {
  applyNodes: boolean;
  applyEdges: boolean;
  nextPrevNodesRef: TNode[];
  nextPrevEdgesRef: TEdge[];
  nextSkipNextNodeSync: boolean;
}

export function computeWorkflowGraphSyncDecision<TNode, TEdge>(params: {
  storeNodes: TNode[];
  storeEdges: TEdge[];
  prevNodesRef: TNode[] | null;
  prevEdgesRef: TEdge[] | null;
  skipNextNodeSync: boolean;
}): WorkflowGraphSyncDecision<TNode, TEdge> {
  const { storeNodes, storeEdges, prevNodesRef, prevEdgesRef, skipNextNodeSync } = params;

  const nodesChanged = storeNodes !== prevNodesRef;
  const edgesChanged = storeEdges !== prevEdgesRef;

  return {
    applyNodes: nodesChanged && !skipNextNodeSync,
    applyEdges: edgesChanged,
    nextPrevNodesRef: storeNodes,
    nextPrevEdgesRef: storeEdges,
    nextSkipNextNodeSync: false,
  };
}
