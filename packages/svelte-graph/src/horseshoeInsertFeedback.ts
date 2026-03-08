import { formatHorseshoeBlockedReason, type HorseshoeBlockedReason } from './horseshoeInvocation.ts';
import type { HorseshoeDisplayState } from './horseshoeDragSession.ts';

export interface HorseshoeInsertFeedbackState {
  pending: boolean;
  rejectionMessage: string | null;
}

export interface HorseshoeStatusContext {
  pending: boolean;
  rejectionMessage: string | null;
  displayState: HorseshoeDisplayState;
  blockedReason: HorseshoeBlockedReason | null;
}

interface InsertRejectionLike {
  message: string;
}

export function createHorseshoeInsertFeedbackState(): HorseshoeInsertFeedbackState {
  return {
    pending: false,
    rejectionMessage: null,
  };
}

export function clearHorseshoeInsertFeedback(): HorseshoeInsertFeedbackState {
  return createHorseshoeInsertFeedbackState();
}

export function startHorseshoeInsertFeedback(): HorseshoeInsertFeedbackState {
  return {
    pending: true,
    rejectionMessage: null,
  };
}

export function rejectHorseshoeInsertFeedback(
  rejection?: InsertRejectionLike,
): HorseshoeInsertFeedbackState {
  return {
    pending: false,
    rejectionMessage: rejection?.message ?? 'Insert failed. Try again.',
  };
}

export function resolveHorseshoeStatusLabel(context: HorseshoeStatusContext): string | null {
  if (context.pending) {
    return 'Inserting node...';
  }

  if (context.rejectionMessage) {
    return context.rejectionMessage;
  }

  if (context.displayState === 'pending') {
    return 'Loading compatible nodes...';
  }

  if (context.displayState === 'blocked' && context.blockedReason) {
    return formatHorseshoeBlockedReason(context.blockedReason);
  }

  return null;
}
