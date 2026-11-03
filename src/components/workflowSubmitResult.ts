import type { NodeStatusProjectionRecord } from '../services/diagnostics/types.ts';
import type { NodeExecutionState, WorkflowGraph, WorkflowRunResponse } from '../services/workflow/types.ts';

/** Apply only returned output bindings, without inferring execution of other nodes. */
export function workflowSubmitOutputOverlays(graph: WorkflowGraph, response: WorkflowRunResponse) {
  const nodeIds = new Set(graph.nodes.map((node) => node.id));
  return response.outputs.filter((output) => nodeIds.has(output.node_id)).map((output) => ({
    nodeId: output.node_id,
    data: { [output.port_id]: output.value },
  }));
}

/** Client-materialized sources have no ledger status and deliberately remain idle. */
export function workflowSubmitStatusOverlays(
  graph: WorkflowGraph,
  workflowRunId: string,
  statuses: NodeStatusProjectionRecord[],
): Array<{ nodeId: string; state: NodeExecutionState; message?: string }> {
  const nodeIds = new Set(graph.nodes.map((node) => node.id));
  const states: Record<NodeStatusProjectionRecord['status'], NodeExecutionState> = {
    queued: 'waiting', running: 'running', waiting: 'waiting', completed: 'success',
    failed: 'error', cancelled: 'error',
  };
  return statuses.filter((node) => node.workflow_run_id === workflowRunId && nodeIds.has(node.node_id))
    .map((node) => ({ nodeId: node.node_id, state: states[node.status], message: node.error ?? undefined }));
}
