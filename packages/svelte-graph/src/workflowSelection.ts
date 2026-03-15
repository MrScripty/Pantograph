import type { Node } from '@xyflow/svelte';

export function collectSelectedNodeIds(nodes: ReadonlyArray<Pick<Node, 'id' | 'selected'>>): string[] {
  return nodes.filter((node) => node.selected === true).map((node) => node.id);
}

export function applySelectedNodeIds<NodeType extends Node>(
  nodes: ReadonlyArray<NodeType>,
  selectedNodeIds: ReadonlyArray<string>,
): NodeType[] {
  const selectedSet = new Set(selectedNodeIds);

  return nodes.map((node) => {
    const selected = selectedSet.has(node.id);

    if (node.selected === selected) {
      return node;
    }

    return {
      ...node,
      selected,
    };
  });
}
