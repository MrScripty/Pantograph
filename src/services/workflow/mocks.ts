// Mock data for frontend development without Rust backend
import type { NodeDefinition } from './types';

// NOTE: These mock definitions use snake_case to match Rust serde serialization
export const MOCK_NODE_DEFINITIONS: NodeDefinition[] = [
  // Input Nodes
  {
    node_type: 'text-input',
    category: 'input',
    label: 'Text Input',
    description: 'User text input field',
    io_binding_origin: 'client_session',
    inputs: [
      { id: 'text', label: 'Text', data_type: 'string', required: false, multiple: false },
    ],
    outputs: [
      { id: 'text', label: 'Text', data_type: 'string', required: true, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'number-input',
    category: 'input',
    label: 'Number Input',
    description: 'User numeric input field',
    io_binding_origin: 'client_session',
    inputs: [
      { id: 'value', label: 'Value', data_type: 'number', required: false, multiple: false },
    ],
    outputs: [
      { id: 'value', label: 'Value', data_type: 'number', required: true, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'boolean-input',
    category: 'input',
    label: 'Boolean Input',
    description: 'User true/false input field',
    io_binding_origin: 'client_session',
    inputs: [
      { id: 'value', label: 'Value', data_type: 'boolean', required: false, multiple: false },
    ],
    outputs: [
      { id: 'value', label: 'Value', data_type: 'boolean', required: true, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'selection-input',
    category: 'input',
    label: 'Selection Input',
    description: 'Metadata-driven dropdown input',
    io_binding_origin: 'client_session',
    inputs: [
      { id: 'value', label: 'Value', data_type: 'any', required: false, multiple: false },
    ],
    outputs: [
      { id: 'value', label: 'Value', data_type: 'any', required: true, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'vector-input',
    category: 'input',
    label: 'Vector Input',
    description: 'User vector input field',
    io_binding_origin: 'client_session',
    inputs: [
      { id: 'vector', label: 'Vector', data_type: 'embedding', required: false, multiple: false },
    ],
    outputs: [
      { id: 'vector', label: 'Vector', data_type: 'embedding', required: true, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'puma-lib',
    category: 'input',
    label: 'Puma-Lib',
    description: 'AI model file path provider',
    io_binding_origin: 'integrated',
    inputs: [],
    outputs: [
      { id: 'model_path', label: 'Model Path', data_type: 'string', required: true, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'image-input',
    category: 'input',
    label: 'Image Input',
    description: 'Canvas image capture',
    io_binding_origin: 'client_session',
    inputs: [],
    outputs: [
      { id: 'image', label: 'Image', data_type: 'image', required: true, multiple: false },
      { id: 'bounds', label: 'Bounds', data_type: 'json', required: true, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'model-provider',
    category: 'input',
    label: 'Model Provider',
    description: 'Provides model selection for inference nodes',
    io_binding_origin: 'integrated',
    inputs: [
      { id: 'model_name', label: 'Model Name', data_type: 'string', required: false, multiple: false },
    ],
    outputs: [
      { id: 'model_name', label: 'Model Name', data_type: 'string', required: true, multiple: false },
      { id: 'model_info', label: 'Model Info', data_type: 'json', required: false, multiple: false },
    ],
    execution_mode: 'reactive',
  },

  // Processing Nodes
  {
    node_type: 'llm-inference',
    category: 'processing',
    label: 'LLM Inference',
    description: 'Text completion via local LLM',
    io_binding_origin: 'integrated',
    inputs: [
      { id: 'prompt', label: 'Prompt', data_type: 'prompt', required: true, multiple: false },
      { id: 'system_prompt', label: 'System Prompt', data_type: 'string', required: false, multiple: false },
      { id: 'model', label: 'Model', data_type: 'string', required: false, multiple: false },
      { id: 'image', label: 'Image', data_type: 'image', required: false, multiple: false },
      { id: 'audio', label: 'Audio', data_type: 'audio', required: false, multiple: false },
    ],
    outputs: [
      { id: 'response', label: 'Response', data_type: 'string', required: true, multiple: false },
      { id: 'stream', label: 'Stream', data_type: 'stream', required: true, multiple: false },
    ],
    execution_mode: 'stream',
  },
  {
    node_type: 'ollama-inference',
    category: 'processing',
    label: 'Ollama Inference',
    description: 'Run inference using local Ollama server',
    io_binding_origin: 'integrated',
    inputs: [
      { id: 'prompt', label: 'Prompt', data_type: 'prompt', required: true, multiple: false },
      { id: 'model', label: 'Model', data_type: 'string', required: true, multiple: false },
      { id: 'system_prompt', label: 'System Prompt', data_type: 'string', required: false, multiple: false },
      { id: 'temperature', label: 'Temperature', data_type: 'number', required: false, multiple: false },
      { id: 'max_tokens', label: 'Max Tokens', data_type: 'number', required: false, multiple: false },
    ],
    outputs: [
      { id: 'response', label: 'Response', data_type: 'string', required: true, multiple: false },
      { id: 'model_used', label: 'Model Used', data_type: 'string', required: false, multiple: false },
      { id: 'stream', label: 'Stream', data_type: 'stream', required: false, multiple: false },
    ],
    execution_mode: 'stream',
  },
  {
    node_type: 'llamacpp-inference',
    category: 'processing',
    label: 'LlamaCpp Inference',
    description: 'Run inference via llama.cpp server (no model duplication)',
    io_binding_origin: 'integrated',
    inputs: [
      { id: 'model_path', label: 'Model Path', data_type: 'string', required: true, multiple: false },
      { id: 'prompt', label: 'Prompt', data_type: 'prompt', required: true, multiple: false },
      { id: 'system_prompt', label: 'System Prompt', data_type: 'string', required: false, multiple: false },
      { id: 'temperature', label: 'Temperature', data_type: 'number', required: false, multiple: false },
      { id: 'max_tokens', label: 'Max Tokens', data_type: 'number', required: false, multiple: false },
    ],
    outputs: [
      { id: 'response', label: 'Response', data_type: 'string', required: true, multiple: false },
      { id: 'model_path', label: 'Model Path', data_type: 'string', required: false, multiple: false },
    ],
    execution_mode: 'stream',
  },
  {
    node_type: 'reranker',
    category: 'processing',
    label: 'LlamaCpp Reranker',
    description: 'Rank candidate documents with a GGUF reranker via llama.cpp',
    io_binding_origin: 'integrated',
    inputs: [
      { id: 'model_path', label: 'Model Path', data_type: 'string', required: true, multiple: false },
      { id: 'query', label: 'Query', data_type: 'string', required: true, multiple: false },
      { id: 'documents', label: 'Documents', data_type: 'json', required: true, multiple: false },
      { id: 'documents_json', label: 'Documents JSON', data_type: 'string', required: false, multiple: false },
      { id: 'top_k', label: 'Top K', data_type: 'number', required: false, multiple: false },
      { id: 'return_documents', label: 'Return Documents', data_type: 'boolean', required: false, multiple: false },
    ],
    outputs: [
      { id: 'results', label: 'Results', data_type: 'json', required: true, multiple: false },
      { id: 'scores', label: 'Scores', data_type: 'json', required: false, multiple: false },
      { id: 'top_document', label: 'Top Document', data_type: 'string', required: false, multiple: false },
      { id: 'top_score', label: 'Top Score', data_type: 'number', required: false, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'vision-analysis',
    category: 'processing',
    label: 'Vision Analysis',
    description: 'Analyze images with vision model',
    io_binding_origin: 'integrated',
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
    io_binding_origin: 'integrated',
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
    node_type: 'agent-tools',
    category: 'tool',
    label: 'Agent Tools',
    description: 'Configure available tools for agent',
    io_binding_origin: 'integrated',
    inputs: [],
    outputs: [
      { id: 'tools', label: 'Tools', data_type: 'tools', required: true, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'read-file',
    category: 'tool',
    label: 'Read File',
    description: 'Read file contents from project',
    io_binding_origin: 'integrated',
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
    io_binding_origin: 'integrated',
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
    io_binding_origin: 'client_session',
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
    io_binding_origin: 'integrated',
    inputs: [
      { id: 'component', label: 'Component', data_type: 'component', required: true, multiple: false },
    ],
    outputs: [
      { id: 'rendered', label: 'Rendered', data_type: 'boolean', required: true, multiple: false },
    ],
    execution_mode: 'reactive',
  },
  {
    node_type: 'vector-output',
    category: 'output',
    label: 'Vector Output',
    description: 'Display vector result',
    io_binding_origin: 'client_session',
    inputs: [
      { id: 'vector', label: 'Vector', data_type: 'embedding', required: true, multiple: false },
    ],
    outputs: [
      { id: 'vector', label: 'Vector', data_type: 'embedding', required: false, multiple: false },
    ],
    execution_mode: 'reactive',
  },

  // Control Nodes
  {
    node_type: 'tool-loop',
    category: 'control',
    label: 'Tool Loop',
    description: 'Multi-turn agent with tool execution',
    io_binding_origin: 'integrated',
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

export function mockValidateConnection(sourceType: string, targetType: string): boolean {
  // Any accepts all types (snake_case to match Rust serde)
  if (targetType === 'any' || sourceType === 'any') return true;

  // Same type always valid
  if (sourceType === targetType) return true;

  // String can connect to Prompt
  if (sourceType === 'string' && targetType === 'prompt') return true;

  // Prompt can connect to String
  if (sourceType === 'prompt' && targetType === 'string') return true;

  // Audio stream can connect to legacy stream ports
  if (
    (sourceType === 'audio_stream' && targetType === 'stream') ||
    (sourceType === 'stream' && targetType === 'audio_stream')
  ) {
    return true;
  }

  // Primitive values can connect to String
  if (
    targetType === 'string' &&
    (sourceType === 'json' || sourceType === 'number' || sourceType === 'boolean')
  ) {
    return true;
  }

  return false;
}
