/**
 * TauriWorkflowBackend — implements WorkflowBackend using Tauri invoke calls.
 *
 * This is the Pantograph-specific backend implementation. Each method maps
 * directly to a Tauri command defined in src-tauri/src/commands/.
 */
import { invoke, Channel } from '@tauri-apps/api/core';
import type {
  WorkflowBackend,
  UndoRedoState,
  NodeDefinition,
  WorkflowGraph,
  WorkflowFile,
  WorkflowMetadata,
  WorkflowEvent,
  GraphNode,
  GraphEdge,
  NodeGroup,
  PortMapping,
  CreateGroupResult,
} from '@pantograph/svelte-graph';

export class TauriWorkflowBackend implements WorkflowBackend {
  private channel: Channel<WorkflowEvent> | null = null;
  private eventListeners: Set<(event: WorkflowEvent) => void> = new Set();

  // --- Node Definitions ---

  async getNodeDefinitions(): Promise<NodeDefinition[]> {
    return invoke<NodeDefinition[]>('get_node_definitions');
  }

  async validateConnection(sourceType: string, targetType: string): Promise<boolean> {
    return invoke<boolean>('validate_workflow_connection', { sourceType, targetType });
  }

  // --- Session Management ---

  async createSession(graph: WorkflowGraph): Promise<string> {
    return invoke<string>('create_workflow_session', { graph });
  }

  async runSession(sessionId: string): Promise<void> {
    this.channel = new Channel<WorkflowEvent>();
    this.channel.onmessage = (event) => {
      this.eventListeners.forEach((listener) => listener(event));
    };
    await invoke('run_workflow_session', { sessionId, channel: this.channel });
  }

  async removeSession(sessionId: string): Promise<void> {
    await invoke('remove_execution', { executionId: sessionId });
  }

  // --- Execution ---

  async executeWorkflow(graph: WorkflowGraph): Promise<void> {
    this.channel = new Channel<WorkflowEvent>();
    this.channel.onmessage = (event) => {
      this.eventListeners.forEach((listener) => listener(event));
    };
    await invoke('execute_workflow_v2', { graph, channel: this.channel });
  }

  // --- Graph Mutation ---

  async addNode(node: GraphNode, sessionId: string): Promise<void> {
    return invoke('add_node_to_execution', { executionId: sessionId, node });
  }

  async addEdge(edge: GraphEdge, sessionId: string): Promise<WorkflowGraph> {
    return invoke<WorkflowGraph>('add_edge_to_execution', { executionId: sessionId, edge });
  }

  async removeEdge(edgeId: string, sessionId: string): Promise<WorkflowGraph> {
    return invoke<WorkflowGraph>('remove_edge_from_execution', { executionId: sessionId, edgeId });
  }

  async updateNodeData(
    nodeId: string,
    data: Record<string, unknown>,
    sessionId: string,
  ): Promise<void> {
    return invoke('update_node_data', { executionId: sessionId, nodeId, data });
  }

  async getExecutionGraph(sessionId: string): Promise<WorkflowGraph> {
    return invoke<WorkflowGraph>('get_execution_graph', { executionId: sessionId });
  }

  // --- Undo/Redo ---

  async getUndoRedoState(sessionId: string): Promise<UndoRedoState> {
    return invoke<UndoRedoState>('get_undo_redo_state', { executionId: sessionId });
  }

  async undo(sessionId: string): Promise<WorkflowGraph> {
    return invoke<WorkflowGraph>('undo_workflow', { executionId: sessionId });
  }

  async redo(sessionId: string): Promise<WorkflowGraph> {
    return invoke<WorkflowGraph>('redo_workflow', { executionId: sessionId });
  }

  // --- Persistence ---

  async saveWorkflow(name: string, graph: WorkflowGraph): Promise<string> {
    return invoke<string>('save_workflow', { name, graph });
  }

  async loadWorkflow(path: string): Promise<WorkflowFile> {
    return invoke<WorkflowFile>('load_workflow', { path });
  }

  async listWorkflows(): Promise<WorkflowMetadata[]> {
    return invoke<WorkflowMetadata[]>('list_workflows');
  }

  async deleteWorkflow(name: string): Promise<void> {
    return invoke('delete_workflow', { name });
  }

  // --- Node Groups ---

  async createGroup(
    name: string,
    selectedNodeIds: string[],
    graph: WorkflowGraph,
  ): Promise<CreateGroupResult> {
    return invoke<CreateGroupResult>('create_node_group', {
      name,
      selectedNodeIds,
      graph,
    });
  }

  async updateGroupPorts(
    group: NodeGroup,
    exposedInputs: PortMapping[],
    exposedOutputs: PortMapping[],
  ): Promise<NodeGroup> {
    return invoke<NodeGroup>('update_group_ports', {
      group,
      exposedInputs,
      exposedOutputs,
    });
  }

  // --- Events ---

  subscribeEvents(listener: (event: WorkflowEvent) => void): () => void {
    this.eventListeners.add(listener);
    return () => this.eventListeners.delete(listener);
  }
}
