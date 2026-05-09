import type { Node } from '@xyflow/svelte';

export type NodeRuntimeOverlayMap = Map<string, Record<string, unknown>>;

const STRUCTURAL_RUNTIME_DATA_KEYS = [
  'audio',
  'audio_mime',
  'streamContent',
  'stream_sequence',
  'stream_is_final',
];

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

export function stripStructuralRuntimeNodeData(
  data: Record<string, unknown>,
): Record<string, unknown> {
  return removeNodeDataKeys(data, STRUCTURAL_RUNTIME_DATA_KEYS).data;
}

export function mergeNodeRuntimeOverlays<NodeType extends Node>(
  nodes: ReadonlyArray<NodeType>,
  overlays: NodeRuntimeOverlayMap,
): NodeType[] {
  return nodes.map((node) => {
    const overlay = overlays.get(node.id);
    return overlay ? { ...node, data: { ...node.data, ...overlay } } : node;
  });
}

export function updateNodeRuntimeOverlay(
  overlays: NodeRuntimeOverlayMap,
  nodeId: string,
  data: Record<string, unknown>,
): NodeRuntimeOverlayMap {
  const nextOverlays = new Map(overlays);
  nextOverlays.set(nodeId, {
    ...(nextOverlays.get(nodeId) ?? {}),
    ...data,
  });
  return nextOverlays;
}

export function clearNodeRuntimeOverlayKeys(
  overlays: NodeRuntimeOverlayMap,
  keys: Iterable<string>,
): NodeRuntimeOverlayMap {
  const runtimeKeys = new Set(keys);
  if (runtimeKeys.size === 0) {
    return new Map(overlays);
  }

  const nextOverlays = new Map<string, Record<string, unknown>>();
  for (const [nodeId, overlay] of overlays.entries()) {
    const { changed, data } = removeNodeDataKeys(overlay, runtimeKeys);
    nextOverlays.set(nodeId, changed ? data : overlay);
  }
  return nextOverlays;
}

export function appendNodeStreamContentOverlay(
  overlays: NodeRuntimeOverlayMap,
  nodeId: string,
  chunk: string,
  sequence?: number | null,
): NodeRuntimeOverlayMap {
  const currentOverlay = overlays.get(nodeId) ?? {};
  if (isStaleStreamSequence(currentOverlay, sequence)) {
    return overlays;
  }
  return updateNodeRuntimeOverlay(overlays, nodeId, {
    streamContent: `${currentOverlay.streamContent || ''}${chunk}`,
    ...(sequence === null || typeof sequence === 'undefined'
      ? {}
      : { stream_sequence: sequence }),
  });
}

export function setNodeStreamContentOverlay(
  overlays: NodeRuntimeOverlayMap,
  nodeId: string,
  content: string,
  sequence?: number | null,
): NodeRuntimeOverlayMap {
  const currentOverlay = overlays.get(nodeId) ?? {};
  if (isStaleStreamSequence(currentOverlay, sequence)) {
    return overlays;
  }
  return updateNodeRuntimeOverlay(overlays, nodeId, {
    streamContent: content,
    ...(sequence === null || typeof sequence === 'undefined'
      ? {}
      : { stream_sequence: sequence }),
  });
}

export function clearNodeStreamContentOverlay(
  overlays: NodeRuntimeOverlayMap,
): NodeRuntimeOverlayMap {
  const nextOverlays = new Map<string, Record<string, unknown>>();
  for (const [nodeId, overlay] of overlays.entries()) {
    nextOverlays.set(
      nodeId,
      'streamContent' in overlay ? { ...overlay, streamContent: '' } : overlay,
    );
  }
  return nextOverlays;
}

function isStaleStreamSequence(
  overlay: Record<string, unknown>,
  nextSequence?: number | null,
): boolean {
  if (nextSequence === null || typeof nextSequence === 'undefined') {
    return false;
  }
  const currentSequence = overlay.stream_sequence;
  return (
    typeof currentSequence === 'number' &&
    Number.isFinite(currentSequence) &&
    nextSequence <= currentSequence
  );
}
