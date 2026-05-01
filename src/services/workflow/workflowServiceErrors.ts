import { invoke } from '@tauri-apps/api/core';

export const WORKFLOW_BACKEND_ERROR_CODES = [
  'invalid_request',
  'workflow_not_found',
  'capability_violation',
  'runtime_not_ready',
  'cancelled',
  'session_not_found',
  'session_evicted',
  'queue_item_not_found',
  'scheduler_busy',
  'output_not_produced',
  'runtime_timeout',
  'internal_error',
] as const;

export type WorkflowBackendErrorCode = (typeof WORKFLOW_BACKEND_ERROR_CODES)[number];
export type WorkflowServiceErrorCode = WorkflowBackendErrorCode | 'transport_error';

export interface WorkflowBackendErrorEnvelope {
  code: WorkflowBackendErrorCode;
  message: string;
  details?: unknown;
  diagnostics?: WorkflowBackendErrorDiagnosticsLink | null;
}

export interface WorkflowBackendErrorDiagnosticsLink {
  workflow_run_id?: string | null;
  diagnostic_event_id?: string | null;
  diagnostics_unavailable?: string | null;
}

export class WorkflowServiceError extends Error {
  readonly code: WorkflowServiceErrorCode;
  readonly details: unknown;
  readonly diagnostics: WorkflowBackendErrorDiagnosticsLink | null;
  readonly backendEnvelope: WorkflowBackendErrorEnvelope | null;

  constructor(
    code: WorkflowServiceErrorCode,
    message: string,
    options: {
      details?: unknown;
      diagnostics?: WorkflowBackendErrorDiagnosticsLink | null;
      backendEnvelope?: WorkflowBackendErrorEnvelope | null;
      cause?: unknown;
    } = {},
  ) {
    super(message, { cause: options.cause });
    this.name = 'WorkflowServiceError';
    this.code = code;
    this.details = options.details;
    this.diagnostics = options.diagnostics ?? null;
    this.backendEnvelope = options.backendEnvelope ?? null;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isBackendErrorCode(value: unknown): value is WorkflowBackendErrorCode {
  return (
    typeof value === 'string' &&
    WORKFLOW_BACKEND_ERROR_CODES.includes(value as WorkflowBackendErrorCode)
  );
}

function parseDiagnosticsLink(value: unknown): WorkflowBackendErrorDiagnosticsLink | null {
  if (!isRecord(value)) {
    return null;
  }
  const workflowRunId = typeof value.workflow_run_id === 'string' ? value.workflow_run_id : null;
  const diagnosticEventId =
    typeof value.diagnostic_event_id === 'string' ? value.diagnostic_event_id : null;
  const diagnosticsUnavailable =
    typeof value.diagnostics_unavailable === 'string' ? value.diagnostics_unavailable : null;
  if (!workflowRunId && !diagnosticEventId && !diagnosticsUnavailable) {
    return null;
  }
  return {
    workflow_run_id: workflowRunId,
    diagnostic_event_id: diagnosticEventId,
    diagnostics_unavailable: diagnosticsUnavailable,
  };
}

function parseJsonCandidate(value: string): unknown {
  try {
    return JSON.parse(value);
  } catch {
    return null;
  }
}

export function parseWorkflowBackendErrorEnvelope(
  error: unknown,
): WorkflowBackendErrorEnvelope | null {
  const candidate =
    typeof error === 'string'
      ? parseJsonCandidate(error)
      : error instanceof Error
        ? parseJsonCandidate(error.message)
        : error;

  if (!isRecord(candidate) || !isBackendErrorCode(candidate.code) || typeof candidate.message !== 'string') {
    return null;
  }

  return {
    code: candidate.code,
    message: candidate.message,
    details: candidate.details,
    diagnostics: parseDiagnosticsLink(candidate.diagnostics),
  };
}

export function normalizeWorkflowServiceError(error: unknown): WorkflowServiceError {
  if (error instanceof WorkflowServiceError) {
    return error;
  }

  const envelope = parseWorkflowBackendErrorEnvelope(error);
  if (envelope) {
    return new WorkflowServiceError(envelope.code, envelope.message, {
      details: envelope.details,
      diagnostics: envelope.diagnostics,
      backendEnvelope: envelope,
      cause: error,
    });
  }

  if (error instanceof Error && error.message.trim().length > 0) {
    return new WorkflowServiceError('transport_error', error.message, { cause: error });
  }

  if (typeof error === 'string' && error.trim().length > 0) {
    return new WorkflowServiceError('transport_error', error);
  }

  return new WorkflowServiceError('transport_error', 'Workflow command failed', { cause: error });
}

export async function invokeWorkflowCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeWorkflowServiceError(error);
  }
}
