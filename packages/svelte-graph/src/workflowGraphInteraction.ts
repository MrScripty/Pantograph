export interface WorkflowGraphInteractionInput {
  canEdit: boolean;
  ctrlPressed: boolean;
  externalPaletteDragActive: boolean;
}

export interface WorkflowGraphInteractionState {
  deleteKey: 'Delete' | null;
  edgesReconnectable: boolean;
  elementsSelectable: boolean;
  nodesConnectable: boolean;
  nodesDraggable: boolean;
  panOnDrag: boolean;
}

export function resolveWorkflowGraphInteractionState({
  canEdit,
  ctrlPressed,
  externalPaletteDragActive,
}: WorkflowGraphInteractionInput): WorkflowGraphInteractionState {
  const paletteDragInactive = !externalPaletteDragActive;

  return {
    deleteKey: canEdit ? 'Delete' : null,
    edgesReconnectable: canEdit && paletteDragInactive,
    elementsSelectable: paletteDragInactive,
    nodesConnectable: canEdit && paletteDragInactive,
    nodesDraggable: canEdit && paletteDragInactive,
    panOnDrag: !ctrlPressed && paletteDragInactive,
  };
}
