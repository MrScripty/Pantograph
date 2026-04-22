import {
  WORKFLOW_PALETTE_DRAG_END_EVENT,
  WORKFLOW_PALETTE_DRAG_START_EVENT,
} from './paletteDragState.ts';

export interface WorkflowGraphWindowListenerTarget {
  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | AddEventListenerOptions,
  ): void;
  removeEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | EventListenerOptions,
  ): void;
}

export interface WorkflowGraphWindowListenerHandlers {
  onKeyDown: (event: KeyboardEvent) => void;
  onPaletteDragEnd: EventListener;
  onPaletteDragStart: EventListener;
}

export function registerWorkflowGraphWindowListeners(
  target: WorkflowGraphWindowListenerTarget,
  { onKeyDown, onPaletteDragEnd, onPaletteDragStart }: WorkflowGraphWindowListenerHandlers,
): () => void {
  const keyDownListener = onKeyDown as EventListener;

  target.addEventListener('keydown', keyDownListener, true);
  target.addEventListener(WORKFLOW_PALETTE_DRAG_START_EVENT, onPaletteDragStart);
  target.addEventListener(WORKFLOW_PALETTE_DRAG_END_EVENT, onPaletteDragEnd);
  target.addEventListener('blur', onPaletteDragEnd);

  return () => {
    target.removeEventListener('keydown', keyDownListener, true);
    target.removeEventListener(WORKFLOW_PALETTE_DRAG_START_EVENT, onPaletteDragStart);
    target.removeEventListener(WORKFLOW_PALETTE_DRAG_END_EVENT, onPaletteDragEnd);
    target.removeEventListener('blur', onPaletteDragEnd);
  };
}
