<script lang="ts">
  import { SvelteFlow, Controls, MiniMap, type NodeTypes, type Node, type Edge } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import { nodes as nodesStore, edges as edgesStore, updateNodePosition } from '../stores/nodeGraphStore';
  import UserInputNode from './nodes/UserInputNode.svelte';
  import SystemPromptNode from './nodes/SystemPromptNode.svelte';
  import ToolsNode from './nodes/ToolsNode.svelte';
  import AgentNode from './nodes/AgentNode.svelte';
  import OutputNode from './nodes/OutputNode.svelte';
  import SystemPromptEditor from './SystemPromptEditor.svelte';

  // Define custom node types
  const nodeTypes: NodeTypes = {
    userInput: UserInputNode,
    systemPrompt: SystemPromptNode,
    tools: ToolsNode,
    agent: AgentNode,
    output: OutputNode,
  };

  // State for the system prompt editor modal
  let showPromptEditor = $state(false);

  // Callback for opening system prompt editor
  function openPromptEditor() {
    showPromptEditor = true;
  }

  // Local state for SvelteFlow (Svelte 5 requires $state.raw for xyflow)
  let nodes = $state.raw<Node[]>($nodesStore);
  let edges = $state.raw<Edge[]>($edgesStore);

  // Inject onEdit callback into system prompt node
  $effect(() => {
    const systemPromptNode = nodes.find(n => n.id === 'system-prompt');
    if (systemPromptNode && !systemPromptNode.data.onEdit) {
      nodes = nodes.map(n =>
        n.id === 'system-prompt'
          ? { ...n, data: { ...n.data, onEdit: openPromptEditor } }
          : n
      );
    }
  });

  // Sync store changes to local state
  $effect(() => {
    nodes = $nodesStore;
  });
  $effect(() => {
    edges = $edgesStore;
  });

  // Handle node drag events - sync back to store
  function onNodeDragStop({ targetNode }: { targetNode: Node | null; nodes: Node[]; event: MouseEvent | TouchEvent }) {
    if (targetNode) {
      updateNodePosition(targetNode.id, targetNode.position);
    }
  }

  // Handle system prompt edit request
  function handleNodeClick({ node }: { node: Node; event: MouseEvent | TouchEvent }) {
    if (node.type === 'systemPrompt') {
      showPromptEditor = true;
    }
  }
</script>

<div class="w-full h-full">
  <SvelteFlow
    bind:nodes
    bind:edges
    {nodeTypes}
    fitViewOptions={{ maxZoom: 0.75 }}
    edgesConnectable={false}
    nodesConnectable={false}
    elementsSelectable={true}
    nodesDraggable={true}
    panOnDrag={true}
    zoomOnScroll={true}
    minZoom={0.25}
    maxZoom={2}
    onnodedragstop={onNodeDragStop}
    onnodeclick={handleNodeClick}
    defaultEdgeOptions={{
      type: 'smoothstep',
      animated: false,
      style: 'stroke: #525252; stroke-width: 2px;',
    }}
  >
    <Controls />
    <MiniMap
      nodeColor={(node) => {
        switch (node.type) {
          case 'userInput':
            return '#2563eb';
          case 'systemPrompt':
            return '#9333ea';
          case 'tools':
            return '#d97706';
          case 'agent':
            return '#16a34a';
          case 'output':
            return '#0891b2';
          default:
            return '#525252';
        }
      }}
      maskColor="rgba(0, 0, 0, 0.8)"
    />
  </SvelteFlow>

  <!-- Overlay hint -->
  <div class="absolute bottom-20 left-1/2 transform -translate-x-1/2 text-neutral-500 text-xs bg-neutral-900/80 px-3 py-1.5 rounded-full backdrop-blur-sm">
    Press Ctrl+` to return to canvas
  </div>
</div>

{#if showPromptEditor}
  <SystemPromptEditor onClose={() => showPromptEditor = false} />
{/if}

<style>
  :global(.svelte-flow) {
    background-color: transparent !important;
    background-image: none !important;
  }

  :global(.svelte-flow__background) {
    display: none !important;
  }

  :global(.svelte-flow__renderer) {
    background-color: transparent !important;
  }

  :global(.svelte-flow__edge-path) {
    stroke: #525252;
    stroke-width: 2px;
  }

  :global(.svelte-flow__controls) {
    background-color: #262626;
    border: 1px solid #404040;
    border-radius: 8px;
  }

  :global(.svelte-flow__controls-button) {
    background-color: #262626;
    border-color: #404040;
    color: #a3a3a3;
  }

  :global(.svelte-flow__controls-button:hover) {
    background-color: #404040;
  }

  :global(.svelte-flow__minimap) {
    background-color: #171717;
    border: 1px solid #404040;
    border-radius: 8px;
  }

  :global(.svelte-flow__node) {
    background-color: transparent !important;
    border: none !important;
    box-shadow: none !important;
  }
</style>
