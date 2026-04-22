import {
  supportsInsertFromConnectionDrag,
  type ConnectionDragState,
} from './connectionDragState.ts';
import type { HorseshoeDragSessionState } from './horseshoeDragSession.ts';
import type { HorseshoeOpenContext } from './horseshoeInvocation.ts';

export interface WorkflowHorseshoeOpenContextInput {
  canEdit: boolean;
  session: HorseshoeDragSessionState;
  connectionDragState: ConnectionDragState;
  hasConnectionIntent: boolean;
  insertableCount: number;
}

export function buildWorkflowHorseshoeOpenContext(
  params: WorkflowHorseshoeOpenContextInput,
): HorseshoeOpenContext {
  return {
    canEdit: params.canEdit,
    connectionDragActive: params.session.dragActive,
    supportsInsert: supportsInsertFromConnectionDrag(params.connectionDragState),
    hasConnectionIntent: params.hasConnectionIntent,
    insertableCount: params.insertableCount,
    anchorPosition: params.session.anchorPosition,
  };
}
