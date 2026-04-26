interface WorkflowGraphSyncInput<TNode, TEdge> {
  storeNodes: TNode[];
  storeEdges: TEdge[];
  prevNodesRef: TNode[] | null;
  prevEdgesRef: TEdge[] | null;
  graphSyncKey: string;
  prevGraphSyncKey: string | null;
  skipNodeSyncGraphKey: string | null;
}

interface WorkflowGraphSyncDecision<TNode, TEdge> {
  applyNodes: boolean;
  applyEdges: boolean;
  nextPrevNodesRef: TNode[];
  nextPrevEdgesRef: TEdge[];
  nextPrevGraphSyncKey: string;
  nextSkipNodeSyncGraphKey: string | null;
}

export function computeWorkflowGraphSyncDecision<TNode, TEdge>({
  storeNodes,
  storeEdges,
  prevNodesRef,
  prevEdgesRef,
  graphSyncKey,
  prevGraphSyncKey,
  skipNodeSyncGraphKey,
}: WorkflowGraphSyncInput<TNode, TEdge>): WorkflowGraphSyncDecision<TNode, TEdge> {
  const nodesChanged = storeNodes !== prevNodesRef;
  const edgesChanged = storeEdges !== prevEdgesRef;
  const graphChanged = graphSyncKey !== prevGraphSyncKey;
  const skipNodeSync = skipNodeSyncGraphKey === graphSyncKey && !graphChanged;

  return {
    applyNodes: nodesChanged && !skipNodeSync,
    applyEdges: edgesChanged,
    nextPrevNodesRef: storeNodes,
    nextPrevEdgesRef: storeEdges,
    nextPrevGraphSyncKey: graphSyncKey,
    nextSkipNodeSyncGraphKey: null,
  };
}
