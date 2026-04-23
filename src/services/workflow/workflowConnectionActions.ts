import { invoke } from '@tauri-apps/api/core';
import type {
  ConnectionAnchor,
  ConnectionCandidatesResponse,
  ConnectionCommitResponse,
  EdgeInsertionPreviewResponse,
  InsertNodeConnectionResponse,
  InsertNodeOnEdgeResponse,
  InsertNodePositionHint,
} from './types.ts';
import {
  normalizeConnectionCandidatesResponse,
  normalizeConnectionCommitResponse,
  normalizeEdgeInsertionPreviewResponse,
  normalizeInsertNodeConnectionResponse,
  normalizeInsertNodeOnEdgeResponse,
  serializeConnectionAnchor,
} from '../../lib/tauriConnectionIntentWire.ts';

export async function getWorkflowConnectionCandidates(
  executionId: string,
  sourceAnchor: ConnectionAnchor,
  graphRevision?: string,
): Promise<ConnectionCandidatesResponse> {
  const response = await invoke<Parameters<typeof normalizeConnectionCandidatesResponse>[0]>(
    'get_connection_candidates',
    {
      executionId,
      sourceAnchor: serializeConnectionAnchor(sourceAnchor),
      graphRevision,
    },
  );
  return normalizeConnectionCandidatesResponse(response);
}

export async function connectWorkflowAnchors(
  executionId: string,
  sourceAnchor: ConnectionAnchor,
  targetAnchor: ConnectionAnchor,
  graphRevision: string,
): Promise<ConnectionCommitResponse> {
  const response = await invoke<Parameters<typeof normalizeConnectionCommitResponse>[0]>(
    'connect_anchors_in_execution',
    {
      executionId,
      sourceAnchor: serializeConnectionAnchor(sourceAnchor),
      targetAnchor: serializeConnectionAnchor(targetAnchor),
      graphRevision,
    },
  );
  return normalizeConnectionCommitResponse(response);
}

export async function insertWorkflowNodeAndConnect(
  executionId: string,
  sourceAnchor: ConnectionAnchor,
  nodeType: string,
  graphRevision: string,
  positionHint: InsertNodePositionHint,
  preferredInputPortId?: string,
): Promise<InsertNodeConnectionResponse> {
  const response = await invoke<Parameters<typeof normalizeInsertNodeConnectionResponse>[0]>(
    'insert_node_and_connect_in_execution',
    {
      executionId,
      sourceAnchor: serializeConnectionAnchor(sourceAnchor),
      nodeType,
      graphRevision,
      positionHint,
      preferredInputPortId,
    },
  );
  return normalizeInsertNodeConnectionResponse(response);
}

export async function previewWorkflowNodeInsertOnEdge(
  executionId: string,
  edgeId: string,
  nodeType: string,
  graphRevision: string,
): Promise<EdgeInsertionPreviewResponse> {
  const response = await invoke<Parameters<typeof normalizeEdgeInsertionPreviewResponse>[0]>(
    'preview_node_insert_on_edge_in_execution',
    {
      executionId,
      edgeId,
      nodeType,
      graphRevision,
    },
  );
  return normalizeEdgeInsertionPreviewResponse(response);
}

export async function insertWorkflowNodeOnEdge(
  executionId: string,
  edgeId: string,
  nodeType: string,
  graphRevision: string,
  positionHint: InsertNodePositionHint,
): Promise<InsertNodeOnEdgeResponse> {
  const response = await invoke<Parameters<typeof normalizeInsertNodeOnEdgeResponse>[0]>(
    'insert_node_on_edge_in_execution',
    {
      executionId,
      edgeId,
      nodeType,
      graphRevision,
      positionHint,
    },
  );
  return normalizeInsertNodeOnEdgeResponse(response);
}
