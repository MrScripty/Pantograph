import type { NodeDefinition } from '../services/workflow/types.ts';
import type { WorkflowContainerViewport } from './workflowContainerBoundary.ts';

export interface WorkflowPaletteDragDataTransfer {
  getData(type: string): string;
}

export interface WorkflowPaletteDragEvent {
  dataTransfer?: WorkflowPaletteDragDataTransfer | null;
}

export interface WorkflowPalettePointerPosition {
  x: number;
  y: number;
}

export const WORKFLOW_PALETTE_DROP_NODE_OFFSET: WorkflowPalettePointerPosition = {
  x: 100,
  y: 50,
};

export function isWorkflowPaletteEdgeInsertEnabled(
  currentGraphType: string | null | undefined,
  currentGraphId: string | null | undefined,
): boolean {
  return !(currentGraphType === 'system' && currentGraphId === 'app-architecture');
}

export function readWorkflowPaletteDragDefinition(
  event: WorkflowPaletteDragEvent,
  onParseError?: (error: unknown) => void,
): NodeDefinition | null {
  const data = event.dataTransfer?.getData('application/json');
  if (!data) {
    return null;
  }

  try {
    return JSON.parse(data) as NodeDefinition;
  } catch (error) {
    onParseError?.(error);
    return null;
  }
}

export function resolveWorkflowPaletteDropPosition(params: {
  pointerPosition: WorkflowPalettePointerPosition | null;
  viewport: WorkflowContainerViewport | null;
  nodeOffset?: WorkflowPalettePointerPosition;
}): WorkflowPalettePointerPosition | null {
  if (!params.pointerPosition) {
    return null;
  }

  const viewport = params.viewport ?? { x: 0, y: 0, zoom: 1 };
  const nodeOffset = params.nodeOffset ?? WORKFLOW_PALETTE_DROP_NODE_OFFSET;

  return {
    x: (params.pointerPosition.x - viewport.x) / viewport.zoom - nodeOffset.x,
    y: (params.pointerPosition.y - viewport.y) / viewport.zoom - nodeOffset.y,
  };
}
