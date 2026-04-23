import type { NodeDefinition } from '../services/workflow/types';
import type { WorkflowContainerViewport } from './workflowContainerBoundary';
import {
  getCommittableEdgeInsertPreview,
  type EdgeInsertPreviewState,
} from './edgeInsertInteraction';
import {
  readWorkflowPaletteDragDefinition,
  resolveWorkflowPaletteDropPosition,
} from './workflowPaletteDrag';

interface WorkflowGraphPaletteHandlerParams {
  canEdit: boolean;
  clearConnectionInteraction: () => void;
  clearEdgeInsertPreview: () => void;
  commitEdgeInsertDrop: (
    definition: NodeDefinition,
    position: { x: number; y: number },
    preview: EdgeInsertPreviewState,
  ) => Promise<boolean>;
  currentViewport: WorkflowContainerViewport | null;
  edgeInsertPreview: EdgeInsertPreviewState;
  event: DragEvent;
  getRelativePointerPosition: (clientX: number, clientY: number) => { x: number; y: number } | null;
  onAddNode: (definition: NodeDefinition, position: { x: number; y: number }) => void;
  refreshEdgeInsertPreview: (event: DragEvent, definition: NodeDefinition) => Promise<void>;
}

function readPaletteDefinition(event: DragEvent): NodeDefinition | null {
  return readWorkflowPaletteDragDefinition(event, (error) => {
    console.warn('[WorkflowGraph] Failed to parse palette drag data:', error);
  });
}

export async function handleWorkflowGraphPaletteDrop({
  canEdit,
  clearConnectionInteraction,
  commitEdgeInsertDrop,
  currentViewport,
  edgeInsertPreview,
  event,
  getRelativePointerPosition,
  onAddNode,
}: WorkflowGraphPaletteHandlerParams) {
  event.preventDefault();
  if (!canEdit) {
    return;
  }

  const definition = readPaletteDefinition(event);
  if (!definition) {
    clearConnectionInteraction();
    return;
  }

  const position = resolveWorkflowPaletteDropPosition({
    pointerPosition: getRelativePointerPosition(event.clientX, event.clientY),
    viewport: currentViewport,
  });
  const activeEdgeInsertPreview = getCommittableEdgeInsertPreview(
    edgeInsertPreview,
    definition.node_type,
  );

  clearConnectionInteraction();
  if (!position) {
    return;
  }

  if (activeEdgeInsertPreview) {
    await commitEdgeInsertDrop(definition, position, activeEdgeInsertPreview);
    return;
  }

  onAddNode(definition, position);
}

export async function handleWorkflowGraphPaletteDragOver({
  canEdit,
  clearEdgeInsertPreview,
  event,
  refreshEdgeInsertPreview,
}: WorkflowGraphPaletteHandlerParams) {
  event.preventDefault();
  if (!canEdit) {
    return;
  }

  event.dataTransfer!.dropEffect = 'copy';
  const definition = readPaletteDefinition(event);
  if (!definition) {
    clearEdgeInsertPreview();
    return;
  }

  await refreshEdgeInsertPreview(event, definition);
}
