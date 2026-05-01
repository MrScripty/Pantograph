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
  onFailure?: (message: string) => void;
  onAddNode: (
    definition: NodeDefinition,
    position: { x: number; y: number },
  ) => Promise<unknown> | unknown;
  refreshEdgeInsertPreview: (event: DragEvent, definition: NodeDefinition) => Promise<void>;
}

function readPaletteDefinition(event: DragEvent): NodeDefinition | null {
  return readWorkflowPaletteDragDefinition(event, (error) => {
    console.warn('[WorkflowGraph] Failed to parse palette drag data:', error);
  });
}

function warnIfAddNodeDidNotApply(result: unknown, onFailure?: (message: string) => void): void {
  if (!result || typeof result !== 'object' || !('status' in result)) {
    return;
  }

  const status = result.status;
  if (status === 'applied') {
    return;
  }

  const message = `Palette drop did not add a node: mutation ${String(status)}.`;
  console.warn('[WorkflowGraph]', message, result);
  onFailure?.(message);
}

export async function handleWorkflowGraphPaletteDrop({
  canEdit,
  clearConnectionInteraction,
  commitEdgeInsertDrop,
  currentViewport,
  edgeInsertPreview,
  event,
  getRelativePointerPosition,
  onFailure,
  onAddNode,
}: WorkflowGraphPaletteHandlerParams) {
  event.preventDefault();
  if (!canEdit) {
    onFailure?.('Palette drop ignored because this graph is not editable.');
    return;
  }

  const definition = readPaletteDefinition(event);
  if (!definition) {
    clearConnectionInteraction();
    onFailure?.('Palette drop did not include a readable node definition.');
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
    onFailure?.('Palette drop did not resolve a graph position.');
    return;
  }

  if (activeEdgeInsertPreview) {
    await commitEdgeInsertDrop(definition, position, activeEdgeInsertPreview);
    return;
  }

  warnIfAddNodeDidNotApply(await onAddNode(definition, position), onFailure);
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

  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy';
  }

  const definition = readPaletteDefinition(event);
  if (!definition) {
    clearEdgeInsertPreview();
    return;
  }

  await refreshEdgeInsertPreview(event, definition);
}
