import type { Connection, Edge } from '@xyflow/svelte';
import {
  applyAcceptedWorkflowGraphMutation,
  commitWorkflowConnectionCore,
  commitWorkflowInsertCandidateCore,
  commitWorkflowReconnectCore,
  loadWorkflowConnectionIntentStateCore,
  removeWorkflowGraphEdgesCore,
  type ConnectionIntentState,
} from '@pantograph/svelte-graph';
import {
  setNodeExecutionState,
  syncEdgesFromBackend,
} from '../stores/workflowStore';
import { workflowService } from '../services/workflow/WorkflowService';
import type {
  ConnectionAnchor,
  ConnectionCommitResponse,
  InsertNodeConnectionResponse,
  InsertNodeOnEdgeResponse,
  InsertNodePositionHint,
  NodeDefinition,
  WorkflowGraph,
} from '../services/workflow/types';
import type { EdgeInsertPreviewState } from './edgeInsertInteraction';

interface EdgeInsertDropParams {
  definition: NodeDefinition;
  position: { x: number; y: number };
  preview: EdgeInsertPreviewState;
}

interface InsertCandidateParams {
  candidateNodeType: string;
  graphRevision: string;
  positionHint: InsertNodePositionHint;
  preferredInputPortId?: string;
  sourceAnchor: ConnectionAnchor;
}

interface ConnectionIntentLoadParams {
  currentIntent: ConnectionIntentState | null;
  graphRevision: string;
  preserveDisplay?: boolean;
  rejection?: ConnectionCommitResponse['rejection'];
  sourceAnchor: ConnectionAnchor;
}

interface ConnectionIntentLoadResult {
  intent: ConnectionIntentState | null;
  type: 'clear' | 'set';
}

interface WorkflowConnectionCommitParams {
  connection: Connection;
  currentGraphRevision: string;
  currentIntent: ConnectionIntentState | null;
}

interface WorkflowConnectionCommitResult {
  intent?: ConnectionIntentState;
  response: ConnectionCommitResponse | null;
}

interface ReconnectCommitParams {
  anchors: {
    sourceAnchor: ConnectionAnchor;
    targetAnchor: ConnectionAnchor;
  };
  fallbackRevision: string;
  oldEdge: Edge;
}

