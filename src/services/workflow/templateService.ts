/**
 * Template Service - Loads and manages workflow templates
 *
 * This service provides access to pre-built workflow templates like
 * the Svelte Code Agent workflow.
 */

import { invoke } from '@tauri-apps/api/core';
import type { OrchestrationGraph, OrchestrationNode, OrchestrationEdge } from '../../stores/orchestrationStore';
import type { WorkflowGraph, GraphNode, GraphEdge } from './types';
import type { NodeGroup, PortMapping } from './groupTypes';

// Import templates statically (bundled with the app)
import svelteCodeAgentTemplate from '../../templates/workflows/svelte-code-agent.json';

/**
 * A workflow template containing an orchestration and its data graphs
 */
export interface WorkflowTemplate {
  name: string;
  description: string;
  version: string;
  orchestration: OrchestrationGraphTemplate;
  dataGraphs: Record<string, DataGraphTemplate>;
  nodeGroups?: Record<string, NodeGroupTemplate>;
}

export interface OrchestrationGraphTemplate {
  id: string;
  name: string;
  description: string;
  nodes: OrchestrationNodeTemplate[];
  edges: OrchestrationEdgeTemplate[];
  dataGraphs: Record<string, string>;
}

export interface OrchestrationNodeTemplate {
  id: string;
  nodeType: string;
  position: [number, number];
  config: Record<string, unknown>;
}

export interface OrchestrationEdgeTemplate {
  id: string;
  source: string;
  sourceHandle: string;
  target: string;
  targetHandle: string;
}

export interface DataGraphTemplate {
  id: string;
  name: string;
  nodes: GraphNode[];
  edges: Array<{
    id: string;
    source: string;
    source_handle: string;
    target: string;
    target_handle: string;
  }>;
}

export interface NodeGroupTemplate {
  id: string;
  name: string;
  description?: string;
  nodes: GraphNode[];
  edges: Array<{
    id: string;
    source: string;
    source_handle: string;
    target: string;
    target_handle: string;
  }>;
  exposed_inputs: PortMapping[];
  exposed_outputs: PortMapping[];
  position: [number, number];
  collapsed: boolean;
}

/**
 * Available workflow templates
 * Note: We use unknown cast due to JSON position arrays not matching tuple types
 */
export const workflowTemplates: Record<string, WorkflowTemplate> = {
  'svelte-code-agent': svelteCodeAgentTemplate as unknown as WorkflowTemplate,
};

/**
 * Get a list of available templates
 */
export function getAvailableTemplates(): Array<{ id: string; name: string; description: string }> {
  return Object.entries(workflowTemplates).map(([id, template]) => ({
    id,
    name: template.name,
    description: template.description,
  }));
}

/**
 * Get a template by ID
 */
export function getTemplate(templateId: string): WorkflowTemplate | undefined {
  return workflowTemplates[templateId];
}

/**
 * Load a template into the orchestration system
 *
 * This will:
 * 1. Create the orchestration graph
 * 2. Register all data graphs
 * 3. Register any node groups
 */
export async function loadTemplate(templateId: string): Promise<{
  orchestration: OrchestrationGraph;
  dataGraphIds: string[];
}> {
  const template = getTemplate(templateId);
  if (!template) {
    throw new Error(`Template '${templateId}' not found`);
  }

  // Generate unique IDs for this instance
  const instanceId = Date.now().toString(36);

  // Convert orchestration template to OrchestrationGraph
  const orchestration: OrchestrationGraph = {
    id: `${template.orchestration.id}-${instanceId}`,
    name: template.orchestration.name,
    description: template.orchestration.description,
    nodes: template.orchestration.nodes.map((node) => ({
      id: `${node.id}-${instanceId}`,
      nodeType: node.nodeType as OrchestrationGraph['nodes'][0]['nodeType'],
      position: node.position,
      config: node.config,
    })),
    edges: template.orchestration.edges.map((edge) => ({
      id: `${edge.id}-${instanceId}`,
      source: `${edge.source}-${instanceId}`,
      sourceHandle: edge.sourceHandle,
      target: `${edge.target}-${instanceId}`,
      targetHandle: edge.targetHandle,
    })),
    dataGraphs: Object.fromEntries(
      Object.entries(template.orchestration.dataGraphs).map(([nodeId, graphId]) => [
        `${nodeId}-${instanceId}`,
        `${graphId}-${instanceId}`,
      ])
    ),
  };

  // Register data graphs with the backend
  const dataGraphIds: string[] = [];
  for (const [graphId, graphTemplate] of Object.entries(template.dataGraphs)) {
    const newGraphId = `${graphId}-${instanceId}`;

    const workflowGraph: WorkflowGraph = {
      nodes: graphTemplate.nodes.map((node) => ({
        ...node,
        id: `${node.id}-${instanceId}`,
      })),
      edges: graphTemplate.edges.map((edge) => ({
        id: `${edge.id}-${instanceId}`,
        source: `${edge.source}-${instanceId}`,
        source_handle: edge.source_handle,
        target: `${edge.target}-${instanceId}`,
        target_handle: edge.target_handle,
      })),
    };

    // Register with backend
    await invoke('register_data_graph', {
      id: newGraphId,
      graph: workflowGraph,
    });

    dataGraphIds.push(newGraphId);
  }

  // Save orchestration to backend
  await invoke('save_orchestration', { graph: orchestration });

  return { orchestration, dataGraphIds };
}

/**
 * Convert a template's data graph to a WorkflowGraph for editing
 */
export function dataGraphTemplateToWorkflowGraph(
  template: DataGraphTemplate,
  instanceId?: string
): WorkflowGraph {
  const suffix = instanceId ? `-${instanceId}` : '';

  return {
    nodes: template.nodes.map((node) => ({
      ...node,
      id: `${node.id}${suffix}`,
    })),
    edges: template.edges.map((edge) => ({
      id: `${edge.id}${suffix}`,
      source: `${edge.source}${suffix}`,
      source_handle: edge.source_handle,
      target: `${edge.target}${suffix}`,
      target_handle: edge.target_handle,
    })),
  };
}

/**
 * Convert a node group template to a NodeGroup
 */
export function nodeGroupTemplateToNodeGroup(
  template: NodeGroupTemplate,
  instanceId?: string
): NodeGroup {
  const suffix = instanceId ? `-${instanceId}` : '';

  return {
    id: `${template.id}${suffix}`,
    name: template.name,
    description: template.description,
    nodes: template.nodes.map((node) => ({
      ...node,
      id: `${node.id}${suffix}`,
    })),
    edges: template.edges.map((edge) => ({
      id: `${edge.id}${suffix}`,
      source: `${edge.source}${suffix}`,
      source_handle: edge.source_handle,
      target: `${edge.target}${suffix}`,
      target_handle: edge.target_handle,
    })),
    exposed_inputs: template.exposed_inputs.map((mapping) => ({
      ...mapping,
      internal_node_id: `${mapping.internal_node_id}${suffix}`,
      group_port_id: `${mapping.group_port_id}${suffix}`,
    })),
    exposed_outputs: template.exposed_outputs.map((mapping) => ({
      ...mapping,
      internal_node_id: `${mapping.internal_node_id}${suffix}`,
      group_port_id: `${mapping.group_port_id}${suffix}`,
    })),
    position: { x: template.position[0], y: template.position[1] },
    collapsed: template.collapsed,
  };
}
