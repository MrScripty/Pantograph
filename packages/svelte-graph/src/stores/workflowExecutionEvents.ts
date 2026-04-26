import type { Edge } from '@xyflow/svelte';
import type {
  WorkflowEvent,
  WorkflowEventData,
  NodeExecutionState,
} from '../types/workflow.ts';
import { projectWorkflowEventOwnership } from '../workflowEventOwnership.ts';

export interface WorkflowExecutionEventStoreActions {
  setNodeExecutionState: (
    nodeId: string,
    state: NodeExecutionState,
    message?: string,
  ) => void;
  updateNodeData: (nodeId: string, data: Record<string, unknown>) => void;
}

export interface WorkflowExecutionEventInput {
  event: WorkflowEvent;
  activeWorkflowRunId: string | null;
  waitingForInput: boolean;
  edges: Edge[];
  workflow: WorkflowExecutionEventStoreActions;
}

export interface WorkflowExecutionEventResult {
  activeWorkflowRunId: string | null;
  waitingForInput: boolean;
  handled: boolean;
  shouldCleanup: boolean;
}

export function applyWorkflowExecutionEvent({
  event,
  activeWorkflowRunId,
  waitingForInput,
  edges,
  workflow,
}: WorkflowExecutionEventInput): WorkflowExecutionEventResult {
  const ownership = projectWorkflowEventOwnership(event, activeWorkflowRunId);
  if (!ownership.relevant) {
    return executionEventResult(ownership.activeWorkflowRunId, waitingForInput, false, false);
  }

  switch (event.type) {
    case 'NodeStarted': {
      const data = event.data as WorkflowEventData['NodeStarted'];
      workflow.setNodeExecutionState(data.node_id, 'running');
      return executionEventResult(ownership.activeWorkflowRunId, false, true, false);
    }
    case 'IncrementalExecutionStarted': {
      const data = event.data as WorkflowEventData['IncrementalExecutionStarted'];
      for (const taskId of data.task_ids) {
        workflow.setNodeExecutionState(taskId, 'running');
      }
      return executionEventResult(ownership.activeWorkflowRunId, false, true, false);
    }
    case 'NodeCompleted': {
      const data = event.data as WorkflowEventData['NodeCompleted'];
      workflow.setNodeExecutionState(data.node_id, 'success');
      applyNodeCompletedOutputs(data.node_id, data.outputs, edges, workflow);
      return executionEventResult(ownership.activeWorkflowRunId, false, true, false);
    }
    case 'NodeError': {
      const data = event.data as WorkflowEventData['NodeError'];
      workflow.setNodeExecutionState(data.node_id, 'error', data.error);
      return executionEventResult(ownership.activeWorkflowRunId, waitingForInput, true, false);
    }
    case 'WaitingForInput': {
      const data = event.data as WorkflowEventData['WaitingForInput'];
      workflow.setNodeExecutionState(
        data.node_id,
        'waiting',
        data.message || 'Waiting for input',
      );
      return executionEventResult(ownership.activeWorkflowRunId, true, true, false);
    }
    case 'GraphModified': {
      const data = event.data as WorkflowEventData['GraphModified'];
      for (const taskId of data.dirty_tasks || []) {
        workflow.setNodeExecutionState(taskId, 'idle');
      }
      return executionEventResult(ownership.activeWorkflowRunId, waitingForInput, true, false);
    }
    case 'Completed':
    case 'Failed':
    case 'Cancelled':
      return executionEventResult(ownership.activeWorkflowRunId, false, true, true);
    default:
      return executionEventResult(ownership.activeWorkflowRunId, waitingForInput, false, false);
  }
}

function applyNodeCompletedOutputs(
  nodeId: string,
  outputs: Record<string, unknown>,
  edges: Edge[],
  workflow: WorkflowExecutionEventStoreActions,
) {
  const outgoingEdges = edges.filter((edge) => edge.source === nodeId);
  for (const edge of outgoingEdges) {
    const sourceHandle = edge.sourceHandle || '';
    const outputValue = outputs[sourceHandle];
    if (typeof outputValue === 'undefined') {
      continue;
    }

    const targetHandle = edge.targetHandle || '';
    workflow.updateNodeData(edge.target, {
      [targetHandle]: outputValue,
    });
  }
}

function executionEventResult(
  activeWorkflowRunId: string | null,
  waitingForInput: boolean,
  handled: boolean,
  shouldCleanup: boolean,
): WorkflowExecutionEventResult {
  return {
    activeWorkflowRunId,
    waitingForInput,
    handled,
    shouldCleanup,
  };
}
