import type { HorseshoeDragSessionState } from './horseshoeDragSession.ts';
import type { HorseshoeInsertFeedbackState } from './horseshoeInsertFeedback.ts';
import {
  clampHorseshoeIndex,
  findBestInsertableMatchIndex,
  rotateHorseshoeIndex,
} from './horseshoeSelector.ts';
import type { InsertableNodeTypeCandidate } from './types/workflow.ts';
import type { HorseshoeKeyboardContext } from './workflowHorseshoeKeyboard.ts';

export interface WorkflowHorseshoeSelectionSnapshot<TCandidate> {
  keyboardContext: HorseshoeKeyboardContext;
  selectedCandidate: TCandidate | null;
}

export function resolveWorkflowHorseshoeSelectionSnapshot<TCandidate>(params: {
  session: HorseshoeDragSessionState;
  feedback: HorseshoeInsertFeedbackState;
  items: readonly TCandidate[] | null | undefined;
  selectedIndex: number;
}): WorkflowHorseshoeSelectionSnapshot<TCandidate> {
  const selectedCandidate = params.items?.[params.selectedIndex] ?? null;

  return {
    keyboardContext: {
      displayState: params.session.displayState,
      dragActive: params.session.dragActive,
      pending: params.feedback.pending,
      hasSelection: selectedCandidate !== null,
    },
    selectedCandidate,
  };
}

export function normalizeWorkflowHorseshoeSelectedIndex(params: {
  selectedIndex: number;
  itemCount: number;
}): number {
  return clampHorseshoeIndex(params.selectedIndex, params.itemCount);
}

export function rotateWorkflowHorseshoeSelection(params: {
  selectedIndex: number;
  delta: number;
  itemCount: number;
}): number | null {
  if (params.itemCount <= 0) {
    return null;
  }

  return rotateHorseshoeIndex(params.selectedIndex, params.delta, params.itemCount);
}

export interface WorkflowHorseshoeQueryUpdate {
  query: string;
  selectedIndex: number;
  resetTimerAction: 'schedule' | 'clear';
}

export function resolveWorkflowHorseshoeQueryUpdate(params: {
  items: readonly InsertableNodeTypeCandidate[] | null | undefined;
  query: string;
  selectedIndex: number;
}): WorkflowHorseshoeQueryUpdate {
  const items = params.items ? [...params.items] : [];

  return {
    query: params.query,
    selectedIndex: findBestInsertableMatchIndex(items, params.query, params.selectedIndex),
    resetTimerAction: params.query ? 'schedule' : 'clear',
  };
}
