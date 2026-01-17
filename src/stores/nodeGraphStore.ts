import { writable, derived } from 'svelte/store';
import type { Node, Edge } from '@xyflow/svelte';

// Define the initial nodes for the agent workflow
const initialNodes: Node[] = [
  {
    id: 'user-input',
    type: 'userInput',
    position: { x: 50, y: 100 },
    data: { label: 'User Input' },
  },
  {
    id: 'system-prompt',
    type: 'systemPrompt',
    position: { x: 50, y: 250 },
    data: { label: 'System Prompt' },
  },
  {
    id: 'tools',
    type: 'tools',
    position: { x: 50, y: 400 },
    data: { label: 'Tools' },
  },
  {
    id: 'agent',
    type: 'agent',
    position: { x: 350, y: 250 },
    data: { label: 'Agent' },
  },
  {
    id: 'output',
    type: 'output',
    position: { x: 650, y: 250 },
    data: { label: 'Output' },
  },
];

// Define the edges (connections) between nodes
const initialEdges: Edge[] = [
  {
    id: 'user-input-to-agent',
    source: 'user-input',
    target: 'agent',
    sourceHandle: 'output',
    targetHandle: 'user-input',
    animated: false,
  },
  {
    id: 'system-prompt-to-agent',
    source: 'system-prompt',
    target: 'agent',
    sourceHandle: 'output',
    targetHandle: 'system-prompt',
    animated: false,
  },
  {
    id: 'tools-to-agent',
    source: 'tools',
    target: 'agent',
    sourceHandle: 'output',
    targetHandle: 'tools',
    animated: false,
  },
  {
    id: 'agent-to-output',
    source: 'agent',
    target: 'output',
    sourceHandle: 'result',
    targetHandle: 'input',
    animated: false,
  },
];

// Create writable stores for nodes and edges
export const nodes = writable<Node[]>(initialNodes);
export const edges = writable<Edge[]>(initialEdges);

// Update node position (for dragging)
export function updateNodePosition(nodeId: string, position: { x: number; y: number }) {
  nodes.update((n) =>
    n.map((node) => (node.id === nodeId ? { ...node, position } : node))
  );
}

// Update node data
export function updateNodeData(nodeId: string, data: Record<string, unknown>) {
  nodes.update((n) =>
    n.map((node) => (node.id === nodeId ? { ...node, data: { ...node.data, ...data } } : node))
  );
}
