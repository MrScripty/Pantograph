<script lang="ts">
  import { onMount } from 'svelte';
  import { SvelteFlow, Controls, MiniMap, type NodeTypes, type Node, type Edge, type Connection } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  import {
    nodes as nodesStore,
    edges as edgesStore,
    nodeDefinitions,
    isEditing,
    updateNodePosition,
    addEdge as storeAddEdge,
    removeEdge,
    addNode,
    removeNode,
    loadDefaultWorkflow,
  } from '../stores/workflowStore';
  import { workflowService } from '../services/workflow/WorkflowService';
  import type { NodeDefinition } from '../services/workflow/types';

  // Import workflow node components
  import TextInputNode from './nodes/workflow/TextInputNode.svelte';
  import LLMInferenceNode from './nodes/workflow/LLMInferenceNode.svelte';
  import TextOutputNode from './nodes/workflow/TextOutputNode.svelte';
  import GenericNode from './nodes/workflow/GenericNode.svelte';

  // Define custom node types for workflow
  const nodeTypes: NodeTypes = {
    'text-input': TextInputNode,
    'llm-inference': LLMInferenceNode,
    'text-output': TextOutputNode,
    // Generic fallback for other node types
    'image-input': GenericNode,
    'system-prompt': GenericNode,
    'vision-analysis': GenericNode,
    'rag-search': GenericNode,
    'read-file': GenericNode,
    'write-file': GenericNode,
    'component-preview': GenericNode,
    'tool-loop': GenericNode,
  };

  // Local state for SvelteFlow (Svelte 5 requires $state.raw for xyflow)
  let nodes = $state.raw<Node[]>($nodesStore);
  let edges = $state.raw<Edge[]>($edgesStore);

  // Sync store changes to local state
  $effect(() => {
    nodes = $nodesStore;
  });
  $effect(() => {
    edges = $edgesStore;
  });

  // Initialize node definitions on mount
  onMount(async () => {
    const definitions = await workflowService.getNodeDefinitions();
    nodeDefinitions.set(definitions);

    // Load default workflow if empty
    if ($nodesStore.length === 0) {
      loadDefaultWorkflow(definitions);
    }
  });

  // Handle node drag events - sync back to store
  function onNodeDragStop({
    targetNode,
  }: {
    targetNode: Node | null;
    nodes: Node[];
    event: MouseEvent | TouchEvent;
  }) {
    if (targetNode) {
      updateNodePosition(targetNode.id, targetNode.position);
    }
  }

  // Handle new connections
  async function handleConnect(connection: Connection) {
    if (!$isEditing) return;

    // Get port types from node data
    const sourceNode = nodes.find((n) => n.id === connection.source);
    const targetNode = nodes.find((n) => n.id === connection.target);

    const sourceDef = sourceNode?.data?.definition as NodeDefinition | undefined;
    const targetDef = targetNode?.data?.definition as NodeDefinition | undefined;

    const sourcePort = sourceDef?.outputs?.find((p) => p.id === connection.sourceHandle);
    const targetPort = targetDef?.inputs?.find((p) => p.id === connection.targetHandle);

    // Validate connection if we have type info
    if (sourcePort && targetPort) {
      const isValid = await workflowService.validateConnection(
        sourcePort.data_type,
        targetPort.data_type
      );

      if (!isValid) {
        console.warn(
          '[WorkflowGraph] Invalid connection:',
          sourcePort.data_type,
          '->',
          targetPort.data_type
        );
        return;
      }
    }

    // Create edge
    const edge: Edge = {
      id: `${connection.source}-${connection.sourceHandle}-${connection.target}-${connection.targetHandle}`,
      source: connection.source!,
      sourceHandle: connection.sourceHandle,
      target: connection.target!,
      targetHandle: connection.targetHandle,
    };

    storeAddEdge(edge);
  }

  // Handle deletion of nodes and edges
  function handleDelete({ nodes: deletedNodes, edges: deletedEdges }: { nodes: Node[]; edges: Edge[] }) {
    if (!$isEditing) return;
    for (const edge of deletedEdges) {
      removeEdge(edge.id);
    }
    for (const node of deletedNodes) {
      removeNode(node.id);
    }
  }

  // Handle drop from palette
  function handleDrop(event: DragEvent) {
    event.preventDefault();

    const data = event.dataTransfer?.getData('application/json');
    if (!data) return;

    const definition: NodeDefinition = JSON.parse(data);

    // Get the SvelteFlow container bounds
    const container = event.currentTarget as HTMLElement;
    const bounds = container.getBoundingClientRect();

    // Convert screen coordinates to approximate flow coordinates
    // Note: This is simplified - in production you'd use the flow's project() function
    const position = {
      x: event.clientX - bounds.left - 100, // Offset for node width
      y: event.clientY - bounds.top - 50, // Offset for node height
    };

    addNode(definition, position);
  }

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
    event.dataTransfer!.dropEffect = 'copy';
  }
</script>

<div
  class="workflow-graph-container w-full h-full"
  ondrop={handleDrop}
  ondragover={handleDragOver}
  role="application"
>
  <SvelteFlow
    bind:nodes
    bind:edges
    {nodeTypes}
    fitViewOptions={{ maxZoom: 1 }}
    nodesConnectable={$isEditing}
    elementsSelectable={true}
    nodesDraggable={true}
    panOnDrag={true}
    zoomOnScroll={true}
    minZoom={0.25}
    maxZoom={2}
    deleteKey={$isEditing ? 'Delete' : null}
    onnodedragstop={onNodeDragStop}
    onconnect={handleConnect}
    ondelete={handleDelete}
    defaultEdgeOptions={{
      type: 'smoothstep',
      animated: false,
      style: 'stroke: #525252; stroke-width: 2px;',
    }}
  >
    <Controls />
    <MiniMap
      nodeColor={(node) => {
        // Color by node category (snake_case to match Rust serde)
        const def = node.data?.definition as NodeDefinition | undefined;
        switch (def?.category) {
          case 'input':
            return '#2563eb';
          case 'processing':
            return '#16a34a';
          case 'tool':
            return '#d97706';
          case 'output':
            return '#0891b2';
          case 'control':
            return '#9333ea';
          default:
            return '#525252';
        }
      }}
      maskColor="rgba(0, 0, 0, 0.8)"
    />
  </SvelteFlow>
</div>

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

  :global(.svelte-flow__edge.selected .svelte-flow__edge-path) {
    stroke: #4f46e5;
    stroke-width: 3px;
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

  :global(.svelte-flow__handle) {
    border-radius: 50%;
  }

  :global(.svelte-flow__handle.connecting) {
    background: #4f46e5 !important;
  }

  :global(.svelte-flow__connection-line) {
    stroke: #4f46e5;
    stroke-width: 2px;
  }
</style>
