/**
 * Pantograph Node Type Registry
 *
 * Uses buildRegistry() from @pantograph/svelte-graph to map engine-provided
 * definitions to Svelte components, then adds Pantograph-specific nodes.
 */
import type { NodeTypeRegistry, NodeDefinition } from '@pantograph/svelte-graph';
import { buildRegistry } from '@pantograph/svelte-graph';

// Specialized workflow node components (Pantograph-only overrides)
import TextInputNode from '../components/nodes/workflow/TextInputNode.svelte';
import VectorInputNode from '../components/nodes/workflow/VectorInputNode.svelte';
import LLMInferenceNode from '../components/nodes/workflow/LLMInferenceNode.svelte';
import OllamaInferenceNode from '../components/nodes/workflow/OllamaInferenceNode.svelte';
import LlamaCppInferenceNode from '../components/nodes/workflow/LlamaCppInferenceNode.svelte';
import EmbeddingNode from '../components/nodes/workflow/EmbeddingNode.svelte';
import PyTorchInferenceNode from '../components/nodes/workflow/PyTorchInferenceNode.svelte';
import DiffusionInferenceNode from '../components/nodes/workflow/DiffusionInferenceNode.svelte';
import ModelProviderNode from '../components/nodes/workflow/ModelProviderNode.svelte';
import TextOutputNode from '../components/nodes/workflow/TextOutputNode.svelte';
import VectorOutputNode from '../components/nodes/workflow/VectorOutputNode.svelte';
import ImageOutputNode from '../components/nodes/workflow/ImageOutputNode.svelte';
import AudioInputNode from '../components/nodes/workflow/AudioInputNode.svelte';
import AudioOutputNode from '../components/nodes/workflow/AudioOutputNode.svelte';
import AudioGenerationNode from '../components/nodes/workflow/AudioGenerationNode.svelte';
import DepthEstimationNode from '../components/nodes/workflow/DepthEstimationNode.svelte';
import PointCloudOutputNode from '../components/nodes/workflow/PointCloudOutputNode.svelte';
import PumaLibNode from '../components/nodes/workflow/PumaLibNode.svelte';
import AgentToolsNode from '../components/nodes/workflow/AgentToolsNode.svelte';
import VectorDbNode from '../components/nodes/workflow/VectorDbNode.svelte';
import NodeGroupNode from '../components/nodes/workflow/NodeGroupNode.svelte';
import LinkedInputNode from '../components/nodes/workflow/LinkedInputNode.svelte';
import ExpandSettingsNode from '../components/nodes/workflow/ExpandSettingsNode.svelte';

// Architecture node components (Pantograph-only, not engine nodes)
import ArchComponentNode from '../components/nodes/architecture/ArchComponentNode.svelte';
import ArchServiceNode from '../components/nodes/architecture/ArchServiceNode.svelte';
import ArchStoreNode from '../components/nodes/architecture/ArchStoreNode.svelte';
import ArchBackendNode from '../components/nodes/architecture/ArchBackendNode.svelte';
import ArchCommandNode from '../components/nodes/architecture/ArchCommandNode.svelte';

/** Specialized component overrides for engine node types */
const SPECIALIZED_NODES: Record<string, any> = {
  'text-input': TextInputNode,
  'vector-input': VectorInputNode,
  'llm-inference': LLMInferenceNode,
  'ollama-inference': OllamaInferenceNode,
  'llamacpp-inference': LlamaCppInferenceNode,
  'embedding': EmbeddingNode,
  'pytorch-inference': PyTorchInferenceNode,
  'diffusion-inference': DiffusionInferenceNode,
  'model-provider': ModelProviderNode,
  'text-output': TextOutputNode,
  'vector-output': VectorOutputNode,
  'image-output': ImageOutputNode,
  'audio-input': AudioInputNode,
  'audio-output': AudioOutputNode,
  'audio-generation': AudioGenerationNode,
  'depth-estimation': DepthEstimationNode,
  'point-cloud-output': PointCloudOutputNode,
  'puma-lib': PumaLibNode,
  'agent-tools': AgentToolsNode,
  'vector-db': VectorDbNode,
  'linked-input': LinkedInputNode,
  'expand-settings': ExpandSettingsNode,
};

/** Non-engine nodes (architecture + grouping, Pantograph desktop only) */
const EXTRA_NODES: Record<string, any> = {
  'node-group': NodeGroupNode,
  'arch-component': ArchComponentNode,
  'arch-service': ArchServiceNode,
  'arch-store': ArchStoreNode,
  'arch-backend': ArchBackendNode,
  'arch-command': ArchCommandNode,
};

/**
 * Build the Pantograph node registry from engine definitions.
 *
 * @param definitions - NodeDefinition[] from the backend (via Tauri command).
 *   Defaults to empty — specialized nodes still work and fallbackNode handles the rest.
 */
export function buildPantographRegistry(definitions: NodeDefinition[] = []): NodeTypeRegistry {
  const registry = buildRegistry(definitions, SPECIALIZED_NODES);
  Object.assign(registry.nodeTypes, EXTRA_NODES);
  return registry;
}
