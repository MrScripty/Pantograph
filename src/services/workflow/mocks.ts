// Mock data for frontend development without Rust backend
import type { NodeDefinition, WorkflowEvent, WorkflowGraph } from './types';

// NOTE: These mock definitions use snake_case to match Rust serde serialization
export const MOCK_NODE_DEFINITIONS: NodeDefinition[] = [
  // Input Nodes
  {
    node_type: 'text-input',
    category: 'input',
    label: 'Text Input',
    description: 'User text input field',
    inputs: [],
    outputs: [
      { id: 'text', label: 'Text', data_type: 'string', required: true, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'image-input',
    category: 'input',
    label: 'Image Input',
    description: 'Canvas image capture',
    inputs: [],
    outputs: [
      { id: 'image', label: 'Image', data_type: 'image', required: true, multiple: false },
      { id: 'bounds', label: 'Bounds', data_type: 'json', required: true, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'system-prompt',
    category: 'input',
    label: 'System Prompt',
    description: 'System prompt configuration',
    inputs: [],
    outputs: [
      { id: 'prompt', label: 'Prompt', data_type: 'string', required: true, multiple: false },
    ],
    execution_mode: 'reactive',
  },

  // Processing Nodes
  {
    node_type: 'llm-inference',
    category: 'processing',
    label: 'LLM Inference',
    description: 'Text completion via local LLM',
    inputs: [
      { id: 'prompt', label: 'Prompt', data_type: 'prompt', required: true, multiple: false },
      { id: 'system_prompt', label: 'System Prompt', data_type: 'string', required: false, multiple: false },
    ],
    outputs: [
      { id: 'response', label: 'Response', data_type: 'string', required: true, multiple: false },
      { id: 'stream', label: 'Stream', data_type: 'stream', required: true, multiple: false },
    ],
    execution_mode: 'stream',
  },
  {
    node_type: 'vision-analysis',
    category: 'processing',
    label: 'Vision Analysis',
    description: 'Analyze images with vision model',
    inputs: [
      { id: 'image', label: 'Image', data_type: 'image', required: true, multiple: false },
      { id: 'prompt', label: 'Prompt', data_type: 'string', required: false, multiple: false },
    ],
    outputs: [
      { id: 'analysis', label: 'Analysis', data_type: 'string', required: true, multiple: false },
    ],
    execution_mode: 'manual',
  },
  {
    node_type: 'rag-search',
    category: 'processing',
    label: 'RAG Search',
    description: 'Search documentation with embeddings',
    inputs: [
      { id: 'query', label: 'Query', data_type: 'string', required: true, multiple: false },
    ],
    outputs: [
      { id: 'documents', label: 'Documents', data_type: 'document', required: true, multiple: true },
      { id: 'context', label: 'Context', data_type: 'string', required: true, multiple: false },
    ],
    execution_mode: 'manual',
  },

  // Tool Nodes
  {
    node_type: 'read-file',
    category: 'tool',
    label: 'Read File',
    description: 'Read file contents from project',
    inputs: [
      { id: 'path', label: 'Path', data_type: 'string', required: true, multiple: false },
    ],
    outputs: [
      { id: 'content', label: 'Content', data_type: 'string', required: true, multiple: false },
    ],
    execution_mode: 'manual',
  },
  {
    node_type: 'write-file',
    category: 'tool',
    label: 'Write File',
    description: 'Write content to file in project',
    inputs: [
      { id: 'path', label: 'Path', data_type: 'string', required: true, multiple: false },
      { id: 'content', label: 'Content', data_type: 'string', required: true, multiple: false },
    ],
    outputs: [
      { id: 'success', label: 'Success', data_type: 'boolean', required: true, multiple: false },
    ],
    execution_mode: 'manual',
  },

  // Output Nodes
  {
    node_type: 'text-output',
    category: 'output',
    label: 'Text Output',
    description: 'Display text result',
    inputs: [
      { id: 'text', label: 'Text', data_type: 'string', required: true, multiple: false },
    ],
    outputs: [],
    execution_mode: 'reactive',
  },
  {
    node_type: 'component-preview',
    category: 'output',
    label: 'Component Preview',
    description: 'Render component on canvas',
    inputs: [
      { id: 'component', label: 'Component', data_type: 'component', required: true, multiple: false },
    ],
    outputs: [
      { id: 'rendered', label: 'Rendered', data_type: 'boolean', required: true, multiple: false },
    ],
    execution_mode: 'reactive',
  },

  // Control Nodes
  {
    node_type: 'tool-loop',
    category: 'control',
    label: 'Tool Loop',
    description: 'Multi-turn agent with tool execution',
    inputs: [
      { id: 'prompt', label: 'Prompt', data_type: 'prompt', required: true, multiple: false },
      { id: 'system_prompt', label: 'System Prompt', data_type: 'string', required: false, multiple: false },
      { id: 'tools', label: 'Tools', data_type: 'tools', required: false, multiple: true },
      { id: 'context', label: 'Context', data_type: 'string', required: false, multiple: false },
    ],
    outputs: [
      { id: 'response', label: 'Response', data_type: 'string', required: true, multiple: false },
      { id: 'stream', label: 'Stream', data_type: 'stream', required: true, multiple: false },
      { id: 'tool_calls', label: 'Tool Calls', data_type: 'json', required: true, multiple: false },
    ],
    execution_mode: 'stream',
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
  // Any accepts all types (snake_case to match Rust serde)
  if (targetType === 'any' || sourceType === 'any') return true;

  // Same type always valid
  if (sourceType === targetType) return true;

  // String can connect to Prompt
  if (sourceType === 'string' && targetType === 'prompt') return true;

  // Json can connect to String
  if (sourceType === 'json' && targetType === 'string') return true;

  // Document can connect to String
  if (sourceType === 'document' && targetType === 'string') return true;

  return false;
}
