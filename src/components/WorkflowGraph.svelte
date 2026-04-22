<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { SvelteFlow, Controls, MiniMap, type Node, type Edge, type Connection } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import {
    HorseshoeInsertSelector,
    clearHorseshoeInsertFeedback,
    clearConnectionDragState,
    closeHorseshoeDisplay,
    clearHorseshoeDragSession,
    createConnectionDragState,
    createHorseshoeInsertFeedbackState,
    createHorseshoeDragSessionState,
    formatHorseshoeBlockedReason,
    rejectHorseshoeInsertFeedback,
    resolveHorseshoeSessionStatusLabel,
    isEditableKeyboardTarget,
    resolveHorseshoeKeyboardAction,
    preserveConnectionIntentState,
    buildWorkflowHorseshoeOpenContext,
    formatWorkflowHorseshoeSessionTrace,
    normalizeWorkflowHorseshoeSelectedIndex,
    resolveWorkflowHorseshoeQueryUpdate,
    requestWorkflowHorseshoeOpen,
    resolveWorkflowHorseshoeSelectionSnapshot,
    resolveWorkflowDragCursorUpdate,
    resolveWorkflowGroupZoomTarget,
    resolveWorkflowInsertPositionHint,
    resolveWorkflowNodeClick,
    resolveWorkflowPointerClientPosition,
    resolveWorkflowRelativePointerPosition,
    markConnectionDragFinalizing,
    rotateWorkflowHorseshoeSelection,
    startHorseshoeInsertFeedback,
    shouldRemoveReconnectedEdge,
    startHorseshoeDrag,
    startConnectionDrag,
    startReconnectDrag,
    supportsInsertFromConnectionDrag,
    syncHorseshoeDisplay,
    applyWorkflowGraphMutationResponse,
    WORKFLOW_PALETTE_DRAG_END_EVENT,
    WORKFLOW_PALETTE_DRAG_START_EVENT,
    CutTool,
    type ConnectionDragState,
    type HorseshoeBlockedReason,
    type HorseshoeInsertFeedbackState,
    type HorseshoeDragSessionState,
    type WorkflowNodeClickState,
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
    setNodeExecutionState,
  } from '../stores/workflowStore';
  import { isReadOnly, currentGraphId, currentGraphType } from '../stores/graphSessionStore';
  import type {
    ConnectionAnchor,
    ConnectionCandidatesResponse,
    ConnectionCommitResponse,
    InsertableNodeTypeCandidate,
  } from '../services/workflow/types';
  import { architectureAsWorkflowGraph } from '../stores/architectureStore';
  import { workflowService } from '../services/workflow/WorkflowService';
  import type { NodeDefinition } from '../services/workflow/types';
  import {
    buildConnectionIntentState,
    edgeToGraphEdge,
    isWorkflowConnectionValid,
  } from './workflowConnections.ts';
  import { computeWorkflowGraphSyncDecision } from './workflowGraphSync';
  import {
    applyEdgeInsertPreviewActiveFlag,
    clearEdgeInsertPreviewState,
    createEdgeInsertPreviewState,
    findEdgeInsertHitTarget,
    getCommittableEdgeInsertPreview,
    setEdgeInsertHoverTarget,
    setEdgeInsertPreviewPending,
    setEdgeInsertPreviewRejected,
    setEdgeInsertPreviewResolved,
    shouldRefreshEdgeInsertPreview,
    updateEdgeInsertHitPoint,
    type EdgeInsertPreviewState,
  } from './edgeInsertInteraction.ts';
  import { resolveReconnectSourceAnchor } from './reconnectInteraction';
  import WorkflowContainerBoundary from './WorkflowContainerBoundary.svelte';
  import WorkflowEdgeInsertPreviewMarker from './WorkflowEdgeInsertPreviewMarker.svelte';
  import {
    resolveWorkflowContainerBounds,
    resolveWorkflowContainerTransitionDecision,
    type WorkflowContainerViewport,
  } from './workflowContainerBoundary.ts';
  import { resolveWorkflowContainerKeyboardAction } from './workflowContainerSelection.ts';
  import { getWorkflowMiniMapNodeColor } from './workflowMiniMap.ts';
  import { resolveWorkflowGraphSource } from './workflowGraphSource.ts';
  import {
    isWorkflowPaletteEdgeInsertEnabled,
    readWorkflowPaletteDragDefinition,
    resolveWorkflowPaletteDropPosition,
  } from './workflowPaletteDrag.ts';
  import { workflowEdgeTypes, workflowNodeTypes } from './workflowGraphTypes.ts';

  // Import view store for zoom transitions
  import {
    tabIntoGroup,
    zoomTarget,
    zoomToOrchestration,
    viewLevel,
  } from '../stores/viewStore';
  import { currentOrchestration } from '../stores/orchestrationStore';

  // Local state for SvelteFlow (Svelte 5 requires $state.raw for xyflow)
  let nodes = $state.raw<Node[]>([]);
  let edges = $state.raw<Edge[]>([]);

  // Determine if we can edit based on both isEditing store and isReadOnly
  let canEdit = $derived($isEditing && !$isReadOnly);

  // Track double-click for group zoom
  let nodeClickState = $state<WorkflowNodeClickState>({
    lastClickTime: 0,
    lastClickNodeId: null,
  });

  // Track if we've already triggered the zoom-out transition
  let transitionTriggered = $state(false);

  // Track if the container border is selected
  let containerSelected = $state(false);

  // Container element reference for size calculations
  let containerElement: HTMLElement;

  // Current viewport state for rendering the container border
  let currentViewport = $state<WorkflowContainerViewport | null>(null);
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

  let containerBounds = $derived(resolveWorkflowContainerBounds(nodes));

  // Handle viewport changes during pan/zoom for border rendering
  function handleMove(_event: MouseEvent | TouchEvent | null, viewport: { x: number; y: number; zoom: number }) {
    currentViewport = viewport;
  }

  // Handle viewport changes to detect when to transition to orchestration view
  function handleMoveEnd(_event: MouseEvent | TouchEvent | null, viewport: { x: number; y: number; zoom: number }) {
    currentViewport = viewport;

    const decision = resolveWorkflowContainerTransitionDecision({
      bounds: containerBounds,
      viewport,
      screenWidth: containerElement?.clientWidth ?? null,
      screenHeight: containerElement?.clientHeight ?? null,
      hasCurrentOrchestration: $currentOrchestration !== null,
      transitionTriggered,
    });

    transitionTriggered = decision.transitionTriggered;
    if (decision.shouldZoomToOrchestration) {
      zoomToOrchestration();
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

  function toggleContainerSelection() {
    containerSelected = !containerSelected;
  }

  // Deselect container when clicking on the graph background
  function handlePaneClick() {
    containerSelected = false;
    clearConnectionInteraction();
  }

  // Sync store changes to local state based on graph type
  $effect(() => {
    const graphSource = resolveWorkflowGraphSource({
      currentGraphType: $currentGraphType,
      currentGraphId: $currentGraphId,
      architectureGraph: $architectureAsWorkflowGraph,
      workflowNodes: $nodesStore,
      workflowEdges: $edgesStore,
    });
    const storeNodes = $nodesStore;
    const storeEdges = $edgesStore;

    if (graphSource.type === 'architecture') {
      nodes = graphSource.nodes;
      edges = graphSource.edges;
      return;
    }

    if (graphSource.type === 'architecture-pending') {
      return;
    }

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

    horseshoeSelectedIndex = normalizeWorkflowHorseshoeSelectedIndex({
      selectedIndex: horseshoeSelectedIndex,
      itemCount: $connectionIntent.insertableNodeTypes.length,
    });

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
    applyHorseshoeSession(closeHorseshoeDisplay(horseshoeSession));
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
    horseshoeLastTrace = formatWorkflowHorseshoeSessionTrace(nextSession);

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
    return buildWorkflowHorseshoeOpenContext({
      canEdit,
      session: horseshoeSession,
      connectionDragState,
      hasConnectionIntent: Boolean($connectionIntent),
      insertableCount: $connectionIntent?.insertableNodeTypes.length ?? 0,
    });
  }

  function getRelativePointerPosition(clientX: number, clientY: number) {
    return resolveWorkflowRelativePointerPosition({
      clientPosition: { clientX, clientY },
      containerBounds: containerElement?.getBoundingClientRect() ?? null,
    });
  }

  function getEventPointerPosition(event: MouseEvent | TouchEvent) {
    return resolveWorkflowRelativePointerPosition({
      clientPosition: resolveWorkflowPointerClientPosition(event),
      containerBounds: containerElement?.getBoundingClientRect() ?? null,
    });
  }

  async function refreshEdgeInsertPreview(event: DragEvent, definition: NodeDefinition) {
    if (
      !externalPaletteDragActive ||
      !isWorkflowPaletteEdgeInsertEnabled($currentGraphType, $currentGraphId)
    ) {
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
        applyWorkflowGraphMutationResponse(
          {
            graph: response.graph,
            workflow_event: response.workflow_event,
            workflow_session_state: response.workflow_session_state,
          },
          { setNodeExecutionState },
        );
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
    const decision = resolveWorkflowDragCursorUpdate({
      pointerPosition: nextPosition,
      session: horseshoeSession,
      insertableNodeTypes: $connectionIntent?.insertableNodeTypes ?? [],
      selectedIndex: horseshoeSelectedIndex,
    });

    if (decision.type === 'select-index') {
      horseshoeSelectedIndex = decision.selectedIndex;
    } else if (decision.type === 'update-anchor') {
      applyHorseshoeSession(decision.session);
    }
  }

  $effect(() => {
    if (!edgeInsertPreview.edgeId) {
      return;
    }

    if (
      !isWorkflowPaletteEdgeInsertEnabled($currentGraphType, $currentGraphId) ||
      !externalPaletteDragActive ||
      !currentGraphRevision ||
      edgeInsertPreview.graphRevision !== currentGraphRevision
    ) {
      clearEdgeInsertPreview();
    }
  });

  $effect(() => {
    const previewEdgeId = edgeInsertPreview.bridge ? edgeInsertPreview.edgeId : null;
    const result = applyEdgeInsertPreviewActiveFlag(edges, previewEdgeId);

    if (result.changed) {
      edges = result.edges;
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
    const request = requestWorkflowHorseshoeOpen({
      session: horseshoeSession,
      connectionDragState,
      openContext: getHorseshoeOpenContext(),
    });
    horseshoeLastTrace = request.trace;
    applyHorseshoeSession(request.session);
  }

  function rotateInsertSelection(delta: number) {
    const selectedIndex = rotateWorkflowHorseshoeSelection({
      selectedIndex: horseshoeSelectedIndex,
      delta,
      itemCount: $connectionIntent?.insertableNodeTypes.length ?? 0,
    });
    if (selectedIndex === null) return;

    horseshoeInsertFeedback = clearHorseshoeInsertFeedback();
    horseshoeSelectedIndex = selectedIndex;
  }

  function updateInsertQuery(nextQuery: string) {
    const queryUpdate = resolveWorkflowHorseshoeQueryUpdate({
      items: $connectionIntent?.insertableNodeTypes,
      query: nextQuery,
      selectedIndex: horseshoeSelectedIndex,
    });
    horseshoeInsertFeedback = clearHorseshoeInsertFeedback();
    horseshoeQuery = queryUpdate.query;
    horseshoeSelectedIndex = queryUpdate.selectedIndex;

    if (queryUpdate.resetTimerAction === 'schedule') {
      scheduleHorseshoeQueryReset();
      return;
    }

    if (horseshoeQueryResetTimer) {
      clearTimeout(horseshoeQueryResetTimer);
      horseshoeQueryResetTimer = null;
    }
  }

  async function commitInsertSelection(candidate: InsertableNodeTypeCandidate) {
    const currentConnectionIntent = $connectionIntent;
    if (
      !currentConnectionIntent ||
      horseshoeInsertFeedback.pending ||
      !supportsInsertFromConnectionDrag(connectionDragState)
    ) return;

    const positionHint = resolveWorkflowInsertPositionHint({
      anchorPosition: horseshoeSession.anchorPosition,
      viewport: currentViewport,
    });
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
        applyWorkflowGraphMutationResponse(
          {
            graph: response.graph,
            workflow_event: response.workflow_event,
            workflow_session_state: response.workflow_session_state,
          },
          { setNodeExecutionState },
        );
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
    const decision = resolveWorkflowNodeClick(nodeClickState, node.id, Date.now());
    nodeClickState = decision.state;

    if (decision.isDoubleClick) {
      handleNodeDoubleClick(node);
    }
  }

  // Handle double-click on a node to zoom into it (for node groups)
  async function handleNodeDoubleClick(node: Node) {
    const target = resolveWorkflowGroupZoomTarget(node);
    if (!target) return;

    zoomTarget.set(target);
    await tabIntoGroup(node.id);
  }

  function checkValidConnection(connection: Edge | Connection): boolean {
    return isWorkflowConnectionValid(connection, nodes, $connectionIntent, isPortTypeCompatible);
  }

  function getGraphRevision(): string {
    return currentGraphRevision;
  }

  function setConnectionIntentState(
    candidates: ConnectionCandidatesResponse,
    rejection?: ConnectionCommitResponse['rejection'],
  ) {
    setConnectionIntent(buildConnectionIntentState(candidates, rejection));
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
          setConnectionIntent(preserveConnectionIntentState({
            sourceAnchor,
            graphRevision: options?.graphRevision ?? getGraphRevision(),
            currentIntent: $connectionIntent,
            rejection: options?.rejection,
          }));
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
      applyWorkflowGraphMutationResponse(
        {
          graph: response.graph,
          workflow_event: response.workflow_event,
          workflow_session_state: response.workflow_session_state,
        },
        { setNodeExecutionState },
      );
      clearConnectionInteraction();
      return response;
    }

    try {
      const backendGraph = await workflowService.getExecutionGraph();
      syncEdgesFromBackend(backendGraph);
    } catch (error) {
      console.warn('[WorkflowGraph] Failed to refresh execution graph after rejected connect:', error);
    }

    setConnectionIntent(preserveConnectionIntentState({
      sourceAnchor,
      graphRevision: response.graph_revision,
      currentIntent: $connectionIntent,
      rejection: response.rejection,
    }));

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

    const definition = readWorkflowPaletteDragDefinition(event, (error) => {
      console.warn('[WorkflowGraph] Failed to parse palette drag data:', error);
    });
    if (!definition) {
      clearConnectionInteraction();
      return;
    }

    const position = resolveWorkflowPaletteDropPosition({
      pointerPosition: getRelativePointerPosition(event.clientX, event.clientY),
      viewport: currentViewport,
    });
    const activeEdgeInsertPreview = getCommittableEdgeInsertPreview(
      edgeInsertPreview,
      definition.node_type,
    );

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

    const definition = readWorkflowPaletteDragDefinition(event, (error) => {
      console.warn('[WorkflowGraph] Failed to parse palette drag data:', error);
    });
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
        applyWorkflowGraphMutationResponse(
          {
            graph: response.graph,
            workflow_event: response.workflow_event,
            workflow_session_state: response.workflow_session_state,
          },
          { setNodeExecutionState },
        );
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
  let cutToolRef: CutTool;
  let isCutting = $state(false);
  let ctrlPressed = $state(false);

  function handleKeyDown(e: KeyboardEvent) {
    const containerAction = resolveWorkflowContainerKeyboardAction({
      key: e.key,
      containerSelected,
    });

    if (containerAction.type === 'zoom-to-orchestration') {
      e.preventDefault();
      containerSelected = false;
      zoomToOrchestration();
      return;
    }

    if (containerAction.type === 'deselect-container') {
      e.preventDefault();
      containerSelected = false;
    }

    if (isEditableKeyboardTarget(e.target as HTMLElement | null)) {
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
    return resolveHorseshoeSessionStatusLabel({
      feedback: horseshoeInsertFeedback,
      session: horseshoeSession,
    });
  }

  function handleWindowKeyDown(e: KeyboardEvent) {
    if (isEditableKeyboardTarget(e.target as HTMLElement | null)) {
      return;
    }

    const selection = resolveWorkflowHorseshoeSelectionSnapshot({
      session: horseshoeSession,
      feedback: horseshoeInsertFeedback,
      items: $connectionIntent?.insertableNodeTypes,
      selectedIndex: horseshoeSelectedIndex,
    });
    const action = resolveHorseshoeKeyboardAction(e, selection.keyboardContext);

    if (action.preventDefault) {
      e.preventDefault();
    }

    switch (action.type) {
      case 'request-open':
        horseshoeLastTrace = 'keydown:space';
        requestHorseshoeOpen();
        return;
      case 'confirm-selection': {
        horseshoeLastTrace = e.key === 'Enter' ? 'keydown:enter' : 'keydown:space';
        if (selection.selectedCandidate) {
          void commitInsertSelection(selection.selectedCandidate);
        }
        return;
      }
      case 'close':
        closeHorseshoeSelector();
        return;
      case 'rotate-selection':
        rotateInsertSelection(action.delta);
        return;
      case 'remove-query-character':
        updateInsertQuery(horseshoeQuery.slice(0, -1));
        return;
      case 'append-query-character':
        updateInsertQuery(`${horseshoeQuery}${action.character}`);
        return;
      case 'noop':
        return;
    }
  }

  function handlePaneMouseDown(e: MouseEvent) {
    if (externalPaletteDragActive) {
      return;
    }

    cutToolRef?.onPaneMouseDown(e);
  }

  function handlePaneMouseMove(e: MouseEvent) {
    updateDragCursorFromMouseEvent(e);
    if (externalPaletteDragActive) {
      return;
    }

    cutToolRef?.onPaneMouseMove(e);
  }

  function handlePaneMouseUp(e: MouseEvent) {
    if (externalPaletteDragActive) {
      return;
    }

    cutToolRef?.onPaneMouseUp(e);
  }

  async function handleEdgesCut(edgeIds: string[]) {
    clearConnectionInteraction();

    for (const edgeId of edgeIds) {
      try {
        const updatedGraph = await workflowService.removeEdge(edgeId);
        syncEdgesFromBackend(updatedGraph);
      } catch (error) {
        console.error('[WorkflowGraph] Failed to remove edge via cut:', error);
      }
    }
  }

</script>

<svelte:window onmousemove={updateDragCursorFromMouseEvent} />

<!-- a11y-reviewed: SvelteFlow graph canvas owns pointer interaction while keyboard graph commands are handled on this focusable container. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- a11y-reviewed: SvelteFlow graph canvas requires a focusable host for keyboard graph commands. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
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
    nodeTypes={workflowNodeTypes}
    edgeTypes={workflowEdgeTypes}
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
    <MiniMap nodeColor={getWorkflowMiniMapNodeColor} maskColor="rgba(0, 0, 0, 0.8)" />

  </SvelteFlow>

  <WorkflowContainerBoundary
    bounds={containerBounds}
    viewport={currentViewport}
    selected={containerSelected}
    onToggleSelected={toggleContainerSelection}
  />

  {#if edgeInsertPreview.bridge && edgeInsertPreview.hitPoint}
    <WorkflowEdgeInsertPreviewMarker hitPoint={edgeInsertPreview.hitPoint} />
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

  <CutTool
    bind:this={cutToolRef}
    edges={edges}
    enabled={canEdit && !externalPaletteDragActive}
    bind:ctrlPressed
    bind:isCutting
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

  /* Cut tool styles */
  .workflow-graph-container {
    position: relative;
    overflow: hidden;
  }

  .workflow-graph-container.cutting {
    cursor: crosshair;
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

</style>
