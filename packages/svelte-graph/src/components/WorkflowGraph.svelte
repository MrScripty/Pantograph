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
  import type {
    NodeDefinition,
    GraphEdge,
    ConnectionAnchor,
    ConnectionCandidatesResponse,
    ConnectionCommitResponse,
  } from '../types/workflow.js';
  import { isPortTypeCompatible } from '../portTypeCompatibility.js';
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
  const connectionIntentStore = stores.workflow.connectionIntent;
  const { isEditing, nodeDefinitions: nodeDefsStore, workflowGraph: workflowGraphStore } =
    stores.workflow;
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
  let containerElement = $state<HTMLElement | null>(null);

  // Current viewport state for container border rendering
  let currentViewport = $state<{ x: number; y: number; zoom: number } | null>(null);

  // CutTool ref and bindable state
  let cutToolRef: CutTool;
  let ctrlPressed = $state(false);
  let isCutting = $state(false);

  // ContainerBorder ref
  let containerBorderRef: ContainerBorder;

  // Track previous store references so we only push genuine changes to SvelteFlow.
  // SvelteFlow enriches node/edge objects with internal metadata (measured, internals).
  // Blindly reassigning from the store overwrites that metadata and causes xyflow to
  // re-reconcile, which can drop edges or lose measured dimensions.
  let _prevNodesRef: Node[] | null = null;
  let _prevEdgesRef: Edge[] | null = null;
  let _skipNextNodeSync = false;

  // Sync store → SvelteFlow local state (only when the respective store changed)
  $effect(() => {
    const storeNodes = $nodesStore;
    const storeEdges = $edgesStore;

    const nodesChanged = storeNodes !== _prevNodesRef;
    const edgesChanged = storeEdges !== _prevEdgesRef;

    _prevNodesRef = storeNodes;
    _prevEdgesRef = storeEdges;

    if (nodesChanged && !_skipNextNodeSync) {
      nodes = storeNodes;
    }
    _skipNextNodeSync = false;

    if (edgesChanged) {
      edges = storeEdges;
    }
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

  /** Synchronous connection gate — prevents SvelteFlow from creating invalid edges. */
  function checkValidConnection(connection: Edge | Connection): boolean {
    if (
      $connectionIntentStore &&
      connection.source === $connectionIntentStore.sourceAnchor.node_id &&
      connection.sourceHandle === $connectionIntentStore.sourceAnchor.port_id &&
      connection.target &&
      connection.targetHandle
    ) {
      return $connectionIntentStore.compatibleTargetKeys.includes(
        `${connection.target}:${connection.targetHandle}`,
      );
    }

    const sourceNode = nodes.find((n) => n.id === connection.source);
    const targetNode = nodes.find((n) => n.id === connection.target);
    const sourceDef = sourceNode?.data?.definition as NodeDefinition | undefined;
    const targetDef = targetNode?.data?.definition as NodeDefinition | undefined;
    const sourcePort = sourceDef?.outputs?.find((p) => p.id === connection.sourceHandle);
    const targetPort = targetDef?.inputs?.find((p) => p.id === connection.targetHandle);
    if (!sourcePort || !targetPort) return true; // allow if definitions are missing
    return isPortTypeCompatible(sourcePort.data_type, targetPort.data_type);
  }

  function getGraphRevision(): string {
    return get(workflowGraphStore).derived_graph?.graph_fingerprint ?? '';
  }

  function edgeToGraphEdge(edge: Edge): GraphEdge {
    return {
      id: edge.id,
      source: edge.source,
      source_handle: edge.sourceHandle || 'output',
      target: edge.target,
      target_handle: edge.targetHandle || 'input',
    };
  }

  function toConnectionIntentState(candidates: ConnectionCandidatesResponse) {
    return {
      sourceAnchor: candidates.source_anchor,
      graphRevision: candidates.graph_revision,
      compatibleNodeIds: candidates.compatible_nodes.map((node) => node.node_id),
      compatibleTargetKeys: candidates.compatible_nodes.flatMap((node) =>
        node.anchors.map((anchor) => `${node.node_id}:${anchor.port_id}`),
      ),
      insertableNodeTypes: candidates.insertable_node_types,
    };
  }

  let connectionIntentRequestId = $state(0);

  async function loadConnectionIntent(sourceAnchor: ConnectionAnchor) {
    const sessionId = get(currentSessionId);
    if (!canEdit || !sessionId) {
      stores.workflow.clearConnectionIntent();
      return;
    }

    const requestId = ++connectionIntentRequestId;

    try {
      const candidates = await backend.getConnectionCandidates(
        sourceAnchor,
        sessionId,
        getGraphRevision(),
      );

      if (requestId !== connectionIntentRequestId) return;
      stores.workflow.setConnectionIntent(toConnectionIntentState(candidates));
    } catch (error) {
      if (requestId === connectionIntentRequestId) {
        stores.workflow.clearConnectionIntent();
      }
      console.error('[WorkflowGraph] Failed to load connection candidates:', error);
    }
  }

  async function commitConnection(connection: Connection): Promise<ConnectionCommitResponse | null> {
    if (
      !connection.source ||
      !connection.sourceHandle ||
      !connection.target ||
      !connection.targetHandle
    ) {
      return null;
    }

    const sessionId = get(currentSessionId);
    if (!sessionId) return null;

    const sourceAnchor = {
      node_id: connection.source,
      port_id: connection.sourceHandle,
    };
    const targetAnchor = {
      node_id: connection.target,
      port_id: connection.targetHandle,
    };

    const response = await backend.connectAnchors(
      sourceAnchor,
      targetAnchor,
      sessionId,
      getGraphRevision(),
    );

    if (response.accepted && response.graph) {
      stores.workflow.syncEdgesFromBackend(response.graph);
      stores.workflow.clearConnectionIntent();
      return response;
    }

    stores.workflow.setConnectionIntent({
      sourceAnchor,
      graphRevision: response.graph_revision,
      compatibleNodeIds: $connectionIntentStore?.compatibleNodeIds ?? [],
      compatibleTargetKeys: $connectionIntentStore?.compatibleTargetKeys ?? [],
      insertableNodeTypes: $connectionIntentStore?.insertableNodeTypes ?? [],
      rejection: response.rejection,
    });

    if (response.rejection) {
      console.warn('[WorkflowGraph] Connection rejected:', response.rejection.message);
    }

    return response;
  }

  // --- Event handlers ---

  function onNodeDragStop({
    targetNode,
  }: {
    targetNode: Node | null;
    nodes: Node[];
    event: MouseEvent | TouchEvent;
  }) {
    if (!canEdit || !targetNode) return;
    // Skip overwriting SvelteFlow's nodes on the next $effect run —
    // SvelteFlow already has the correct position via bind:nodes.
    _skipNextNodeSync = true;
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

  async function handleConnectStart(
    _event: MouseEvent | TouchEvent,
    params: { nodeId: string; handleId: string | null; handleType: 'source' | 'target' },
  ) {
    if (!canEdit || params.handleType !== 'source' || !params.handleId) {
      stores.workflow.clearConnectionIntent();
      return;
    }

    await loadConnectionIntent({
      node_id: params.nodeId,
      port_id: params.handleId,
    });
  }

  function handleConnectEnd(
    _event: MouseEvent | TouchEvent,
    _connectionState: { isValid: boolean },
  ) {
    stores.workflow.clearConnectionIntent();
  }

  async function handleConnect(connection: Connection) {
    if (!canEdit) return;

    const response = await commitConnection(connection);
    if (!response?.accepted) return;

    if (connection.sourceHandle === 'inference_settings') {
      const sourceNode = stores.workflow.getNodeById(connection.source!);
      const settings = sourceNode?.data?.inference_settings as
        | Array<{
            key: string;
            label: string;
            param_type: 'Number' | 'Integer' | 'String' | 'Boolean';
            default: unknown;
            description?: string;
            constraints?: {
              min?: number;
              max?: number;
              allowed_values?: unknown[];
            };
          }>
        | undefined;
      if (settings && settings.length > 0) {
        stores.workflow.syncExpandPorts(connection.source!, settings);
        stores.workflow.syncInferencePorts(connection.source!, settings);
      }
    }
  }

  async function handleDelete({ nodes: deletedNodes, edges: deletedEdges }: { nodes: Node[]; edges: Edge[] }) {
    if (!canEdit) return;

    const sessionId = get(currentSessionId) || '';
    stores.workflow.clearConnectionIntent();

    for (const edge of deletedEdges) {
      try {
        const updatedGraph = await backend.removeEdge(edge.id, sessionId);
        stores.workflow.syncEdgesFromBackend(updatedGraph);
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
    stores.workflow.clearConnectionIntent();

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
  let reconnectingSourceAnchor = $state<ConnectionAnchor | null>(null);

  async function handleReconnectStart(
    _event: MouseEvent | TouchEvent,
    edge: Edge,
    handleType: 'source' | 'target',
  ) {
    if (!canEdit) return;
    edgeReconnectSuccessful = false;
    reconnectingEdgeId = edge.id;

    if (handleType === 'target' && edge.sourceHandle) {
      reconnectingSourceAnchor = {
        node_id: edge.source,
        port_id: edge.sourceHandle,
      };
      await loadConnectionIntent(reconnectingSourceAnchor);
      return;
    }

    reconnectingSourceAnchor = null;
    stores.workflow.clearConnectionIntent();
  }

  async function handleReconnect(oldEdge: Edge, newConnection: Connection) {
    if (!canEdit) return;
    edgeReconnectSuccessful = true;

    const sessionId = get(currentSessionId);
    if (!sessionId) return;

    try {
      const graphAfterRemoval = await backend.removeEdge(oldEdge.id, sessionId);
      stores.workflow.syncEdgesFromBackend(graphAfterRemoval);

      const response = await backend.connectAnchors(
        {
          node_id: newConnection.source!,
          port_id: newConnection.sourceHandle!,
        },
        {
          node_id: newConnection.target!,
          port_id: newConnection.targetHandle!,
        },
        sessionId,
        graphAfterRemoval.derived_graph?.graph_fingerprint ?? getGraphRevision(),
      );

      if (response.accepted && response.graph) {
        stores.workflow.syncEdgesFromBackend(response.graph);
        stores.workflow.clearConnectionIntent();
        return;
      }

      const restoredGraph = await backend.addEdge(edgeToGraphEdge(oldEdge), sessionId);
      stores.workflow.syncEdgesFromBackend(restoredGraph);

      if (response.rejection) {
        stores.workflow.setConnectionIntent({
          sourceAnchor:
            reconnectingSourceAnchor ??
            {
              node_id: newConnection.source!,
              port_id: newConnection.sourceHandle!,
            },
          graphRevision: response.graph_revision,
          compatibleNodeIds: $connectionIntentStore?.compatibleNodeIds ?? [],
          compatibleTargetKeys: $connectionIntentStore?.compatibleTargetKeys ?? [],
          insertableNodeTypes: $connectionIntentStore?.insertableNodeTypes ?? [],
          rejection: response.rejection,
        });
        console.warn('[WorkflowGraph] Reconnection rejected:', response.rejection.message);
      }
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
      try {
        const sessionId = get(currentSessionId) || '';
        const updatedGraph = await backend.removeEdge(reconnectingEdgeId, sessionId);
        stores.workflow.syncEdgesFromBackend(updatedGraph);
      } catch (error) {
        console.error('[WorkflowGraph] Failed to notify backend of edge removal:', error);
      }
    }

    reconnectingEdgeId = null;
    reconnectingSourceAnchor = null;
    stores.workflow.clearConnectionIntent();
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
    stores.workflow.clearConnectionIntent();
  }

  // --- Cut tool edge removal ---

  async function handleEdgesCut(edgeIds: string[]) {
    const sessionId = get(currentSessionId) || '';
    stores.workflow.clearConnectionIntent();
    for (const edgeId of edgeIds) {
      try {
        const updatedGraph = await backend.removeEdge(edgeId, sessionId);
        stores.workflow.syncEdgesFromBackend(updatedGraph);
      } catch (error) {
        console.error('[WorkflowGraph] Failed to notify backend of edge cut:', error);
      }
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
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
    isValidConnection={checkValidConnection}
    onnodedragstop={onNodeDragStop}
    onnodeclick={onNodeClick}
    onconnectstart={handleConnectStart}
    onclickconnectstart={handleConnectStart}
    onconnectend={handleConnectEnd}
    onclickconnectend={handleConnectEnd}
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
