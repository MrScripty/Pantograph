import type {
  ConnectionAnchor,
  ConnectionCandidatesResponse,
  ConnectionCommitResponse,
  EdgeInsertionPreviewResponse,
  InsertNodeConnectionResponse,
  InsertNodeOnEdgeResponse,
  InsertNodePositionHint,
  NodeDefinition,
  WorkflowGraph,
  GraphEdge,
} from '../types/workflow.js';
import { isPortTypeCompatible } from '../portTypeCompatibility.js';
import { buildDerivedGraph, computeGraphFingerprint } from '../graphRevision.js';

function cloneGraphWithoutEdge(graph: WorkflowGraph, edgeId: string): WorkflowGraph {
  return {
    ...structuredClone(graph),
    edges: graph.edges.filter((edge) => edge.id !== edgeId),
  };
}

function findDefinition(
  nodeDefinitions: NodeDefinition[],
  nodeType: string | undefined,
): NodeDefinition | undefined {
  return nodeDefinitions.find((definition) => definition.node_type === nodeType);
}

function resolveEdgeInsertBridge(
  nodeDefinitions: NodeDefinition[],
  graph: WorkflowGraph,
  edgeId: string,
  nodeType: string,
): EdgeInsertionPreviewResponse {
  const currentRevision = computeGraphFingerprint(graph);
  const edge = graph.edges.find((candidate) => candidate.id === edgeId);
  if (!edge) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'unknown_edge',
        message: `edge '${edgeId}' was not found`,
      },
    };
  }

  const sourceNode = graph.nodes.find((node) => node.id === edge.source);
  const targetNode = graph.nodes.find((node) => node.id === edge.target);
  const sourceDef = findDefinition(nodeDefinitions, sourceNode?.node_type);
  const targetDef = findDefinition(nodeDefinitions, targetNode?.node_type);
  const insertDef = findDefinition(nodeDefinitions, nodeType);
  const sourcePort = sourceDef?.outputs.find((port) => port.id === edge.source_handle);
  const targetPort = targetDef?.inputs.find((port) => port.id === edge.target_handle);

  if (!sourcePort) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'unknown_source_anchor',
        message: `source anchor '${edge.source}.${edge.source_handle}' was not found`,
      },
    };
  }
  if (!targetPort) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'unknown_target_anchor',
        message: `target anchor '${edge.target}.${edge.target_handle}' was not found`,
      },
    };
  }
  if (!insertDef) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'unknown_insert_node_type',
        message: `insertable node type '${nodeType}' is unknown`,
      },
    };
  }

  const graphWithoutEdge = cloneGraphWithoutEdge(graph, edgeId);
  for (const inputPort of insertDef.inputs) {
    if (!isPortTypeCompatible(sourcePort.data_type, inputPort.data_type)) {
      continue;
    }

    for (const outputPort of insertDef.outputs) {
      if (!isPortTypeCompatible(outputPort.data_type, targetPort.data_type)) {
        continue;
      }

      const targetOccupied = graphWithoutEdge.edges.some(
        (candidate) =>
          candidate.target === edge.target && candidate.target_handle === edge.target_handle,
      );
      if (!targetPort.multiple && targetOccupied) {
        continue;
      }

      return {
        accepted: true,
        graph_revision: currentRevision,
        bridge: {
          input_port_id: inputPort.id,
          output_port_id: outputPort.id,
        },
      };
    }
  }

  return {
    accepted: false,
    graph_revision: currentRevision,
    rejection: {
      reason: 'no_compatible_insert_path',
      message: `node type '${nodeType}' has no valid path between edge '${edgeId}'`,
    },
  };
}

