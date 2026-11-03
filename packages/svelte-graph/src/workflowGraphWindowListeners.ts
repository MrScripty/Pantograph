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
  target.addEventListener('dragend', onPaletteDragEnd, true);
  target.addEventListener('drop', onPaletteDragEnd);
  target.addEventListener('blur', onPaletteDragEnd);

  return () => {
    target.removeEventListener('keydown', keyDownListener, true);
    target.removeEventListener(WORKFLOW_PALETTE_DRAG_START_EVENT, onPaletteDragStart);
    target.removeEventListener(WORKFLOW_PALETTE_DRAG_END_EVENT, onPaletteDragEnd);
    target.removeEventListener('dragend', onPaletteDragEnd, true);
    target.removeEventListener('drop', onPaletteDragEnd);
    target.removeEventListener('blur', onPaletteDragEnd);
  };
}

export interface WorkflowGraphMountOptions<Definitions> {
  loadNodeDefinitions(): Promise<Definitions>;
  applyNodeDefinitions(definitions: Definitions): void;
  onFailure(error: unknown): void;
}

/** Register synchronously; the same mount owns the deferred definitions read. */
export function createWorkflowGraphMount<Definitions>(
  target: WorkflowGraphWindowListenerTarget,
  handlers: WorkflowGraphWindowListenerHandlers,
  options: WorkflowGraphMountOptions<Definitions>,
): { stop(): void; ready: Promise<void> } {
  let active = true;
  const removeListeners = registerWorkflowGraphWindowListeners(target, handlers);
  const ready = Promise.resolve().then(async () => {
    if (!active) return;
    try {
      const definitions = await options.loadNodeDefinitions();
      if (active) options.applyNodeDefinitions(definitions);
    } catch (error) {
      if (active) options.onFailure(error);
    }
  });

  return {
    ready,
    stop() {
      if (!active) return;
      active = false;
      removeListeners();
    },
  };
}
