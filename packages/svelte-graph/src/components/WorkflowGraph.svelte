<script lang="ts">
  import { onMount } from 'svelte';
  import {
    SvelteFlow,
    Controls,
    MiniMap,
    type NodeTypes,
    type EdgeTypes,
    type Node,
    type Edge,
    type Connection,
  } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import { get } from 'svelte/store';

  import { useGraphContext } from '../context/useGraphContext.js';
  import type { NodeDefinition, GraphEdge } from '../types/workflow.js';
  import CutTool from './CutTool.svelte';
  import ContainerBorder from './ContainerBorder.svelte';
  import ReconnectableEdge from './edges/ReconnectableEdge.svelte';

  const { backend, registry, stores } = useGraphContext();

  interface Props {
    /** Whether to show the orchestration container border overlay */
    showContainerBorder?: boolean;
    /** Called when the container border becomes fully visible (zoom-out transition) */
    onContainerZoomOut?: () => void;
  }

  let { showContainerBorder = false, onContainerZoomOut }: Props = $props();

  // --- Store destructuring for reactive $-prefix access ---
  const nodesStore = stores.workflow.nodes;
  const edgesStore = stores.workflow.edges;
  const { isEditing, nodeDefinitions: nodeDefsStore } = stores.workflow;
  const { isReadOnly, currentSessionId } = stores.session;
  const { viewLevel } = stores.view;

  // Build node/edge types from registry
  const nodeTypes: NodeTypes = registry.nodeTypes as unknown as NodeTypes;
  const edgeTypes: EdgeTypes = (registry.edgeTypes || { reconnectable: ReconnectableEdge }) as unknown as EdgeTypes;

  // Local state for SvelteFlow (Svelte 5 requires $state.raw for xyflow)
  let nodes = $state.raw<Node[]>([]);
  let edges = $state.raw<Edge[]>([]);

  let canEdit = $derived($isEditing && !$isReadOnly);

  // Double-click tracking for group zoom
  let lastClickTime = $state(0);
  let lastClickNodeId = $state<string | null>(null);
  const DOUBLE_CLICK_THRESHOLD = 300;

  // Container element reference for size calculations
  let containerElement: HTMLElement;

  // Current viewport state for container border rendering
  let currentViewport = $state<{ x: number; y: number; zoom: number } | null>(null);

  // CutTool ref and bindable state
  let cutToolRef: CutTool;
  let ctrlPressed = $state(false);
  let isCutting = $state(false);

  // ContainerBorder ref
  let containerBorderRef: ContainerBorder;

  // Sync store to local state
  $effect(() => {
    nodes = $nodesStore;
    edges = $edgesStore;
  });

  // Reset container border transition when returning to data-graph view
  $effect(() => {
    if ($viewLevel === 'data-graph') {
      containerBorderRef?.resetTransition();
    }
  });

  // Initialize node definitions on mount
  onMount(async () => {
    const definitions = await backend.getNodeDefinitions();
    nodeDefsStore.set(definitions);
  });

  // --- Event handlers ---

  function onNodeDragStop({
    targetNode,
  }: {
    targetNode: Node | null;
    nodes: Node[];
    event: MouseEvent | TouchEvent;
  }) {
    if (!canEdit || !targetNode) return;
    stores.workflow.updateNodePosition(targetNode.id, targetNode.position);
  }

  function onNodeClick({ node }: { node: Node }) {
    const now = Date.now();
    if (lastClickNodeId === node.id && now - lastClickTime < DOUBLE_CLICK_THRESHOLD) {
      handleNodeDoubleClick(node);
    }
    lastClickTime = now;
    lastClickNodeId = node.id;
  }

  async function handleNodeDoubleClick(node: Node) {
    const isGroup = node.data?.isGroup === true || node.type === 'node-group';
    if (isGroup) {
      stores.view.zoomTarget.set({
        nodeId: node.id,
        position: node.position,
        bounds: {
          width: (node.measured?.width || node.width || 200) as number,
          height: (node.measured?.height || node.height || 100) as number,
        },
      });
      await stores.view.tabIntoGroup(node.id);
    }
  }

  async function handleConnect(connection: Connection) {
    if (!canEdit) return;

    const sourceNode = nodes.find((n) => n.id === connection.source);
    const targetNode = nodes.find((n) => n.id === connection.target);
    const sourceDef = sourceNode?.data?.definition as NodeDefinition | undefined;
    const targetDef = targetNode?.data?.definition as NodeDefinition | undefined;
    const sourcePort = sourceDef?.outputs?.find((p) => p.id === connection.sourceHandle);
    const targetPort = targetDef?.inputs?.find((p) => p.id === connection.targetHandle);

    if (sourcePort && targetPort) {
      const isValid = await backend.validateConnection(sourcePort.data_type, targetPort.data_type);
      if (!isValid) {
        console.warn('[WorkflowGraph] Invalid connection:', sourcePort.data_type, '->', targetPort.data_type);
        return;
      }
    }

    const edgeId = `${connection.source}-${connection.sourceHandle}-${connection.target}-${connection.targetHandle}`;

    const graphEdge: GraphEdge = {
      id: edgeId,
      source: connection.source!,
      source_handle: connection.sourceHandle!,
      target: connection.target!,
      target_handle: connection.targetHandle!,
    };

    // Add edge to local store (client is source of truth)
    stores.workflow.addEdge({
      id: edgeId,
      source: connection.source!,
      sourceHandle: connection.sourceHandle!,
      target: connection.target!,
      targetHandle: connection.targetHandle!,
    });

    // Notify backend (fire-and-forget)
    try {
      const sessionId = get(currentSessionId) || '';
      await backend.addEdge(graphEdge, sessionId);
    } catch (error) {
      console.error('[WorkflowGraph] Failed to notify backend of edge:', error);
    }
  }

  async function handleDelete({ nodes: deletedNodes, edges: deletedEdges }: { nodes: Node[]; edges: Edge[] }) {
    if (!canEdit) return;

    const sessionId = get(currentSessionId) || '';

    for (const edge of deletedEdges) {
      stores.workflow.removeEdge(edge.id);
      try {
        await backend.removeEdge(edge.id, sessionId);
      } catch (error) {
        console.error('[WorkflowGraph] Failed to notify backend of edge removal:', error);
      }
    }

    for (const node of deletedNodes) {
      stores.workflow.removeNode(node.id);
    }
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    if (!canEdit) return;

    const data = event.dataTransfer?.getData('application/json');
    if (!data) return;

    const definition: NodeDefinition = JSON.parse(data);
    const container = event.currentTarget as HTMLElement;
    const bounds = container.getBoundingClientRect();
    const position = {
      x: event.clientX - bounds.left - 100,
      y: event.clientY - bounds.top - 50,
    };

    stores.workflow.addNode(definition, position);
  }

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
    if (!canEdit) return;
    event.dataTransfer!.dropEffect = 'copy';
  }

  // --- Edge Reconnection ---
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

    const sourceNode = nodes.find((n) => n.id === newConnection.source);
    const targetNode = nodes.find((n) => n.id === newConnection.target);
    const sourceDef = sourceNode?.data?.definition as NodeDefinition | undefined;
    const targetDef = targetNode?.data?.definition as NodeDefinition | undefined;
    const sourcePort = sourceDef?.outputs?.find((p) => p.id === newConnection.sourceHandle);
    const targetPort = targetDef?.inputs?.find((p) => p.id === newConnection.targetHandle);

    if (sourcePort && targetPort) {
      const isValid = await backend.validateConnection(sourcePort.data_type, targetPort.data_type);
      if (!isValid) {
        console.warn('[WorkflowGraph] Invalid reconnection');
        return;
      }
    }

    const newEdgeId = `${newConnection.source}-${newConnection.sourceHandle}-${newConnection.target}-${newConnection.targetHandle}`;

    // Update local store (client is source of truth)
    stores.workflow.removeEdge(oldEdge.id);
    stores.workflow.addEdge({
      id: newEdgeId,
      source: newConnection.source!,
      sourceHandle: newConnection.sourceHandle!,
      target: newConnection.target!,
      targetHandle: newConnection.targetHandle!,
    });

    // Notify backend (fire-and-forget)
    try {
      const sessionId = get(currentSessionId) || '';
      await backend.removeEdge(oldEdge.id, sessionId);
      await backend.addEdge({
        id: newEdgeId,
        source: newConnection.source!,
        source_handle: newConnection.sourceHandle!,
        target: newConnection.target!,
        target_handle: newConnection.targetHandle!,
      }, sessionId);
    } catch (error) {
      console.error('[WorkflowGraph] Failed to notify backend of reconnection:', error);
    }
  }

  async function handleReconnectEnd(
    _event: MouseEvent | TouchEvent,
    _edge: Edge,
    _handleType: unknown,
    connectionState: { isValid: boolean },
  ) {
    if (!canEdit) return;

    if (!connectionState.isValid && reconnectingEdgeId) {
      stores.workflow.removeEdge(reconnectingEdgeId);
      try {
        const sessionId = get(currentSessionId) || '';
        await backend.removeEdge(reconnectingEdgeId, sessionId);
      } catch (error) {
        console.error('[WorkflowGraph] Failed to notify backend of edge removal:', error);
      }
    }

    reconnectingEdgeId = null;
  }

  // --- Viewport handling ---

  function handleMove(
    _event: MouseEvent | TouchEvent | null,
    viewport: { x: number; y: number; zoom: number },
  ) {
    currentViewport = viewport;
  }

  function handleMoveEnd(
    _event: MouseEvent | TouchEvent | null,
    viewport: { x: number; y: number; zoom: number },
  ) {
    currentViewport = viewport;
    containerBorderRef?.checkVisibility();
  }

  function handlePaneClick() {
    containerBorderRef?.deselect();
  }

  // --- Cut tool edge removal ---

  async function handleEdgesCut(edgeIds: string[]) {
    const sessionId = get(currentSessionId) || '';
    for (const edgeId of edgeIds) {
      stores.workflow.removeEdge(edgeId);
      try {
        await backend.removeEdge(edgeId, sessionId);
      } catch (error) {
        console.error('[WorkflowGraph] Failed to notify backend of edge cut:', error);
      }
    }
  }
