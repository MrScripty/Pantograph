import type { Connection, Edge } from '@xyflow/svelte';
import {
  applyWorkflowGraphMutationResponse,
  preserveConnectionIntentState,
  type ConnectionIntentState,
} from '@pantograph/svelte-graph';
import {
  setNodeExecutionState,
  syncEdgesFromBackend,
} from '../stores/workflowStore';
import { workflowService } from '../services/workflow/WorkflowService';
import type {
  ConnectionAnchor,
  ConnectionCandidatesResponse,
  ConnectionCommitResponse,
  InsertNodeConnectionResponse,
  InsertNodeOnEdgeResponse,
  InsertNodePositionHint,
  NodeDefinition,
  WorkflowGraph,
} from '../services/workflow/types';
import type { EdgeInsertPreviewState } from './edgeInsertInteraction';
import {
  buildConnectionIntentState,
  edgeToGraphEdge,
  resolveConnectionCommitGraphRevision,
  resolveWorkflowConnectionAnchors,
} from './workflowConnections';

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

interface ConnectionCommitParams {
  requestedRevision: string;
  sourceAnchor: ConnectionAnchor;
  targetAnchor: ConnectionAnchor;
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
  if (!response.accepted || !response.graph) {
    return false;
  }

  if (!syncGraphForSession(response.graph, sessionId)) {
    return false;
  }

  applyWorkflowGraphMutationResponse(
    {
      graph: response.graph,
      workflow_event: response.workflow_event,
      workflow_session_state: response.workflow_session_state,
    },
    { setNodeExecutionState },
  );
  return true;
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
  const response = await workflowService.insertNodeAndConnect(
    sourceAnchor,
    candidateNodeType,
    graphRevision,
    positionHint,
    preferredInputPortId,
    sessionId ?? undefined,
  );
  applyAcceptedGraphMutation(response, sessionId);
  return response;
}

export async function commitWorkflowConnectionAnchors({
  requestedRevision,
  sourceAnchor,
  targetAnchor,
}: ConnectionCommitParams): Promise<ConnectionCommitResponse> {
  const sessionId = currentSessionId();
  const response = await workflowService.connectAnchors(
    sourceAnchor,
    targetAnchor,
    requestedRevision,
    sessionId ?? undefined,
  );

  if (response.accepted && response.graph) {
    applyAcceptedGraphMutation(response, sessionId);
    return response;
  }

  await refreshWorkflowEdgesFromBackend(
    '[WorkflowGraph] Failed to refresh execution graph after rejected connect:',
  );
  return response;
}

export async function loadWorkflowConnectionIntentState({
  currentIntent,
  graphRevision,
  preserveDisplay,
  rejection,
  sourceAnchor,
}: ConnectionIntentLoadParams): Promise<ConnectionIntentLoadResult> {
  try {
    const sessionId = currentSessionId();
    const candidates: ConnectionCandidatesResponse = await workflowService.getConnectionCandidates(
      sourceAnchor,
      sessionId ?? undefined,
      graphRevision,
    );
    if (!isCurrentSession(sessionId)) {
      return { type: 'clear', intent: null };
    }

    return {
      type: 'set',
      intent: buildConnectionIntentState(candidates, rejection),
    };
  } catch (error) {
    console.error('[WorkflowGraph] Failed to load connection candidates:', error);
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

export async function commitWorkflowConnection({
  connection,
  currentGraphRevision,
  currentIntent,
}: WorkflowConnectionCommitParams): Promise<WorkflowConnectionCommitResult> {
  const anchors = resolveWorkflowConnectionAnchors(connection);
  if (!anchors) {
    return { response: null };
  }

  const requestedRevision = resolveConnectionCommitGraphRevision({
    sourceAnchor: anchors.sourceAnchor,
    currentIntent,
    currentGraphRevision,
  });
  const response = await commitWorkflowConnectionAnchors({
    sourceAnchor: anchors.sourceAnchor,
    targetAnchor: anchors.targetAnchor,
    requestedRevision,
  });

  if (response.accepted) {
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

export async function removeWorkflowGraphEdges(edgeIds: string[], errorMessage: string) {
  const sessionId = currentSessionId();
  if (edgeIds.length === 0) {
    return;
  }

  try {
    const updatedGraph = await workflowService.removeEdges(edgeIds, sessionId ?? undefined);
    syncGraphForSession(updatedGraph, sessionId);
  } catch (error) {
    console.error(errorMessage, error);
  }
}

export async function removeWorkflowGraphEdge(edgeId: string, errorMessage: string) {
  await removeWorkflowGraphEdges([edgeId], errorMessage);
}

export async function commitWorkflowReconnect({
  anchors,
  fallbackRevision,
  oldEdge,
}: ReconnectCommitParams): Promise<WorkflowReconnectCommitResult> {
  try {
    const sessionId = currentSessionId();
    const graphAfterRemoval = await workflowService.removeEdge(oldEdge.id, sessionId ?? undefined);
    if (!syncGraphForSession(graphAfterRemoval, sessionId)) {
      return { type: 'stale' };
    }

    const response = await workflowService.connectAnchors(
      anchors.sourceAnchor,
      anchors.targetAnchor,
      graphAfterRemoval.derived_graph?.graph_fingerprint ?? fallbackRevision,
      sessionId ?? undefined,
    );

    if (response.accepted && response.graph) {
      return applyAcceptedGraphMutation(response, sessionId)
        ? { type: 'accepted' }
        : { type: 'stale' };
    }

    const restoredGraph = await workflowService.addEdge(
      edgeToGraphEdge(oldEdge),
      sessionId ?? undefined,
    );
    if (!syncGraphForSession(restoredGraph, sessionId)) {
      return { type: 'stale' };
    }
    return {
      type: 'rejected',
      graphRevision: response.graph_revision,
      rejection: response.rejection,
      sourceAnchor: anchors.sourceAnchor,
    };
  } catch (error) {
    return { type: 'failed', error };
  }
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
