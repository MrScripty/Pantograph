import type {
  NodeExecutionState,
  WorkflowGraphMutationResponse,
} from '../types/workflow.ts';

export interface WorkflowGraphMutationStoreActions {
  setNodeExecutionState: (
    nodeId: string,
    state: NodeExecutionState,
    message?: string,
  ) => void;
}

export function applyWorkflowGraphMutationResponse(
  response: WorkflowGraphMutationResponse,
  workflow: WorkflowGraphMutationStoreActions,
): boolean {
  const event = response.workflow_event;
  if (!event || event.type !== 'GraphModified') {
    return false;
  }

  for (const taskId of event.data.dirty_tasks || []) {
    workflow.setNodeExecutionState(taskId, 'idle');
  }

  return true;
}
