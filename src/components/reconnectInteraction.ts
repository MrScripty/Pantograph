import type { ConnectionAnchor } from '../services/workflow/types';

export const RECONNECT_ANCHOR_INSET_PX = 0;

type ReconnectHandleType = 'source' | 'target';

type ReconnectSourceEdge = {
  source: string;
  sourceHandle: string | null | undefined;
};

type ReconnectableEdge = ReconnectSourceEdge & {
  sourceX: number;
  sourceY: number;
  targetX: number;
  targetY: number;
};

export function resolveReconnectSourceAnchor(
  edge: ReconnectSourceEdge,
  handleType: ReconnectHandleType,
): ConnectionAnchor | null {
  if ((handleType === 'source' || handleType === 'target') && edge.sourceHandle) {
    return {
      node_id: edge.source,
      port_id: edge.sourceHandle,
    };
  }

  return null;
}

export function insetReconnectAnchorPosition(
  edge: ReconnectableEdge,
  handleType: ReconnectHandleType,
  insetPx: number = RECONNECT_ANCHOR_INSET_PX,
) {
  const fromX = handleType === 'source' ? edge.sourceX : edge.targetX;
  const fromY = handleType === 'source' ? edge.sourceY : edge.targetY;
  const toX = handleType === 'source' ? edge.targetX : edge.sourceX;
  const toY = handleType === 'source' ? edge.targetY : edge.sourceY;

  const deltaX = toX - fromX;
  const deltaY = toY - fromY;
  const edgeLength = Math.hypot(deltaX, deltaY);

  if (edgeLength === 0) {
    return { x: fromX, y: fromY };
  }

  const appliedInset = Math.min(insetPx, edgeLength / 2);
  const scale = appliedInset / edgeLength;

  return {
    x: fromX + deltaX * scale,
    y: fromY + deltaY * scale,
  };
}
