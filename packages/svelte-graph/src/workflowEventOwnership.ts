export interface ExecutionScopedWorkflowEvent {
  type?: string;
  data: {
    execution_id?: string | null;
    ownership?: WorkflowEventOwnershipProjection | null;
  };
}

export interface WorkflowEventOwnershipProjection {
  eventExecutionId: string | null;
  activeExecutionId: string | null;
  relevant: boolean;
}

export function getWorkflowEventExecutionId(event: ExecutionScopedWorkflowEvent): string | null {
  return (
    normalizeExecutionId(event.data.ownership?.eventExecutionId) ??
    normalizeExecutionId(event.data.execution_id)
  );
}

function normalizeExecutionId(executionId: string | null | undefined): string | null {
  return typeof executionId === 'string' && executionId.trim().length > 0
    ? executionId
    : null;
}

function normalizeBackendOwnership(
  ownership: WorkflowEventOwnershipProjection | null | undefined,
): WorkflowEventOwnershipProjection | null {
  if (!ownership) {
    return null;
  }

  const eventExecutionId = normalizeExecutionId(ownership.eventExecutionId);
  const activeExecutionId = normalizeExecutionId(ownership.activeExecutionId);
  if (eventExecutionId === null || activeExecutionId === null) {
    return null;
  }

  return {
    eventExecutionId,
    activeExecutionId,
    relevant: ownership.relevant,
  };
}

export function projectWorkflowEventOwnership(
  event: ExecutionScopedWorkflowEvent,
  currentExecutionId: string | null,
): WorkflowEventOwnershipProjection {
  const backendOwnership = normalizeBackendOwnership(event.data.ownership);
  if (backendOwnership !== null) {
    return backendOwnership;
  }

  const eventExecutionId = getWorkflowEventExecutionId(event);
  const activeExecutionId = currentExecutionId ?? eventExecutionId;

  return {
    eventExecutionId,
    activeExecutionId,
    relevant: activeExecutionId === null || eventExecutionId === activeExecutionId,
  };
}

export function claimWorkflowExecutionIdFromEvent(
  event: ExecutionScopedWorkflowEvent,
  currentExecutionId: string | null,
): string | null {
  return projectWorkflowEventOwnership(event, currentExecutionId).activeExecutionId;
}

export function isWorkflowEventRelevantToExecution(
  event: ExecutionScopedWorkflowEvent,
  expectedExecutionId: string | null,
): boolean {
  return projectWorkflowEventOwnership(event, expectedExecutionId).relevant;
}
