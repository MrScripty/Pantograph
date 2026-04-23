import { get } from 'svelte/store';
import type { Connection, Edge } from '@xyflow/svelte';

import { applyWorkflowGraphMutationResponse } from '../stores/workflowGraphMutationResponse.js';
import type { WorkflowStores } from '../stores/createWorkflowStores.js';
import type { WorkflowBackend } from '../types/backend.js';
import type {
  ConnectionAnchor,
  ConnectionCandidatesResponse,
  ConnectionCommitResponse,
  ConnectionIntentState,
  InsertNodeConnectionResponse,
  InsertNodePositionHint,
} from '../types/workflow.js';
import {
  buildConnectionIntentState,
  edgeToGraphEdge,
  preserveConnectionIntentState,
  resolveConnectionCommitGraphRevision,
  resolveWorkflowConnectionAnchors,
} from '../workflowConnections.js';

type WorkflowGraphMutationStores = Pick<
  WorkflowStores,
  | 'loadWorkflow'
  | 'setConnectionIntent'
  | 'setNodeExecutionState'
  | 'syncEdgesFromBackend'
  | 'workflowMetadata'
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

interface ReconnectRejectedResult {
  intent: ConnectionIntentState;
  type: 'rejected';
}

export type WorkflowReconnectCommitResult =
  | ReconnectAcceptedResult
  | ReconnectFailedResult
  | ReconnectInvalidResult
  | ReconnectRejectedResult;

function applyAcceptedInsertMutation(
  workflowStores: WorkflowGraphMutationStores,
  response: InsertNodeConnectionResponse,
): boolean {
  if (!response.accepted || !response.graph) {
    return false;
  }

  workflowStores.loadWorkflow(response.graph, get(workflowStores.workflowMetadata) ?? undefined);
  applyWorkflowGraphMutationResponse(
    {
      graph: response.graph,
      workflow_event: response.workflow_event,
      workflow_session_state: response.workflow_session_state,
    },
    {
      setNodeExecutionState: workflowStores.setNodeExecutionState,
    },
  );
  return true;
}

function applyAcceptedConnectionMutation(
  workflowStores: WorkflowGraphMutationStores,
  response: ConnectionCommitResponse,
): boolean {
  if (!response.accepted || !response.graph) {
    return false;
  }

  workflowStores.syncEdgesFromBackend(response.graph);
  applyWorkflowGraphMutationResponse(
    {
      graph: response.graph,
      workflow_event: response.workflow_event,
      workflow_session_state: response.workflow_session_state,
    },
    {
      setNodeExecutionState: workflowStores.setNodeExecutionState,
    },
  );
  return true;
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
  try {
    const candidates: ConnectionCandidatesResponse = await backend.getConnectionCandidates(
      sourceAnchor,
      sessionId,
      graphRevision,
    );
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
  const response = await backend.insertNodeAndConnect(
    sourceAnchor,
    candidateNodeType,
    sessionId,
    graphRevision,
    positionHint,
    preferredInputPortId,
  );
  applyAcceptedInsertMutation(workflowStores, response);
  return response;
}

export async function commitWorkflowConnection({
  backend,
  connection,
  currentGraphRevision,
  currentIntent,
  sessionId,
  workflowStores,
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
  const response = await backend.connectAnchors(
    anchors.sourceAnchor,
    anchors.targetAnchor,
    sessionId,
    requestedRevision,
  );

  if (applyAcceptedConnectionMutation(workflowStores, response)) {
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

export async function removeWorkflowGraphEdges({
  backend,
  edgeIds,
  errorMessage,
  sessionId,
  workflowStores,
}: RemoveWorkflowGraphEdgesParams) {
  for (const edgeId of edgeIds) {
    try {
      const response = await backend.removeEdge(edgeId, sessionId);
      if (response.graph) {
        workflowStores.syncEdgesFromBackend(response.graph);
      }
    } catch (error) {
      console.error(errorMessage, error);
    }
  }
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

  try {
    const graphAfterRemoval = await backend.removeEdge(oldEdge.id, sessionId);
    if (graphAfterRemoval.graph) {
      workflowStores.syncEdgesFromBackend(graphAfterRemoval.graph);
    }

    const response = await backend.connectAnchors(
      anchors.sourceAnchor,
      anchors.targetAnchor,
      sessionId,
      graphAfterRemoval.graph?.derived_graph?.graph_fingerprint ?? fallbackRevision,
    );

    if (applyAcceptedConnectionMutation(workflowStores, response)) {
      return { type: 'accepted' };
    }

    const restoredGraph = await backend.addEdge(edgeToGraphEdge(oldEdge), sessionId);
    if (restoredGraph.graph) {
      workflowStores.syncEdgesFromBackend(restoredGraph.graph);
    }
    return {
      type: 'rejected',
      intent: preserveConnectionIntentState({
        sourceAnchor: reconnectingSourceAnchor ?? anchors.sourceAnchor,
        graphRevision: response.graph_revision,
        currentIntent,
        rejection: response.rejection,
      }),
    };
  } catch (error) {
    return { type: 'failed', error };
  }
}
