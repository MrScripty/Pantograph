import type { Edge } from '@xyflow/svelte';

import type { ConnectionAnchor, ConnectionIntentState } from './types/workflow.ts';
import { resolveReconnectSourceAnchor } from './reconnectInteraction.ts';

export type WorkflowReconnectStartDecision =
  | {
      sourceAnchor: ConnectionAnchor;
      type: 'start';
    }
  | {
      type: 'clear';
    }
  | {
      type: 'ignore';
    };

export type WorkflowReconnectCommitResult =
  | {
      type: 'accepted' | 'invalid' | 'stale';
    }
  | {
      error: unknown;
      type: 'failed';
    }
  | {
      intent: ConnectionIntentState;
      type: 'rejected';
    };

export type WorkflowReconnectResultDecision =
  | {
      type: 'clear';
    }
  | {
      intent: ConnectionIntentState;
      message?: string;
      type: 'set-intent';
    }
  | {
      error: unknown;
      type: 'log-failure';
    };

export function resolveWorkflowReconnectStartDecision(params: {
  canEdit: boolean;
  edge: Edge;
  handleType: 'source' | 'target';
}): WorkflowReconnectStartDecision {
  if (!params.canEdit) {
    return { type: 'ignore' };
  }

  const sourceAnchor = resolveReconnectSourceAnchor(
    {
      source: params.edge.source,
      sourceHandle: params.edge.sourceHandle,
    },
    params.handleType,
  );
  if (!sourceAnchor) {
    return { type: 'clear' };
  }

  return {
    sourceAnchor,
    type: 'start',
  };
}

export function resolveWorkflowReconnectResultDecision(
  result: WorkflowReconnectCommitResult,
): WorkflowReconnectResultDecision {
  if (
    result.type === 'accepted' ||
    result.type === 'invalid' ||
    result.type === 'stale'
  ) {
    return { type: 'clear' };
  }

  if (result.type === 'rejected') {
    return {
      intent: result.intent,
      message: result.intent.rejection?.message,
      type: 'set-intent',
    };
  }

  if (result.type === 'failed') {
    return {
      error: result.error,
      type: 'log-failure',
    };
  }

  return { type: 'clear' };
}