export function mockGetConnectionCandidates(
  nodeDefinitions: NodeDefinition[],
  graph: WorkflowGraph,
  sourceAnchor: ConnectionAnchor,
  graphRevision?: string,
): ConnectionCandidatesResponse {
  const sourceNode = graph.nodes.find((node) => node.id === sourceAnchor.node_id);
  if (!sourceNode) throw new Error(`Source node not found: ${sourceAnchor.node_id}`);
  const sourceDef = nodeDefinitions.find((def) => def.node_type === sourceNode.node_type);
  const sourcePort = sourceDef?.outputs.find((port) => port.id === sourceAnchor.port_id);
  if (!sourcePort) throw new Error(`Source anchor not found: ${sourceAnchor.node_id}.${sourceAnchor.port_id}`);

  const compatibleNodes = graph.nodes
    .filter((node) => node.id !== sourceAnchor.node_id)
    .map((node) => {
      const definition = nodeDefinitions.find((def) => def.node_type === node.node_type);
      if (!definition) return null;

      const anchors = definition.inputs
        .filter((port) => {
          if (!isPortTypeCompatible(sourcePort.data_type, port.data_type)) return false;
          if (!port.multiple) {
            return !graph.edges.some(
              (edge) => edge.target === node.id && edge.target_handle === port.id,
            );
          }
          return true;
        })
        .map((port) => ({
          port_id: port.id,
          port_label: port.label,
          data_type: port.data_type,
          multiple: port.multiple,
        }));

      if (anchors.length === 0) return null;

      return {
        node_id: node.id,
        node_type: node.node_type,
        node_label: String(node.data.label ?? definition.label),
        position: node.position,
        anchors,
      };
    })
    .filter((node): node is NonNullable<typeof node> => node !== null);

  const insertableNodeTypes = nodeDefinitions
    .map((definition) => {
      const matchingInputPortIds = definition.inputs
        .filter((port) => isPortTypeCompatible(sourcePort.data_type, port.data_type))
        .map((port) => port.id);
      if (matchingInputPortIds.length === 0) return null;
      return {
        node_type: definition.node_type,
        category: definition.category,
        label: definition.label,
        description: definition.description,
        matching_input_port_ids: matchingInputPortIds,
      };
    })
    .filter((node): node is NonNullable<typeof node> => node !== null);

  const currentRevision = computeGraphFingerprint(graph);
  return {
    graph_revision: currentRevision,
    revision_matches: !graphRevision || graphRevision === currentRevision,
    source_anchor: sourceAnchor,
    compatible_nodes: compatibleNodes,
    insertable_node_types: insertableNodeTypes,
  };
}

export function mockConnectAnchors(
  nodeDefinitions: NodeDefinition[],
  graph: WorkflowGraph,
  sourceAnchor: ConnectionAnchor,
  targetAnchor: ConnectionAnchor,
  graphRevision: string,
): ConnectionCommitResponse {
  const currentRevision = computeGraphFingerprint(graph);
  if (graphRevision !== currentRevision) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'stale_revision',
        message: `graph revision '${graphRevision}' is stale`,
      },
    };
  }

  const sourceNode = graph.nodes.find((node) => node.id === sourceAnchor.node_id);
  const targetNode = graph.nodes.find((node) => node.id === targetAnchor.node_id);
  const sourceDef = nodeDefinitions.find((def) => def.node_type === sourceNode?.node_type);
  const targetDef = nodeDefinitions.find((def) => def.node_type === targetNode?.node_type);
  const sourcePort = sourceDef?.outputs.find((port) => port.id === sourceAnchor.port_id);
  const targetPort = targetDef?.inputs.find((port) => port.id === targetAnchor.port_id);

  if (!sourcePort) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'unknown_source_anchor',
        message: `source anchor '${sourceAnchor.node_id}.${sourceAnchor.port_id}' was not found`,
      },
    };
  }
  if (!targetPort) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'unknown_target_anchor',
        message: `target anchor '${targetAnchor.node_id}.${targetAnchor.port_id}' was not found`,
      },
    };
  }
  if (!isPortTypeCompatible(sourcePort.data_type, targetPort.data_type)) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'incompatible_types',
        message: `${sourcePort.data_type} cannot connect to ${targetPort.data_type}`,
      },
    };
  }

  const edge: GraphEdge = {
    id: `${sourceAnchor.node_id}-${sourceAnchor.port_id}-${targetAnchor.node_id}-${targetAnchor.port_id}`,
    source: sourceAnchor.node_id,
    source_handle: sourceAnchor.port_id,
    target: targetAnchor.node_id,
    target_handle: targetAnchor.port_id,
  };
  graph.edges.push(edge);
  graph.derived_graph = buildDerivedGraph(graph);
  return {
    accepted: true,
    graph_revision: graph.derived_graph.graph_fingerprint,
    graph: structuredClone(graph),
  };
}

