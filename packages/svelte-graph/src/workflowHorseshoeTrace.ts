import type { ConnectionDragMode } from './connectionDragState.ts';
import {
  formatHorseshoeBlockedReason,
  type HorseshoeBlockedReason,
} from './horseshoeInvocation.ts';
import type { HorseshoeDragSessionState } from './horseshoeDragSession.ts';

export interface WorkflowHorseshoeBlockedReasonLogDecision {
  message: string | null;
  nextLoggedBlockedReason: HorseshoeBlockedReason | null;
  shouldLog: boolean;
}

export function resolveWorkflowHorseshoeBlockedReasonLog(params: {
  blockedReason: HorseshoeBlockedReason | null;
  lastLoggedBlockedReason: HorseshoeBlockedReason | null;
}): WorkflowHorseshoeBlockedReasonLogDecision {
  if (!params.blockedReason || params.blockedReason === params.lastLoggedBlockedReason) {
    return {
      message: null,
      nextLoggedBlockedReason: params.lastLoggedBlockedReason,
      shouldLog: false,
    };
  }

  return {
    message: formatHorseshoeBlockedReason(params.blockedReason),
    nextLoggedBlockedReason: params.blockedReason,
    shouldLog: true,
  };
}

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
