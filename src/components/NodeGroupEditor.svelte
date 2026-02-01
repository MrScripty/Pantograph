<script lang="ts">
  import { SvelteFlow, Controls, MiniMap, type NodeTypes, type EdgeTypes, type Node, type Edge, type Connection } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  import type { NodeGroup, PortMapping } from '../services/workflow/groupTypes';
  import type { NodeDefinition, GraphEdge } from '../services/workflow/types';
  import { nodeDefinitions, expandedGroupId, collapseGroup } from '../stores/workflowStore';
  import { workflowService } from '../services/workflow/WorkflowService';

  // Import workflow node components (reuse from main workflow)
  import TextInputNode from './nodes/workflow/TextInputNode.svelte';
  import LLMInferenceNode from './nodes/workflow/LLMInferenceNode.svelte';
  import OllamaInferenceNode from './nodes/workflow/OllamaInferenceNode.svelte';
  import LlamaCppInferenceNode from './nodes/workflow/LlamaCppInferenceNode.svelte';
  import ModelProviderNode from './nodes/workflow/ModelProviderNode.svelte';
  import TextOutputNode from './nodes/workflow/TextOutputNode.svelte';
  import GenericNode from './nodes/workflow/GenericNode.svelte';
  import PumaLibNode from './nodes/workflow/PumaLibNode.svelte';
  import AgentToolsNode from './nodes/workflow/AgentToolsNode.svelte';
  import VectorDbNode from './nodes/workflow/VectorDbNode.svelte';

  // Import custom edge components
  import ReconnectableEdge from './edges/ReconnectableEdge.svelte';

  interface Props {
    group: NodeGroup;
    onSave: (nodes: Node[], edges: Edge[], exposedInputs: PortMapping[], exposedOutputs: PortMapping[]) => void;
    onCancel: () => void;
  }

  let { group, onSave, onCancel }: Props = $props();

  // Define custom edge types
  const edgeTypes: EdgeTypes = {
    reconnectable: ReconnectableEdge,
  };

  // Define custom node types
  const nodeTypes: NodeTypes = {
    'text-input': TextInputNode,
    'llm-inference': LLMInferenceNode,
    'ollama-inference': OllamaInferenceNode,
    'llamacpp-inference': LlamaCppInferenceNode,
    'model-provider': ModelProviderNode,
    'text-output': TextOutputNode,
    'puma-lib': PumaLibNode,
    'agent-tools': AgentToolsNode,
    'vector-db': VectorDbNode,
    // Generic fallback
    'image-input': GenericNode,
    'vision-analysis': GenericNode,
    'rag-search': GenericNode,
    'read-file': GenericNode,
    'write-file': GenericNode,
    'component-preview': GenericNode,
    'tool-loop': GenericNode,
  };

  // Convert group nodes/edges to SvelteFlow format
  let nodes = $state.raw<Node[]>(
    group.nodes.map((n) => {
      const definition = $nodeDefinitions.find((d) => d.node_type === n.node_type);
      return {
        id: n.id,
        type: n.node_type,
        position: n.position,
        data: {
          ...n.data,
          definition,
        },
      };
    })
  );

  let edges = $state.raw<Edge[]>(
    group.edges.map((e) => ({
      id: e.id,
      source: e.source,
      sourceHandle: e.source_handle,
      target: e.target,
      targetHandle: e.target_handle,
    }))
  );

  // Track exposed ports
  let exposedInputs = $state<PortMapping[]>([...group.exposed_inputs]);
  let exposedOutputs = $state<PortMapping[]>([...group.exposed_outputs]);

  // Handle new connections
  async function handleConnect(connection: Connection) {
    const newEdge: Edge = {
      id: `${connection.source}-${connection.sourceHandle}-${connection.target}-${connection.targetHandle}`,
      source: connection.source!,
      sourceHandle: connection.sourceHandle!,
      target: connection.target!,
      targetHandle: connection.targetHandle!,
    };

    edges = [...edges, newEdge];
  }

  // Handle deletion
  function handleDelete({ nodes: deletedNodes, edges: deletedEdges }: { nodes: Node[]; edges: Edge[] }) {
    if (deletedNodes.length > 0) {
      const deletedIds = new Set(deletedNodes.map((n) => n.id));
      nodes = nodes.filter((n) => !deletedIds.has(n.id));
      // Also remove edges connected to deleted nodes
      edges = edges.filter((e) => !deletedIds.has(e.source) && !deletedIds.has(e.target));
      // Remove port mappings for deleted nodes
      exposedInputs = exposedInputs.filter((p) => !deletedIds.has(p.internal_node_id));
      exposedOutputs = exposedOutputs.filter((p) => !deletedIds.has(p.internal_node_id));
    }

    if (deletedEdges.length > 0) {
      const deletedIds = new Set(deletedEdges.map((e) => e.id));
      edges = edges.filter((e) => !deletedIds.has(e.id));
    }
  }

  function handleSave() {
    // Convert back to graph format
    const graphNodes = nodes.map((n) => ({
      id: n.id,
      node_type: n.type || 'unknown',
      position: n.position,
      data: n.data,
    }));

    const graphEdges: GraphEdge[] = edges.map((e) => ({
      id: e.id,
      source: e.source,
      source_handle: e.sourceHandle || 'output',
      target: e.target,
      target_handle: e.targetHandle || 'input',
    }));

    onSave(nodes, edges, exposedInputs, exposedOutputs);
  }

  function handleBack() {
    collapseGroup();
    onCancel();
  }
