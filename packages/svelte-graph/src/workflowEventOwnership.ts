export interface ExecutionScopedWorkflowEvent {
  type?: string;
  data: {
    execution_id?: string | null;
  };
}

export interface WorkflowEventOwnershipProjection {
  eventExecutionId: string | null;
  activeExecutionId: string | null;
  relevant: boolean;
}

export function getWorkflowEventExecutionId(event: ExecutionScopedWorkflowEvent): string | null {
  return typeof event.data.execution_id === 'string' && event.data.execution_id.trim().length > 0
    ? event.data.execution_id
    : null;
}

export function projectWorkflowEventOwnership(
  event: ExecutionScopedWorkflowEvent,
  currentExecutionId: string | null,
): WorkflowEventOwnershipProjection {
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
