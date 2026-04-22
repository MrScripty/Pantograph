import type { ConnectionDragState } from './connectionDragState.ts';
import {
  requestHorseshoeDisplay,
  type HorseshoeDragSessionState,
} from './horseshoeDragSession.ts';
import type { HorseshoeOpenContext } from './horseshoeInvocation.ts';
import { formatWorkflowHorseshoeOpenRequestTrace } from './workflowHorseshoeTrace.ts';

export interface WorkflowHorseshoeOpenRequestResult {
  trace: string;
  session: HorseshoeDragSessionState;
}

export function requestWorkflowHorseshoeOpen(params: {
  session: HorseshoeDragSessionState;
  connectionDragState: ConnectionDragState;
  openContext: HorseshoeOpenContext;
}): WorkflowHorseshoeOpenRequestResult {
  return {
    trace: formatWorkflowHorseshoeOpenRequestTrace({
      dragActive: params.session.dragActive,
      connectionMode: params.connectionDragState.mode,
      hasConnectionIntent: params.openContext.hasConnectionIntent,
      insertableCount: params.openContext.insertableCount,
      hasAnchorPosition: Boolean(params.session.anchorPosition),
    }),
    session: requestHorseshoeDisplay(params.session, params.openContext),
  };
}
