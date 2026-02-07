/**
 * Workflow Store — thin re-export layer over singleton store instances.
 *
 * Pantograph-specific node components (TextInputNode, LLMInferenceNode, etc.)
 * import from this file. The underlying stores are the same instances used by
 * the @pantograph/svelte-graph context system.
 */
import { derived } from 'svelte/store';
import { workflowStores, viewStores } from './storeInstances';

// Re-export types from the package
export type { WorkflowGraph, WorkflowMetadata, NodeExecutionState, NodeExecutionInfo, NodeDefinition } from '@pantograph/svelte-graph';
export type { NodeGroup, PortMapping, CreateGroupResult } from '@pantograph/svelte-graph';

// --- Re-export view-related stores (backward compatibility for NodeGroupNode.svelte) ---
export const groupStack = viewStores.groupStack;
export const tabIntoGroup = viewStores.tabIntoGroup;
export const tabOutOfGroup = viewStores.tabOutOfGroup;

// Derived from view stores
export const expandedGroupId = derived(viewStores.groupStack, ($stack) =>
  $stack.length > 0 ? $stack[$stack.length - 1] : null
);

// --- Re-export ViewportState type ---
export type { ViewportState } from '@pantograph/svelte-graph';

// --- Writable stores ---
export const nodes = workflowStores.nodes;
export const edges = workflowStores.edges;
export const nodeDefinitions = workflowStores.nodeDefinitions;
export const workflowMetadata = workflowStores.workflowMetadata;
export const isDirty = workflowStores.isDirty;
export const isExecuting = workflowStores.isExecuting;
export const isEditing = workflowStores.isEditing;
export const nodeExecutionStates = workflowStores.nodeExecutionStates;
export const currentViewport = workflowStores.currentViewport;
export const nodeGroups = workflowStores.nodeGroups;
export const selectedNodeIds = workflowStores.selectedNodeIds;

// --- Derived stores ---
export const workflowGraph = workflowStores.workflowGraph;
export const nodeDefinitionsByCategory = workflowStores.nodeDefinitionsByCategory;

// --- Node actions ---
export const addNode = workflowStores.addNode;
export const removeNode = workflowStores.removeNode;
export const updateNodePosition = workflowStores.updateNodePosition;
export const updateNodeData = workflowStores.updateNodeData;
export const getNodeById = workflowStores.getNodeById;
export const isNodeGroup = workflowStores.isNodeGroup;
export const getConnectedNodes = workflowStores.getConnectedNodes;
export const getNodesBounds = workflowStores.getNodesBounds;

// --- Edge actions ---
export const addEdge = workflowStores.addEdge;
export const removeEdge = workflowStores.removeEdge;
export const syncEdgesFromBackend = workflowStores.syncEdgesFromBackend;

// --- Execution actions ---
export const setNodeExecutionState = workflowStores.setNodeExecutionState;
export const getNodeExecutionInfo = workflowStores.getNodeExecutionInfo;
export const resetExecutionStates = workflowStores.resetExecutionStates;

// --- Workflow actions ---
export const loadWorkflow = workflowStores.loadWorkflow;
export const clearWorkflow = workflowStores.clearWorkflow;
export const loadDefaultWorkflow = workflowStores.loadDefaultWorkflow;
export const updateViewport = workflowStores.updateViewport;

// --- Group actions ---
export const createGroup = workflowStores.createGroup;
export const ungroupNodes = workflowStores.ungroupNodes;
export const updateGroupPorts = workflowStores.updateGroupPorts;
export const getGroupById = workflowStores.getGroupById;
export const collapseGroup = workflowStores.collapseGroup;
