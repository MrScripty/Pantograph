import type { ConnectionAnchor } from './types/workflow.ts';

export type ConnectionDragMode = 'idle' | 'connect' | 'reconnect';

export interface ConnectionDragState {
  mode: ConnectionDragMode;
  reconnectingEdgeId: string | null;
  reconnectingSourceAnchor: ConnectionAnchor | null;
  finalizing: boolean;
}

export function createConnectionDragState(): ConnectionDragState {
  return {
    mode: 'idle',
    reconnectingEdgeId: null,
    reconnectingSourceAnchor: null,
    finalizing: false,
  };
}

export function startConnectionDrag(): ConnectionDragState {
  return {
    mode: 'connect',
    reconnectingEdgeId: null,
    reconnectingSourceAnchor: null,
    finalizing: false,
  };
}

export function startReconnectDrag(
  reconnectingEdgeId: string,
  reconnectingSourceAnchor: ConnectionAnchor,
): ConnectionDragState {
  return {
    mode: 'reconnect',
    reconnectingEdgeId,
    reconnectingSourceAnchor,
    finalizing: false,
  };
}

export function markConnectionDragFinalizing(state: ConnectionDragState): ConnectionDragState {
  if (state.mode === 'idle' || state.finalizing) {
    return state;
  }

  return {
    ...state,
    finalizing: true,
  };
}

export function clearConnectionDragState(): ConnectionDragState {
  return createConnectionDragState();
}

export function supportsInsertFromConnectionDrag(state: ConnectionDragState): boolean {
  return state.mode !== 'reconnect';
}

export function shouldRemoveReconnectedEdge(
  state: ConnectionDragState,
  connectionState: { isValid: boolean },
): string | null {
  if (state.mode !== 'reconnect' || state.finalizing || connectionState.isValid) {
    return null;
  }

  return state.reconnectingEdgeId;
}
