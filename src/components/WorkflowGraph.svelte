<script lang="ts">
  import { onMount } from 'svelte';
  import { SvelteFlow, Controls, MiniMap, type NodeTypes, type EdgeTypes, type Node, type Edge, type Connection } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  import {
    nodes as nodesStore,
    edges as edgesStore,
    nodeDefinitions,
    isEditing,
    updateNodePosition,
    addNode,
    removeNode,
    syncEdgesFromBackend,
  } from '../stores/workflowStore';
  import { isReadOnly, currentGraphId, currentGraphType } from '../stores/graphSessionStore';
  import type { GraphEdge } from '../services/workflow/types';
  import { architectureAsWorkflowGraph } from '../stores/architectureStore';
  import { workflowService } from '../services/workflow/WorkflowService';
  import type { NodeDefinition } from '../services/workflow/types';

  // Import workflow node components
  import TextInputNode from './nodes/workflow/TextInputNode.svelte';
  import LLMInferenceNode from './nodes/workflow/LLMInferenceNode.svelte';
  import TextOutputNode from './nodes/workflow/TextOutputNode.svelte';
  import GenericNode from './nodes/workflow/GenericNode.svelte';
  import SystemPromptNode from './nodes/workflow/SystemPromptNode.svelte';
  import PumaLibNode from './nodes/workflow/PumaLibNode.svelte';
  import AgentToolsNode from './nodes/workflow/AgentToolsNode.svelte';
  import VectorDbNode from './nodes/workflow/VectorDbNode.svelte';

  // Import architecture node components
  import ArchComponentNode from './nodes/architecture/ArchComponentNode.svelte';
  import ArchServiceNode from './nodes/architecture/ArchServiceNode.svelte';
  import ArchStoreNode from './nodes/architecture/ArchStoreNode.svelte';
  import ArchBackendNode from './nodes/architecture/ArchBackendNode.svelte';
  import ArchCommandNode from './nodes/architecture/ArchCommandNode.svelte';

  // Import custom edge components
  import ReconnectableEdge from './edges/ReconnectableEdge.svelte';

  // Define custom edge types
  const edgeTypes: EdgeTypes = {
    reconnectable: ReconnectableEdge,
  };

  // Define custom node types for workflow
  const nodeTypes: NodeTypes = {
    'text-input': TextInputNode,
    'llm-inference': LLMInferenceNode,
    'text-output': TextOutputNode,
    'system-prompt': SystemPromptNode,
    'puma-lib': PumaLibNode,
    'agent-tools': AgentToolsNode,
    'vector-db': VectorDbNode,
    // Generic fallback for other node types
    'image-input': GenericNode,
    'vision-analysis': GenericNode,
    'rag-search': GenericNode,
    'read-file': GenericNode,
    'write-file': GenericNode,
    'component-preview': GenericNode,
    'tool-loop': GenericNode,
    // Architecture node types
    'arch-component': ArchComponentNode,
    'arch-service': ArchServiceNode,
    'arch-store': ArchStoreNode,
    'arch-backend': ArchBackendNode,
    'arch-command': ArchCommandNode,
  };

  // Local state for SvelteFlow (Svelte 5 requires $state.raw for xyflow)
  let nodes = $state.raw<Node[]>([]);
  let edges = $state.raw<Edge[]>([]);

  // Determine if we can edit based on both isEditing store and isReadOnly
  let canEdit = $derived($isEditing && !$isReadOnly);

  // Sync store changes to local state based on graph type
  // Combining into a single effect to ensure proper reactivity tracking
  $effect(() => {
    const graphType = $currentGraphType;
    const graphId = $currentGraphId;
    const archGraph = $architectureAsWorkflowGraph;
    const workflowNodes = $nodesStore;
    const workflowEdges = $edgesStore;

    console.log('[WorkflowGraph] Syncing graph:', { graphType, graphId, workflowNodeCount: workflowNodes.length });

    if (graphType === 'system' && graphId === 'app-architecture') {
      // Load architecture graph
      if (archGraph) {
        nodes = archGraph.nodes;
        edges = archGraph.edges;
      }
    } else {
      // Load workflow graph from store
      nodes = workflowNodes;
      edges = workflowEdges;
    }
  });

  // Initialize node definitions on mount
  onMount(async () => {
    const definitions = await workflowService.getNodeDefinitions();
    nodeDefinitions.set(definitions);
  });

  // Handle node drag events - sync back to store
  function onNodeDragStop({
    targetNode,
  }: {
    targetNode: Node | null;
    nodes: Node[];
    event: MouseEvent | TouchEvent;
  }) {
    if (!canEdit) return;
    if (targetNode) {
      updateNodePosition(targetNode.id, targetNode.position);
    }
  }

  // Handle new connections - routes through backend for single source of truth
  async function handleConnect(connection: Connection) {
    if (!canEdit) return;

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

    // Create edge via backend
    const edge: GraphEdge = {
      id: `${connection.source}-${connection.sourceHandle}-${connection.target}-${connection.targetHandle}`,
      source: connection.source!,
      source_handle: connection.sourceHandle!,
      target: connection.target!,
      target_handle: connection.targetHandle!,
    };

    try {
      const updatedGraph = await workflowService.addEdge(edge);
      syncEdgesFromBackend(updatedGraph);
    } catch (error) {
      console.error('[WorkflowGraph] Failed to add edge:', error);
    }
  }

  // Handle deletion of nodes and edges - edge deletion routes through backend
  async function handleDelete({ nodes: deletedNodes, edges: deletedEdges }: { nodes: Node[]; edges: Edge[] }) {
    if (!canEdit) return;

    // Delete edges via backend
    for (const edge of deletedEdges) {
      try {
        const updatedGraph = await workflowService.removeEdge(edge.id);
        syncEdgesFromBackend(updatedGraph);
      } catch (error) {
        console.error('[WorkflowGraph] Failed to remove edge:', error);
      }
    }

    // Delete nodes (still local for now - could be moved to backend later)
    for (const node of deletedNodes) {
      removeNode(node.id);
    }
  }

  // Handle drop from palette
  function handleDrop(event: DragEvent) {
    event.preventDefault();

    if (!canEdit) return;

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
    if (!canEdit) return;
    event.dataTransfer!.dropEffect = 'copy';
  }

  // --- Edge Reconnection (drag-off-anchor to disconnect) ---
  let edgeReconnectSuccessful = $state(false);
  let reconnectingEdgeId = $state<string | null>(null);

  function handleReconnectStart(_event: MouseEvent | TouchEvent, edge: Edge) {
    if (!canEdit) return;
    edgeReconnectSuccessful = false;
    reconnectingEdgeId = edge.id;
  }

  async function handleReconnect(oldEdge: Edge, newConnection: Connection) {
    if (!canEdit) return;
    edgeReconnectSuccessful = true;

    // Validate the new connection
    const sourceNode = nodes.find((n) => n.id === newConnection.source);
    const targetNode = nodes.find((n) => n.id === newConnection.target);
    const sourceDef = sourceNode?.data?.definition as NodeDefinition | undefined;
    const targetDef = targetNode?.data?.definition as NodeDefinition | undefined;
    const sourcePort = sourceDef?.outputs?.find((p) => p.id === newConnection.sourceHandle);
    const targetPort = targetDef?.inputs?.find((p) => p.id === newConnection.targetHandle);

    if (sourcePort && targetPort) {
      const isValid = await workflowService.validateConnection(
        sourcePort.data_type,
        targetPort.data_type
      );
      if (!isValid) {
        console.warn('[WorkflowGraph] Invalid reconnection');
        return;
      }
    }

    try {
      // Remove old edge via backend
      await workflowService.removeEdge(oldEdge.id);

      // Add new edge via backend
      const newEdge: GraphEdge = {
        id: `${newConnection.source}-${newConnection.sourceHandle}-${newConnection.target}-${newConnection.targetHandle}`,
        source: newConnection.source!,
        source_handle: newConnection.sourceHandle!,
        target: newConnection.target!,
        target_handle: newConnection.targetHandle!,
      };
      const updatedGraph = await workflowService.addEdge(newEdge);
      syncEdgesFromBackend(updatedGraph);
    } catch (error) {
      console.error('[WorkflowGraph] Failed to reconnect edge:', error);
    }
  }

  async function handleReconnectEnd(_event: MouseEvent | TouchEvent, _edge: Edge, _handleType: unknown, connectionState: { isValid: boolean }) {
    if (!canEdit) return;

    // If reconnect was not successful (dropped on empty space), remove the edge
    if (!connectionState.isValid && reconnectingEdgeId) {
      try {
        const updatedGraph = await workflowService.removeEdge(reconnectingEdgeId);
        syncEdgesFromBackend(updatedGraph);
      } catch (error) {
        console.error('[WorkflowGraph] Failed to remove edge on reconnect end:', error);
      }
    }

    reconnectingEdgeId = null;
  }

  // --- Cut Tool (Ctrl+drag to cut edges) ---
  let isCutting = $state(false);
  let cutStart = $state<{ x: number; y: number } | null>(null);
  let cutEnd = $state<{ x: number; y: number } | null>(null);
  let ctrlPressed = $state(false);

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Control') {
      ctrlPressed = true;
    }
  }

  function handleKeyUp(e: KeyboardEvent) {
    if (e.key === 'Control') {
      ctrlPressed = false;
      if (isCutting) {
        finishCut();
      }
    }
  }

  function handlePaneMouseDown(e: MouseEvent) {
    if (!canEdit || !ctrlPressed) return;

    // Only start cut if clicking on the pane (not on a node)
    const target = e.target as HTMLElement;
    if (target.closest('.svelte-flow__node') || target.closest('.svelte-flow__handle')) return;

    isCutting = true;
    const container = (e.currentTarget as HTMLElement).querySelector('.svelte-flow');
    if (!container) return;
    const rect = container.getBoundingClientRect();
    cutStart = { x: e.clientX - rect.left, y: e.clientY - rect.top };
    cutEnd = cutStart;
  }

  function handlePaneMouseMove(e: MouseEvent) {
    if (!isCutting || !cutStart) return;

    const container = (e.currentTarget as HTMLElement).querySelector('.svelte-flow');
    if (!container) return;
    const rect = container.getBoundingClientRect();
    cutEnd = { x: e.clientX - rect.left, y: e.clientY - rect.top };
  }

  function handlePaneMouseUp() {
    if (isCutting) {
      finishCut();
    }
  }

  async function finishCut() {
    if (!cutStart || !cutEnd) {
      isCutting = false;
      cutStart = null;
      cutEnd = null;
      return;
    }

    // Find edges that intersect with the cut line
    const edgesToRemove = edges.filter((edge) => {
      const edgeEl = document.querySelector(`[data-id="${edge.id}"] path`);
      if (!edgeEl) return false;

      return lineIntersectsPath(cutStart!, cutEnd!, edgeEl as SVGPathElement);
    });

    // Remove intersecting edges via backend
    for (const edge of edgesToRemove) {
      try {
        const updatedGraph = await workflowService.removeEdge(edge.id);
        syncEdgesFromBackend(updatedGraph);
      } catch (error) {
        console.error('[WorkflowGraph] Failed to remove edge via cut:', error);
      }
    }

    isCutting = false;
    cutStart = null;
    cutEnd = null;
  }

  // Utility function to check if a line intersects an SVG path
  function lineIntersectsPath(
    p1: { x: number; y: number },
    p2: { x: number; y: number },
    path: SVGPathElement
  ): boolean {
    const pathLength = path.getTotalLength();
    const samples = 20;

    for (let i = 0; i < samples; i++) {
      const t1 = (i / samples) * pathLength;
      const t2 = ((i + 1) / samples) * pathLength;

      const point1 = path.getPointAtLength(t1);
      const point2 = path.getPointAtLength(t2);

      if (
        linesIntersect(p1, p2, { x: point1.x, y: point1.y }, { x: point2.x, y: point2.y })
      ) {
        return true;
      }
    }
    return false;
  }

  // Line-line intersection check
  function linesIntersect(
    a1: { x: number; y: number },
    a2: { x: number; y: number },
    b1: { x: number; y: number },
    b2: { x: number; y: number }
  ): boolean {
    const det = (a2.x - a1.x) * (b2.y - b1.y) - (b2.x - b1.x) * (a2.y - a1.y);
    if (det === 0) return false;

    const lambda = ((b2.y - b1.y) * (b2.x - a1.x) + (b1.x - b2.x) * (b2.y - a1.y)) / det;
    const gamma = ((a1.y - a2.y) * (b2.x - a1.x) + (a2.x - a1.x) * (b2.y - a1.y)) / det;

    return 0 < lambda && lambda < 1 && 0 < gamma && gamma < 1;
  }
