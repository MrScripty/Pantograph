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
import LLMInferenceNode from '../components/nodes/workflow/LLMInferenceNode.svelte';
import OllamaInferenceNode from '../components/nodes/workflow/OllamaInferenceNode.svelte';
import LlamaCppInferenceNode from '../components/nodes/workflow/LlamaCppInferenceNode.svelte';
import ModelProviderNode from '../components/nodes/workflow/ModelProviderNode.svelte';
import TextOutputNode from '../components/nodes/workflow/TextOutputNode.svelte';
import PumaLibNode from '../components/nodes/workflow/PumaLibNode.svelte';
import AgentToolsNode from '../components/nodes/workflow/AgentToolsNode.svelte';
import VectorDbNode from '../components/nodes/workflow/VectorDbNode.svelte';
import NodeGroupNode from '../components/nodes/workflow/NodeGroupNode.svelte';
import LinkedInputNode from '../components/nodes/workflow/LinkedInputNode.svelte';

// Architecture node components (Pantograph-only, not engine nodes)
import ArchComponentNode from '../components/nodes/architecture/ArchComponentNode.svelte';
import ArchServiceNode from '../components/nodes/architecture/ArchServiceNode.svelte';
import ArchStoreNode from '../components/nodes/architecture/ArchStoreNode.svelte';
import ArchBackendNode from '../components/nodes/architecture/ArchBackendNode.svelte';
import ArchCommandNode from '../components/nodes/architecture/ArchCommandNode.svelte';

/** Specialized component overrides for engine node types */
const SPECIALIZED_NODES: Record<string, any> = {
  'text-input': TextInputNode,
  'llm-inference': LLMInferenceNode,
  'ollama-inference': OllamaInferenceNode,
  'llamacpp-inference': LlamaCppInferenceNode,
  'model-provider': ModelProviderNode,
  'text-output': TextOutputNode,
  'puma-lib': PumaLibNode,
  'agent-tools': AgentToolsNode,
  'vector-db': VectorDbNode,
  'linked-input': LinkedInputNode,
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
