/**
 * Pantograph Node Type Registry
 *
 * Maps all node type strings to their Svelte components.
 * Consumed by createGraphContext() to inject into the graph editor.
 */
import type { NodeTypeRegistry } from '@pantograph/svelte-graph';
import { GenericNode, ReconnectableEdge } from '@pantograph/svelte-graph';

// Workflow-specific node components (Pantograph-only)
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

// Architecture node components (Pantograph-only)
import ArchComponentNode from '../components/nodes/architecture/ArchComponentNode.svelte';
import ArchServiceNode from '../components/nodes/architecture/ArchServiceNode.svelte';
import ArchStoreNode from '../components/nodes/architecture/ArchStoreNode.svelte';
import ArchBackendNode from '../components/nodes/architecture/ArchBackendNode.svelte';
import ArchCommandNode from '../components/nodes/architecture/ArchCommandNode.svelte';

export const PANTOGRAPH_NODE_REGISTRY: NodeTypeRegistry = {
  nodeTypes: {
    // Specific workflow node components
    'text-input': TextInputNode,
    'llm-inference': LLMInferenceNode,
    'ollama-inference': OllamaInferenceNode,
    'llamacpp-inference': LlamaCppInferenceNode,
    'model-provider': ModelProviderNode,
    'text-output': TextOutputNode,
    'puma-lib': PumaLibNode,
    'agent-tools': AgentToolsNode,
    'vector-db': VectorDbNode,
    'node-group': NodeGroupNode,
    'linked-input': LinkedInputNode,

    // Generic fallbacks for node types without specific components
    'image-input': GenericNode,
    'vision-analysis': GenericNode,
    'rag-search': GenericNode,
    'read-file': GenericNode,
    'write-file': GenericNode,
    'component-preview': GenericNode,
    'tool-loop': GenericNode,

    // Architecture node types (for system graph view)
    'arch-component': ArchComponentNode,
    'arch-service': ArchServiceNode,
    'arch-store': ArchStoreNode,
    'arch-backend': ArchBackendNode,
    'arch-command': ArchCommandNode,
  },
  fallbackNode: GenericNode,
  edgeTypes: {
    reconnectable: ReconnectableEdge,
  },
};
