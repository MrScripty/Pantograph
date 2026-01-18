import type { ArchitectureGraph, ArchNodeCategory, ArchNodeDefinition } from './types';

export interface NodePositions {
  [nodeId: string]: { x: number; y: number };
}

// Layer order for horizontal layout (left to right)
const LAYER_ORDER: ArchNodeCategory[] = [
  'component',
  'service',
  'store',
  'command',
  'backend'
];

const LAYER_SPACING = 350;  // Horizontal spacing between layers
const NODE_SPACING = 120;   // Vertical spacing between nodes

/**
 * Calculate positions for architecture nodes in a layered layout.
 * Components on the left, flowing through services/stores to backend on the right.
 */
export function layoutArchitecture(graph: ArchitectureGraph): NodePositions {
  const positions: NodePositions = {};

  // Group nodes by category
  const nodesByCategory = new Map<ArchNodeCategory, ArchNodeDefinition[]>();

  for (const category of LAYER_ORDER) {
    nodesByCategory.set(category, []);
  }

  for (const node of graph.nodes) {
    const categoryNodes = nodesByCategory.get(node.category);
    if (categoryNodes) {
      categoryNodes.push(node);
    }
  }

  // Position each layer
  LAYER_ORDER.forEach((category, layerIndex) => {
    const nodes = nodesByCategory.get(category) || [];
    const layerX = layerIndex * LAYER_SPACING;

    // Center the nodes vertically
    const totalHeight = (nodes.length - 1) * NODE_SPACING;
    const startY = -totalHeight / 2;

    nodes.forEach((node, nodeIndex) => {
      positions[node.id] = {
        x: layerX,
        y: startY + nodeIndex * NODE_SPACING
      };
    });
  });

  return positions;
}

/**
 * Get the layer index for a node category (used for sorting/filtering).
 */
export function getCategoryLayer(category: ArchNodeCategory): number {
  return LAYER_ORDER.indexOf(category);
}