interface ReconnectRejectedResult {
  graphRevision: string;
  rejection: ConnectionCommitResponse['rejection'];
  sourceAnchor: ConnectionAnchor;
  type: 'rejected';
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

export type WorkflowReconnectCommitResult =
  | ReconnectAcceptedResult
  | ReconnectRejectedResult
  | ReconnectFailedResult
  | ReconnectStaleResult;

function currentSessionId(): string | null {
  return workflowService.getCurrentExecutionId();
}

function isCurrentSession(sessionId: string | null): sessionId is string {
  return sessionId !== null && workflowService.getCurrentExecutionId() === sessionId;
}

function syncGraphForSession(graph: WorkflowGraph, sessionId: string | null): boolean {
  if (!isCurrentSession(sessionId)) {
    return false;
  }

  return syncEdgesFromBackend(graph, { sessionId });
}

function applyAcceptedGraphMutation(
  response: InsertNodeConnectionResponse | InsertNodeOnEdgeResponse | ConnectionCommitResponse,
  sessionId: string | null,
) {
  return applyAcceptedWorkflowGraphMutation(response, {
    setNodeExecutionState,
    syncGraph: (graph) => syncGraphForSession(graph, sessionId),
  });
}

export async function commitWorkflowEdgeInsertDrop({
  definition,
  position,
  preview,
}: EdgeInsertDropParams): Promise<boolean> {
  if (!preview.edgeId || !preview.graphRevision || !preview.bridge) {
    return false;
  }

  try {
    const sessionId = currentSessionId();
    const response = await workflowService.insertNodeOnEdge(
      preview.edgeId,
      definition.node_type,
      preview.graphRevision,
      { position },
      sessionId ?? undefined,
    );

    if (applyAcceptedGraphMutation(response, sessionId)) {
      return true;
    }

    await refreshWorkflowEdgesFromBackend(
      '[WorkflowGraph] Failed to refresh graph after rejected edge insertion:',
    );

    if (response.rejection) {
      console.warn('[WorkflowGraph] Edge insertion rejected:', response.rejection.message);
    }
  } catch (error) {
    console.error('[WorkflowGraph] Failed to insert node on edge:', error);
  }

  return false;
}

export async function commitWorkflowInsertCandidate({
  candidateNodeType,
  graphRevision,
  positionHint,
  preferredInputPortId,
  sourceAnchor,
}: InsertCandidateParams): Promise<InsertNodeConnectionResponse> {
  const sessionId = currentSessionId();
  return commitWorkflowInsertCandidateCore({
    applyAcceptedMutation: (response) =>
      applyAcceptedGraphMutation(response, sessionId),
    candidateNodeType,
    graphRevision,
    insertNodeAndConnect: (
      insertSourceAnchor,
      insertCandidateNodeType,
      insertGraphRevision,
      insertPositionHint,
      insertPreferredInputPortId,
    ) => workflowService.insertNodeAndConnect(
      insertSourceAnchor,
      insertCandidateNodeType,
      insertGraphRevision,
      insertPositionHint,
      insertPreferredInputPortId,
      sessionId ?? undefined,
    ),
    positionHint,
    preferredInputPortId,
    sourceAnchor,
  });
}

export async function loadWorkflowConnectionIntentState({
  currentIntent,
  graphRevision,
  preserveDisplay,
  rejection,
  sourceAnchor,
}: ConnectionIntentLoadParams): Promise<ConnectionIntentLoadResult> {
  const sessionId = currentSessionId();
  return loadWorkflowConnectionIntentStateCore({
    currentIntent,
    failureMessage: '[WorkflowGraph] Failed to load connection candidates:',
    getConnectionCandidates: () => workflowService.getConnectionCandidates(
      sourceAnchor,
      sessionId ?? undefined,
      graphRevision,
    ),
    graphRevision,
    isCurrentRequest: () => isCurrentSession(sessionId),
    preserveDisplay,
    rejection,
    sourceAnchor,
  });
}

export async function commitWorkflowConnection({
  connection,
  currentGraphRevision,
  currentIntent,
}: WorkflowConnectionCommitParams): Promise<WorkflowConnectionCommitResult> {
  const sessionId = currentSessionId();
  const result = await commitWorkflowConnectionCore({
    applyAcceptedMutation: (response) =>
      applyAcceptedGraphMutation(response, sessionId),
    connectAnchors: (sourceAnchor, targetAnchor, graphRevision) =>
      workflowService.connectAnchors(
        sourceAnchor,
        targetAnchor,
        graphRevision,
        sessionId ?? undefined,
      ),
    connection,
    currentGraphRevision,
    currentIntent,
  });

  if (result.response && !result.response.accepted) {
    await refreshWorkflowEdgesFromBackend(
      '[WorkflowGraph] Failed to refresh execution graph after rejected connect:',
    );
  }
  return result;
}

export async function removeWorkflowGraphEdges(edgeIds: string[], errorMessage: string) {
  const sessionId = currentSessionId();
  await removeWorkflowGraphEdgesCore({
    edgeIds,
    errorMessage,
    removeEdges: (ids) => workflowService.removeEdges(ids, sessionId ?? undefined),
    syncGraph: (graph) => syncGraphForSession(graph, sessionId),
  });
}

export async function removeWorkflowGraphEdge(edgeId: string, errorMessage: string) {
  await removeWorkflowGraphEdges([edgeId], errorMessage);
}

export async function commitWorkflowReconnect({
  anchors,
  fallbackRevision,
  oldEdge,
}: ReconnectCommitParams): Promise<WorkflowReconnectCommitResult> {
  const sessionId = currentSessionId();
  return commitWorkflowReconnectCore({
    anchors,
    applyAcceptedMutation: (response) =>
      applyAcceptedGraphMutation(response, sessionId),
    connectAnchors: (sourceAnchor, targetAnchor, graphRevision) =>
      workflowService.connectAnchors(
        sourceAnchor,
        targetAnchor,
        graphRevision,
        sessionId ?? undefined,
      ),
    fallbackRevision,
    oldEdge,
    removeEdge: (edgeId) => workflowService.removeEdge(edgeId, sessionId ?? undefined),
    restoreEdge: (edge) => workflowService.addEdge(edge, sessionId ?? undefined),
    syncGraph: (graph) => syncGraphForSession(graph, sessionId),
  });
}

export async function refreshWorkflowEdgesFromBackend(warningMessage: string) {
  try {
    const sessionId = currentSessionId();
    const backendGraph = await workflowService.getExecutionGraph(sessionId ?? undefined);
    syncGraphForSession(backendGraph, sessionId);
  } catch (error) {
    console.warn(warningMessage, error);
  }
}

export function syncWorkflowEdgesFromGraph(graph: WorkflowGraph) {
  syncGraphForSession(graph, currentSessionId());
}
