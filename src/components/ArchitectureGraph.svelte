<script lang="ts">
  import { SvelteFlow, Controls, MiniMap, type NodeTypes, type Node, type Edge } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  import { architectureNodes, architectureEdges } from '../stores/architectureStore';
  import { CATEGORY_COLORS } from '../services/architecture/types';

  // Import architecture node components
  import ArchComponentNode from './nodes/architecture/ArchComponentNode.svelte';
  import ArchServiceNode from './nodes/architecture/ArchServiceNode.svelte';
  import ArchStoreNode from './nodes/architecture/ArchStoreNode.svelte';
  import ArchBackendNode from './nodes/architecture/ArchBackendNode.svelte';
  import ArchCommandNode from './nodes/architecture/ArchCommandNode.svelte';

  // Define custom node types for architecture
  const nodeTypes: NodeTypes = {
    'arch-component': ArchComponentNode,
    'arch-service': ArchServiceNode,
    'arch-store': ArchStoreNode,
    'arch-backend': ArchBackendNode,
    'arch-command': ArchCommandNode,
  };

  // Local state for SvelteFlow (Svelte 5 requires $state.raw for xyflow)
  let nodes = $state.raw<Node[]>($architectureNodes);
  let edges = $state.raw<Edge[]>($architectureEdges);

  // Sync store changes to local state
  $effect(() => {
    nodes = $architectureNodes;
  });
  $effect(() => {
    edges = $architectureEdges;
  });

  // Handle node selection (log file path)
  function handleNodeClick({ node }: { node: Node }) {
    if (node.data?.filePath) {
      console.log(`[Architecture] Selected: ${node.data.label}`);
      console.log(`[Architecture] File: ${node.data.filePath}`);
    }
  }
</script>

<div class="architecture-graph-container w-full h-full">
  <SvelteFlow
    bind:nodes
    bind:edges
    {nodeTypes}
    fitView
    fitViewOptions={{ padding: 0.2, maxZoom: 1 }}
    nodesConnectable={false}
    elementsSelectable={true}
    nodesDraggable={true}
    panOnDrag={true}
    zoomOnScroll={true}
    minZoom={0.1}
    maxZoom={2}
    deleteKey={null}
    onnodeclick={handleNodeClick}
    defaultEdgeOptions={{
      type: 'smoothstep',
      animated: false,
    }}
  >
    <Controls />
    <MiniMap
      nodeColor={(node) => {
        const category = node.data?.category;
        return CATEGORY_COLORS[category as keyof typeof CATEGORY_COLORS] || '#525252';
      }}
      maskColor="rgba(0, 0, 0, 0.8)"
    />
  </SvelteFlow>
</div>

<style>
  :global(.architecture-graph-container .svelte-flow) {
    background-color: transparent !important;
    background-image: none !important;
  }

  :global(.architecture-graph-container .svelte-flow__background) {
    display: none !important;
  }

  :global(.architecture-graph-container .svelte-flow__renderer) {
    background-color: transparent !important;
  }

  :global(.architecture-graph-container .svelte-flow__edge-path) {
    stroke: #525252;
    stroke-width: 2px;
  }

  :global(.architecture-graph-container .svelte-flow__edge.selected .svelte-flow__edge-path) {
    stroke: #4f46e5;
    stroke-width: 3px;
  }

  :global(.architecture-graph-container .svelte-flow__controls) {
    background-color: #262626;
    border: 1px solid #404040;
    border-radius: 8px;
  }

  :global(.architecture-graph-container .svelte-flow__controls-button) {
    background-color: #262626;
    border-color: #404040;
    color: #a3a3a3;
  }

  :global(.architecture-graph-container .svelte-flow__controls-button:hover) {
    background-color: #404040;
  }

  :global(.architecture-graph-container .svelte-flow__minimap) {
    background-color: #171717;
    border: 1px solid #404040;
    border-radius: 8px;
  }

  :global(.architecture-graph-container .svelte-flow__node) {
    background-color: transparent !important;
    border: none !important;
    box-shadow: none !important;
  }

  :global(.architecture-graph-container .svelte-flow__handle) {
    border-radius: 50%;
  }
</style>
