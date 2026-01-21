import { writable, derived, get } from 'svelte/store';
import type { Node, Edge } from '@xyflow/svelte';
import type {
  WorkflowGraph,
  WorkflowMetadata,
  NodeExecutionState,
  NodeDefinition,
} from '../services/workflow/types';

// --- State ---

export const nodes = writable<Node[]>([]);
export const edges = writable<Edge[]>([]);
export const nodeDefinitions = writable<NodeDefinition[]>([]);
export const workflowMetadata = writable<WorkflowMetadata | null>(null);
export const isDirty = writable<boolean>(false);
export const isExecuting = writable<boolean>(false);
export const isEditing = writable<boolean>(true);
export const nodeExecutionStates = writable<Map<string, NodeExecutionState>>(new Map());

// --- Derived ---

export const workflowGraph = derived(
  [nodes, edges],
  ([$nodes, $edges]): WorkflowGraph => ({
    nodes: $nodes.map((n) => ({
      id: n.id,
      node_type: n.type || 'unknown',
      position: n.position,
      data: n.data,
    })),
    edges: $edges.map((e) => ({
      id: e.id,
      source: e.source,
      source_handle: e.sourceHandle || 'output',
      target: e.target,
      target_handle: e.targetHandle || 'input',
    })),
  })
);

export const nodeDefinitionsByCategory = derived(nodeDefinitions, ($defs) => {
  const grouped = new Map<string, NodeDefinition[]>();
  for (const def of $defs) {
    const list = grouped.get(def.category) || [];
    list.push(def);
    grouped.set(def.category, list);
  }
  return grouped;
});

// --- Actions ---

export function addNode(definition: NodeDefinition, position: { x: number; y: number }) {
  const id = `${definition.node_type}-${Date.now()}`;

  const newNode: Node = {
    id,
    type: definition.node_type,
    position,
    data: {
      label: definition.label,
      definition,
      // Initialize empty values for inputs
      ...Object.fromEntries(definition.inputs.map((input) => [input.id, null])),
    },
  };

  nodes.update((n) => [...n, newNode]);
  isDirty.set(true);
}

export function removeNode(nodeId: string) {
  nodes.update((n) => n.filter((node) => node.id !== nodeId));
  edges.update((e) => e.filter((edge) => edge.source !== nodeId && edge.target !== nodeId));
  isDirty.set(true);
}

export function addEdge(edge: Edge) {
  // Prevent duplicate edges
  edges.update((e) => {
    const exists = e.some(
      (existing) =>
        existing.source === edge.source &&
        existing.sourceHandle === edge.sourceHandle &&
        existing.target === edge.target &&
        existing.targetHandle === edge.targetHandle
    );
    if (exists) return e;
    return [...e, edge];
  });
  isDirty.set(true);
}

export function removeEdge(edgeId: string) {
  edges.update((e) => e.filter((edge) => edge.id !== edgeId));
  isDirty.set(true);
}

export function updateNodePosition(nodeId: string, position: { x: number; y: number }) {
  nodes.update((n) =>
    n.map((node) => (node.id === nodeId ? { ...node, position } : node))
  );
  isDirty.set(true);
}

export function updateNodeData(nodeId: string, data: Record<string, unknown>) {
  nodes.update((n) =>
    n.map((node) =>
      node.id === nodeId ? { ...node, data: { ...node.data, ...data } } : node
    )
  );
  isDirty.set(true);
}

export function setNodeExecutionState(nodeId: string, state: NodeExecutionState) {
  nodeExecutionStates.update((map) => {
    const newMap = new Map(map);
    newMap.set(nodeId, state);
    return newMap;
  });
}

export function resetExecutionStates() {
  nodeExecutionStates.set(new Map());
}

export function loadWorkflow(graph: WorkflowGraph, metadata?: WorkflowMetadata) {
  // Get current node definitions to attach to loaded nodes
  const definitions = get(nodeDefinitions);

  // Track which nodes were migrated from system-prompt
  const migratedNodeIds = new Set<string>();

  // Convert GraphNodes to SvelteFlow Nodes
  // Migrate system-prompt nodes to text-input
  nodes.set(
    graph.nodes.map((n) => {
      let nodeType = n.node_type;
      let nodeData = { ...n.data };

      // Migration: system-prompt -> text-input
      if (nodeType === 'system-prompt') {
        nodeType = 'text-input';
        migratedNodeIds.add(n.id);
        // Migrate 'prompt' field to 'text' if present
        if ('prompt' in nodeData && !('text' in nodeData)) {
          nodeData.text = nodeData.prompt;
          delete nodeData.prompt;
        }
      }

      // Look up the definition for this node type
      const definition = definitions.find((d) => d.node_type === nodeType);

      return {
        id: n.id,
        type: nodeType,
        position: n.position,
        data: {
          ...nodeData,
          definition, // Attach definition so BaseNode can render inputs/outputs
        },
      };
    })
  );

  // Convert GraphEdges to SvelteFlow Edges
  // Migrate 'prompt' handles to 'text' for migrated nodes
  edges.set(
    graph.edges.map((e) => {
      let sourceHandle = e.source_handle;
      let targetHandle = e.target_handle;

      // If the source was a migrated system-prompt node, change 'prompt' to 'text'
      if (migratedNodeIds.has(e.source) && sourceHandle === 'prompt') {
        sourceHandle = 'text';
      }
      // If the target was a migrated system-prompt node, change 'prompt' to 'text'
      if (migratedNodeIds.has(e.target) && targetHandle === 'prompt') {
        targetHandle = 'text';
      }

      return {
        id: e.id,
        source: e.source,
        sourceHandle,
        target: e.target,
        targetHandle,
      };
    })
  );

  workflowMetadata.set(metadata || null);
  isDirty.set(false);
}

export function clearWorkflow() {
  nodes.set([]);
  edges.set([]);
  workflowMetadata.set(null);
  isDirty.set(false);
  resetExecutionStates();
}

// --- Default Workflow ---

export function loadDefaultWorkflow(definitions: NodeDefinition[]) {
  const textInputDef = definitions.find((d) => d.node_type === 'text-input');
  const llmDef = definitions.find((d) => d.node_type === 'llm-inference');
  const outputDef = definitions.find((d) => d.node_type === 'text-output');

  const defaultNodes: Node[] = [
    {
      id: 'user-input',
      type: 'text-input',
      position: { x: 50, y: 150 },
      data: { label: 'User Input', text: '', definition: textInputDef },
    },
    {
      id: 'llm',
      type: 'llm-inference',
      position: { x: 350, y: 150 },
      data: { label: 'LLM Inference', definition: llmDef },
    },
    {
      id: 'output',
      type: 'text-output',
      position: { x: 650, y: 150 },
      data: { label: 'Output', text: '', definition: outputDef },
    },
  ];

  const defaultEdges: Edge[] = [
    {
      id: 'input-to-llm',
      source: 'user-input',
      sourceHandle: 'text',
      target: 'llm',
      targetHandle: 'prompt',
    },
    {
      id: 'llm-to-output',
      source: 'llm',
      sourceHandle: 'response',
      target: 'output',
      targetHandle: 'text',
    },
  ];

  nodes.set(defaultNodes);
  edges.set(defaultEdges);
  isDirty.set(false);
}
