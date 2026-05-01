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

function camelToSnake(value: string): string {
  return value.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
}

function normalizeEventData(value: Record<string, unknown>): Record<string, unknown> {
  return Object.fromEntries(
    Object.entries(value)
      .filter(([key]) => key !== 'type' && key !== 'data')
      .map(([key, entryValue]) => [camelToSnake(key), entryValue]),
  );
}

export function normalizeWorkflowGraphMutationEvent(
  value: unknown,
): WorkflowGraphMutationResponse['workflow_event'] {
  if (!isRecord(value)) {
    return null;
  }

  if (value.type === 'GraphModified' && isRecord(value.data)) {
    return value as WorkflowGraphMutationResponse['workflow_event'];
  }

  if (value.type === 'graphModified' || value.type === 'GraphModified') {
    return {
      type: 'GraphModified',
      data: normalizeEventData(value),
    } as WorkflowGraphMutationResponse['workflow_event'];
  }

  return null;
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

  const workflowEvent =
    typeof value.workflow_event === 'undefined' || value.workflow_event === null
      ? value.workflow_event
      : normalizeWorkflowGraphMutationEvent(value.workflow_event);

  if (
    typeof value.workflow_event !== 'undefined'
    && value.workflow_event !== null
    && !workflowEvent
  ) {
    throw new Error('Invalid workflow graph mutation response: invalid workflow_event payload');
  }

  const workflowSessionState =
    value.workflow_session_state ?? value.workflow_execution_session_state;

  if (
    typeof workflowSessionState !== 'undefined'
    && workflowSessionState !== null
    && !isWorkflowSessionStateView(workflowSessionState)
  ) {
    throw new Error(
      'Invalid workflow graph mutation response: invalid workflow_session_state payload',
    );
  }

  return {
    graph: value.graph,
    workflow_event:
      typeof workflowEvent === 'undefined'
        ? undefined
        : workflowEvent,
    workflow_session_state:
      typeof workflowSessionState === 'undefined'
        ? undefined
        : (workflowSessionState as WorkflowGraphMutationResponse['workflow_session_state']),
  };
}
