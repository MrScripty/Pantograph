<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { SvelteFlow, Controls, MiniMap, type NodeTypes, type EdgeTypes, type Node, type Edge, type Connection } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import {
    HorseshoeInsertSelector,
    clearHorseshoeInsertFeedback,
    clearConnectionDragState,
    clearHorseshoeDragSession,
    createConnectionDragState,
    createHorseshoeInsertFeedbackState,
    createHorseshoeDragSessionState,
    formatHorseshoeBlockedReason,
    findBestInsertableMatchIndex,
    findNearestVisibleHorseshoeIndex,
    isSpaceKey,
    rejectHorseshoeInsertFeedback,
    resolveHorseshoeSpaceKeyAction,
    resolveHorseshoeStatusLabel,
    markConnectionDragFinalizing,
    requestHorseshoeDisplay,
    rotateHorseshoeIndex,
    startHorseshoeInsertFeedback,
    shouldUpdateHorseshoeAnchorFromPointer,
    shouldRemoveReconnectedEdge,
    startHorseshoeDrag,
    startConnectionDrag,
    startReconnectDrag,
    supportsInsertFromConnectionDrag,
    syncHorseshoeDisplay,
    updateHorseshoeAnchor,
    WORKFLOW_PALETTE_DRAG_END_EVENT,
    WORKFLOW_PALETTE_DRAG_START_EVENT,
    type ConnectionDragState,
    type HorseshoeBlockedReason,
    type HorseshoeInsertFeedbackState,
    type HorseshoeDragSessionState,
    isPortTypeCompatible,
  } from '@pantograph/svelte-graph';

  import {
    nodes as nodesStore,
    edges as edgesStore,
    connectionIntent,
    nodeDefinitions,
    isEditing,
    updateNodePosition,
    addNode,
    removeNode,
    syncEdgesFromBackend,
    workflowGraph,
    workflowMetadata,
    setConnectionIntent,
    clearConnectionIntent,
    loadWorkflow,
    selectedNodeIds,
  } from '../stores/workflowStore';
  import { isReadOnly, currentGraphId, currentGraphType } from '../stores/graphSessionStore';
  import type {
    GraphEdge,
    ConnectionAnchor,
    ConnectionCandidatesResponse,
    ConnectionCommitResponse,
    InsertableNodeTypeCandidate,
  } from '../services/workflow/types';
  import { architectureAsWorkflowGraph } from '../stores/architectureStore';
  import { workflowService } from '../services/workflow/WorkflowService';
  import type { NodeDefinition } from '../services/workflow/types';
  import { computeWorkflowGraphSyncDecision } from './workflowGraphSync';
  import {
    applyMatrixToPoint,
    findRenderedEdgePath,
    isCutModifierPressed,
    shouldStartCutGesture,
    toContainerRelativePoint,
  } from './cutInteraction';
  import {
    clearEdgeInsertPreviewState,
    createEdgeInsertPreviewState,
    findEdgeInsertHitTarget,
    setEdgeInsertHoverTarget,
    setEdgeInsertPreviewPending,
    setEdgeInsertPreviewRejected,
    setEdgeInsertPreviewResolved,
    shouldRefreshEdgeInsertPreview,
    updateEdgeInsertHitPoint,
    type EdgeInsertPreviewState,
  } from './edgeInsertInteraction.ts';
  import { resolveReconnectSourceAnchor } from './reconnectInteraction';

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
  import NumberInputNode from './nodes/workflow/NumberInputNode.svelte';
  import BooleanInputNode from './nodes/workflow/BooleanInputNode.svelte';
  import SelectionInputNode from './nodes/workflow/SelectionInputNode.svelte';
  import VectorInputNode from './nodes/workflow/VectorInputNode.svelte';
  import LLMInferenceNode from './nodes/workflow/LLMInferenceNode.svelte';
  import OllamaInferenceNode from './nodes/workflow/OllamaInferenceNode.svelte';
  import LlamaCppInferenceNode from './nodes/workflow/LlamaCppInferenceNode.svelte';
  import EmbeddingNode from './nodes/workflow/EmbeddingNode.svelte';
  import RerankerNode from './nodes/workflow/RerankerNode.svelte';
  import PyTorchInferenceNode from './nodes/workflow/PyTorchInferenceNode.svelte';
  import OnnxInferenceNode from './nodes/workflow/OnnxInferenceNode.svelte';
  import DiffusionInferenceNode from './nodes/workflow/DiffusionInferenceNode.svelte';
  import ModelProviderNode from './nodes/workflow/ModelProviderNode.svelte';
  import TextOutputNode from './nodes/workflow/TextOutputNode.svelte';
  import VectorOutputNode from './nodes/workflow/VectorOutputNode.svelte';
  import ImageOutputNode from './nodes/workflow/ImageOutputNode.svelte';
  import AudioInputNode from './nodes/workflow/AudioInputNode.svelte';
  import AudioOutputNode from './nodes/workflow/AudioOutputNode.svelte';
  import AudioGenerationNode from './nodes/workflow/AudioGenerationNode.svelte';
  import DependencyEnvironmentNode from './nodes/workflow/DependencyEnvironmentNode.svelte';
  import DepthEstimationNode from './nodes/workflow/DepthEstimationNode.svelte';
  import PointCloudOutputNode from './nodes/workflow/PointCloudOutputNode.svelte';
  import GenericNode from './nodes/workflow/GenericNode.svelte';
  import PumaLibNode from './nodes/workflow/PumaLibNode.svelte';
  import AgentToolsNode from './nodes/workflow/AgentToolsNode.svelte';
  import NodeGroupNode from './nodes/workflow/NodeGroupNode.svelte';
  import LinkedInputNode from './nodes/workflow/LinkedInputNode.svelte';
  import MaskedTextInputNode from './nodes/workflow/MaskedTextInputNode.svelte';
  import ExpandSettingsNode from './nodes/workflow/ExpandSettingsNode.svelte';

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
    'number-input': NumberInputNode,
    'boolean-input': BooleanInputNode,
    'selection-input': SelectionInputNode,
    'vector-input': VectorInputNode,
    'llm-inference': LLMInferenceNode,
    'ollama-inference': OllamaInferenceNode,
    'llamacpp-inference': LlamaCppInferenceNode,
    'embedding': EmbeddingNode,
    'reranker': RerankerNode,
    'pytorch-inference': PyTorchInferenceNode,
    'onnx-inference': OnnxInferenceNode,
    'diffusion-inference': DiffusionInferenceNode,
    'model-provider': ModelProviderNode,
    'text-output': TextOutputNode,
    'vector-output': VectorOutputNode,
    'image-output': ImageOutputNode,
    'audio-input': AudioInputNode,
    'audio-output': AudioOutputNode,
    'audio-generation': AudioGenerationNode,
    'dependency-environment': DependencyEnvironmentNode,
    'depth-estimation': DepthEstimationNode,
    'point-cloud-output': PointCloudOutputNode,
    'puma-lib': PumaLibNode,
    'agent-tools': AgentToolsNode,
    'node-group': NodeGroupNode,
    'linked-input': LinkedInputNode,
    'masked-text-input': MaskedTextInputNode,
    'expand-settings': ExpandSettingsNode,
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
  let connectionDragState = $state<ConnectionDragState>(createConnectionDragState());
  let horseshoeSession = $state<HorseshoeDragSessionState>(createHorseshoeDragSessionState());
  let horseshoeInsertFeedback =
    $state<HorseshoeInsertFeedbackState>(createHorseshoeInsertFeedbackState());
  let horseshoeSelectedIndex = $state(0);
  let horseshoeQuery = $state('');
  let externalPaletteDragActive = $state(false);
  let horseshoeQueryResetTimer: ReturnType<typeof setTimeout> | null = null;
  let lastLoggedHorseshoeBlockedReason = $state<HorseshoeBlockedReason | null>(null);
  let horseshoeLastTrace = $state('idle');
  let edgeInsertPreview = $state<EdgeInsertPreviewState>(createEdgeInsertPreviewState());
  let edgeInsertPreviewRequestId = $state(0);
  let currentGraphRevision = $derived($workflowGraph.derived_graph?.graph_fingerprint ?? '');

  // Track previous store references so we only push genuine changes to SvelteFlow.
  // SvelteFlow enriches node/edge objects with internal metadata (measured, internals).
  // Blindly reassigning from the store overwrites that metadata and causes xyflow to
  // re-reconcile, which can drop edges or lose measured dimensions.
  let _prevNodesRef: Node[] | null = null;
  let _prevEdgesRef: Edge[] | null = null;
  let _skipNextNodeSync = false;

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

  function handleWorkflowPaletteDragStart() {
    externalPaletteDragActive = true;
    containerSelected = false;
    clearConnectionInteraction();
  }

  function handleWorkflowPaletteDragEnd() {
    externalPaletteDragActive = false;
    clearEdgeInsertPreview();
  }

  function handleSelectionChange({ nodes: selectedNodes }: { nodes: Node[]; edges: Edge[] }) {
    selectedNodeIds.set(selectedNodes.map((node) => node.id));

    if (selectedNodes.length > 0) {
      containerSelected = false;
    }
  }

  // Handle container border click to select/deselect
  function handleContainerClick(event: MouseEvent) {
    event.stopPropagation();
    containerSelected = !containerSelected;
    console.log('[WorkflowGraph] Container clicked, selected:', containerSelected);
  }

  // Deselect container when clicking on the graph background
  function handlePaneClick() {
    containerSelected = false;
    clearConnectionInteraction();
  }

  // Sync store changes to local state based on graph type
  $effect(() => {
    const graphType = $currentGraphType;
    const graphId = $currentGraphId;
    const archGraph = $architectureAsWorkflowGraph;
    const storeNodes = $nodesStore;
    const storeEdges = $edgesStore;

    console.log('[WorkflowGraph] Syncing graph:', {
      graphType,
      graphId,
      workflowNodeCount: storeNodes.length,
    });

    if (graphType === 'system' && graphId === 'app-architecture') {
      // Load architecture graph
      if (archGraph) {
        nodes = archGraph.nodes;
        edges = archGraph.edges;
      }
    } else {
      const syncDecision = computeWorkflowGraphSyncDecision({
        storeNodes,
        storeEdges,
        prevNodesRef: _prevNodesRef,
        prevEdgesRef: _prevEdgesRef,
        skipNextNodeSync: _skipNextNodeSync,
      });

      _prevNodesRef = syncDecision.nextPrevNodesRef;
      _prevEdgesRef = syncDecision.nextPrevEdgesRef;

      if (syncDecision.applyNodes) {
        nodes = storeNodes;
      }
      _skipNextNodeSync = syncDecision.nextSkipNextNodeSync;

      if (syncDecision.applyEdges) {
        edges = storeEdges;
      }
    }
  });

  // Initialize node definitions on mount
  onMount(async () => {
    window.addEventListener('keydown', handleWindowKeyDown, true);
    window.addEventListener(WORKFLOW_PALETTE_DRAG_START_EVENT, handleWorkflowPaletteDragStart);
    window.addEventListener(WORKFLOW_PALETTE_DRAG_END_EVENT, handleWorkflowPaletteDragEnd);
    window.addEventListener('blur', handleWorkflowPaletteDragEnd);

    const definitions = await workflowService.getNodeDefinitions();
    nodeDefinitions.set(definitions);

    return () => {
      window.removeEventListener('keydown', handleWindowKeyDown, true);
      window.removeEventListener(WORKFLOW_PALETTE_DRAG_START_EVENT, handleWorkflowPaletteDragStart);
      window.removeEventListener(WORKFLOW_PALETTE_DRAG_END_EVENT, handleWorkflowPaletteDragEnd);
      window.removeEventListener('blur', handleWorkflowPaletteDragEnd);
    };
  });

  onDestroy(() => {
    if (horseshoeQueryResetTimer) {
      clearTimeout(horseshoeQueryResetTimer);
    }
  });

  $effect(() => {
    if (!$connectionIntent) {
      if (
        !horseshoeSession.dragActive &&
        !horseshoeInsertFeedback.pending &&
        horseshoeSession.displayState !== 'hidden'
      ) {
        closeHorseshoeSelector();
      }

      const nextSession = syncHorseshoeDisplay(horseshoeSession, getHorseshoeOpenContext());
      if (nextSession !== horseshoeSession) {
        applyHorseshoeSession(nextSession);
      }
      return;
    }

    if ($connectionIntent.insertableNodeTypes.length > 0) {
      horseshoeSelectedIndex = Math.max(
        0,
        Math.min(horseshoeSelectedIndex, $connectionIntent.insertableNodeTypes.length - 1),
      );
    } else {
      horseshoeSelectedIndex = 0;
    }

    const nextSession = syncHorseshoeDisplay(horseshoeSession, getHorseshoeOpenContext());
    if (nextSession !== horseshoeSession) {
      applyHorseshoeSession(nextSession);
    }
  });

  $effect(() => {
    if (!horseshoeSession.blockedReason || horseshoeSession.blockedReason === lastLoggedHorseshoeBlockedReason) {
      return;
    }

    lastLoggedHorseshoeBlockedReason = horseshoeSession.blockedReason;
    console.warn('[WorkflowGraph] Horseshoe blocked:', formatHorseshoeBlockedReason(horseshoeSession.blockedReason));
  });

  function closeHorseshoeSelector() {
    if (
      horseshoeSession.displayState === 'hidden' &&
      !horseshoeSession.openRequested &&
      horseshoeSession.blockedReason === null
    ) {
      return;
    }

    applyHorseshoeSession({
      ...horseshoeSession,
      openRequested: false,
      displayState: 'hidden',
      blockedReason: null,
    });
  }

  function clearConnectionDragTracking() {
    connectionDragState = clearConnectionDragState();
    horseshoeInsertFeedback = clearHorseshoeInsertFeedback();
    applyHorseshoeSession(clearHorseshoeDragSession());
  }

  function clearConnectionInteraction() {
    clearEdgeInsertPreview();
    clearConnectionDragTracking();
    clearConnectionIntent();
  }

  function clearEdgeInsertPreview() {
    edgeInsertPreviewRequestId += 1;
    edgeInsertPreview = clearEdgeInsertPreviewState();
  }

  function applyHorseshoeSession(nextSession: HorseshoeDragSessionState) {
    if (nextSession === horseshoeSession) {
      return;
    }

    const previousDisplayState = horseshoeSession.displayState;
    horseshoeSession = nextSession;
    horseshoeLastTrace = [
      'session',
      nextSession.displayState,
      nextSession.openRequested ? 'requested' : 'idle',
      nextSession.blockedReason ?? 'clear',
      nextSession.anchorPosition ? 'anchor' : 'no-anchor',
    ].join(':');

    if (nextSession.displayState === 'open' && previousDisplayState !== 'open') {
      horseshoeQuery = '';
      horseshoeSelectedIndex = 0;
    }

    if (nextSession.displayState === 'hidden') {
      horseshoeInsertFeedback = clearHorseshoeInsertFeedback();
      horseshoeSelectedIndex = 0;
      horseshoeQuery = '';

      if (horseshoeQueryResetTimer) {
        clearTimeout(horseshoeQueryResetTimer);
        horseshoeQueryResetTimer = null;
      }
    }
  }

  function getHorseshoeOpenContext() {
    return {
      canEdit,
      connectionDragActive: horseshoeSession.dragActive,
      supportsInsert: supportsInsertFromConnectionDrag(connectionDragState),
      hasConnectionIntent: Boolean($connectionIntent),
      insertableCount: $connectionIntent?.insertableNodeTypes.length ?? 0,
      anchorPosition: horseshoeSession.anchorPosition,
    };
  }

  function getRelativePointerPosition(clientX: number, clientY: number) {
    if (!containerElement) return null;
    const bounds = containerElement.getBoundingClientRect();
    return {
      x: clientX - bounds.left,
      y: clientY - bounds.top,
    };
  }

  function getEventPointerPosition(event: MouseEvent | TouchEvent) {
    if ('touches' in event) {
      const touch = event.touches[0] ?? event.changedTouches[0];
      if (!touch) return null;
      return getRelativePointerPosition(touch.clientX, touch.clientY);
    }

    return getRelativePointerPosition(event.clientX, event.clientY);
  }

  function isWorkflowPaletteEdgeInsertEnabled() {
    return !($currentGraphType === 'system' && $currentGraphId === 'app-architecture');
  }

  function getPaletteDragDefinition(event: DragEvent): NodeDefinition | null {
    const data = event.dataTransfer?.getData('application/json');
    if (!data) {
      return null;
    }

    try {
      return JSON.parse(data) as NodeDefinition;
    } catch (error) {
      console.warn('[WorkflowGraph] Failed to parse palette drag data:', error);
      return null;
    }
  }

  function getDropPosition(clientX: number, clientY: number) {
    const pointerPosition = getRelativePointerPosition(clientX, clientY);
    if (!pointerPosition) {
      return null;
    }

    const viewport = currentViewport ?? { x: 0, y: 0, zoom: 1 };
    return {
      x: (pointerPosition.x - viewport.x) / viewport.zoom - 100,
      y: (pointerPosition.y - viewport.y) / viewport.zoom - 50,
    };
  }

  async function refreshEdgeInsertPreview(event: DragEvent, definition: NodeDefinition) {
    if (!externalPaletteDragActive || !isWorkflowPaletteEdgeInsertEnabled()) {
      clearEdgeInsertPreview();
      return;
    }

    const hitPoint = getRelativePointerPosition(event.clientX, event.clientY);
    const flowRoot = containerElement?.querySelector('.svelte-flow');
    const graphRevision = getGraphRevision();
    if (!hitPoint || !flowRoot || !graphRevision) {
      clearEdgeInsertPreview();
      return;
    }

    const hitTarget = findEdgeInsertHitTarget({
      root: flowRoot,
      hitPoint,
      containerRect: flowRoot.getBoundingClientRect(),
    });
    if (!hitTarget) {
      clearEdgeInsertPreview();
      return;
    }

    if (
      !shouldRefreshEdgeInsertPreview(
        edgeInsertPreview,
        hitTarget.edgeId,
        definition.node_type,
        graphRevision,
      )
    ) {
      edgeInsertPreview = updateEdgeInsertHitPoint(edgeInsertPreview, hitTarget.hitPoint);
      return;
    }

    edgeInsertPreview = setEdgeInsertPreviewPending(
      setEdgeInsertHoverTarget(
        edgeInsertPreview,
        hitTarget,
        definition.node_type,
        graphRevision,
      ),
    );

    const requestId = ++edgeInsertPreviewRequestId;
    try {
      const response = await workflowService.previewNodeInsertOnEdge(
        hitTarget.edgeId,
        definition.node_type,
        graphRevision,
      );

      if (
        requestId !== edgeInsertPreviewRequestId ||
        edgeInsertPreview.edgeId !== hitTarget.edgeId ||
        edgeInsertPreview.nodeType !== definition.node_type ||
        edgeInsertPreview.graphRevision !== graphRevision
      ) {
        return;
      }

      if (response.accepted && response.bridge) {
        edgeInsertPreview = setEdgeInsertPreviewResolved(edgeInsertPreview, response.bridge);
        return;
      }

      edgeInsertPreview = setEdgeInsertPreviewRejected(edgeInsertPreview, response.rejection);
    } catch (error) {
      if (
        requestId === edgeInsertPreviewRequestId &&
        edgeInsertPreview.edgeId === hitTarget.edgeId &&
        edgeInsertPreview.nodeType === definition.node_type &&
        edgeInsertPreview.graphRevision === graphRevision
      ) {
        edgeInsertPreview = setEdgeInsertPreviewRejected(edgeInsertPreview);
      }
      console.error('[WorkflowGraph] Failed to preview edge insertion:', error);
    }
  }

  async function commitEdgeInsertDrop(
    definition: NodeDefinition,
    position: { x: number; y: number },
    preview: EdgeInsertPreviewState,
  ) {
    if (!preview.edgeId || !preview.graphRevision || !preview.bridge) {
      return false;
    }

    try {
      const response = await workflowService.insertNodeOnEdge(
        preview.edgeId,
        definition.node_type,
        preview.graphRevision,
        { position },
      );

      if (response.accepted && response.graph) {
        loadWorkflow(response.graph, get(workflowMetadata) ?? undefined);
        return true;
      }

      try {
        const backendGraph = await workflowService.getExecutionGraph();
        syncEdgesFromBackend(backendGraph);
      } catch (refreshError) {
        console.warn('[WorkflowGraph] Failed to refresh graph after rejected edge insertion:', refreshError);
      }

      if (response.rejection) {
        console.warn('[WorkflowGraph] Edge insertion rejected:', response.rejection.message);
      }
    } catch (error) {
      console.error('[WorkflowGraph] Failed to insert node on edge:', error);
    }

    return false;
  }

  function updateDragCursorFromMouseEvent(event: MouseEvent) {
    const nextPosition = getRelativePointerPosition(event.clientX, event.clientY);
    if (!nextPosition) return;

    if (!shouldUpdateHorseshoeAnchorFromPointer(horseshoeSession.displayState)) {
      const nextIndex = horseshoeSession.anchorPosition
        ? findNearestVisibleHorseshoeIndex(
            $connectionIntent?.insertableNodeTypes ?? [],
            horseshoeSelectedIndex,
            nextPosition,
            horseshoeSession.anchorPosition,
          )
        : null;

      if (nextIndex !== null) {
        horseshoeSelectedIndex = nextIndex;
      }
      return;
    }

    applyHorseshoeSession(updateHorseshoeAnchor(horseshoeSession, nextPosition));
  }

  $effect(() => {
    if (!edgeInsertPreview.edgeId) {
      return;
    }

    if (
      !isWorkflowPaletteEdgeInsertEnabled() ||
      !externalPaletteDragActive ||
      !currentGraphRevision ||
      edgeInsertPreview.graphRevision !== currentGraphRevision
    ) {
      clearEdgeInsertPreview();
    }
  });

  $effect(() => {
    const previewEdgeId = edgeInsertPreview.bridge ? edgeInsertPreview.edgeId : null;
    let changed = false;

    const nextEdges = edges.map((edge) => {
      const edgeData = (edge.data ?? {}) as Record<string, unknown>;
      const isPreviewActive = edge.id === previewEdgeId;
      const hasPreviewFlag = edgeData.edgeInsertPreviewActive === true;

      if (isPreviewActive === hasPreviewFlag) {
        return edge;
      }

      changed = true;
      const nextData = { ...edgeData };
      if (isPreviewActive) {
        nextData.edgeInsertPreviewActive = true;
      } else {
        delete nextData.edgeInsertPreviewActive;
      }

      return {
        ...edge,
        data: nextData,
      };
    });

    if (changed) {
      edges = nextEdges;
    }
  });

  function scheduleHorseshoeQueryReset() {
    if (horseshoeQueryResetTimer) {
      clearTimeout(horseshoeQueryResetTimer);
    }

    horseshoeQueryResetTimer = setTimeout(() => {
      horseshoeQuery = '';
      horseshoeQueryResetTimer = null;
    }, 900);
  }

  function requestHorseshoeOpen() {
    horseshoeLastTrace = [
      'request-open',
      horseshoeSession.dragActive ? 'drag' : 'idle',
      connectionDragState.mode,
      $connectionIntent ? 'intent' : 'no-intent',
      `${$connectionIntent?.insertableNodeTypes.length ?? 0}-insertables`,
      horseshoeSession.anchorPosition ? 'anchor' : 'no-anchor',
    ].join(':');
    applyHorseshoeSession(requestHorseshoeDisplay(horseshoeSession, getHorseshoeOpenContext()));
  }

  function rotateInsertSelection(delta: number) {
    if (!$connectionIntent || $connectionIntent.insertableNodeTypes.length === 0) return;

    horseshoeInsertFeedback = clearHorseshoeInsertFeedback();
    horseshoeSelectedIndex = rotateHorseshoeIndex(
      horseshoeSelectedIndex,
      delta,
      $connectionIntent.insertableNodeTypes.length,
    );
  }

  function updateInsertQuery(nextQuery: string) {
    horseshoeInsertFeedback = clearHorseshoeInsertFeedback();
    horseshoeQuery = nextQuery;
    horseshoeSelectedIndex = findBestInsertableMatchIndex(
      $connectionIntent?.insertableNodeTypes ?? [],
      nextQuery,
      horseshoeSelectedIndex,
    );

    if (nextQuery) {
      scheduleHorseshoeQueryReset();
      return;
    }

    if (horseshoeQueryResetTimer) {
      clearTimeout(horseshoeQueryResetTimer);
      horseshoeQueryResetTimer = null;
    }
  }

  function getInsertPositionHint() {
    if (!horseshoeSession.anchorPosition) return null;

    const viewport = currentViewport ?? { x: 0, y: 0, zoom: 1 };
    return {
      position: {
        x: (horseshoeSession.anchorPosition.x - viewport.x) / viewport.zoom,
        y: (horseshoeSession.anchorPosition.y - viewport.y) / viewport.zoom,
      },
    };
  }

  async function commitInsertSelection(candidate: InsertableNodeTypeCandidate) {
    const currentConnectionIntent = $connectionIntent;
    if (
      !currentConnectionIntent ||
      horseshoeInsertFeedback.pending ||
      !supportsInsertFromConnectionDrag(connectionDragState)
    ) return;

    const positionHint = getInsertPositionHint();
    if (!positionHint) return;

    horseshoeInsertFeedback = startHorseshoeInsertFeedback();

    try {
      const response = await workflowService.insertNodeAndConnect(
        currentConnectionIntent.sourceAnchor,
        candidate.node_type,
        currentConnectionIntent.graphRevision || getGraphRevision(),
        positionHint,
        candidate.matching_input_port_ids[0],
      );

      if (response.accepted && response.graph) {
        loadWorkflow(response.graph, get(workflowMetadata) ?? undefined);
        clearConnectionInteraction();
        return;
      }

      horseshoeInsertFeedback = rejectHorseshoeInsertFeedback(response.rejection);
      horseshoeLastTrace = `insert-rejected:${response.rejection?.reason ?? 'unknown'}`;
      await loadConnectionIntent(currentConnectionIntent.sourceAnchor, {
        preserveDisplay: true,
        graphRevision: response.graph_revision,
        rejection: response.rejection,
      });

      if (response.rejection) {
        console.error('[WorkflowGraph] Insert rejected:', {
          reason: response.rejection.reason,
          message: response.rejection.message,
          graphRevision: response.graph_revision,
        });
      }
    } catch (error) {
      horseshoeInsertFeedback = rejectHorseshoeInsertFeedback();
      horseshoeLastTrace = 'insert-error';
      console.error('[WorkflowGraph] Failed to insert compatible node:', error);
    }
  }

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
      // SvelteFlow already has the correct local position via bind:nodes.
      // Skip the next store-to-local node sync so we do not discard internals.
      _skipNextNodeSync = true;
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

  function checkValidConnection(connection: Edge | Connection): boolean {
    if (
      $connectionIntent &&
      connection.source === $connectionIntent.sourceAnchor.node_id &&
      connection.sourceHandle === $connectionIntent.sourceAnchor.port_id &&
      connection.target &&
      connection.targetHandle
    ) {
      return $connectionIntent.compatibleTargetKeys.includes(
        `${connection.target}:${connection.targetHandle}`
      );
    }

    const sourceNode = nodes.find((n) => n.id === connection.source);
    const targetNode = nodes.find((n) => n.id === connection.target);
    const sourceDef = sourceNode?.data?.definition as NodeDefinition | undefined;
    const targetDef = targetNode?.data?.definition as NodeDefinition | undefined;
    const sourcePort = sourceDef?.outputs?.find((p) => p.id === connection.sourceHandle);
    const targetPort = targetDef?.inputs?.find((p) => p.id === connection.targetHandle);
    if (!sourcePort || !targetPort) return true;
    return isPortTypeCompatible(sourcePort.data_type, targetPort.data_type);
  }

  function getGraphRevision(): string {
    return currentGraphRevision;
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
        node.anchors.map((anchor) => `${node.node_id}:${anchor.port_id}`)
      ),
      insertableNodeTypes: candidates.insertable_node_types,
    };
  }

  function setConnectionIntentState(
    candidates: ConnectionCandidatesResponse,
    rejection?: ConnectionCommitResponse['rejection'],
  ) {
    setConnectionIntent({
      ...toConnectionIntentState(candidates),
      rejection,
    });
  }

  let connectionIntentRequestId = $state(0);

  async function loadConnectionIntent(
    sourceAnchor: ConnectionAnchor,
    options?: {
      preserveDisplay?: boolean;
      graphRevision?: string;
      rejection?: ConnectionCommitResponse['rejection'];
    },
  ) {
    if (!canEdit) {
      clearConnectionInteraction();
      return;
    }

    const requestId = ++connectionIntentRequestId;
    if (!options?.preserveDisplay) {
      horseshoeInsertFeedback = clearHorseshoeInsertFeedback();
      closeHorseshoeSelector();
    }

    try {
      const candidates = await workflowService.getConnectionCandidates(
        sourceAnchor,
        undefined,
        options?.graphRevision ?? getGraphRevision()
      );

      if (requestId !== connectionIntentRequestId) return;
      setConnectionIntentState(candidates, options?.rejection);
    } catch (error) {
      if (requestId === connectionIntentRequestId) {
        if (options?.preserveDisplay) {
          setConnectionIntent({
            sourceAnchor,
            graphRevision: options?.graphRevision ?? getGraphRevision(),
            compatibleNodeIds: $connectionIntent?.compatibleNodeIds ?? [],
            compatibleTargetKeys: $connectionIntent?.compatibleTargetKeys ?? [],
            insertableNodeTypes: $connectionIntent?.insertableNodeTypes ?? [],
            rejection: options?.rejection,
          });
        } else {
          clearConnectionInteraction();
        }
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

    const sourceAnchor = {
      node_id: connection.source,
      port_id: connection.sourceHandle,
    };
    const targetAnchor = {
      node_id: connection.target,
      port_id: connection.targetHandle,
    };
    const activeIntent = $connectionIntent;
    const requestedRevision =
      activeIntent &&
      activeIntent.sourceAnchor.node_id === sourceAnchor.node_id &&
      activeIntent.sourceAnchor.port_id === sourceAnchor.port_id
        ? activeIntent.graphRevision
        : getGraphRevision();

    const response = await workflowService.connectAnchors(
      sourceAnchor,
      targetAnchor,
      requestedRevision
    );

    if (response.accepted && response.graph) {
      syncEdgesFromBackend(response.graph);
      clearConnectionInteraction();
      return response;
    }

    try {
      const backendGraph = await workflowService.getExecutionGraph();
      syncEdgesFromBackend(backendGraph);
    } catch (error) {
      console.warn('[WorkflowGraph] Failed to refresh execution graph after rejected connect:', error);
    }

    setConnectionIntent({
      sourceAnchor,
      graphRevision: response.graph_revision,
      compatibleNodeIds: $connectionIntent?.compatibleNodeIds ?? [],
      compatibleTargetKeys: $connectionIntent?.compatibleTargetKeys ?? [],
      insertableNodeTypes: $connectionIntent?.insertableNodeTypes ?? [],
      rejection: response.rejection,
    });

    if (response.rejection) {
      console.warn('[WorkflowGraph] Connection rejected:', response.rejection.message);
    }

    return response;
  }

  async function handleConnectStart(
    _event: MouseEvent | TouchEvent,
    params: { nodeId: string; handleId: string | null; handleType: 'source' | 'target' }
  ) {
    if (!canEdit || params.handleType !== 'source' || !params.handleId) {
      clearConnectionInteraction();
      return;
    }

    const pointerPosition = getEventPointerPosition(_event);
    connectionDragState = startConnectionDrag();
    horseshoeLastTrace = `connect-start:${params.nodeId}:${params.handleId ?? 'unknown'}`;
    applyHorseshoeSession(startHorseshoeDrag(pointerPosition));

    await loadConnectionIntent({
      node_id: params.nodeId,
      port_id: params.handleId,
    });
  }

  function handleConnectEnd(
    _event: MouseEvent | TouchEvent,
    _connectionState: { isValid: boolean }
  ) {
    if (
      horseshoeSession.displayState === 'open' ||
      horseshoeInsertFeedback.pending ||
      horseshoeSession.openRequested
    ) return;
    clearConnectionInteraction();
  }

  // Handle new connections - routes through backend for single source of truth
  async function handleConnect(connection: Connection) {
    if (!canEdit) return;
    try {
      const response = await commitConnection(connection);
      if (!response?.accepted) {
        return;
      }
    } catch (error) {
      console.error('[WorkflowGraph] Failed to add edge:', error);
    }
  }

  // Handle deletion of nodes and edges - edge deletion routes through backend
  async function handleDelete({ nodes: deletedNodes, edges: deletedEdges }: { nodes: Node[]; edges: Edge[] }) {
    if (!canEdit) return;
    clearConnectionInteraction();

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
  async function handleDrop(event: DragEvent) {
    event.preventDefault();

    if (!canEdit) return;

    const definition = getPaletteDragDefinition(event);
    if (!definition) {
      clearConnectionInteraction();
      return;
    }

    const position = getDropPosition(event.clientX, event.clientY);
    const activeEdgeInsertPreview =
      edgeInsertPreview.edgeId &&
      edgeInsertPreview.nodeType === definition.node_type &&
      edgeInsertPreview.graphRevision &&
      edgeInsertPreview.bridge
        ? { ...edgeInsertPreview }
        : null;

    clearConnectionInteraction();
    if (!position) {
      return;
    }

    if (activeEdgeInsertPreview) {
      await commitEdgeInsertDrop(definition, position, activeEdgeInsertPreview);
      return;
    }

    addNode(definition, position);
  }

  async function handleDragOver(event: DragEvent) {
    event.preventDefault();
    if (!canEdit) return;
    event.dataTransfer!.dropEffect = 'copy';

    const definition = getPaletteDragDefinition(event);
    if (!definition) {
      clearEdgeInsertPreview();
      return;
    }

    await refreshEdgeInsertPreview(event, definition);
  }

  // --- Edge Reconnection (drag-off-anchor to disconnect) ---

  async function handleReconnectStart(
    _event: MouseEvent | TouchEvent,
    edge: Edge,
    handleType: 'source' | 'target'
  ) {
    if (!canEdit) return;
    const sourceAnchor = resolveReconnectSourceAnchor(edge, handleType);

    if (sourceAnchor) {
      connectionDragState = startReconnectDrag(edge.id, sourceAnchor);
      applyHorseshoeSession(startHorseshoeDrag(getEventPointerPosition(_event)));
      await loadConnectionIntent(sourceAnchor);
      return;
    }

    clearConnectionInteraction();
  }

  async function handleReconnect(oldEdge: Edge, newConnection: Connection) {
    if (!canEdit) return;
    connectionDragState = markConnectionDragFinalizing(connectionDragState);

    try {
      const graphAfterRemoval = await workflowService.removeEdge(oldEdge.id);
      syncEdgesFromBackend(graphAfterRemoval);

      const response = await workflowService.connectAnchors(
        {
          node_id: newConnection.source!,
          port_id: newConnection.sourceHandle!,
        },
        {
          node_id: newConnection.target!,
          port_id: newConnection.targetHandle!,
        },
        graphAfterRemoval.derived_graph?.graph_fingerprint ?? getGraphRevision()
      );

      if (response.accepted && response.graph) {
        syncEdgesFromBackend(response.graph);
        clearConnectionInteraction();
        return;
      }

      const restoredGraph = await workflowService.addEdge(edgeToGraphEdge(oldEdge));
      syncEdgesFromBackend(restoredGraph);

      if (response.rejection) {
        setConnectionIntent({
          sourceAnchor:
            connectionDragState.reconnectingSourceAnchor ??
            {
              node_id: newConnection.source!,
              port_id: newConnection.sourceHandle!,
            },
          graphRevision: response.graph_revision,
          compatibleNodeIds: $connectionIntent?.compatibleNodeIds ?? [],
          compatibleTargetKeys: $connectionIntent?.compatibleTargetKeys ?? [],
          insertableNodeTypes: $connectionIntent?.insertableNodeTypes ?? [],
          rejection: response.rejection,
        });
        console.warn('[WorkflowGraph] Reconnection rejected:', response.rejection.message);
      }
    } catch (error) {
      console.error('[WorkflowGraph] Failed to reconnect edge:', error);
    }
  }

  async function handleReconnectEnd(_event: MouseEvent | TouchEvent, _edge: Edge, _handleType: unknown, connectionState: { isValid: boolean }) {
    if (!canEdit) return;

    const reconnectingEdgeId = shouldRemoveReconnectedEdge(connectionDragState, connectionState);
    if (reconnectingEdgeId) {
      try {
        const updatedGraph = await workflowService.removeEdge(reconnectingEdgeId);
        syncEdgesFromBackend(updatedGraph);
      } catch (error) {
        console.error('[WorkflowGraph] Failed to remove edge on reconnect end:', error);
      }
    }

    clearConnectionInteraction();
  }

  // --- Cut Tool (Ctrl+drag to cut edges) ---
  let isCutting = $state(false);
  let cutStart = $state<{ x: number; y: number } | null>(null);
  let cutEnd = $state<{ x: number; y: number } | null>(null);
  let cutContainerRect = $state<DOMRect | null>(null);
  let ctrlPressed = $state(false);
  let isFinalizingCut = $state(false);

  function handleKeyDown(e: KeyboardEvent) {
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

    const target = e.target as HTMLElement | null;
    if (
      target &&
      (target.isContentEditable ||
        ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))
    ) {
      return;
    }

    if (horseshoeSession.displayState === 'hidden') {
      if (e.key === 'Escape') {
        clearConnectionInteraction();
      }
      return;
    }

    if (e.key === 'Escape') {
      e.preventDefault();
      closeHorseshoeSelector();
      return;
    }
  }

  function getHorseshoeStatusLabel(): string | null {
    return resolveHorseshoeStatusLabel({
      pending: horseshoeInsertFeedback.pending,
      rejectionMessage: horseshoeInsertFeedback.rejectionMessage,
      displayState: horseshoeSession.displayState,
      blockedReason: horseshoeSession.blockedReason,
    });
  }

  function handleWindowKeyDown(e: KeyboardEvent) {
    if (isCutModifierPressed(e)) {
      ctrlPressed = true;
    }

    const target = e.target as HTMLElement | null;
    if (
      target &&
      (target.isContentEditable ||
        ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))
    ) {
      return;
    }

    if (!horseshoeSession.dragActive && horseshoeSession.displayState === 'hidden') {
      return;
    }

    const spaceAction = isSpaceKey(e)
      ? resolveHorseshoeSpaceKeyAction({
          displayState: horseshoeSession.displayState,
          dragActive: horseshoeSession.dragActive,
          pending: horseshoeInsertFeedback.pending,
          hasSelection: Boolean($connectionIntent?.insertableNodeTypes[horseshoeSelectedIndex]),
        })
      : 'noop';

    if (spaceAction !== 'noop') {
      e.preventDefault();
      horseshoeLastTrace = 'keydown:space';
      if (spaceAction === 'confirm') {
        const candidate = $connectionIntent?.insertableNodeTypes[horseshoeSelectedIndex];
        if (candidate) {
          void commitInsertSelection(candidate);
        }
      } else {
        requestHorseshoeOpen();
      }
      return;
    }

    if (horseshoeSession.displayState === 'hidden') return;

    if (e.key === 'Escape') {
      e.preventDefault();
      closeHorseshoeSelector();
      return;
    }

    if (horseshoeSession.displayState !== 'open') return;

    if (e.key === 'Enter') {
      e.preventDefault();
      const candidate = $connectionIntent?.insertableNodeTypes[horseshoeSelectedIndex];
      if (candidate) {
        void commitInsertSelection(candidate);
      }
      return;
    }

    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      rotateInsertSelection(-1);
      return;
    }

    if (e.key === 'ArrowRight') {
      e.preventDefault();
      rotateInsertSelection(1);
      return;
    }

    if (e.key === 'Backspace') {
      e.preventDefault();
      updateInsertQuery(horseshoeQuery.slice(0, -1));
      return;
    }

    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      updateInsertQuery(`${horseshoeQuery}${e.key}`);
    }
  }

  function handleKeyUp(e: KeyboardEvent) {
    if (!isCutModifierPressed(e) && !ctrlPressed) {
      return;
    }

    ctrlPressed = e.ctrlKey || e.metaKey;
    if (!ctrlPressed && isCutting) {
      void finishCut();
    }
  }

  function handlePaneMouseDown(e: MouseEvent) {
    if (externalPaletteDragActive) {
      return;
    }

    const modifierPressedAtStart = isCutModifierPressed(e);
    ctrlPressed = modifierPressedAtStart;

    if (
      !shouldStartCutGesture({
        canEdit,
        modifierPressed: modifierPressedAtStart,
        target: e.target as HTMLElement | null,
      })
    ) {
      return;
    }

    isCutting = true;
    const container = (e.currentTarget as HTMLElement).querySelector('.svelte-flow');
    if (!container) return;
    const rect = container.getBoundingClientRect();
    cutContainerRect = rect;
    cutStart = { x: e.clientX - rect.left, y: e.clientY - rect.top };
    cutEnd = cutStart;
  }

  function handlePaneMouseMove(e: MouseEvent) {
    updateDragCursorFromMouseEvent(e);
    if (externalPaletteDragActive) {
      return;
    }

    if (!isCutting || !cutStart) return;

    const container = (e.currentTarget as HTMLElement).querySelector('.svelte-flow');
    if (!container) return;
    const rect = container.getBoundingClientRect();
    cutContainerRect = rect;
    cutEnd = { x: e.clientX - rect.left, y: e.clientY - rect.top };
  }

  function handlePaneMouseUp(e: MouseEvent) {
    if (externalPaletteDragActive) {
      return;
    }

    ctrlPressed = isCutModifierPressed(e);
    if (isCutting) {
      void finishCut();
    }
  }

  async function finishCut() {
    if (isFinalizingCut) {
      return;
    }

    isFinalizingCut = true;
    try {
      if (!cutStart || !cutEnd) {
        isCutting = false;
        cutStart = null;
        cutEnd = null;
        cutContainerRect = null;
        return;
      }

      clearConnectionInteraction();

      // Find edges that intersect with the cut line
      const edgesToRemove = edges.filter((edge) => {
        const edgeEl = findRenderedEdgePath(document, edge.id);
        if (!edgeEl) return false;

        return lineIntersectsPath(cutStart!, cutEnd!, edgeEl as SVGPathElement, cutContainerRect);
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
      cutContainerRect = null;
    } finally {
      isFinalizingCut = false;
    }
  }

  // Utility function to check if a line intersects an SVG path
  function lineIntersectsPath(
    p1: { x: number; y: number },
    p2: { x: number; y: number },
    path: SVGPathElement,
    containerRect: DOMRect | null,
  ): boolean {
    const screenMatrix = path.getScreenCTM();
    if (!screenMatrix || !containerRect) {
      return false;
    }

    const pathLength = path.getTotalLength();
    const samples = 20;

    for (let i = 0; i < samples; i++) {
      const t1 = (i / samples) * pathLength;
      const t2 = ((i + 1) / samples) * pathLength;

      const point1 = path.getPointAtLength(t1);
      const point2 = path.getPointAtLength(t2);
      const containerPoint1 = toContainerRelativePoint(
        applyMatrixToPoint(point1, screenMatrix),
        containerRect,
      );
      const containerPoint2 = toContainerRelativePoint(
        applyMatrixToPoint(point2, screenMatrix),
        containerRect,
      );

      if (
        linesIntersect(p1, p2, containerPoint1, containerPoint2)
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

<svelte:window onkeyup={handleKeyUp} onmousemove={updateDragCursorFromMouseEvent} />

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="workflow-graph-container w-full h-full"
  class:cutting={isCutting}
  bind:this={containerElement}
  tabindex={canEdit ? 0 : -1}
  data-horseshoe-blocked-reason={horseshoeSession.blockedReason ?? undefined}
  data-horseshoe-display-state={horseshoeSession.displayState}
  data-horseshoe-last-trace={horseshoeLastTrace}
  ondrop={handleDrop}
  ondragover={handleDragOver}
  onkeydown={handleKeyDown}
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
    nodesConnectable={canEdit && !externalPaletteDragActive}
    elementsSelectable={!externalPaletteDragActive}
    nodesDraggable={canEdit && !externalPaletteDragActive}
    panOnDrag={!ctrlPressed && !externalPaletteDragActive}
    panActivationKey={null}
    zoomOnScroll={true}
    minZoom={0.25}
    maxZoom={2}
    deleteKey={canEdit ? 'Delete' : null}
    edgesReconnectable={canEdit && !externalPaletteDragActive}
    isValidConnection={checkValidConnection}
    onnodedragstop={onNodeDragStop}
    onnodeclick={onNodeClick}
    onselectionchange={handleSelectionChange}
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
    <button
      type="button"
      class="container-edge top"
      onclick={handleContainerClick}
      aria-label="Select orchestration boundary"
      style="
        position: absolute;
        left: {x}px;
        top: {y - edgeWidth/2}px;
        width: {w}px;
        height: {edgeWidth}px;
        border: 0;
        padding: 0;
        background: transparent;
        cursor: pointer;
        pointer-events: auto;
        z-index: 2;
      "
    ></button>
    <button
      type="button"
      class="container-edge bottom"
      onclick={handleContainerClick}
      aria-label="Select orchestration boundary"
      style="
        position: absolute;
        left: {x}px;
        top: {y + h - edgeWidth/2}px;
        width: {w}px;
        height: {edgeWidth}px;
        border: 0;
        padding: 0;
        background: transparent;
        cursor: pointer;
        pointer-events: auto;
        z-index: 2;
      "
    ></button>
    <button
      type="button"
      class="container-edge left"
      onclick={handleContainerClick}
      aria-label="Select orchestration boundary"
      style="
        position: absolute;
        left: {x - edgeWidth/2}px;
        top: {y}px;
        width: {edgeWidth}px;
        height: {h}px;
        border: 0;
        padding: 0;
        background: transparent;
        cursor: pointer;
        pointer-events: auto;
        z-index: 2;
      "
    ></button>
    <button
      type="button"
      class="container-edge right"
      onclick={handleContainerClick}
      aria-label="Select orchestration boundary"
      style="
        position: absolute;
        left: {x + w - edgeWidth/2}px;
        top: {y}px;
        width: {edgeWidth}px;
        height: {h}px;
        border: 0;
        padding: 0;
        background: transparent;
        cursor: pointer;
        pointer-events: auto;
        z-index: 2;
      "
    ></button>

    <!-- Input anchor (left side) -->
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

  {#if edgeInsertPreview.bridge && edgeInsertPreview.hitPoint}
    <div
      class="edge-insert-preview-marker"
      style="
        left: {edgeInsertPreview.hitPoint.x}px;
        top: {edgeInsertPreview.hitPoint.y}px;
      "
      aria-hidden="true"
    >
      <div class="edge-insert-preview-marker-core"></div>
    </div>
  {/if}

  <HorseshoeInsertSelector
    displayState={horseshoeSession.displayState}
    anchorPosition={horseshoeSession.anchorPosition}
    items={$connectionIntent?.insertableNodeTypes ?? []}
    selectedIndex={horseshoeSelectedIndex}
    query={horseshoeQuery}
    pending={horseshoeInsertFeedback.pending}
    statusLabel={getHorseshoeStatusLabel()}
    onSelect={(candidate) => void commitInsertSelection(candidate)}
    onRotate={rotateInsertSelection}
    onCancel={closeHorseshoeSelector}
  />

  {#if horseshoeSession.dragActive || horseshoeSession.displayState !== 'hidden' || horseshoeLastTrace !== 'idle'}
    <div class="horseshoe-debug">
      <div>trace: {horseshoeLastTrace}</div>
      <div>state: {horseshoeSession.displayState}</div>
      {#if horseshoeSession.blockedReason}
        <div>blocked: {horseshoeSession.blockedReason}</div>
      {/if}
    </div>
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

  .edge-insert-preview-marker {
    position: absolute;
    width: 22px;
    height: 22px;
    margin-left: -11px;
    margin-top: -11px;
    border: 1px solid rgba(186, 230, 253, 0.9);
    border-radius: 999px;
    background:
      radial-gradient(circle, rgba(125, 211, 252, 0.24) 0%, rgba(125, 211, 252, 0.08) 58%, transparent 72%);
    box-shadow:
      0 0 0 1px rgba(14, 116, 144, 0.35),
      0 0 16px rgba(125, 211, 252, 0.28);
    pointer-events: none;
    z-index: 1200;
  }

  .edge-insert-preview-marker-core {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 8px;
    height: 8px;
    margin-top: -4px;
    margin-left: -4px;
    border-radius: 999px;
    background: #e0f2fe;
    box-shadow: 0 0 10px rgba(224, 242, 254, 0.8);
  }

  .horseshoe-debug {
    position: absolute;
    top: 0.75rem;
    right: 0.75rem;
    z-index: 1400;
    pointer-events: none;
    padding: 0.5rem 0.65rem;
    border-radius: 0.5rem;
    background: rgba(10, 10, 10, 0.88);
    border: 1px solid rgba(82, 82, 91, 0.9);
    color: #e5e7eb;
    font-size: 0.72rem;
    line-height: 1.35;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.32);
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
