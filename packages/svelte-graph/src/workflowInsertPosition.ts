import type { InsertNodePositionHint } from './types/workflow.js';

export interface WorkflowInsertAnchorPosition {
  x: number;
  y: number;
}

export interface WorkflowInsertViewport {
  x: number;
  y: number;
  zoom: number;
}

export function resolveWorkflowInsertPositionHint(params: {
  anchorPosition: WorkflowInsertAnchorPosition | null;
  viewport: WorkflowInsertViewport | null;
}): InsertNodePositionHint | null {
  if (!params.anchorPosition) {
    return null;
  }

  const viewport = params.viewport ?? { x: 0, y: 0, zoom: 1 };

  return {
    position: {
      x: (params.anchorPosition.x - viewport.x) / viewport.zoom,
      y: (params.anchorPosition.y - viewport.y) / viewport.zoom,
    },
  };
}
