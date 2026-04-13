export interface ExecutionScopedWorkflowEvent {
  type?: string;
  data: {
    execution_id?: string | null;
  };
}

export function getWorkflowEventExecutionId(event: ExecutionScopedWorkflowEvent): string | null {
  return typeof event.data.execution_id === 'string' && event.data.execution_id.trim().length > 0
    ? event.data.execution_id
    : null;
}

export function claimWorkflowExecutionIdFromEvent(
  event: ExecutionScopedWorkflowEvent,
  currentExecutionId: string | null,
): string | null {
  if (currentExecutionId) {
    return currentExecutionId;
  }

  return event.type === 'Started' ? getWorkflowEventExecutionId(event) : null;
}

export function isWorkflowEventRelevantToExecution(
  event: ExecutionScopedWorkflowEvent,
  expectedExecutionId: string | null,
): boolean {
  if (!expectedExecutionId) {
    return true;
  }

  const eventExecutionId = getWorkflowEventExecutionId(event);

  return eventExecutionId === null || eventExecutionId === expectedExecutionId;
}
