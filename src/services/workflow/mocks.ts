// Mock data for frontend development without Rust backend
import type { NodeDefinition, WorkflowEvent, WorkflowGraph } from './types';

export const MOCK_NODE_DEFINITIONS: NodeDefinition[] = [
  // Input Nodes
  {
    node_type: 'text-input',
    category: 'Input',
    label: 'Text Input',
    description: 'User text input field',
    inputs: [],
    outputs: [
      { id: 'text', label: 'Text', data_type: 'String', required: true, multiple: false },
    ],
    execution_mode: 'Reactive',
  },
  {
    node_type: 'image-input',
    category: 'Input',
    label: 'Image Input',
    description: 'Canvas image capture',
    inputs: [],
    outputs: [
      { id: 'image', label: 'Image', data_type: 'Image', required: true, multiple: false },
      { id: 'bounds', label: 'Bounds', data_type: 'Json', required: true, multiple: false },
    ],
    execution_mode: 'Reactive',
  },
  {
    node_type: 'system-prompt',
    category: 'Input',
    label: 'System Prompt',
    description: 'System prompt configuration',
    inputs: [],
    outputs: [
      { id: 'prompt', label: 'Prompt', data_type: 'String', required: true, multiple: false },
    ],
    execution_mode: 'Reactive',
  },

  // Processing Nodes
  {
    node_type: 'llm-inference',
    category: 'Processing',
    label: 'LLM Inference',
    description: 'Text completion via local LLM',
    inputs: [
      { id: 'prompt', label: 'Prompt', data_type: 'Prompt', required: true, multiple: false },
      { id: 'system_prompt', label: 'System Prompt', data_type: 'String', required: false, multiple: false },
    ],
    outputs: [
      { id: 'response', label: 'Response', data_type: 'String', required: true, multiple: false },
      { id: 'stream', label: 'Stream', data_type: 'Stream', required: true, multiple: false },
    ],
    execution_mode: 'Stream',
  },
  {
    node_type: 'vision-analysis',
    category: 'Processing',
    label: 'Vision Analysis',
    description: 'Analyze images with vision model',
    inputs: [
      { id: 'image', label: 'Image', data_type: 'Image', required: true, multiple: false },
      { id: 'prompt', label: 'Prompt', data_type: 'String', required: false, multiple: false },
    ],
    outputs: [
      { id: 'analysis', label: 'Analysis', data_type: 'String', required: true, multiple: false },
    ],
    execution_mode: 'Manual',
  },
  {
    node_type: 'rag-search',
    category: 'Processing',
    label: 'RAG Search',
    description: 'Search documentation with embeddings',
    inputs: [
      { id: 'query', label: 'Query', data_type: 'String', required: true, multiple: false },
    ],
    outputs: [
      { id: 'documents', label: 'Documents', data_type: 'Document', required: true, multiple: true },
      { id: 'context', label: 'Context', data_type: 'String', required: true, multiple: false },
    ],
    execution_mode: 'Manual',
  },

  // Tool Nodes
  {
    node_type: 'read-file',
    category: 'Tool',
    label: 'Read File',
    description: 'Read file contents from project',
    inputs: [
      { id: 'path', label: 'Path', data_type: 'String', required: true, multiple: false },
    ],
    outputs: [
      { id: 'content', label: 'Content', data_type: 'String', required: true, multiple: false },
    ],
    execution_mode: 'Manual',
  },
  {
    node_type: 'write-file',
    category: 'Tool',
    label: 'Write File',
    description: 'Write content to file in project',
    inputs: [
      { id: 'path', label: 'Path', data_type: 'String', required: true, multiple: false },
      { id: 'content', label: 'Content', data_type: 'String', required: true, multiple: false },
    ],
    outputs: [
      { id: 'success', label: 'Success', data_type: 'Boolean', required: true, multiple: false },
    ],
    execution_mode: 'Manual',
  },

  // Output Nodes
  {
    node_type: 'text-output',
    category: 'Output',
    label: 'Text Output',
    description: 'Display text result',
    inputs: [
      { id: 'text', label: 'Text', data_type: 'String', required: true, multiple: false },
    ],
    outputs: [],
    execution_mode: 'Reactive',
  },
  {
    node_type: 'component-preview',
    category: 'Output',
    label: 'Component Preview',
    description: 'Render component on canvas',
    inputs: [
      { id: 'component', label: 'Component', data_type: 'Component', required: true, multiple: false },
    ],
    outputs: [
      { id: 'rendered', label: 'Rendered', data_type: 'Boolean', required: true, multiple: false },
    ],
    execution_mode: 'Reactive',
  },

  // Control Nodes
  {
    node_type: 'tool-loop',
    category: 'Control',
    label: 'Tool Loop',
    description: 'Multi-turn agent with tool execution',
    inputs: [
      { id: 'prompt', label: 'Prompt', data_type: 'Prompt', required: true, multiple: false },
      { id: 'system_prompt', label: 'System Prompt', data_type: 'String', required: false, multiple: false },
      { id: 'tools', label: 'Tools', data_type: 'Tools', required: false, multiple: true },
      { id: 'context', label: 'Context', data_type: 'String', required: false, multiple: false },
    ],
    outputs: [
      { id: 'response', label: 'Response', data_type: 'String', required: true, multiple: false },
      { id: 'stream', label: 'Stream', data_type: 'Stream', required: true, multiple: false },
      { id: 'tool_calls', label: 'Tool Calls', data_type: 'Json', required: true, multiple: false },
    ],
    execution_mode: 'Stream',
  },
];

export async function mockExecuteWorkflow(
  graph: WorkflowGraph,
  onEvent: (event: WorkflowEvent) => void
): Promise<void> {
  onEvent({
    type: 'Started',
    data: { workflow_id: `mock-${Date.now()}`, node_count: graph.nodes.length },
  });

  // Sort nodes topologically (simplified - just process in order for mock)
  for (const node of graph.nodes) {
    onEvent({
      type: 'NodeStarted',
      data: { node_id: node.id, node_type: node.node_type },
    });

    // Simulate processing time
    await new Promise((resolve) => setTimeout(resolve, 500));

    // Simulate streaming for LLM nodes
    if (node.node_type === 'llm-inference' || node.node_type === 'tool-loop') {
      const chunks = ['Hello', ', ', 'this is ', 'a mock ', 'response!'];
      for (const chunk of chunks) {
        onEvent({
          type: 'NodeStream',
          data: { node_id: node.id, port: 'stream', chunk: { type: 'text', content: chunk } },
        });
        await new Promise((resolve) => setTimeout(resolve, 150));
      }
    }

    onEvent({
      type: 'NodeCompleted',
      data: { node_id: node.id, outputs: { response: 'Hello, this is a mock response!' } },
    });
  }

  onEvent({ type: 'Completed', data: { outputs: {} } });
}

export function mockValidateConnection(sourceType: string, targetType: string): boolean {
  // Any accepts all types
  if (targetType === 'Any' || sourceType === 'Any') return true;

  // Same type always valid
  if (sourceType === targetType) return true;

  // String can connect to Prompt
  if (sourceType === 'String' && targetType === 'Prompt') return true;

  // Json can connect to String
  if (sourceType === 'Json' && targetType === 'String') return true;

  // Document can connect to String
  if (sourceType === 'Document' && targetType === 'String') return true;

  return false;
}
