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

  // Import view store for zoom transitions
  import {
    tabIntoGroup,
    zoomTarget,
    zoomToOrchestration,
    viewLevel,
  } from '../stores/viewStore';
  import { currentOrchestration } from '../stores/orchestrationStore';

  // Import workflow node components
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
  import NodeGroupNode from './nodes/workflow/NodeGroupNode.svelte';
  import LinkedInputNode from './nodes/workflow/LinkedInputNode.svelte';


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
    'ollama-inference': OllamaInferenceNode,
    'llamacpp-inference': LlamaCppInferenceNode,
    'model-provider': ModelProviderNode,
    'text-output': TextOutputNode,
    'puma-lib': PumaLibNode,
    'agent-tools': AgentToolsNode,
    'vector-db': VectorDbNode,
    'node-group': NodeGroupNode,
    'linked-input': LinkedInputNode,
    // Generic fallback for other node types
    'image-input': GenericNode,
    'vision-analysis': GenericNode,
    'rag-search': GenericNode,
    'read-file': GenericNode,
    'write-file': GenericNode,
    'component-preview': GenericNode,
    'tool-loop': GenericNode,
    'unload-model': GenericNode,
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

  // Track double-click for group zoom
  let lastClickTime = $state(0);
  let lastClickNodeId = $state<string | null>(null);
  const DOUBLE_CLICK_THRESHOLD = 300; // ms

  // --- Container border and zoom-out transition ---
  // Container margin around all nodes (represents orchestration node padding)
  const CONTAINER_MARGIN = 100;
  // Extra margin needed for visibility check before transition
  const VISIBILITY_MARGIN = 50;

  // Track if we've already triggered the zoom-out transition
  let transitionTriggered = $state(false);

  // Track if the container border is selected
  let containerSelected = $state(false);

  // Container element reference for size calculations
  let containerElement: HTMLElement;

  // Current viewport state for rendering the container border
  let currentViewport = $state<{ x: number; y: number; zoom: number } | null>(null);

  // Calculate container bounds from all nodes (represents orchestration node boundary)
  let containerBounds = $derived.by(() => {
    if (nodes.length === 0) return null;

    // Find bounding box of all nodes
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;

    for (const node of nodes) {
      const width = (node.measured?.width || node.width || 200) as number;
      const height = (node.measured?.height || node.height || 100) as number;

      minX = Math.min(minX, node.position.x);
      minY = Math.min(minY, node.position.y);
      maxX = Math.max(maxX, node.position.x + width);
      maxY = Math.max(maxY, node.position.y + height);
    }

    return {
      x: minX - CONTAINER_MARGIN,
      y: minY - CONTAINER_MARGIN,
      width: (maxX - minX) + (CONTAINER_MARGIN * 2),
      height: (maxY - minY) + (CONTAINER_MARGIN * 2),
    };
  });

  // Check if container is fully visible within the viewport
  function isContainerFullyVisible(
    bounds: { x: number; y: number; width: number; height: number },
    viewport: { x: number; y: number; zoom: number },
    screenWidth: number,
    screenHeight: number
  ): boolean {
    // Convert flow coordinates to screen coordinates
    const screenX = bounds.x * viewport.zoom + viewport.x;
    const screenY = bounds.y * viewport.zoom + viewport.y;
    const screenW = bounds.width * viewport.zoom;
    const screenH = bounds.height * viewport.zoom;

    // Check if container fits within viewport with margin
    return (
      screenX >= VISIBILITY_MARGIN &&
      screenY >= VISIBILITY_MARGIN &&
      screenX + screenW <= screenWidth - VISIBILITY_MARGIN &&
      screenY + screenH <= screenHeight - VISIBILITY_MARGIN
    );
  }

  // Handle viewport changes during pan/zoom for border rendering
  function handleMove(_event: MouseEvent | TouchEvent | null, viewport: { x: number; y: number; zoom: number }) {
    currentViewport = viewport;
  }

  // Handle viewport changes to detect when to transition to orchestration view
  function handleMoveEnd(_event: MouseEvent | TouchEvent | null, viewport: { x: number; y: number; zoom: number }) {
    // Always update current viewport for border rendering
    currentViewport = viewport;

    // Debug logging to diagnose zoom-out transition
    console.log('[WorkflowGraph] handleMoveEnd:', {
      hasContainerBounds: !!containerBounds,
      hasContainerElement: !!containerElement,
      currentOrchestration: $currentOrchestration,
      zoom: viewport.zoom,
    });

    if (!containerBounds || !containerElement || $currentOrchestration === null) return;

    const screenWidth = containerElement.clientWidth;
    const screenHeight = containerElement.clientHeight;

    const fullyVisible = isContainerFullyVisible(containerBounds, viewport, screenWidth, screenHeight);

    // Trigger transition when container becomes fully visible
    if (fullyVisible && !transitionTriggered) {
      transitionTriggered = true;
      zoomToOrchestration();
    }

    // Reset trigger when zoomed back in (container not fully visible)
    if (!fullyVisible) {
      transitionTriggered = false;
    }
  }

  // Reset transition state when returning to data-graph view
  $effect(() => {
    if ($viewLevel === 'data-graph') {
      transitionTriggered = false;
    }
  });

  // Handle container border click to select/deselect
  function handleContainerClick(event: MouseEvent) {
    event.stopPropagation();
    containerSelected = !containerSelected;
    console.log('[WorkflowGraph] Container clicked, selected:', containerSelected);
  }

  // Deselect container when clicking on the graph background
  function handlePaneClick() {
    containerSelected = false;
  }

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

  // Handle node click for double-click detection (to zoom into groups)
  function onNodeClick({ node }: { node: Node }) {
    const now = Date.now();
    const isDoubleClick =
      lastClickNodeId === node.id && now - lastClickTime < DOUBLE_CLICK_THRESHOLD;

    if (isDoubleClick) {
      handleNodeDoubleClick(node);
    }

    lastClickTime = now;
    lastClickNodeId = node.id;
  }

  // Handle double-click on a node to zoom into it (for node groups)
  async function handleNodeDoubleClick(node: Node) {
    // Check if this node is a group (will be determined by Workstream B's NodeGroup type)
    const isNodeGroup = node.data?.isGroup === true || node.type === 'node-group';

    if (isNodeGroup) {
      // Update zoom target position for animation origin
      zoomTarget.set({
        nodeId: node.id,
        position: node.position,
        bounds: {
          width: (node.measured?.width || node.width || 200) as number,
          height: (node.measured?.height || node.height || 100) as number,
        },
      });

      // Trigger zoom into group
      await tabIntoGroup(node.id);
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
    // Tab transitions to orchestration view when container is selected
    if (e.key === 'Tab') {
      console.log('[WorkflowGraph] Tab pressed, containerSelected:', containerSelected);
      if (containerSelected) {
        e.preventDefault();
        containerSelected = false;
        console.log('[WorkflowGraph] Transitioning to orchestration view');
        zoomToOrchestration();
      }
    }
    // Escape deselects the container
    if (e.key === 'Escape' && containerSelected) {
      e.preventDefault();
      containerSelected = false;
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
  bind:this={containerElement}
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
    onnodeclick={onNodeClick}
    onconnect={handleConnect}
    ondelete={handleDelete}
    onreconnectstart={handleReconnectStart}
    onreconnect={handleReconnect}
    onreconnectend={handleReconnectEnd}
    onmove={handleMove}
    onmoveend={handleMoveEnd}
    onpaneclick={handlePaneClick}
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
        // Node groups get a special purple color
        if (node.type === 'node-group' || node.data?.isGroup) {
          return '#7c3aed';
        }
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

  <!-- Container border overlay (represents orchestration node boundary) -->
  <!-- Uses edge divs for click detection so interior doesn't block canvas panning -->
  {#if containerBounds && currentViewport}
    {@const x = containerBounds.x * currentViewport.zoom + currentViewport.x}
    {@const y = containerBounds.y * currentViewport.zoom + currentViewport.y}
    {@const w = containerBounds.width * currentViewport.zoom}
    {@const h = containerBounds.height * currentViewport.zoom}
    {@const edgeWidth = 12}

    <!-- Visual border (pointer-events: none) -->
    <div
      class="container-border-visual"
      style="
        position: absolute;
        left: {x}px;
        top: {y}px;
        width: {w}px;
        height: {h}px;
        border: 3px solid {containerSelected ? '#93c5fd' : '#60a5fa'};
        border-radius: 8px;
        pointer-events: none;
        z-index: 1;
        box-shadow:
          0 0 15px rgba(96, 165, 250, 0.4),
          0 0 30px rgba(96, 165, 250, 0.2),
          inset 0 0 15px rgba(96, 165, 250, 0.05);
        transition: border-color 0.15s ease, box-shadow 0.15s ease;
      "
    ></div>

    <!-- Clickable edge zones (invisible, only for click detection) -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="container-edge top"
      onclick={handleContainerClick}
      style="
        position: absolute;
        left: {x}px;
        top: {y - edgeWidth/2}px;
        width: {w}px;
        height: {edgeWidth}px;
        cursor: pointer;
        pointer-events: auto;
        z-index: 2;
      "
    ></div>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="container-edge bottom"
      onclick={handleContainerClick}
      style="
        position: absolute;
        left: {x}px;
        top: {y + h - edgeWidth/2}px;
        width: {w}px;
        height: {edgeWidth}px;
        cursor: pointer;
        pointer-events: auto;
        z-index: 2;
      "
    ></div>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="container-edge left"
      onclick={handleContainerClick}
      style="
        position: absolute;
        left: {x - edgeWidth/2}px;
        top: {y}px;
        width: {edgeWidth}px;
        height: {h}px;
        cursor: pointer;
        pointer-events: auto;
        z-index: 2;
      "
    ></div>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="container-edge right"
      onclick={handleContainerClick}
      style="
        position: absolute;
        left: {x + w - edgeWidth/2}px;
        top: {y}px;
        width: {edgeWidth}px;
        height: {h}px;
        cursor: pointer;
        pointer-events: auto;
        z-index: 2;
      "
    ></div>

    <!-- Input anchor (left side) -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="container-anchor input"
      style="
        position: absolute;
        left: {x - 8}px;
        top: {y + h / 2 - 8}px;
        width: 16px;
        height: 16px;
        background: #3b82f6;
        border: 2px solid #1e3a5f;
        border-radius: 50%;
        pointer-events: auto;
        z-index: 3;
        box-shadow: 0 0 8px rgba(59, 130, 246, 0.6);
      "
    ></div>
    <!-- Output anchor (right side) -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="container-anchor output"
      style="
        position: absolute;
        left: {x + w - 8}px;
        top: {y + h / 2 - 8}px;
        width: 16px;
        height: 16px;
        background: #3b82f6;
        border: 2px solid #1e3a5f;
        border-radius: 50%;
        pointer-events: auto;
        z-index: 3;
        box-shadow: 0 0 8px rgba(59, 130, 246, 0.6);
      "
    ></div>
  {/if}

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
    stroke: #60a5fa;
    stroke-width: 2px;
    filter: drop-shadow(0 0 3px rgba(96, 165, 250, 0.6));
  }

  :global(.svelte-flow__edge.selected .svelte-flow__edge-path) {
    stroke: #93c5fd;
    stroke-width: 3px;
    filter: drop-shadow(0 0 6px rgba(147, 197, 253, 0.8));
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
    background: #60a5fa !important;
    box-shadow: 0 0 10px rgba(96, 165, 250, 0.8);
  }

  :global(.svelte-flow__connection-line) {
    stroke: #60a5fa;
    stroke-width: 2px;
    filter: drop-shadow(0 0 4px rgba(96, 165, 250, 0.7));
  }

  /* Cut tool styles */
  .workflow-graph-container {
    position: relative;
    overflow: hidden;
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