</script>

<svelte:window onkeydown={handleKeyDown} onkeyup={handleKeyUp} />

<div
  class="workflow-graph-container w-full h-full"
  class:cutting={isCutting}
  ondrop={handleDrop}
  ondragover={handleDragOver}
  onmousedown={handlePaneMouseDown}
  onmousemove={handlePaneMouseMove}
  onmouseup={handlePaneMouseUp}
  role="application"
>
  <SvelteFlow
    bind:nodes
    bind:edges
    {nodeTypes}
    {edgeTypes}
    fitViewOptions={{ maxZoom: 1 }}
    nodesConnectable={canEdit}
    elementsSelectable={true}
    nodesDraggable={canEdit}
    panOnDrag={!ctrlPressed}
    zoomOnScroll={true}
    minZoom={0.25}
    maxZoom={2}
    deleteKey={canEdit ? 'Delete' : null}
    edgesReconnectable={canEdit}
    onnodedragstop={onNodeDragStop}
    onconnect={handleConnect}
    ondelete={handleDelete}
    onreconnectstart={handleReconnectStart}
    onreconnect={handleReconnect}
    onreconnectend={handleReconnectEnd}
    defaultEdgeOptions={{
      type: 'reconnectable',
      animated: false,
      style: 'stroke: #525252; stroke-width: 2px;',
      interactionWidth: 20,
      selectable: true,
      focusable: true,
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

  <!-- Cut line overlay -->
  {#if isCutting && cutStart && cutEnd}
    <svg class="cut-overlay">
      <line
        x1={cutStart.x}
        y1={cutStart.y}
        x2={cutEnd.x}
        y2={cutEnd.y}
        stroke="#ef4444"
        stroke-width="2"
        stroke-dasharray="5,5"
      />
    </svg>
  {/if}
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

  /* Cut tool styles */
  .workflow-graph-container {
    position: relative;
  }

  .workflow-graph-container.cutting {
    cursor: crosshair;
  }

  .cut-overlay {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 1000;
  }
</style>
