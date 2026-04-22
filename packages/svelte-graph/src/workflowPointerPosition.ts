export interface WorkflowPointerPosition {
  x: number;
  y: number;
}

export interface WorkflowPointerClientPosition {
  clientX: number;
  clientY: number;
}

export interface WorkflowPointerBounds {
  left: number;
  top: number;
}

export interface WorkflowPointerTouchListLike {
  readonly length: number;
  readonly [index: number]: WorkflowPointerClientPosition | undefined;
}

export type WorkflowPointerEventLike =
  | WorkflowPointerClientPosition
  | {
      touches: WorkflowPointerTouchListLike;
      changedTouches: WorkflowPointerTouchListLike;
    };

function isWorkflowTouchEventLike(
  event: WorkflowPointerEventLike,
): event is Extract<WorkflowPointerEventLike, { touches: WorkflowPointerTouchListLike }> {
  return 'touches' in event;
}

export function resolveWorkflowPointerClientPosition(
  event: WorkflowPointerEventLike,
): WorkflowPointerClientPosition | null {
  if (isWorkflowTouchEventLike(event)) {
    return event.touches[0] ?? event.changedTouches[0] ?? null;
  }

  return event;
}

export function resolveWorkflowRelativePointerPosition(params: {
  clientPosition: WorkflowPointerClientPosition | null;
  containerBounds: WorkflowPointerBounds | null;
}): WorkflowPointerPosition | null {
  if (!params.clientPosition || !params.containerBounds) {
    return null;
  }

  return {
    x: params.clientPosition.clientX - params.containerBounds.left,
    y: params.clientPosition.clientY - params.containerBounds.top,
  };
}
