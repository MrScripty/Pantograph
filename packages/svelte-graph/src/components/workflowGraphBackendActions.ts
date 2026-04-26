import type { Connection, Edge } from '@xyflow/svelte';

import type { WorkflowStores } from '../stores/createWorkflowStores.js';
import type { WorkflowBackend } from '../types/backend.js';
import type {
  ConnectionAnchor,
  ConnectionCommitResponse,
  ConnectionIntentState,
  InsertNodeConnectionResponse,
  InsertNodePositionHint,
} from '../types/workflow.js';
import {
  preserveConnectionIntentState,
  resolveWorkflowConnectionAnchors,
} from '../workflowConnections.js';
import {
  applyAcceptedWorkflowGraphMutation,
  commitWorkflowConnectionCore,
  commitWorkflowInsertCandidateCore,
  commitWorkflowReconnectCore,
  loadWorkflowConnectionIntentStateCore,
  removeWorkflowGraphEdgesCore,
} from '../workflowGraphBackendActionCore.js';

type WorkflowGraphMutationStores = Pick<
  WorkflowStores,
  | 'setConnectionIntent'
  | 'setNodeExecutionState'
  | 'syncEdgesFromBackend'
>;

interface WorkflowGraphBackendActionContext {
  backend: WorkflowBackend;
  workflowStores: WorkflowGraphMutationStores;
}

interface ConnectionIntentLoadParams extends WorkflowGraphBackendActionContext {
  currentIntent: ConnectionIntentState | null;
  graphRevision: string;
  preserveDisplay?: boolean;
  rejection?: ConnectionCommitResponse['rejection'];
  sessionId: string;
  sourceAnchor: ConnectionAnchor;
}

interface ConnectionIntentLoadResult {
  intent: ConnectionIntentState | null;
  type: 'clear' | 'set';
}

interface InsertCandidateParams extends WorkflowGraphBackendActionContext {
  candidateNodeType: string;
  graphRevision: string;
  positionHint: InsertNodePositionHint;
  preferredInputPortId?: string;
  sessionId: string;
  sourceAnchor: ConnectionAnchor;
}

interface WorkflowConnectionCommitParams extends WorkflowGraphBackendActionContext {
  connection: Connection;
  currentGraphRevision: string;
  currentIntent: ConnectionIntentState | null;
  sessionId: string;
}

interface WorkflowConnectionCommitResult {
  intent?: ConnectionIntentState;
  response: ConnectionCommitResponse | null;
}

interface RemoveWorkflowGraphEdgesParams extends WorkflowGraphBackendActionContext {
  edgeIds: string[];
  errorMessage: string;
  sessionId: string;
}

interface ReconnectCommitParams extends WorkflowGraphBackendActionContext {
  currentIntent: ConnectionIntentState | null;
  fallbackRevision: string;
  newConnection: Connection;
  oldEdge: Edge;
  reconnectingSourceAnchor: ConnectionAnchor | null;
  sessionId: string;
}

interface ReconnectAcceptedResult {
  type: 'accepted';
}

interface ReconnectFailedResult {
  error: unknown;
  type: 'failed';
}

interface ReconnectInvalidResult {
  type: 'invalid';
}

interface ReconnectStaleResult {
  type: 'stale';
}

interface ReconnectRejectedResult {
  intent: ConnectionIntentState;
  type: 'rejected';
}

export type WorkflowReconnectCommitResult =
  | ReconnectAcceptedResult
  | ReconnectFailedResult
  | ReconnectInvalidResult
  | ReconnectStaleResult
  | ReconnectRejectedResult;

function applyAcceptedMutation(
  workflowStores: WorkflowGraphMutationStores,
  response: ConnectionCommitResponse | InsertNodeConnectionResponse,
  sessionId: string,
): boolean {
  return applyAcceptedWorkflowGraphMutation(response, {
    setNodeExecutionState: workflowStores.setNodeExecutionState,
    syncGraph: (graph) => workflowStores.syncEdgesFromBackend(graph, { sessionId }),
  });
}

