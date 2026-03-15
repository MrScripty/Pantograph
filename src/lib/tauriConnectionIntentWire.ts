import type {
  ConnectionAnchor as AppConnectionAnchor,
  ConnectionCandidatesResponse as AppConnectionCandidatesResponse,
  ConnectionCommitResponse as AppConnectionCommitResponse,
  ConnectionRejection as AppConnectionRejection,
  EdgeInsertionPreviewResponse as AppEdgeInsertionPreviewResponse,
  InsertNodeConnectionResponse as AppInsertNodeConnectionResponse,
  InsertNodeOnEdgeResponse as AppInsertNodeOnEdgeResponse,
} from '../services/workflow/types';
import type {
  ConnectionAnchor as PackageConnectionAnchor,
  ConnectionCandidatesResponse as PackageConnectionCandidatesResponse,
  ConnectionCommitResponse as PackageConnectionCommitResponse,
  ConnectionRejection as PackageConnectionRejection,
  EdgeInsertionPreviewResponse as PackageEdgeInsertionPreviewResponse,
  InsertNodeConnectionResponse as PackageInsertNodeConnectionResponse,
  InsertNodeOnEdgeResponse as PackageInsertNodeOnEdgeResponse,
} from '@pantograph/svelte-graph';

type AnyConnectionAnchor = AppConnectionAnchor | PackageConnectionAnchor;
type AnyCandidatesResponse =
  | AppConnectionCandidatesResponse
  | PackageConnectionCandidatesResponse;
type AnyCommitResponse = AppConnectionCommitResponse | PackageConnectionCommitResponse;
type AnyEdgeInsertionPreviewResponse =
  | AppEdgeInsertionPreviewResponse
  | PackageEdgeInsertionPreviewResponse;
type AnyInsertResponse = AppInsertNodeConnectionResponse | PackageInsertNodeConnectionResponse;
type AnyInsertNodeOnEdgeResponse =
  | AppInsertNodeOnEdgeResponse
  | PackageInsertNodeOnEdgeResponse;
type AnyConnectionRejection = AppConnectionRejection | PackageConnectionRejection;

interface WireEdgeInsertionBridge {
  inputPortId?: string;
  input_port_id?: string;
  outputPortId?: string;
  output_port_id?: string;
}

interface WireConnectionAnchor {
  nodeId: string;
  portId: string;
}

interface WireConnectionTargetAnchorCandidate {
  portId?: string;
  port_id?: string;
  portLabel?: string;
  port_label?: string;
  dataType?: string;
  data_type?: string;
  multiple: boolean;
}

interface WireConnectionTargetNodeCandidate {
  nodeId?: string;
  node_id?: string;
  nodeType?: string;
  node_type?: string;
  nodeLabel?: string;
  node_label?: string;
  position: { x: number; y: number };
  anchors: WireConnectionTargetAnchorCandidate[];
}

interface WireInsertableNodeTypeCandidate {
  nodeType?: string;
  node_type?: string;
  category: string;
  label: string;
  description: string;
  matchingInputPortIds?: string[];
  matching_input_port_ids?: string[];
}

interface WireConnectionRejection {
  reason: string;
  message: string;
}

interface WireConnectionCandidatesResponse {
  graphRevision?: string;
  graph_revision?: string;
  revisionMatches?: boolean;
  revision_matches?: boolean;
  sourceAnchor?: WireConnectionAnchor;
  source_anchor?: AnyConnectionAnchor;
  compatibleNodes?: WireConnectionTargetNodeCandidate[];
  compatible_nodes?: PackageConnectionCandidatesResponse['compatible_nodes'];
  insertableNodeTypes?: WireInsertableNodeTypeCandidate[];
  insertable_node_types?: PackageConnectionCandidatesResponse['insertable_node_types'];
}

interface WireConnectionCommitResponse {
  accepted: boolean;
  graphRevision?: string;
  graph_revision?: string;
  graph?: AnyCommitResponse['graph'];
  rejection?: WireConnectionRejection;
}

interface WireInsertNodeConnectionResponse {
  accepted: boolean;
  graphRevision?: string;
  graph_revision?: string;
  insertedNodeId?: string;
  inserted_node_id?: string;
  graph?: AnyInsertResponse['graph'];
  rejection?: WireConnectionRejection;
}

interface WireEdgeInsertionPreviewResponse {
  accepted: boolean;
  graphRevision?: string;
  graph_revision?: string;
  bridge?: WireEdgeInsertionBridge;
  rejection?: WireConnectionRejection;
}

