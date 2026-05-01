import type { NodeDefinition } from './types/workflow.js';

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

export interface WorkflowPaletteContainerBounds {
  left: number;
  top: number;
}

export const WORKFLOW_PALETTE_DROP_NODE_OFFSET: WorkflowPalettePointerPosition = {
  x: 100,
  y: 50,
};
export const WORKFLOW_PALETTE_DRAG_DATA_TYPES = [
  'application/json',
  'text/plain',
] as const;

let activeWorkflowPaletteDefinition: NodeDefinition | null = null;

export function setActiveWorkflowPaletteDragDefinition(definition: NodeDefinition): void {
  activeWorkflowPaletteDefinition = definition;
}

export function clearActiveWorkflowPaletteDragDefinition(): void {
  activeWorkflowPaletteDefinition = null;
}

export function readWorkflowPaletteDragDefinition(
  event: WorkflowPaletteDragEvent,
  onParseError?: (error: unknown) => void,
): NodeDefinition | null {
  const data = WORKFLOW_PALETTE_DRAG_DATA_TYPES
    .map((type) => event.dataTransfer?.getData(type) ?? '')
    .find((value) => value.length > 0);
  if (!data) {
    return activeWorkflowPaletteDefinition;
  }

  try {
    return JSON.parse(data) as NodeDefinition;
  } catch (error) {
    onParseError?.(error);
    return null;
  }
}

export function resolveWorkflowPaletteDropPosition(params: {
  clientPosition: WorkflowPalettePointerPosition;
  containerBounds: WorkflowPaletteContainerBounds;
  nodeOffset?: WorkflowPalettePointerPosition;
}): WorkflowPalettePointerPosition {
  const nodeOffset = params.nodeOffset ?? WORKFLOW_PALETTE_DROP_NODE_OFFSET;

  return {
    x: params.clientPosition.x - params.containerBounds.left - nodeOffset.x,
    y: params.clientPosition.y - params.containerBounds.top - nodeOffset.y,
  };
}
