import {
  normalizeWorkflowServiceError,
  type WorkflowServiceErrorCode,
} from '../../services/workflow/workflowServiceErrors.ts';

const WORKFLOW_ERROR_LABELS: Record<WorkflowServiceErrorCode, string> = {
  invalid_request: 'Invalid request',
  workflow_not_found: 'Workflow not found',
  capability_violation: 'Capability violation',
  runtime_not_ready: 'Runtime not ready',
  cancelled: 'Cancelled',
  session_not_found: 'Session not found',
  session_evicted: 'Session evicted',
  queue_item_not_found: 'Queue item not found',
  scheduler_busy: 'Scheduler busy',
  output_not_produced: 'Output not produced',
  runtime_timeout: 'Runtime timeout',
  internal_error: 'Internal error',
  transport_error: 'Transport error',
};

export function formatWorkflowCommandError(error: unknown): string {
  const normalized = normalizeWorkflowServiceError(error);
  const label = WORKFLOW_ERROR_LABELS[normalized.code];

  if (normalized.message.trim().length === 0) {
    return label;
  }

  return `${label}: ${normalized.message}`;
}
