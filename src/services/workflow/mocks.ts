// Mock data for frontend development without Rust backend
import type { InferencePortPayloadContract, NodeDefinition } from './types';

// NOTE: These mock definitions use snake_case to match Rust serde serialization
type InferenceTaskId = InferencePortPayloadContract['task_id'];
type InferencePortPayloadRole = InferencePortPayloadContract['role'];

const TEXT_CHAT_TASK_IDS: InferenceTaskId[] = ['text_generation', 'chat_completion'];
const LLM_TASK_IDS: InferenceTaskId[] = [
  ...TEXT_CHAT_TASK_IDS,
  'embedding',
  'rerank',
  'image_generation',
  'audio_transcription',
];

function taskRolePayloads(
  taskIds: InferenceTaskId[],
  role: InferencePortPayloadRole,
): InferencePortPayloadContract[] {
  return taskIds.map((task_id) => ({ task_id, role }));
}

const TEXT_CHAT_INPUT_PAYLOADS: InferencePortPayloadContract[] = [
  { task_id: 'text_generation', role: 'task_input', input_kind: 'text_generation' },
  { task_id: 'chat_completion', role: 'task_input', input_kind: 'text_generation' },
];
const TEXT_CHAT_OUTPUT_PAYLOADS: InferencePortPayloadContract[] = [
  { task_id: 'text_generation', role: 'task_output', result_kind: 'text_generation' },
  { task_id: 'chat_completion', role: 'task_output', result_kind: 'text_generation' },
];
const IMAGE_GENERATION_PROMPT_PAYLOADS: InferencePortPayloadContract[] = [
  { task_id: 'image_generation', role: 'task_input', input_kind: 'image_generation' },
];
const IMAGE_GENERATION_RESULT_PAYLOADS: InferencePortPayloadContract[] = [
  { task_id: 'image_generation', role: 'task_output', result_kind: 'image_generation' },
];
const AUDIO_TRANSCRIPTION_AUDIO_PAYLOADS: InferencePortPayloadContract[] = [
  { task_id: 'audio_transcription', role: 'task_input', input_kind: 'audio_transcription' },
];
const AUDIO_TRANSCRIPTION_RESPONSE_PAYLOADS: InferencePortPayloadContract[] = [
  { task_id: 'audio_transcription', role: 'task_output', result_kind: 'audio_transcription' },
];
const EMBEDDING_TEXT_PAYLOADS: InferencePortPayloadContract[] = [
  { task_id: 'embedding', role: 'task_input', input_kind: 'embedding' },
];
const EMBEDDING_RESULT_PAYLOADS: InferencePortPayloadContract[] = [
  { task_id: 'embedding', role: 'task_output', result_kind: 'embedding' },
];
const RERANK_INPUT_PAYLOADS: InferencePortPayloadContract[] = [
  { task_id: 'rerank', role: 'task_input', input_kind: 'rerank' },
];
const RERANK_RESULT_PAYLOADS: InferencePortPayloadContract[] = [
  { task_id: 'rerank', role: 'task_output', result_kind: 'rerank' },
];
const LLM_MODEL_REFERENCE_PAYLOADS = taskRolePayloads(LLM_TASK_IDS, 'model_reference');
const LLM_OPTIONS_PAYLOADS = taskRolePayloads(LLM_TASK_IDS, 'options');
const TEXT_CHAT_OPTIONS_PAYLOADS = taskRolePayloads(TEXT_CHAT_TASK_IDS, 'options');
const IMAGE_GENERATION_OPTIONS_PAYLOADS = taskRolePayloads(['image_generation'], 'options');
const TEXT_CHAT_USAGE_PAYLOADS = taskRolePayloads(TEXT_CHAT_TASK_IDS, 'usage');
const TEXT_CHAT_KV_OUTPUT_PAYLOADS = taskRolePayloads(TEXT_CHAT_TASK_IDS, 'cache_handle');
const LLM_DIAGNOSTIC_PAYLOADS = taskRolePayloads(LLM_TASK_IDS, 'diagnostics');
const PROMPT_PAYLOADS: InferencePortPayloadContract[] = [
  ...TEXT_CHAT_INPUT_PAYLOADS,
  ...IMAGE_GENERATION_PROMPT_PAYLOADS,
];
const RESPONSE_PAYLOADS: InferencePortPayloadContract[] = [
  ...TEXT_CHAT_OUTPUT_PAYLOADS,
  ...AUDIO_TRANSCRIPTION_RESPONSE_PAYLOADS,
];
const RESULTS_PAYLOADS: InferencePortPayloadContract[] = [
  ...RERANK_RESULT_PAYLOADS,
  ...IMAGE_GENERATION_RESULT_PAYLOADS,
];

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
    description: 'Canonical Pumas model reference provider',
    io_binding_origin: 'integrated',
    inputs: [],
    outputs: [
      {
        id: 'model_path',
        label: 'Model Path',
        data_type: 'string',
        required: false,
        multiple: false,
        options_provider: { node_type: 'puma-lib', port_id: 'model_path' },
      },
      { id: 'pumas_model_ref', label: 'Pumas Model Ref', data_type: 'json', required: false, multiple: false },
      { id: 'model_id', label: 'Model ID', data_type: 'string', required: false, multiple: false },
      { id: 'model_type', label: 'Model Type', data_type: 'string', required: false, multiple: false },
      { id: 'task_type_primary', label: 'Task Type', data_type: 'string', required: false, multiple: false },
      { id: 'backend_key', label: 'Backend Key', data_type: 'string', required: false, multiple: false },
      { id: 'recommended_backend', label: 'Recommended Backend', data_type: 'string', required: false, multiple: false },
      { id: 'platform_context', label: 'Platform Context', data_type: 'json', required: false, multiple: false },
      { id: 'selected_binding_ids', label: 'Selected Bindings', data_type: 'json', required: false, multiple: false },
      { id: 'dependency_bindings', label: 'Dependency Bindings', data_type: 'json', required: false, multiple: false },
      { id: 'dependency_requirements_id', label: 'Dependency Requirements ID', data_type: 'string', required: false, multiple: false },
      { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: false, multiple: false },
      { id: 'dependency_requirements', label: 'Dependency Requirements', data_type: 'json', required: false, multiple: false },
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
    description: 'Canonical model inference across text, embedding, rerank, and multimodal tasks',
    io_binding_origin: 'integrated',
    inputs: [
      { id: 'task_kind', label: 'Task Kind', data_type: 'string', required: false, multiple: false, inference_payloads: LLM_OPTIONS_PAYLOADS },
      { id: 'backend_key', label: 'Backend Key', data_type: 'string', required: false, multiple: false, inference_payloads: LLM_OPTIONS_PAYLOADS },
      { id: 'pumas_model_ref', label: 'Pumas Model Ref', data_type: 'json', required: false, multiple: false, inference_payloads: LLM_MODEL_REFERENCE_PAYLOADS },
      { id: 'text', label: 'Text', data_type: 'string', required: false, multiple: false, inference_payloads: EMBEDDING_TEXT_PAYLOADS },
      { id: 'query', label: 'Query', data_type: 'string', required: false, multiple: false, inference_payloads: RERANK_INPUT_PAYLOADS },
      { id: 'documents', label: 'Documents', data_type: 'json', required: false, multiple: false, inference_payloads: RERANK_INPUT_PAYLOADS },
      { id: 'documents_json', label: 'Documents JSON', data_type: 'string', required: false, multiple: false, inference_payloads: RERANK_INPUT_PAYLOADS },
      { id: 'prompt', label: 'Prompt', data_type: 'prompt', required: false, multiple: false, inference_payloads: PROMPT_PAYLOADS },
      { id: 'system_prompt', label: 'System Prompt', data_type: 'string', required: false, multiple: false },
      { id: 'audio', label: 'Audio', data_type: 'audio', required: false, multiple: false, inference_payloads: AUDIO_TRANSCRIPTION_AUDIO_PAYLOADS },
      { id: 'context', label: 'Context', data_type: 'string', required: false, multiple: false },
      { id: 'tools', label: 'Tools', data_type: 'tools', required: false, multiple: true },
      { id: 'kv_cache_in', label: 'KV Cache In', data_type: 'kv_cache', required: false, multiple: false },
      { id: 'generation_options', label: 'Generation Options', data_type: 'json', required: false, multiple: false, inference_payloads: TEXT_CHAT_OPTIONS_PAYLOADS },
      { id: 'task_options', label: 'Task Options', data_type: 'json', required: false, multiple: false, inference_payloads: LLM_OPTIONS_PAYLOADS },
      { id: 'denoising_scheduler', label: 'Denoising Scheduler', data_type: 'string', required: false, multiple: false, inference_payloads: IMAGE_GENERATION_OPTIONS_PAYLOADS },
      { id: 'inference_settings', label: 'Inference Settings', data_type: 'json', required: false, multiple: false, inference_payloads: LLM_OPTIONS_PAYLOADS },
    ],
    outputs: [
      { id: 'response', label: 'Response', data_type: 'string', required: false, multiple: false, inference_payloads: RESPONSE_PAYLOADS },
      { id: 'results', label: 'Results', data_type: 'json', required: false, multiple: false, inference_payloads: RESULTS_PAYLOADS },
      { id: 'scores', label: 'Scores', data_type: 'json', required: false, multiple: false, inference_payloads: RERANK_RESULT_PAYLOADS },
      { id: 'top_document', label: 'Top Document', data_type: 'string', required: false, multiple: false, inference_payloads: RERANK_RESULT_PAYLOADS },
      { id: 'top_score', label: 'Top Score', data_type: 'number', required: false, multiple: false, inference_payloads: RERANK_RESULT_PAYLOADS },
      { id: 'embedding', label: 'Embedding', data_type: 'embedding', required: false, multiple: false, inference_payloads: EMBEDDING_RESULT_PAYLOADS },
      { id: 'metadata', label: 'Metadata', data_type: 'json', required: false, multiple: false, inference_payloads: LLM_DIAGNOSTIC_PAYLOADS },
      { id: 'model_ref', label: 'Model Ref', data_type: 'json', required: false, multiple: false, inference_payloads: LLM_MODEL_REFERENCE_PAYLOADS },
      { id: 'tool_calls', label: 'Tool Calls', data_type: 'json', required: false, multiple: false, inference_payloads: TEXT_CHAT_OUTPUT_PAYLOADS },
      { id: 'has_tool_calls', label: 'Has Tool Calls', data_type: 'boolean', required: false, multiple: false, inference_payloads: TEXT_CHAT_OUTPUT_PAYLOADS },
      { id: 'kv_cache_out', label: 'KV Cache Out', data_type: 'kv_cache', required: false, multiple: false, inference_payloads: TEXT_CHAT_KV_OUTPUT_PAYLOADS },
      { id: 'stream', label: 'Stream', data_type: 'stream', required: false, multiple: false, inference_payloads: TEXT_CHAT_OUTPUT_PAYLOADS },
      { id: 'diagnostics', label: 'Diagnostics', data_type: 'json', required: false, multiple: false, inference_payloads: LLM_DIAGNOSTIC_PAYLOADS },
      { id: 'usage', label: 'Usage', data_type: 'json', required: false, multiple: false, inference_payloads: TEXT_CHAT_USAGE_PAYLOADS },
    ],
    execution_mode: 'stream',
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
