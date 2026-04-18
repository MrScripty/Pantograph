import type { WorkflowGraph, WorkflowGraphMutationResponse } from '../services/workflow/types.ts';

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isWorkflowGraph(value: unknown): value is WorkflowGraph {
  if (!isRecord(value)) {
    return false;
  }

  return Array.isArray(value.nodes) && Array.isArray(value.edges);
}

function isGraphModifiedEvent(value: unknown): boolean {
  if (!isRecord(value)) {
    return false;
  }

  return value.type === 'GraphModified' && isRecord(value.data);
}

export function parseWorkflowGraphMutationResponse(
  value: unknown,
): WorkflowGraphMutationResponse {
  if (!isRecord(value) || !isWorkflowGraph(value.graph)) {
    throw new Error('Invalid workflow graph mutation response: missing graph payload');
  }

  if (
    typeof value.workflow_event !== 'undefined'
    && value.workflow_event !== null
    && !isGraphModifiedEvent(value.workflow_event)
  ) {
    throw new Error('Invalid workflow graph mutation response: invalid workflow_event payload');
  }

  return {
    graph: value.graph,
    workflow_event:
      typeof value.workflow_event === 'undefined'
        ? undefined
        : (value.workflow_event as WorkflowGraphMutationResponse['workflow_event']),
  };
}
