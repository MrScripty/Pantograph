import type {
  WorkflowGraphCurrentValidationSummaryResponse,
  WorkflowGraphValidationSubmitGate,
} from '../services/workflow/types';

/** All identities are supplied by the authored graph and the backend response. */
export function currentWorkflowValidation(
  response: WorkflowGraphCurrentValidationSummaryResponse | null,
  graphSessionId: string | null,
  graphRevision: string | null,
): WorkflowGraphCurrentValidationSummaryResponse | null {
  if (!response || !graphSessionId || !graphRevision ||
      response.graph_session_id !== graphSessionId ||
      response.requested_graph_revision !== graphRevision ||
      response.current_graph_revision !== graphRevision) return null;
  const group = response.group_preflight;
  if (group && (group.graph_session_id !== graphSessionId ||
      group.graph_revision !== graphRevision ||
      group.validation_session_id !== response.validation_session_id)) return null;
  return response;
}

export function executableWorkflowValidation(
  response: WorkflowGraphCurrentValidationSummaryResponse | null,
  graphSessionId: string | null,
  graphRevision: string | null,
): boolean {
  const current = currentWorkflowValidation(response, graphSessionId, graphRevision);
  return Boolean(current && current.state === 'current' &&
    current.validation_session_id?.trim() && current.submit_gate.allowed &&
    current.summary?.status === 'executable' && current.summary.executable &&
    current.summary.blocking_diagnostics_count === 0 &&
    !current.summary.enqueue_disabled_reasons?.length &&
    !current.group_preflight?.failures?.some(failure => failure.blocking_submission));
}

export function currentWorkflowSubmitGate(
  response: WorkflowGraphCurrentValidationSummaryResponse | null,
  graphSessionId: string | null,
  graphRevision: string | null,
): WorkflowGraphValidationSubmitGate | null {
  const current = currentWorkflowValidation(response, graphSessionId, graphRevision);
  if (!current) return null;
  if (!current.submit_gate.allowed) return current.submit_gate;
  return executableWorkflowValidation(current, graphSessionId, graphRevision) ? current.submit_gate : null;
}

export interface WorkflowValidationReadTicket {
  readonly key: string | null;
}

/** Refreshes and lifecycle reads share one fence. Events revoke, never grant authority. */
export class WorkflowValidationReadFence {
  private current: WorkflowValidationReadTicket | null = null;

  begin(key: string | null): WorkflowValidationReadTicket {
    this.current = Object.freeze({ key });
    return this.current;
  }

  isCurrent(ticket: WorkflowValidationReadTicket | null): boolean {
    return ticket !== null && ticket === this.current;
  }
}
