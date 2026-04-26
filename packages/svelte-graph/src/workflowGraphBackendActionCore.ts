import type { Connection, Edge } from '@xyflow/svelte';

import { applyWorkflowGraphMutationResponse } from './stores/workflowGraphMutationResponse.ts';
import type {
  ConnectionAnchor,
  ConnectionCandidatesResponse,
  ConnectionCommitResponse,
  ConnectionIntentState,
  GraphEdge,
  InsertNodeConnectionResponse,
  InsertNodeOnEdgeResponse,
  InsertNodePositionHint,
  NodeExecutionState,
  WorkflowGraph,
  WorkflowGraphMutationResponse,
} from './types/workflow.ts';
import {
  buildConnectionIntentState,
  edgeToGraphEdge,
  preserveConnectionIntentState,
  resolveConnectionCommitGraphRevision,
  resolveWorkflowConnectionAnchors,
} from './workflowConnections.ts';

type AcceptedGraphMutationResponse =
  | ConnectionCommitResponse
  | InsertNodeConnectionResponse
  | InsertNodeOnEdgeResponse;

type GraphMutationLikeResponse =
  | WorkflowGraph
  | WorkflowGraphMutationResponse;

export interface WorkflowGraphMutationProjectionContext {
  setNodeExecutionState: (
    nodeId: string,
    state: NodeExecutionState,
    message?: string,
  ) => void;
  syncGraph: (graph: WorkflowGraph) => boolean;
}

export interface WorkflowConnectionIntentLoadCoreParams {
  currentIntent: ConnectionIntentState | null;
  failureMessage: string;
  getConnectionCandidates: () => Promise<ConnectionCandidatesResponse>;
  graphRevision: string;
  isCurrentRequest?: () => boolean;
  preserveDisplay?: boolean;
  rejection?: ConnectionCommitResponse['rejection'];
  sourceAnchor: ConnectionAnchor;
}

export interface WorkflowConnectionIntentLoadResult {
  intent: ConnectionIntentState | null;
  type: 'clear' | 'set';
}

export interface WorkflowInsertCandidateCoreParams {
  applyAcceptedMutation: (response: InsertNodeConnectionResponse) => boolean;
  candidateNodeType: string;
  graphRevision: string;
  insertNodeAndConnect: (
    sourceAnchor: ConnectionAnchor,
    candidateNodeType: string,
    graphRevision: string,
    positionHint: InsertNodePositionHint,
    preferredInputPortId?: string,
  ) => Promise<InsertNodeConnectionResponse>;
  positionHint: InsertNodePositionHint;
  preferredInputPortId?: string;
  sourceAnchor: ConnectionAnchor;
}

export interface WorkflowConnectionCommitCoreParams {
  applyAcceptedMutation: (response: ConnectionCommitResponse) => boolean;
  connectAnchors: (
    sourceAnchor: ConnectionAnchor,
    targetAnchor: ConnectionAnchor,
    graphRevision: string,
  ) => Promise<ConnectionCommitResponse>;
  connection: Connection;
  currentGraphRevision: string;
  currentIntent: ConnectionIntentState | null;
}

export interface WorkflowConnectionCommitResult {
  intent?: ConnectionIntentState;
  response: ConnectionCommitResponse | null;
}

export interface WorkflowGraphEdgeRemovalCoreParams {
  edgeIds: string[];
  errorMessage: string;
  removeEdges: (edgeIds: string[]) => Promise<GraphMutationLikeResponse>;
  syncGraph: (graph: WorkflowGraph) => boolean;
}

export interface WorkflowReconnectCoreParams {
  anchors: {
    sourceAnchor: ConnectionAnchor;
    targetAnchor: ConnectionAnchor;
  };
  applyAcceptedMutation: (response: ConnectionCommitResponse) => boolean;
  connectAnchors: (
    sourceAnchor: ConnectionAnchor,
    targetAnchor: ConnectionAnchor,
    graphRevision: string,
  ) => Promise<ConnectionCommitResponse>;
  fallbackRevision: string;
  oldEdge: Edge;
  rejectionSourceAnchor?: ConnectionAnchor;
  removeEdge: (edgeId: string) => Promise<GraphMutationLikeResponse>;
  restoreEdge: (edge: GraphEdge) => Promise<GraphMutationLikeResponse>;
  syncGraph: (graph: WorkflowGraph) => boolean;
}

interface ReconnectAcceptedResult {
  type: 'accepted';
}

interface ReconnectFailedResult {
  error: unknown;
  type: 'failed';
}

interface ReconnectStaleResult {
  type: 'stale';
}

interface ReconnectRejectedResult {
  graphRevision: string;
  rejection: ConnectionCommitResponse['rejection'];
  sourceAnchor: ConnectionAnchor;
  type: 'rejected';
}

export type WorkflowReconnectCoreResult =
  | ReconnectAcceptedResult
  | ReconnectFailedResult
  | ReconnectStaleResult
  | ReconnectRejectedResult;

