import type { ConnectionDragMode } from './connectionDragState.ts';
import type { HorseshoeDragSessionState } from './horseshoeDragSession.ts';

export function formatWorkflowHorseshoeSessionTrace(
  session: HorseshoeDragSessionState,
): string {
  return [
    'session',
    session.displayState,
    session.openRequested ? 'requested' : 'idle',
    session.blockedReason ?? 'clear',
    session.anchorPosition ? 'anchor' : 'no-anchor',
  ].join(':');
}

export function formatWorkflowHorseshoeOpenRequestTrace(params: {
  dragActive: boolean;
  connectionMode: ConnectionDragMode;
  hasConnectionIntent: boolean;
  insertableCount: number;
  hasAnchorPosition: boolean;
}): string {
  return [
    'request-open',
    params.dragActive ? 'drag' : 'idle',
    params.connectionMode,
    params.hasConnectionIntent ? 'intent' : 'no-intent',
    `${params.insertableCount}-insertables`,
    params.hasAnchorPosition ? 'anchor' : 'no-anchor',
  ].join(':');
}
