import type { Node } from '@xyflow/svelte';

export interface RuntimeDataCleanupResult {
  changed: boolean;
  data: Record<string, unknown>;
}

export function removeNodeDataKeys(
  data: Record<string, unknown>,
  keys: Iterable<string>
): RuntimeDataCleanupResult {
  const nextData = { ...data };
  let changed = false;

  for (const key of keys) {
    if (!(key in nextData)) continue;
    delete nextData[key];
    changed = true;
  }

  return { changed, data: nextData };
}

export function updateNodeRuntimeDataInNodes<NodeType extends Node>(
  nodes: ReadonlyArray<NodeType>,
  nodeId: string,
  data: Record<string, unknown>,
): NodeType[] {
  return nodes.map((node) =>
    node.id === nodeId ? { ...node, data: { ...node.data, ...data } } : node,
  );
}

export function clearNodeRuntimeDataKeysInNodes<NodeType extends Node>(
  nodes: ReadonlyArray<NodeType>,
  keys: Iterable<string>,
): NodeType[] {
  const runtimeKeys = new Set(keys);
  if (runtimeKeys.size === 0) {
    return [...nodes];
  }

  return nodes.map((node) => {
    const { changed, data } = removeNodeDataKeys(node.data, runtimeKeys);
    return changed ? { ...node, data } : node;
  });
}

export function appendNodeStreamContent<NodeType extends Node>(
  nodes: ReadonlyArray<NodeType>,
  nodeId: string,
  chunk: string,
): NodeType[] {
  return nodes.map((node) =>
    node.id === nodeId
      ? {
          ...node,
          data: {
            ...node.data,
            streamContent: `${node.data.streamContent || ''}${chunk}`,
          },
        }
      : node,
  );
}

export function setNodeStreamContent<NodeType extends Node>(
  nodes: ReadonlyArray<NodeType>,
  nodeId: string,
  content: string,
): NodeType[] {
  return nodes.map((node) =>
    node.id === nodeId ? { ...node, data: { ...node.data, streamContent: content } } : node,
  );
}

export function clearNodeStreamContent<NodeType extends Node>(
  nodes: ReadonlyArray<NodeType>,
): NodeType[] {
  return nodes.map((node) =>
    node.data.streamContent
      ? { ...node, data: { ...node.data, streamContent: '' } }
      : node,
  );
}
