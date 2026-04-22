import {
  clearConnectionDragState,
  type ConnectionDragState,
} from './connectionDragState.ts';
import {
  clearHorseshoeDragSession,
  type HorseshoeDragSessionState,
} from './horseshoeDragSession.ts';
import {
  clearHorseshoeInsertFeedback,
  type HorseshoeInsertFeedbackState,
} from './horseshoeInsertFeedback.ts';

export interface WorkflowConnectionDragInteractionState {
  connectionDragState: ConnectionDragState;
  horseshoeSession: HorseshoeDragSessionState;
  feedback: HorseshoeInsertFeedbackState;
}

export function clearWorkflowConnectionDragInteraction(): WorkflowConnectionDragInteractionState {
  return {
    connectionDragState: clearConnectionDragState(),
    horseshoeSession: clearHorseshoeDragSession(),
    feedback: clearHorseshoeInsertFeedback(),
  };
}

export function shouldClearWorkflowConnectionInteractionAfterConnectEnd(params: {
  session: HorseshoeDragSessionState;
  feedback: HorseshoeInsertFeedbackState;
}): boolean {
  return (
    params.session.displayState !== 'open' &&
    !params.feedback.pending &&
    !params.session.openRequested
  );
}