function extractGraph(response: GraphMutationLikeResponse): WorkflowGraph | null {
  if ('nodes' in response && 'edges' in response) {
    return response;
  }

  return response.graph ?? null;
}

export function applyAcceptedWorkflowGraphMutation(
  response: AcceptedGraphMutationResponse,
  context: WorkflowGraphMutationProjectionContext,
): boolean {
  if (!response.accepted || !response.graph) {
    return false;
  }

  if (!context.syncGraph(response.graph)) {
    return false;
  }

  applyWorkflowGraphMutationResponse(
    {
      graph: response.graph,
      workflow_event: response.workflow_event,
      workflow_session_state: response.workflow_session_state,
    },
    {
      setNodeExecutionState: context.setNodeExecutionState,
    },
  );
  return true;
}

export async function loadWorkflowConnectionIntentStateCore({
  currentIntent,
  failureMessage,
  getConnectionCandidates,
  graphRevision,
  isCurrentRequest,
  preserveDisplay,
  rejection,
  sourceAnchor,
}: WorkflowConnectionIntentLoadCoreParams): Promise<WorkflowConnectionIntentLoadResult> {
  try {
    const candidates = await getConnectionCandidates();
    if (isCurrentRequest && !isCurrentRequest()) {
      return { type: 'clear', intent: null };
    }

    return {
      type: 'set',
      intent: buildConnectionIntentState(candidates, rejection),
    };
  } catch (error) {
    console.error(failureMessage, error);
    if (!preserveDisplay) {
      return { type: 'clear', intent: null };
    }

    return {
      type: 'set',
      intent: preserveConnectionIntentState({
        sourceAnchor,
        graphRevision,
        currentIntent,
        rejection,
      }),
    };
  }
}

export async function commitWorkflowInsertCandidateCore({
  applyAcceptedMutation,
  candidateNodeType,
  graphRevision,
  insertNodeAndConnect,
  positionHint,
  preferredInputPortId,
  sourceAnchor,
}: WorkflowInsertCandidateCoreParams): Promise<InsertNodeConnectionResponse> {
  const response = await insertNodeAndConnect(
    sourceAnchor,
    candidateNodeType,
    graphRevision,
    positionHint,
    preferredInputPortId,
  );
  applyAcceptedMutation(response);
  return response;
}

export async function commitWorkflowConnectionCore({
  applyAcceptedMutation,
  connectAnchors,
  connection,
  currentGraphRevision,
  currentIntent,
}: WorkflowConnectionCommitCoreParams): Promise<WorkflowConnectionCommitResult> {
  const anchors = resolveWorkflowConnectionAnchors(connection);
  if (!anchors) {
    return { response: null };
  }

  const requestedRevision = resolveConnectionCommitGraphRevision({
    sourceAnchor: anchors.sourceAnchor,
    currentIntent,
    currentGraphRevision,
  });
  const response = await connectAnchors(
    anchors.sourceAnchor,
    anchors.targetAnchor,
    requestedRevision,
  );

  if (response.accepted) {
    applyAcceptedMutation(response);
    return { response };
  }

  return {
    response,
    intent: preserveConnectionIntentState({
      sourceAnchor: anchors.sourceAnchor,
      graphRevision: response.graph_revision,
      currentIntent,
      rejection: response.rejection,
    }),
  };
}

export async function removeWorkflowGraphEdgesCore({
  edgeIds,
  errorMessage,
  removeEdges,
  syncGraph,
}: WorkflowGraphEdgeRemovalCoreParams): Promise<void> {
  if (edgeIds.length === 0) {
    return;
  }

  try {
    const response = await removeEdges(edgeIds);
    const graph = extractGraph(response);
    if (graph) {
      syncGraph(graph);
    }
  } catch (error) {
    console.error(errorMessage, error);
  }
}

export async function commitWorkflowReconnectCore({
  anchors,
  applyAcceptedMutation,
  connectAnchors,
  fallbackRevision,
  oldEdge,
  rejectionSourceAnchor,
  removeEdge,
  restoreEdge,
  syncGraph,
}: WorkflowReconnectCoreParams): Promise<WorkflowReconnectCoreResult> {
  try {
    const removalGraph = extractGraph(await removeEdge(oldEdge.id));
    if (!removalGraph || !syncGraph(removalGraph)) {
      return { type: 'stale' };
    }

    const response = await connectAnchors(
      anchors.sourceAnchor,
      anchors.targetAnchor,
      removalGraph.derived_graph?.graph_fingerprint ?? fallbackRevision,
    );

    if (response.accepted) {
      return applyAcceptedMutation(response)
        ? { type: 'accepted' }
        : { type: 'stale' };
    }

    const restoredGraph = extractGraph(await restoreEdge(edgeToGraphEdge(oldEdge)));
    if (!restoredGraph || !syncGraph(restoredGraph)) {
      return { type: 'stale' };
    }

    return {
      type: 'rejected',
      graphRevision: response.graph_revision,
      rejection: response.rejection,
      sourceAnchor: rejectionSourceAnchor ?? anchors.sourceAnchor,
    };
  } catch (error) {
    return { type: 'failed', error };
  }
}
