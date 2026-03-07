import {
  resolveHorseshoeOpenRequest,
  type HorseshoeBlockedReason,
  type HorseshoeOpenContext,
} from './horseshoeInvocation.ts';

export type HorseshoeDisplayState = 'hidden' | 'pending' | 'blocked' | 'open';

export interface HorseshoeAnchorPosition {
  x: number;
  y: number;
}

export interface HorseshoeDragSessionState {
  dragActive: boolean;
  openRequested: boolean;
  displayState: HorseshoeDisplayState;
  blockedReason: HorseshoeBlockedReason | null;
  anchorPosition: HorseshoeAnchorPosition | null;
}

export function createHorseshoeDragSessionState(): HorseshoeDragSessionState {
  return {
    dragActive: false,
    openRequested: false,
    displayState: 'hidden',
    blockedReason: null,
    anchorPosition: null,
  };
}

export function startHorseshoeDrag(
  anchorPosition: HorseshoeAnchorPosition | null,
): HorseshoeDragSessionState {
  return {
    dragActive: true,
    openRequested: false,
    displayState: 'hidden',
    blockedReason: null,
    anchorPosition,
  };
}

export function updateHorseshoeAnchor(
  state: HorseshoeDragSessionState,
  anchorPosition: HorseshoeAnchorPosition | null,
): HorseshoeDragSessionState {
  if (!state.dragActive || !anchorPosition) {
    return state;
  }

  return {
    ...state,
    anchorPosition,
  };
}

export function clearHorseshoeDragSession(): HorseshoeDragSessionState {
  return createHorseshoeDragSessionState();
}

function shouldKeepOpenRequested(reason: HorseshoeBlockedReason | null): boolean {
  return reason === 'candidates_pending' || reason === 'missing_anchor_position';
}

function reconcileFromResolution(
  state: HorseshoeDragSessionState,
  context: HorseshoeOpenContext,
): HorseshoeDragSessionState {
  const resolution = resolveHorseshoeOpenRequest({
    ...context,
    connectionDragActive: state.dragActive,
    anchorPosition: state.anchorPosition,
  });

  if (resolution.action === 'open') {
    return {
      ...state,
      openRequested: false,
      displayState: 'open',
      blockedReason: null,
    };
  }

  if (resolution.action === 'queue') {
    return {
      ...state,
      openRequested: true,
      displayState: 'pending',
      blockedReason: resolution.reason,
    };
  }

  return {
    ...state,
    openRequested: shouldKeepOpenRequested(resolution.reason),
    displayState: 'blocked',
    blockedReason: resolution.reason,
  };
}

export function requestHorseshoeDisplay(
  state: HorseshoeDragSessionState,
  context: HorseshoeOpenContext,
): HorseshoeDragSessionState {
  return reconcileFromResolution(
    {
      ...state,
      openRequested: true,
    },
    context,
  );
}

export function syncHorseshoeDisplay(
  state: HorseshoeDragSessionState,
  context: HorseshoeOpenContext,
): HorseshoeDragSessionState {
  if (!state.openRequested) {
    return state;
  }

  return reconcileFromResolution(state, context);
}
