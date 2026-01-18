import { invoke, Channel } from '@tauri-apps/api/core';
import type {
  NodeDefinition,
  WorkflowEvent,
  WorkflowGraph,
  WorkflowFile,
  WorkflowMetadata,
} from './types';
import {
  MOCK_NODE_DEFINITIONS,
  mockExecuteWorkflow,
  mockValidateConnection,
} from './mocks';

// Toggle this when Rust backend is ready
const USE_MOCKS = true;

export class WorkflowService {
  private channel: Channel<WorkflowEvent> | null = null;
  private eventListeners: Set<(event: WorkflowEvent) => void> = new Set();

  // --- Node Definitions ---

  async getNodeDefinitions(): Promise<NodeDefinition[]> {
    if (USE_MOCKS) {
      return MOCK_NODE_DEFINITIONS;
    }
    return invoke<NodeDefinition[]>('get_node_definitions');
  }

  getNodeDefinition(nodeType: string): NodeDefinition | undefined {
    if (USE_MOCKS) {
      return MOCK_NODE_DEFINITIONS.find((d) => d.node_type === nodeType);
    }
    // When using real backend, definitions should be cached
    return undefined;
  }

  // --- Connection Validation ---

  async validateConnection(sourceType: string, targetType: string): Promise<boolean> {
    if (USE_MOCKS) {
      return mockValidateConnection(sourceType, targetType);
    }
    return invoke<boolean>('validate_workflow_connection', {
      sourceType,
      targetType,
    });
  }

  // --- Workflow Execution ---

  async executeWorkflow(graph: WorkflowGraph): Promise<void> {
    if (USE_MOCKS) {
      return mockExecuteWorkflow(graph, (event) => {
        this.eventListeners.forEach((listener) => listener(event));
      });
    }

    this.channel = new Channel<WorkflowEvent>();

    this.channel.onmessage = (event) => {
      this.eventListeners.forEach((listener) => listener(event));
    };

    await invoke('execute_workflow', {
      graph,
      channel: this.channel,
    });
  }

  subscribeEvents(listener: (event: WorkflowEvent) => void): () => void {
    this.eventListeners.add(listener);
    return () => this.eventListeners.delete(listener);
  }

  // --- Workflow Persistence ---

  async saveWorkflow(name: string, graph: WorkflowGraph): Promise<string> {
    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Saving workflow', name, graph);
      return `/mock/workflows/${name}.json`;
    }
    return invoke<string>('save_workflow', { name, graph });
  }

  async loadWorkflow(path: string): Promise<WorkflowFile> {
    if (USE_MOCKS) {
      console.log('[WorkflowService] Mock: Loading workflow', path);
      return {
        version: '1.0',
        metadata: {
          name: 'Mock Workflow',
          created: new Date().toISOString(),
          modified: new Date().toISOString(),
        },
        graph: { nodes: [], edges: [] },
      };
    }
    return invoke<WorkflowFile>('load_workflow', { path });
  }

  async listWorkflows(): Promise<WorkflowMetadata[]> {
    if (USE_MOCKS) {
      return [
        {
          name: 'Default Agent',
          description: 'Standard agent workflow with tools',
          created: new Date().toISOString(),
          modified: new Date().toISOString(),
        },
      ];
    }
    return invoke<WorkflowMetadata[]>('list_workflows');
  }
}

export const workflowService = new WorkflowService();
