import type { EdgeTypes, NodeTypes } from '@xyflow/svelte';

import ReconnectableEdge from './edges/ReconnectableEdge.svelte';
import ArchBackendNode from './nodes/architecture/ArchBackendNode.svelte';
import ArchCommandNode from './nodes/architecture/ArchCommandNode.svelte';
import ArchComponentNode from './nodes/architecture/ArchComponentNode.svelte';
import ArchServiceNode from './nodes/architecture/ArchServiceNode.svelte';
import ArchStoreNode from './nodes/architecture/ArchStoreNode.svelte';
import AgentToolsNode from './nodes/workflow/AgentToolsNode.svelte';
import AudioGenerationNode from './nodes/workflow/AudioGenerationNode.svelte';
import AudioInputNode from './nodes/workflow/AudioInputNode.svelte';
import AudioOutputNode from './nodes/workflow/AudioOutputNode.svelte';
import BooleanInputNode from './nodes/workflow/BooleanInputNode.svelte';
import DependencyEnvironmentNode from './nodes/workflow/DependencyEnvironmentNode.svelte';
import DepthEstimationNode from './nodes/workflow/DepthEstimationNode.svelte';
import DiffusionInferenceNode from './nodes/workflow/DiffusionInferenceNode.svelte';
import EmbeddingNode from './nodes/workflow/EmbeddingNode.svelte';
import ExpandSettingsNode from './nodes/workflow/ExpandSettingsNode.svelte';
import GenericNode from './nodes/workflow/GenericNode.svelte';
import ImageOutputNode from './nodes/workflow/ImageOutputNode.svelte';
import LinkedInputNode from './nodes/workflow/LinkedInputNode.svelte';
import LlamaCppInferenceNode from './nodes/workflow/LlamaCppInferenceNode.svelte';
import LLMInferenceNode from './nodes/workflow/LLMInferenceNode.svelte';
import MaskedTextInputNode from './nodes/workflow/MaskedTextInputNode.svelte';
import ModelProviderNode from './nodes/workflow/ModelProviderNode.svelte';
import NodeGroupNode from './nodes/workflow/NodeGroupNode.svelte';
import NumberInputNode from './nodes/workflow/NumberInputNode.svelte';
import OllamaInferenceNode from './nodes/workflow/OllamaInferenceNode.svelte';
import OnnxInferenceNode from './nodes/workflow/OnnxInferenceNode.svelte';
import PointCloudOutputNode from './nodes/workflow/PointCloudOutputNode.svelte';
import PumaLibNode from './nodes/workflow/PumaLibNode.svelte';
import PyTorchInferenceNode from './nodes/workflow/PyTorchInferenceNode.svelte';
import RerankerNode from './nodes/workflow/RerankerNode.svelte';
import SelectionInputNode from './nodes/workflow/SelectionInputNode.svelte';
import TextInputNode from './nodes/workflow/TextInputNode.svelte';
import TextOutputNode from './nodes/workflow/TextOutputNode.svelte';
import VectorInputNode from './nodes/workflow/VectorInputNode.svelte';
import VectorOutputNode from './nodes/workflow/VectorOutputNode.svelte';

export const workflowEdgeTypes: EdgeTypes = {
  reconnectable: ReconnectableEdge,
};

export const workflowNodeTypes: NodeTypes = {
  'text-input': TextInputNode,
  'number-input': NumberInputNode,
  'boolean-input': BooleanInputNode,
  'selection-input': SelectionInputNode,
  'vector-input': VectorInputNode,
  'llm-inference': LLMInferenceNode,
  'ollama-inference': OllamaInferenceNode,
  'llamacpp-inference': LlamaCppInferenceNode,
  embedding: EmbeddingNode,
  reranker: RerankerNode,
  'pytorch-inference': PyTorchInferenceNode,
  'onnx-inference': OnnxInferenceNode,
  'diffusion-inference': DiffusionInferenceNode,
  'model-provider': ModelProviderNode,
  'text-output': TextOutputNode,
  'vector-output': VectorOutputNode,
  'image-output': ImageOutputNode,
  'audio-input': AudioInputNode,
  'audio-output': AudioOutputNode,
  'audio-generation': AudioGenerationNode,
  'dependency-environment': DependencyEnvironmentNode,
  'depth-estimation': DepthEstimationNode,
  'point-cloud-output': PointCloudOutputNode,
  'puma-lib': PumaLibNode,
  'agent-tools': AgentToolsNode,
  'node-group': NodeGroupNode,
  'linked-input': LinkedInputNode,
  'masked-text-input': MaskedTextInputNode,
  'expand-settings': ExpandSettingsNode,
  'image-input': GenericNode,
  'vision-analysis': GenericNode,
  'rag-search': GenericNode,
  'read-file': GenericNode,
  'write-file': GenericNode,
  'component-preview': GenericNode,
  'tool-loop': GenericNode,
  'unload-model': GenericNode,
  'arch-component': ArchComponentNode,
  'arch-service': ArchServiceNode,
  'arch-store': ArchStoreNode,
  'arch-backend': ArchBackendNode,
  'arch-command': ArchCommandNode,
};
