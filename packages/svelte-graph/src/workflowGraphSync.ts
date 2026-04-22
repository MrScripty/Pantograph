interface WorkflowGraphSyncInput<TNode, TEdge> {
  storeNodes: TNode[];
  storeEdges: TEdge[];
  prevNodesRef: TNode[] | null;
  prevEdgesRef: TEdge[] | null;
  skipNextNodeSync: boolean;
}

interface WorkflowGraphSyncDecision<TNode, TEdge> {
  applyNodes: boolean;
  applyEdges: boolean;
  nextPrevNodesRef: TNode[];
  nextPrevEdgesRef: TEdge[];
  nextSkipNextNodeSync: boolean;
}

export function computeWorkflowGraphSyncDecision<TNode, TEdge>({
  storeNodes,
  storeEdges,
  prevNodesRef,
  prevEdgesRef,
  skipNextNodeSync,
}: WorkflowGraphSyncInput<TNode, TEdge>): WorkflowGraphSyncDecision<TNode, TEdge> {
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
