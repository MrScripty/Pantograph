import type { Edge, Node } from '@xyflow/svelte';

import type { NodeDefinition, WorkflowGraph } from '../types/workflow.js';
import { applySelectedNodeIds } from '../workflowSelection.ts';
import { resolveNodeDefinitionOverlay } from './definitionOverlay.ts';

export interface WorkflowGraphMaterialization {
  graphNodes: Node[];
  graphEdges: Edge[];
  graph: WorkflowGraph;
}

export function projectWorkflowGraphStoreState(params: {
  nodes: Node[];
  edges: Edge[];
  derivedGraph: WorkflowGraph['derived_graph'] | undefined;
}): WorkflowGraph {
  return {
    nodes: params.nodes.map((node) => ({
      id: node.id,
      node_type: node.type || 'unknown',
      position: node.position,
      data: node.data,
    })),
    edges: params.edges.map((edge) => ({
      id: edge.id,
      source: edge.source,
      source_handle: edge.sourceHandle || 'output',
      target: edge.target,
      target_handle: edge.targetHandle || 'input',
    })),
    ...(typeof params.derivedGraph === 'undefined' ? {} : { derived_graph: params.derivedGraph }),
  };
}

export function materializeWorkflowGraphSnapshot(params: {
  graph: WorkflowGraph;
  definitions: NodeDefinition[];
  selectedNodeIds: string[];
}): WorkflowGraphMaterialization {
  const graphNodes = applySelectedNodeIds(
    params.graph.nodes.map((node) => {
      const nodeType = node.node_type;
      const nodeData = { ...node.data };
      const definition = resolveNodeDefinitionOverlay(nodeType, nodeData, params.definitions);

      return {
        id: node.id,
        type: nodeType,
        position: node.position,
        data: { ...nodeData, definition },
      };
    }),
    params.selectedNodeIds,
  );

  const graphEdges: Edge[] = params.graph.edges.map((edge) => ({
    id: edge.id,
    source: edge.source,
    sourceHandle: edge.source_handle,
    target: edge.target,
    targetHandle: edge.target_handle,
  }));

  return { graphNodes, graphEdges, graph: params.graph };
}