export function mockInsertNodeAndConnect(
  nodeDefinitions: NodeDefinition[],
  graph: WorkflowGraph,
  sourceAnchor: ConnectionAnchor,
  nodeType: string,
  graphRevision: string,
  positionHint: InsertNodePositionHint,
  preferredInputPortId?: string,
): InsertNodeConnectionResponse {
  const currentRevision = computeGraphFingerprint(graph);
  if (graphRevision !== currentRevision) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'stale_revision',
        message: `graph revision '${graphRevision}' is stale`,
      },
    };
  }

  const sourceNode = graph.nodes.find((node) => node.id === sourceAnchor.node_id);
  const sourceDef = nodeDefinitions.find((def) => def.node_type === sourceNode?.node_type);
  const sourcePort = sourceDef?.outputs.find((port) => port.id === sourceAnchor.port_id);
  if (!sourcePort) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'unknown_source_anchor',
        message: `source anchor '${sourceAnchor.node_id}.${sourceAnchor.port_id}' was not found`,
      },
    };
  }

  const insertDef = nodeDefinitions.find((def) => def.node_type === nodeType);
  if (!insertDef) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'unknown_insert_node_type',
        message: `insertable node type '${nodeType}' is unknown`,
      },
    };
  }

  const targetPort =
    insertDef.inputs.find(
      (port) =>
        preferredInputPortId &&
        port.id === preferredInputPortId &&
        isPortTypeCompatible(sourcePort.data_type, port.data_type),
    ) ??
    insertDef.inputs
      .filter((port) => isPortTypeCompatible(sourcePort.data_type, port.data_type))
      .sort((left, right) => left.label.localeCompare(right.label) || left.id.localeCompare(right.id))[0];

  if (!targetPort) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'no_compatible_insert_input',
        message: `node type '${nodeType}' has no compatible input for ${sourcePort.data_type}`,
      },
    };
  }

  const insertedNodeId = `${nodeType}-${Date.now()}`;
  graph.nodes.push({
    id: insertedNodeId,
    node_type: nodeType,
    position: positionHint.position,
    data: {
      label: insertDef.label,
      ...Object.fromEntries(insertDef.inputs.map((input) => [input.id, null])),
    },
  });
  graph.edges.push({
    id: `${sourceAnchor.node_id}-${sourceAnchor.port_id}-${insertedNodeId}-${targetPort.id}`,
    source: sourceAnchor.node_id,
    source_handle: sourceAnchor.port_id,
    target: insertedNodeId,
    target_handle: targetPort.id,
  });
  graph.derived_graph = buildDerivedGraph(graph);

  return {
    accepted: true,
    graph_revision: graph.derived_graph.graph_fingerprint,
    inserted_node_id: insertedNodeId,
    graph: structuredClone(graph),
  };
}

export function mockPreviewNodeInsertOnEdge(
  nodeDefinitions: NodeDefinition[],
  graph: WorkflowGraph,
  edgeId: string,
  nodeType: string,
  graphRevision: string,
): EdgeInsertionPreviewResponse {
  const currentRevision = computeGraphFingerprint(graph);
  if (graphRevision !== currentRevision) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'stale_revision',
        message: `graph revision '${graphRevision}' is stale`,
      },
    };
  }

  return resolveEdgeInsertBridge(nodeDefinitions, graph, edgeId, nodeType);
}

export function mockInsertNodeOnEdge(
  nodeDefinitions: NodeDefinition[],
  graph: WorkflowGraph,
  edgeId: string,
  nodeType: string,
  graphRevision: string,
  positionHint: InsertNodePositionHint,
): InsertNodeOnEdgeResponse {
  const currentRevision = computeGraphFingerprint(graph);
  if (graphRevision !== currentRevision) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'stale_revision',
        message: `graph revision '${graphRevision}' is stale`,
      },
    };
  }

  const preview = resolveEdgeInsertBridge(nodeDefinitions, graph, edgeId, nodeType);
  if (!preview.accepted || !preview.bridge) {
    return {
      accepted: false,
      graph_revision: preview.graph_revision,
      rejection: preview.rejection,
    };
  }

  const edge = graph.edges.find((candidate) => candidate.id === edgeId);
  if (!edge) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'unknown_edge',
        message: `edge '${edgeId}' was not found`,
      },
    };
  }

  const insertDef = findDefinition(nodeDefinitions, nodeType);
  if (!insertDef) {
    return {
      accepted: false,
      graph_revision: currentRevision,
      rejection: {
        reason: 'unknown_insert_node_type',
        message: `insertable node type '${nodeType}' is unknown`,
      },
    };
  }

  const insertedNodeId = `${nodeType}-${Date.now()}`;
  graph.edges = graph.edges.filter((candidate) => candidate.id !== edgeId);
  graph.nodes.push({
    id: insertedNodeId,
    node_type: nodeType,
    position: positionHint.position,
    data: {
      label: insertDef.label,
      ...Object.fromEntries(insertDef.inputs.map((input) => [input.id, null])),
    },
  });
  graph.edges.push({
    id: `${edge.source}-${edge.source_handle}-${insertedNodeId}-${preview.bridge.input_port_id}`,
    source: edge.source,
    source_handle: edge.source_handle,
    target: insertedNodeId,
    target_handle: preview.bridge.input_port_id,
  });
  graph.edges.push({
    id: `${insertedNodeId}-${preview.bridge.output_port_id}-${edge.target}-${edge.target_handle}`,
    source: insertedNodeId,
    source_handle: preview.bridge.output_port_id,
    target: edge.target,
    target_handle: edge.target_handle,
  });
  graph.derived_graph = buildDerivedGraph(graph);

  return {
    accepted: true,
    graph_revision: graph.derived_graph.graph_fingerprint,
    inserted_node_id: insertedNodeId,
    bridge: preview.bridge,
    graph: structuredClone(graph),
  };
}
