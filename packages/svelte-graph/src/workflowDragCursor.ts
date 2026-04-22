import { updateHorseshoeAnchor, type HorseshoeDragSessionState } from './horseshoeDragSession.ts';
import { shouldUpdateHorseshoeAnchorFromPointer } from './horseshoeInvocation.ts';
import { findNearestVisibleHorseshoeIndex } from './horseshoeSelector.ts';

export interface WorkflowDragCursorPosition {
  x: number;
  y: number;
}

export type WorkflowDragCursorDecision =
  | { type: 'noop' }
  | { type: 'select-index'; selectedIndex: number }
  | { type: 'update-anchor'; session: HorseshoeDragSessionState };

export function resolveWorkflowDragCursorUpdate<T>(params: {
  pointerPosition: WorkflowDragCursorPosition | null;
  session: HorseshoeDragSessionState;
  insertableNodeTypes: T[];
  selectedIndex: number;
}): WorkflowDragCursorDecision {
  if (!params.pointerPosition) {
    return { type: 'noop' };
  }

  if (!shouldUpdateHorseshoeAnchorFromPointer(params.session.displayState)) {
    if (!params.session.anchorPosition) {
      return { type: 'noop' };
    }

    const nextIndex = findNearestVisibleHorseshoeIndex(
      params.insertableNodeTypes,
      params.selectedIndex,
      params.pointerPosition,
      params.session.anchorPosition,
    );

    return nextIndex === null
      ? { type: 'noop' }
      : { type: 'select-index', selectedIndex: nextIndex };
  }

  return {
    type: 'update-anchor',
    session: updateHorseshoeAnchor(params.session, params.pointerPosition),
  };
}
