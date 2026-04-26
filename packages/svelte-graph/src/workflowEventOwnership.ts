export interface ExecutionScopedWorkflowEvent {
  type?: string;
  data: {
    workflow_run_id?: string | null;
    ownership?: WorkflowEventOwnershipProjection | null;
  };
}

export interface WorkflowEventOwnershipProjection {
  eventWorkflowRunId: string | null;
  activeWorkflowRunId: string | null;
  relevant: boolean;
}

export function getWorkflowEventWorkflowRunId(event: ExecutionScopedWorkflowEvent): string | null {
  return (
    normalizeWorkflowRunId(event.data.ownership?.eventWorkflowRunId) ??
    normalizeWorkflowRunId(event.data.workflow_run_id)
  );
}

function normalizeWorkflowRunId(workflowRunId: string | null | undefined): string | null {
  return typeof workflowRunId === 'string' && workflowRunId.trim().length > 0
    ? workflowRunId
    : null;
}

function normalizeBackendOwnership(
  ownership: WorkflowEventOwnershipProjection | null | undefined,
): WorkflowEventOwnershipProjection | null {
  if (!ownership) {
    return null;
  }

  const eventWorkflowRunId = normalizeWorkflowRunId(ownership.eventWorkflowRunId);
  const activeWorkflowRunId = normalizeWorkflowRunId(ownership.activeWorkflowRunId);
  if (eventWorkflowRunId === null || activeWorkflowRunId === null) {
    return null;
  }

  return {
    eventWorkflowRunId,
    activeWorkflowRunId,
    relevant: ownership.relevant,
  };
}

export function projectWorkflowEventOwnership(
  event: ExecutionScopedWorkflowEvent,
  currentWorkflowRunId: string | null,
): WorkflowEventOwnershipProjection {
  const backendOwnership = normalizeBackendOwnership(event.data.ownership);
  if (backendOwnership !== null) {
    return backendOwnership;
  }

  const eventWorkflowRunId = getWorkflowEventWorkflowRunId(event);
  const activeWorkflowRunId = currentWorkflowRunId ?? eventWorkflowRunId;

  return {
    eventWorkflowRunId,
    activeWorkflowRunId,
    relevant: activeWorkflowRunId === null || eventWorkflowRunId === activeWorkflowRunId,
  };
}

export function claimWorkflowRunIdFromEvent(
  event: ExecutionScopedWorkflowEvent,
  currentWorkflowRunId: string | null,
): string | null {
  return projectWorkflowEventOwnership(event, currentWorkflowRunId).activeWorkflowRunId;
}

export function isWorkflowEventRelevantToWorkflowRun(
  event: ExecutionScopedWorkflowEvent,
  expectedWorkflowRunId: string | null,
): boolean {
  return projectWorkflowEventOwnership(event, expectedWorkflowRunId).relevant;
}
