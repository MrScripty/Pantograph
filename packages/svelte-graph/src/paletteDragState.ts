export const WORKFLOW_PALETTE_DRAG_START_EVENT = 'pantograph:workflow-palette-drag-start';
export const WORKFLOW_PALETTE_DRAG_END_EVENT = 'pantograph:workflow-palette-drag-end';

export function dispatchWorkflowPaletteDragStart(target: EventTarget): void {
  target.dispatchEvent(new CustomEvent(WORKFLOW_PALETTE_DRAG_START_EVENT));
}

export function dispatchWorkflowPaletteDragEnd(target: EventTarget): void {
  target.dispatchEvent(new CustomEvent(WORKFLOW_PALETTE_DRAG_END_EVENT));
}