export async function loadWorkflowConnectionIntentState({
  backend,
  currentIntent,
  graphRevision,
  preserveDisplay,
  rejection,
  sessionId,
  sourceAnchor,
}: ConnectionIntentLoadParams): Promise<ConnectionIntentLoadResult> {
  return loadWorkflowConnectionIntentStateCore({
    currentIntent,
    failureMessage: '[WorkflowGraph] Failed to load connection candidates:',
    getConnectionCandidates: () => backend.getConnectionCandidates(
      sourceAnchor,
      sessionId,
      graphRevision,
    ),
    graphRevision,
    preserveDisplay,
    rejection,
    sourceAnchor,
  });
}

export async function commitWorkflowInsertCandidate({
  backend,
  candidateNodeType,
  graphRevision,
  positionHint,
  preferredInputPortId,
  sessionId,
  sourceAnchor,
  workflowStores,
}: InsertCandidateParams): Promise<InsertNodeConnectionResponse> {
  return commitWorkflowInsertCandidateCore({
    applyAcceptedMutation: (response) =>
      applyAcceptedMutation(workflowStores, response, sessionId),
    candidateNodeType,
    graphRevision,
    insertNodeAndConnect: (
      insertSourceAnchor,
      insertCandidateNodeType,
      insertGraphRevision,
      insertPositionHint,
      insertPreferredInputPortId,
    ) => backend.insertNodeAndConnect(
      insertSourceAnchor,
      insertCandidateNodeType,
      sessionId,
      insertGraphRevision,
      insertPositionHint,
      insertPreferredInputPortId,
    ),
    positionHint,
    preferredInputPortId,
    sourceAnchor,
  });
}

export async function commitWorkflowConnection({
  backend,
  connection,
  currentGraphRevision,
  currentIntent,
  sessionId,
  workflowStores,
}: WorkflowConnectionCommitParams): Promise<WorkflowConnectionCommitResult> {
  return commitWorkflowConnectionCore({
    applyAcceptedMutation: (response) =>
      applyAcceptedMutation(workflowStores, response, sessionId),
    connectAnchors: (sourceAnchor, targetAnchor, graphRevision) =>
      backend.connectAnchors(sourceAnchor, targetAnchor, sessionId, graphRevision),
    connection,
    currentGraphRevision,
    currentIntent,
  });
}

export async function removeWorkflowGraphEdges({
  backend,
  edgeIds,
  errorMessage,
  sessionId,
  workflowStores,
}: RemoveWorkflowGraphEdgesParams) {
  await removeWorkflowGraphEdgesCore({
    edgeIds,
    errorMessage,
    removeEdges: (ids) => backend.removeEdges(ids, sessionId),
    syncGraph: (graph) => workflowStores.syncEdgesFromBackend(graph, { sessionId }),
  });
}

export async function commitWorkflowReconnect({
  backend,
  currentIntent,
  fallbackRevision,
  newConnection,
  oldEdge,
  reconnectingSourceAnchor,
  sessionId,
  workflowStores,
}: ReconnectCommitParams): Promise<WorkflowReconnectCommitResult> {
  const anchors = resolveWorkflowConnectionAnchors(newConnection);
  if (!anchors) {
    return { type: 'invalid' };
  }

  const result = await commitWorkflowReconnectCore({
    anchors,
    applyAcceptedMutation: (response) =>
      applyAcceptedMutation(workflowStores, response, sessionId),
    connectAnchors: (sourceAnchor, targetAnchor, graphRevision) =>
      backend.connectAnchors(sourceAnchor, targetAnchor, sessionId, graphRevision),
    fallbackRevision,
    oldEdge,
    rejectionSourceAnchor: reconnectingSourceAnchor ?? anchors.sourceAnchor,
    removeEdge: (edgeId) => backend.removeEdge(edgeId, sessionId),
    restoreEdge: (edge) => backend.addEdge(edge, sessionId),
    syncGraph: (graph) => workflowStores.syncEdgesFromBackend(graph, { sessionId }),
  });

  if (result.type === 'rejected') {
    return {
      type: 'rejected',
      intent: preserveConnectionIntentState({
        sourceAnchor: result.sourceAnchor,
        graphRevision: result.graphRevision,
        currentIntent,
        rejection: result.rejection,
      }),
    };
  }

  return result;
}
