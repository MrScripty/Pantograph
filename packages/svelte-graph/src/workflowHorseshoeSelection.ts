import type { HorseshoeDragSessionState } from './horseshoeDragSession.ts';
import type { HorseshoeInsertFeedbackState } from './horseshoeInsertFeedback.ts';
import { clampHorseshoeIndex } from './horseshoeSelector.ts';
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
