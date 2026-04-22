export type WorkflowContainerKeyboardAction =
  | { type: 'noop' }
  | { type: 'deselect-container'; preventDefault: true }
  | { type: 'zoom-to-orchestration'; preventDefault: true };

export function resolveWorkflowContainerKeyboardAction(params: {
  key: string;
  containerSelected: boolean;
}): WorkflowContainerKeyboardAction {
  if (!params.containerSelected) {
    return { type: 'noop' };
  }

  if (params.key === 'Tab') {
    return { type: 'zoom-to-orchestration', preventDefault: true };
  }

  if (params.key === 'Escape') {
    return { type: 'deselect-container', preventDefault: true };
  }

  return { type: 'noop' };
}

export function resolveWorkflowContainerSelectionAfterGraphSelection(params: {
  containerSelected: boolean;
  selectedNodeCount: number;
}): boolean {
  return params.selectedNodeCount > 0 ? false : params.containerSelected;
}

export function toggleWorkflowContainerSelection(containerSelected: boolean): boolean {
  return !containerSelected;
}

export function clearWorkflowContainerSelection(): false {
  return false;
}
