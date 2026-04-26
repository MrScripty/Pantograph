import { get, writable } from 'svelte/store';

import type { NodeExecutionInfo, NodeExecutionState } from '../types/workflow.js';

export interface WorkflowExecutionStateActions {
  getNodeExecutionInfo: (nodeId: string) => NodeExecutionInfo | undefined;
  nodeExecutionStates: ReturnType<typeof writable<Map<string, NodeExecutionInfo>>>;
  resetExecutionStates: () => void;
  setNodeExecutionState: (
    nodeId: string,
    state: NodeExecutionState,
    message?: string,
  ) => void;
}

export function createWorkflowExecutionState(params: {
  clearRuntimeOverlays: () => void;
}): WorkflowExecutionStateActions {
  const nodeExecutionStates = writable<Map<string, NodeExecutionInfo>>(new Map());

  function setNodeExecutionState(
    nodeId: string,
    state: NodeExecutionState,
    message?: string,
  ): void {
    nodeExecutionStates.update((map) => {
      const newMap = new Map(map);
      newMap.set(nodeId, { state, message });
      return newMap;
    });
  }

  function getNodeExecutionInfo(nodeId: string): NodeExecutionInfo | undefined {
    return get(nodeExecutionStates).get(nodeId);
  }

  function resetExecutionStates(): void {
    nodeExecutionStates.set(new Map());
    params.clearRuntimeOverlays();
  }

  return {
    getNodeExecutionInfo,
    nodeExecutionStates,
    resetExecutionStates,
    setNodeExecutionState,
  };
}
