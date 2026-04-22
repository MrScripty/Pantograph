import type { Edge, Node } from '@xyflow/svelte';

import type { NodeGroup } from '../types/groups.js';

export interface WorkflowNodeBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export function isWorkflowNodeGroupData(value: unknown): value is NodeGroup {
  if (typeof value !== 'object' || value === null) {
    return false;
  }

  const candidate = value as Partial<NodeGroup>;
  return (
    typeof candidate.id === 'string' &&
    typeof candidate.name === 'string' &&
    Array.isArray(candidate.nodes) &&
    Array.isArray(candidate.edges) &&
    Array.isArray(candidate.exposed_inputs) &&
    Array.isArray(candidate.exposed_outputs)
  );
}

export function extractWorkflowNodeGroups(graphNodes: Node[]): Map<string, NodeGroup> {
  const groups = new Map<string, NodeGroup>();
  for (const node of graphNodes) {
    const group = node.data?.group;
    if (isWorkflowNodeGroupData(group)) {
      groups.set(group.id, group);
    }
  }
  return groups;
}

export function findWorkflowGroupContainingNodeIds(
  groups: Map<string, NodeGroup>,
  nodeIds: string[],
): NodeGroup | null {
  const selected = new Set(nodeIds);
  for (const group of groups.values()) {
    const groupNodeIds = new Set(group.nodes.map((node) => node.id));
    if (selected.size === groupNodeIds.size && nodeIds.every((nodeId) => groupNodeIds.has(nodeId))) {
      return group;
    }
  }
  return null;
}

export function getWorkflowConnectedNodes(
  currentNodes: Node[],
  currentEdges: Edge[],
  nodeId: string,
): { inputs: Node[]; outputs: Node[] } {
  const inputNodeIds = currentEdges.filter((edge) => edge.target === nodeId).map((edge) => edge.source);
  const outputNodeIds = currentEdges.filter((edge) => edge.source === nodeId).map((edge) => edge.target);
  return {
    inputs: currentNodes.filter((node) => inputNodeIds.includes(node.id)),
    outputs: currentNodes.filter((node) => outputNodeIds.includes(node.id)),
  };
}

export function getWorkflowNodesBounds(
  currentNodes: Node[],
  nodeIds: string[],
): WorkflowNodeBounds | null {
  const targetNodes = currentNodes.filter((node) => nodeIds.includes(node.id));
  if (targetNodes.length === 0) return null;

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const node of targetNodes) {
    const width = (node.measured?.width || node.width || 200) as number;
    const height = (node.measured?.height || node.height || 100) as number;
    minX = Math.min(minX, node.position.x);
    minY = Math.min(minY, node.position.y);
    maxX = Math.max(maxX, node.position.x + width);
    maxY = Math.max(maxY, node.position.y + height);
  }

  return { x: minX, y: minY, width: maxX - minX, height: maxY - minY };
}
