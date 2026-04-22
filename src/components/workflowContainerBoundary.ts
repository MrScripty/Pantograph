export interface WorkflowContainerBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface WorkflowContainerViewport {
  x: number;
  y: number;
  zoom: number;
}

interface WorkflowContainerNodeLike {
  position: {
    x: number;
    y: number;
  };
  measured?: {
    width?: number | null;
    height?: number | null;
  } | null;
  width?: number | null;
  height?: number | null;
}

export const WORKFLOW_CONTAINER_MARGIN = 100;
export const WORKFLOW_CONTAINER_VISIBILITY_MARGIN = 50;

export function resolveWorkflowContainerBounds(
  nodes: WorkflowContainerNodeLike[],
  margin = WORKFLOW_CONTAINER_MARGIN
): WorkflowContainerBounds | null {
  if (nodes.length === 0) return null;

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const node of nodes) {
    const width = node.measured?.width || node.width || 200;
    const height = node.measured?.height || node.height || 100;

    minX = Math.min(minX, node.position.x);
    minY = Math.min(minY, node.position.y);
    maxX = Math.max(maxX, node.position.x + width);
    maxY = Math.max(maxY, node.position.y + height);
  }

  return {
    x: minX - margin,
    y: minY - margin,
    width: maxX - minX + margin * 2,
    height: maxY - minY + margin * 2,
  };
}

export function isWorkflowContainerFullyVisible(
  bounds: WorkflowContainerBounds,
  viewport: WorkflowContainerViewport,
  screenWidth: number,
  screenHeight: number,
  visibilityMargin = WORKFLOW_CONTAINER_VISIBILITY_MARGIN
): boolean {
  const screenX = bounds.x * viewport.zoom + viewport.x;
  const screenY = bounds.y * viewport.zoom + viewport.y;
  const screenW = bounds.width * viewport.zoom;
  const screenH = bounds.height * viewport.zoom;

  return (
    screenX >= visibilityMargin &&
    screenY >= visibilityMargin &&
    screenX + screenW <= screenWidth - visibilityMargin &&
    screenY + screenH <= screenHeight - visibilityMargin
  );
}