interface WireInsertNodeOnEdgeResponse {
  accepted: boolean;
  graphRevision?: string;
  graph_revision?: string;
  insertedNodeId?: string;
  inserted_node_id?: string;
  bridge?: WireEdgeInsertionBridge;
  graph?: AnyInsertNodeOnEdgeResponse['graph'];
  rejection?: WireConnectionRejection;
}

function readString(value: string | undefined, fallback = ''): string {
  return value ?? fallback;
}

function normalizeConnectionRejection(
  rejection: WireConnectionRejection | undefined,
): AnyConnectionRejection | undefined {
  if (!rejection) {
    return undefined;
  }

  return {
    reason: rejection.reason as AnyConnectionRejection['reason'],
    message: rejection.message,
  };
}

function normalizeEdgeInsertionBridge(bridge: WireEdgeInsertionBridge | undefined) {
  if (!bridge) {
    return undefined;
  }

  return {
    input_port_id: readString(bridge.input_port_id ?? bridge.inputPortId),
    output_port_id: readString(bridge.output_port_id ?? bridge.outputPortId),
  };
}

export function serializeConnectionAnchor(anchor: AnyConnectionAnchor): WireConnectionAnchor {
  return {
    nodeId: anchor.node_id,
    portId: anchor.port_id,
  };
}

export function normalizeConnectionCandidatesResponse(
  response: WireConnectionCandidatesResponse,
): AnyCandidatesResponse {
  return {
    graph_revision: readString(response.graph_revision ?? response.graphRevision),
    revision_matches: response.revision_matches ?? response.revisionMatches ?? false,
    source_anchor: response.source_anchor ?? {
      node_id: readString(response.sourceAnchor?.nodeId),
      port_id: readString(response.sourceAnchor?.portId),
    },
    compatible_nodes:
      response.compatible_nodes ??
      (response.compatibleNodes ?? []).map((node) => ({
        node_id: readString(node.node_id ?? node.nodeId),
        node_type: readString(node.node_type ?? node.nodeType),
        node_label: readString(node.node_label ?? node.nodeLabel),
        position: node.position,
        anchors: node.anchors.map((anchor) => ({
          port_id: readString(anchor.port_id ?? anchor.portId),
          port_label: readString(anchor.port_label ?? anchor.portLabel),
          data_type: readString(anchor.data_type ?? anchor.dataType) as PackageConnectionCandidatesResponse['compatible_nodes'][number]['anchors'][number]['data_type'],
          multiple: anchor.multiple,
        })),
      })),
    insertable_node_types:
      response.insertable_node_types ??
      (response.insertableNodeTypes ?? []).map((candidate) => ({
        node_type: readString(candidate.node_type ?? candidate.nodeType),
        category: candidate.category as PackageConnectionCandidatesResponse['insertable_node_types'][number]['category'],
        label: candidate.label,
        description: candidate.description,
        matching_input_port_ids:
          candidate.matching_input_port_ids ?? candidate.matchingInputPortIds ?? [],
      })),
  };
}

export function normalizeConnectionCommitResponse(
  response: WireConnectionCommitResponse,
): AnyCommitResponse {
  return {
    accepted: response.accepted,
    graph_revision: readString(response.graph_revision ?? response.graphRevision),
    graph: response.graph,
    rejection: normalizeConnectionRejection(response.rejection),
  };
}

export function normalizeInsertNodeConnectionResponse(
  response: WireInsertNodeConnectionResponse,
): AnyInsertResponse {
  return {
    accepted: response.accepted,
    graph_revision: readString(response.graph_revision ?? response.graphRevision),
    inserted_node_id: response.inserted_node_id ?? response.insertedNodeId,
    graph: response.graph,
    rejection: normalizeConnectionRejection(response.rejection),
  };
}

export function normalizeEdgeInsertionPreviewResponse(
  response: WireEdgeInsertionPreviewResponse,
): AnyEdgeInsertionPreviewResponse {
  return {
    accepted: response.accepted,
    graph_revision: readString(response.graph_revision ?? response.graphRevision),
    bridge: normalizeEdgeInsertionBridge(response.bridge),
    rejection: normalizeConnectionRejection(response.rejection),
  };
}

export function normalizeInsertNodeOnEdgeResponse(
  response: WireInsertNodeOnEdgeResponse,
): AnyInsertNodeOnEdgeResponse {
  return {
    accepted: response.accepted,
    graph_revision: readString(response.graph_revision ?? response.graphRevision),
    inserted_node_id: response.inserted_node_id ?? response.insertedNodeId,
    bridge: normalizeEdgeInsertionBridge(response.bridge),
    graph: response.graph,
    rejection: normalizeConnectionRejection(response.rejection),
  };
}
