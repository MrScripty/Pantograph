import { clearHorseshoeInsertFeedback, type HorseshoeInsertFeedbackState } from './horseshoeInsertFeedback.ts';
import type { HorseshoeDragSessionState } from './horseshoeDragSession.ts';
import { formatWorkflowHorseshoeSessionTrace } from './workflowHorseshoeTrace.ts';

export interface WorkflowHorseshoeSessionViewState {
  session: HorseshoeDragSessionState;
  feedback: HorseshoeInsertFeedbackState;
  selectedIndex: number;
  query: string;
}

export interface WorkflowHorseshoeSessionUpdate {
  changed: boolean;
  state: WorkflowHorseshoeSessionViewState;
  trace: string;
  clearQueryResetTimer: boolean;
}

export function resolveWorkflowHorseshoeSessionUpdate(params: {
  current: WorkflowHorseshoeSessionViewState;
  nextSession: HorseshoeDragSessionState;
}): WorkflowHorseshoeSessionUpdate {
  if (params.nextSession === params.current.session) {
    return {
      changed: false,
      state: params.current,
      trace: formatWorkflowHorseshoeSessionTrace(params.current.session),
      clearQueryResetTimer: false,
    };
  }

  const previousDisplayState = params.current.session.displayState;
  const nextState: WorkflowHorseshoeSessionViewState = {
    ...params.current,
    session: params.nextSession,
  };
  let clearQueryResetTimer = false;

  if (params.nextSession.displayState === 'open' && previousDisplayState !== 'open') {
    nextState.query = '';
    nextState.selectedIndex = 0;
  }

  if (params.nextSession.displayState === 'hidden') {
    nextState.feedback = clearHorseshoeInsertFeedback();
    nextState.selectedIndex = 0;
    nextState.query = '';
    clearQueryResetTimer = true;
  }

  return {
    changed: true,
    state: nextState,
    trace: formatWorkflowHorseshoeSessionTrace(params.nextSession),
    clearQueryResetTimer,
  };
}