</script>

<div
  class="workflow-graph-container"
  class:cutting={isCutting}
  bind:this={containerElement}
  ondrop={handleDrop}
  ondragover={handleDragOver}
  onmousedown={(e) => cutToolRef?.onPaneMouseDown(e)}
  onmousemove={(e) => cutToolRef?.onPaneMouseMove(e)}
  onmouseup={() => cutToolRef?.onPaneMouseUp()}
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
        if (node.type === 'node-group' || node.data?.isGroup) return '#7c3aed';
        const def = node.data?.definition as NodeDefinition | undefined;
        switch (def?.category) {
          case 'input': return '#2563eb';
          case 'processing': return '#16a34a';
          case 'tool': return '#d97706';
          case 'output': return '#0891b2';
          case 'control': return '#9333ea';
          default: return '#525252';
        }
      }}
      maskColor="rgba(0, 0, 0, 0.8)"
    />
  </SvelteFlow>

  <ContainerBorder
    bind:this={containerBorderRef}
    {nodes}
    {currentViewport}
    showBorder={showContainerBorder}
    onZoomOut={onContainerZoomOut}
    containerWidth={containerElement?.clientWidth ?? 0}
    containerHeight={containerElement?.clientHeight ?? 0}
  />

  <CutTool
    bind:this={cutToolRef}
    bind:ctrlPressed
    bind:isCutting
    {edges}
    enabled={canEdit}
    onEdgesCut={handleEdgesCut}
  />
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

  .workflow-graph-container {
    width: 100%;
    height: 100%;
    position: relative;
    overflow: hidden;
  }

  .workflow-graph-container.cutting {
    cursor: crosshair;
  }
</style>
