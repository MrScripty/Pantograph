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

function isWorkflowSessionStateView(value: unknown): boolean {
  if (!isRecord(value)) {
    return false;
  }

  return typeof value.contract_version === 'number' && typeof value.residency === 'string';
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

  if (
    typeof value.workflow_session_state !== 'undefined'
    && value.workflow_session_state !== null
    && !isWorkflowSessionStateView(value.workflow_session_state)
  ) {
    throw new Error(
      'Invalid workflow graph mutation response: invalid workflow_session_state payload',
    );
  }

  return {
    graph: value.graph,
    workflow_event:
      typeof value.workflow_event === 'undefined'
        ? undefined
        : (value.workflow_event as WorkflowGraphMutationResponse['workflow_event']),
    workflow_session_state:
      typeof value.workflow_session_state === 'undefined'
        ? undefined
        : (value.workflow_session_state as WorkflowGraphMutationResponse['workflow_session_state']),
  };
}
