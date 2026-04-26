import { get, type Writable } from 'svelte/store';

import type { WorkflowBackend } from '../types/backend.js';
import type { NodeGroup, PortMapping } from '../types/groups.js';
import {
  findWorkflowGroupContainingNodeIds,
} from './workflowStoreGraphQueries.ts';
import type {
  WorkflowGraphMutationResult,
  WorkflowMutationDispatch,
} from './workflowMutationDispatch.ts';

export interface WorkflowGroupActions {
  collapseGroup: () => void;
  createGroup: (name: string, nodeIds: string[]) => Promise<NodeGroup | null>;
  getGroupById: (groupId: string) => NodeGroup | undefined;
  ungroupNodes: (groupId: string) => Promise<boolean>;
  updateGroupPorts: (
    groupId: string,
    exposedInputs: PortMapping[],
    exposedOutputs: PortMapping[],
  ) => Promise<boolean>;
}

function isApplied(result: WorkflowGraphMutationResult): boolean {
  return result.status === 'applied';
}

export function createWorkflowGroupActions(params: {
  backend: WorkflowBackend;
  mutationDispatch: WorkflowMutationDispatch;
  nodeGroups: Writable<Map<string, NodeGroup>>;
  tabOutOfGroup?: () => Promise<void>;
}): WorkflowGroupActions {
  async function createGroup(name: string, nodeIds: string[]): Promise<NodeGroup | null> {
    if (nodeIds.length < 2) {
      console.warn('[workflowStores] Cannot create group with less than 2 nodes');
      return null;
    }

    const result = await params.mutationDispatch.syncGraphMutationFromBackend(
      'create group',
      (sessionId) => params.backend.createGroup(name, nodeIds, sessionId),
    );
    if (!isApplied(result)) {
      return null;
    }
    return findWorkflowGroupContainingNodeIds(get(params.nodeGroups), nodeIds);
  }

  async function ungroupNodes(groupId: string): Promise<boolean> {
    const result = await params.mutationDispatch.syncGraphMutationFromBackend(
      'ungroup',
      (sessionId) => params.backend.ungroup(groupId, sessionId),
    );
    return isApplied(result);
  }

  async function updateGroupPorts(
    groupId: string,
    exposedInputs: PortMapping[],
    exposedOutputs: PortMapping[],
  ): Promise<boolean> {
    const result = await params.mutationDispatch.syncGraphMutationFromBackend(
      'update group ports',
      (sessionId) =>
        params.backend.updateGroupPorts(
          groupId,
          exposedInputs,
          exposedOutputs,
          sessionId,
        ),
    );
    return isApplied(result);
  }

  function getGroupById(groupId: string): NodeGroup | undefined {
    return get(params.nodeGroups).get(groupId);
  }

  function collapseGroup(): void {
    params.tabOutOfGroup?.();
  }

  return {
    collapseGroup,
    createGroup,
    getGroupById,
    ungroupNodes,
    updateGroupPorts,
  };
}