</script>

<div class="group-editor-container w-full h-full flex flex-col">
  <!-- Header bar -->
  <div class="editor-header flex items-center gap-4 px-4 py-2 bg-purple-900/30 border-b border-purple-600/30">
    <button
      class="flex items-center gap-2 text-purple-300 hover:text-purple-100 transition-colors"
      onclick={handleBack}
    >
      <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
      </svg>
      Back
    </button>

    <div class="flex items-center gap-2">
      <svg class="w-5 h-5 text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
      </svg>
      <span class="text-lg font-medium text-purple-200">Editing: {group.name}</span>
    </div>

    <div class="ml-auto flex items-center gap-2">
      <span class="text-sm text-purple-400">{nodes.length} nodes</span>
      <button
        class="px-3 py-1 text-sm bg-purple-600 hover:bg-purple-500 text-white rounded transition-colors"
        onclick={handleSave}
      >
        Save & Close
      </button>
    </div>
  </div>

  <!-- Graph editor -->
  <div class="flex-1 relative">
    <SvelteFlow
      bind:nodes
      bind:edges
      {nodeTypes}
      {edgeTypes}
      fitView
      fitViewOptions={{ maxZoom: 1, padding: 0.2 }}
      nodesConnectable={true}
      elementsSelectable={true}
      nodesDraggable={true}
      panOnDrag={true}
      zoomOnScroll={true}
      minZoom={0.25}
      maxZoom={2}
      deleteKey="Delete"
      onconnect={handleConnect}
      ondelete={handleDelete}
      defaultEdgeOptions={{
        type: 'reconnectable',
        animated: false,
        style: 'stroke: #7c3aed; stroke-width: 2px;',
        interactionWidth: 20,
        selectable: true,
        focusable: true,
      }}
    >
      <Controls />
      <MiniMap
        nodeColor={() => '#7c3aed'}
        maskColor="rgba(139, 92, 246, 0.1)"
      />
    </SvelteFlow>

    <!-- Exposed ports indicator -->
    <div class="absolute bottom-4 left-4 bg-neutral-800/90 rounded-lg p-3 text-sm">
      <div class="text-purple-300 font-medium mb-2">Exposed Ports</div>
      <div class="flex gap-4">
        <div>
          <div class="text-neutral-400 text-xs mb-1">Inputs ({exposedInputs.length})</div>
          {#each exposedInputs as input}
            <div class="text-purple-400 text-xs">{input.group_port_label}</div>
          {/each}
          {#if exposedInputs.length === 0}
            <div class="text-neutral-500 text-xs">None</div>
          {/if}
        </div>
        <div>
          <div class="text-neutral-400 text-xs mb-1">Outputs ({exposedOutputs.length})</div>
          {#each exposedOutputs as output}
            <div class="text-purple-400 text-xs">{output.group_port_label}</div>
          {/each}
          {#if exposedOutputs.length === 0}
            <div class="text-neutral-500 text-xs">None</div>
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  :global(.group-editor-container .svelte-flow) {
    background-color: transparent !important;
    background-image: none !important;
  }

  :global(.group-editor-container .svelte-flow__background) {
    display: none !important;
  }

  :global(.group-editor-container .svelte-flow__edge-path) {
    stroke: #7c3aed;
    stroke-width: 2px;
  }

  :global(.group-editor-container .svelte-flow__edge.selected .svelte-flow__edge-path) {
    stroke: #a78bfa;
    stroke-width: 3px;
  }

  :global(.group-editor-container .svelte-flow__controls) {
    background-color: #262626;
    border: 1px solid #7c3aed;
    border-radius: 8px;
  }

  :global(.group-editor-container .svelte-flow__controls-button) {
    background-color: #262626;
    border-color: #7c3aed;
    color: #a3a3a3;
  }

  :global(.group-editor-container .svelte-flow__controls-button:hover) {
    background-color: #7c3aed;
  }

  :global(.group-editor-container .svelte-flow__minimap) {
    background-color: #171717;
    border: 1px solid #7c3aed;
    border-radius: 8px;
  }
</style>
