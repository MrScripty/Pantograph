import type { Component } from 'svelte';
import type { NodeDefinition } from '../types/workflow.js';
import type { NodeTypeRegistry } from '../types/registry.js';
import GenericNode from '../components/nodes/GenericNode.svelte';
import ReconnectableEdge from '../components/edges/ReconnectableEdge.svelte';

/**
 * Build a NodeTypeRegistry from engine-provided definitions.
 *
 * Maps each NodeDefinition to either a specialized component (if provided
 * in the overrides map) or the GenericNode fallback. Consumers can add
 * non-engine node types (e.g., architecture nodes) on top.
 */
export function buildRegistry(
  definitions: NodeDefinition[],
  specializedNodes?: Record<string, Component<any>>,
): NodeTypeRegistry {
  const nodeTypes: Record<string, Component<any>> = {};
  for (const def of definitions) {
    nodeTypes[def.node_type] = specializedNodes?.[def.node_type] ?? GenericNode;
  }
  return {
    nodeTypes,
    fallbackNode: GenericNode,
    edgeTypes: { reconnectable: ReconnectableEdge },
  };
}
