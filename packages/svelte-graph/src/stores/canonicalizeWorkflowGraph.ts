import type {
  GraphEdge,
  GraphNode,
  NodeDefinition,
  WorkflowGraph,
} from '../types/workflow.js';
import { resolveNodeDefinitionOverlay } from './definitionOverlay.ts';
import {
  buildExpandSettingsSchema,
  buildDynamicExpandDefinition,
  buildDynamicInferenceDefinition,
  buildMergedInferenceSettings,
  type InferenceParamSchema,
} from './inferenceSettingsPorts.ts';

interface CanonicalizedGraph {
  graph: WorkflowGraph;
  migratedNodeIds: Set<string>;
}

function isInferenceParamSchemaArray(value: unknown): value is InferenceParamSchema[] {
  return Array.isArray(value) && value.every((entry) => {
    if (!entry || typeof entry !== 'object') return false;
    const record = entry as Record<string, unknown>;
    return typeof record.key === 'string';
  });
}

function findConnectedTargets(
  edges: GraphEdge[],
  sourceId: string,
  sourceHandle: string,
): string[] {
  return edges
    .filter((edge) => edge.source === sourceId && edge.source_handle === sourceHandle)
    .map((edge) => edge.target);
}

function hasEdge(
  edges: GraphEdge[],
  source: string,
  sourceHandle: string,
  target: string,
  targetHandle: string,
): boolean {
  return edges.some((edge) =>
    edge.source === source &&
    edge.source_handle === sourceHandle &&
    edge.target === target &&
    edge.target_handle === targetHandle
  );
}

function canonicalizeLegacyNodeTypes(graph: WorkflowGraph): CanonicalizedGraph {
  const migratedNodeIds = new Set<string>();
  const nodes = graph.nodes.map((node) => {
    if (node.node_type !== 'system-prompt') {
      return {
        ...node,
        data: { ...node.data },
      };
    }

    const data = { ...node.data };
    migratedNodeIds.add(node.id);
    if ('prompt' in data && !('text' in data)) {
      data.text = data.prompt;
      delete data.prompt;
    }

    return {
      ...node,
      node_type: 'text-input',
      data,
    };
  });

  const edges = graph.edges.map((edge) => {
    let sourceHandle = edge.source_handle;
    let targetHandle = edge.target_handle;
    if (migratedNodeIds.has(edge.source) && sourceHandle === 'prompt') sourceHandle = 'text';
    if (migratedNodeIds.has(edge.target) && targetHandle === 'prompt') targetHandle = 'text';
    return {
      ...edge,
      source_handle: sourceHandle,
      target_handle: targetHandle,
    };
  });

  return {
    graph: {
      ...graph,
      nodes,
      edges,
    },
    migratedNodeIds,
  };
}

export function canonicalizeWorkflowGraph(
  graph: WorkflowGraph,
  definitions: NodeDefinition[],
): WorkflowGraph {
  const { graph: migratedGraph } = canonicalizeLegacyNodeTypes(graph);
  const nodes = migratedGraph.nodes.map((node) => ({
    ...node,
    data: { ...node.data },
  }));
  const edges = [...migratedGraph.edges];
  const nodesById = new Map(nodes.map((node) => [node.id, node]));
  const baseDefinitionsByType = new Map(
    definitions.map((definition) => [definition.node_type, definition]),
  );

  function getBaseDefinition(nodeType: string): NodeDefinition | undefined {
    return baseDefinitionsByType.get(nodeType);
  }

  function getResolvedDefinition(node: GraphNode): NodeDefinition | undefined {
    return resolveNodeDefinitionOverlay(node.node_type, node.data, definitions);
  }

  function setNodeDefinition(nodeId: string, definition: NodeDefinition): void {
    const node = nodesById.get(nodeId);
    if (!node) return;
    node.data = {
      ...node.data,
      definition,
    };
  }

  function setNodeInferenceSettings(
    nodeId: string,
    inferenceSettings: InferenceParamSchema[],
  ): void {
    const node = nodesById.get(nodeId);
    if (!node) return;
    node.data = {
      ...node.data,
      inference_settings: inferenceSettings,
    };
  }

  function reconcileInferenceNode(
    nodeId: string,
    inferenceSettings: InferenceParamSchema[],
  ): InferenceParamSchema[] {
    const node = nodesById.get(nodeId);
    if (!node) return [];

    const baseDefinition = getBaseDefinition(node.node_type);
    const currentDefinition = getResolvedDefinition(node);
    if (!baseDefinition || !currentDefinition) return [];

    const mergedSettings = buildMergedInferenceSettings(baseDefinition, inferenceSettings);
    setNodeDefinition(
      nodeId,
      buildDynamicInferenceDefinition(currentDefinition, baseDefinition, mergedSettings),
    );
    return mergedSettings;
  }

  for (const node of nodes) {
    if (node.node_type === 'expand-settings') continue;
    if (!isInferenceParamSchemaArray(node.data.inference_settings)) continue;

    const inferenceSettings = node.data.inference_settings;
    const downstreamIds = findConnectedTargets(edges, node.id, 'inference_settings');

    for (const targetId of downstreamIds) {
      const targetNode = nodesById.get(targetId);
      if (!targetNode) continue;

      if (targetNode.node_type !== 'expand-settings') {
        reconcileInferenceNode(targetId, inferenceSettings);
        continue;
      }

      const baseExpandDefinition = getBaseDefinition('expand-settings');
      const currentExpandDefinition = getResolvedDefinition(targetNode);
      if (!baseExpandDefinition || !currentExpandDefinition) continue;

      const downstreamInferenceIds = findConnectedTargets(
        edges,
        targetId,
        'inference_settings',
      );
      const downstreamBaseDefinitions = downstreamInferenceIds
        .map((downstreamId) => {
          const downstreamNode = nodesById.get(downstreamId);
          return downstreamNode
            ? getBaseDefinition(downstreamNode.node_type)
            : undefined;
        })
        .filter((definition): definition is NodeDefinition => definition !== undefined);

      const mergedExpandSettings = buildExpandSettingsSchema(
        downstreamBaseDefinitions,
        inferenceSettings,
      );
      setNodeDefinition(
        targetId,
        buildDynamicExpandDefinition(
          currentExpandDefinition,
          baseExpandDefinition,
          mergedExpandSettings,
        ),
      );
      setNodeInferenceSettings(targetId, mergedExpandSettings);

      for (const downstreamInferenceId of downstreamInferenceIds) {
        const targetSettings = reconcileInferenceNode(
          downstreamInferenceId,
          inferenceSettings,
        );

        for (const param of targetSettings) {
          if (
            hasEdge(
              edges,
              targetId,
              param.key,
              downstreamInferenceId,
              param.key,
            )
          ) {
            continue;
          }

          edges.push({
            id: `${targetId}-${param.key}-${downstreamInferenceId}-${param.key}`,
            source: targetId,
            source_handle: param.key,
            target: downstreamInferenceId,
            target_handle: param.key,
          });
        }
      }
    }
  }

  return {
    ...migratedGraph,
    nodes,
    edges,
  };
}
