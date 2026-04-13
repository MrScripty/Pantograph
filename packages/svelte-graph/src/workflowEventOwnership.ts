export interface ExecutionScopedWorkflowEvent {
  data: {
    execution_id?: string | null;
  };
}

export function isWorkflowEventRelevantToExecution(
  event: ExecutionScopedWorkflowEvent,
  expectedExecutionId: string | null,
): boolean {
  if (!expectedExecutionId) {
    return true;
  }

  const eventExecutionId =
    typeof event.data.execution_id === 'string' && event.data.execution_id.trim().length > 0
      ? event.data.execution_id
      : null;

  return eventExecutionId === null || eventExecutionId === expectedExecutionId;
}
