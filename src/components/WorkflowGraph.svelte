<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import type { Node, Edge, Connection } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import {
    buildWorkflowHorseshoeOpenContext, clearHorseshoeInsertFeedback,
    clearWorkflowConnectionDragInteraction, closeHorseshoeDisplay, collectSelectedNodeIds,
    createConnectionDragState, createHorseshoeDragSessionState,
    createHorseshoeInsertFeedbackState, isPortTypeCompatible, markConnectionDragFinalizing,
    normalizeWorkflowHorseshoeSelectedIndex, preserveConnectionIntentState,
    registerWorkflowGraphWindowListeners, rejectHorseshoeInsertFeedback,
    requestWorkflowHorseshoeOpen, resolveWorkflowDragCursorUpdate,
    resolveWorkflowGraphInteractionState, resolveWorkflowGroupZoomTarget,
    resolveWorkflowHorseshoeBlockedReasonLog, resolveWorkflowHorseshoeQueryUpdate,
    resolveWorkflowHorseshoeSessionUpdate, resolveWorkflowInsertPositionHint,
    resolveWorkflowNodeClick, resolveWorkflowPointerClientPosition,
    resolveWorkflowRelativePointerPosition, rotateWorkflowHorseshoeSelection,
    shouldClearWorkflowConnectionInteractionAfterConnectEnd, shouldRemoveReconnectedEdge,
    startConnectionDrag, startHorseshoeDrag, startHorseshoeInsertFeedback,
    startReconnectDrag, supportsInsertFromConnectionDrag, syncHorseshoeDisplay,
    type ConnectionDragState, type HorseshoeBlockedReason, type HorseshoeDragSessionState,
    type HorseshoeInsertFeedbackState, type WorkflowNodeClickState,
  } from '@pantograph/svelte-graph';

  import {
    addNode, clearConnectionIntent, connectionIntent, edges as edgesStore, isEditing,
    nodeDefinitions, nodes as nodesStore, removeNode, selectedNodeIds, setConnectionIntent,
    updateNodePosition, workflowGraph,
  } from '../stores/workflowStore';
  import { isReadOnly, currentGraphId, currentGraphType } from '../stores/graphSessionStore';
  import type {
    ConnectionAnchor,
    ConnectionCommitResponse,
    InsertableNodeTypeCandidate,
  } from '../services/workflow/types';
  import { architectureAsWorkflowGraph } from '../stores/architectureStore';
  import { workflowService } from '../services/workflow/WorkflowService';
  import type { NodeDefinition } from '../services/workflow/types';
  import {
    isWorkflowConnectionValid,
    resolveWorkflowConnectionAnchors,
  } from './workflowConnections.ts';
  import { computeWorkflowGraphSyncDecision } from './workflowGraphSync';
  import {
    applyEdgeInsertPreviewActiveFlag,
    clearEdgeInsertPreviewState,
    createEdgeInsertPreviewState,
    shouldClearEdgeInsertPreviewForGraphState,
    type EdgeInsertPreviewState,
  } from './edgeInsertInteraction.ts';
  import { refreshWorkflowGraphEdgeInsertPreview } from './workflowGraphEdgeInsertPreview.ts';
  import { resolveReconnectSourceAnchor } from './reconnectInteraction';
  import WorkflowGraphCanvas from './WorkflowGraphCanvas.svelte';
  import {
    resolveWorkflowContainerBounds,
    resolveWorkflowContainerTransitionDecision,
    type WorkflowContainerViewport,
  } from './workflowContainerBoundary.ts';
  import {
    clearWorkflowContainerSelection,
    resolveWorkflowContainerSelectionAfterGraphSelection,
    toggleWorkflowContainerSelection,
  } from './workflowContainerSelection.ts';
  import {
    handleWorkflowGraphContainerKeyDown,
    handleWorkflowGraphWindowKeyDown,
  } from './workflowGraphKeyboardActions.ts';
  import { resolveWorkflowGraphSource } from './workflowGraphSource.ts';
  import {
    commitWorkflowConnection,
    commitWorkflowEdgeInsertDrop,
    commitWorkflowInsertCandidate,
    commitWorkflowReconnect,
    loadWorkflowConnectionIntentState,
    removeWorkflowGraphEdge,
    removeWorkflowGraphEdges,
  } from './workflowGraphBackendActions.ts';
  import {
    isWorkflowPaletteEdgeInsertEnabled,
  } from './workflowPaletteDrag.ts';
  import {
    handleWorkflowGraphPaletteDragOver,
    handleWorkflowGraphPaletteDrop,
  } from './workflowGraphPaletteHandlers.ts';

  import {
    tabIntoGroup,
    zoomTarget,
    zoomToOrchestration,
    viewLevel,
  } from '../stores/viewStore';
  import { currentOrchestration } from '../stores/orchestrationStore';

  let nodes = $state.raw<Node[]>([]);
  let edges = $state.raw<Edge[]>([]);

  let canEdit = $derived($isEditing && !$isReadOnly);

  let nodeClickState = $state<WorkflowNodeClickState>({
    lastClickTime: 0,
    lastClickNodeId: null,
  });

  let transitionTriggered = $state(false);

  let containerSelected = $state(false);

  let containerElement = $state<HTMLElement | undefined>(undefined);

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
  let graphInteractionState = $derived(
    resolveWorkflowGraphInteractionState({
      canEdit,
      ctrlPressed,
      externalPaletteDragActive,
    }),
  );
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

  function handleMove(_event: MouseEvent | TouchEvent | null, viewport: { x: number; y: number; zoom: number }) {
    currentViewport = viewport;
  }

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

  $effect(() => {
    if ($viewLevel === 'data-graph') {
      transitionTriggered = false;
    }
  });

  function handleWorkflowPaletteDragStart() {
    externalPaletteDragActive = true;
    containerSelected = clearWorkflowContainerSelection();
    clearConnectionInteraction();
  }

  function handleWorkflowPaletteDragEnd() {
    externalPaletteDragActive = false;
    clearEdgeInsertPreview();
  }

  function handleSelectionChange({ nodes: selectedNodes }: { nodes: Node[]; edges: Edge[] }) {
    selectedNodeIds.set(collectSelectedNodeIds(selectedNodes));
    containerSelected = resolveWorkflowContainerSelectionAfterGraphSelection({
      containerSelected,
      selectedNodeCount: selectedNodes.length,
    });
  }

  function toggleContainerSelection() {
    containerSelected = toggleWorkflowContainerSelection(containerSelected);
  }

  function handlePaneClick() {
    containerSelected = clearWorkflowContainerSelection();
    clearConnectionInteraction();
  }

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

  onMount(async () => {
    const removeWindowListeners = registerWorkflowGraphWindowListeners(window, {
      onKeyDown: handleWindowKeyDown,
      onPaletteDragEnd: handleWorkflowPaletteDragEnd,
      onPaletteDragStart: handleWorkflowPaletteDragStart,
    });

    const definitions = await workflowService.getNodeDefinitions();
    nodeDefinitions.set(definitions);

    return removeWindowListeners;
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
    const blockedLog = resolveWorkflowHorseshoeBlockedReasonLog({
      blockedReason: horseshoeSession.blockedReason,
      lastLoggedBlockedReason: lastLoggedHorseshoeBlockedReason,
    });
    lastLoggedHorseshoeBlockedReason = blockedLog.nextLoggedBlockedReason;
    if (!blockedLog.shouldLog) {
      return;
    }

    console.warn('[WorkflowGraph] Horseshoe blocked:', blockedLog.message);
  });

  function closeHorseshoeSelector() {
    applyHorseshoeSession(closeHorseshoeDisplay(horseshoeSession));
  }

  function clearConnectionDragTracking() {
    const reset = clearWorkflowConnectionDragInteraction();
    connectionDragState = reset.connectionDragState;
    horseshoeInsertFeedback = reset.feedback;
    applyHorseshoeSession(reset.horseshoeSession);
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
    const update = resolveWorkflowHorseshoeSessionUpdate({
      current: {
        session: horseshoeSession,
        feedback: horseshoeInsertFeedback,
        selectedIndex: horseshoeSelectedIndex,
        query: horseshoeQuery,
      },
      nextSession,
    });

    if (!update.changed) {
      return;
    }

    horseshoeSession = update.state.session;
    horseshoeInsertFeedback = update.state.feedback;
    horseshoeSelectedIndex = update.state.selectedIndex;
    horseshoeQuery = update.state.query;
    horseshoeLastTrace = update.trace;

    if (update.clearQueryResetTimer) {
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
    await refreshWorkflowGraphEdgeInsertPreview({
      bumpRequestId: () => ++edgeInsertPreviewRequestId,
      containerElement,
      definition,
      edgeInsertEnabled: isWorkflowPaletteEdgeInsertEnabled($currentGraphType, $currentGraphId),
      externalPaletteDragActive,
      getRequestId: () => edgeInsertPreviewRequestId,
      getState: () => edgeInsertPreview,
      graphRevision: getGraphRevision(),
      hitPoint: getRelativePointerPosition(event.clientX, event.clientY),
      setState: (state) => {
        edgeInsertPreview = state;
      },
    });
  }

  async function commitEdgeInsertDrop(
    definition: NodeDefinition,
    position: { x: number; y: number },
    preview: EdgeInsertPreviewState,
  ) {
    return commitWorkflowEdgeInsertDrop({ definition, position, preview });
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
    if (shouldClearEdgeInsertPreviewForGraphState({
      state: edgeInsertPreview,
      edgeInsertEnabled: isWorkflowPaletteEdgeInsertEnabled($currentGraphType, $currentGraphId),
      externalPaletteDragActive,
      currentGraphRevision,
    })) {
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
      const response = await commitWorkflowInsertCandidate({
        sourceAnchor: currentConnectionIntent.sourceAnchor,
        candidateNodeType: candidate.node_type,
        graphRevision: currentConnectionIntent.graphRevision || getGraphRevision(),
        positionHint,
        preferredInputPortId: candidate.matching_input_port_ids[0],
      });

      if (response.accepted) {
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

  function onNodeClick({ node }: { node: Node }) {
    const decision = resolveWorkflowNodeClick(nodeClickState, node.id, Date.now());
    nodeClickState = decision.state;

    if (decision.isDoubleClick) {
      handleNodeDoubleClick(node);
    }
  }

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

    const result = await loadWorkflowConnectionIntentState({
      sourceAnchor,
      graphRevision: options?.graphRevision ?? getGraphRevision(),
      currentIntent: $connectionIntent,
      preserveDisplay: options?.preserveDisplay,
      rejection: options?.rejection,
    });

    if (requestId !== connectionIntentRequestId) {
      return;
    }

    if (result.type === 'set') {
      setConnectionIntent(result.intent);
      return;
    }

    clearConnectionInteraction();
  }

  async function commitConnection(connection: Connection): Promise<ConnectionCommitResponse | null> {
    const result = await commitWorkflowConnection({
      connection,
      currentIntent: $connectionIntent,
      currentGraphRevision: getGraphRevision(),
    });
    const response = result.response;

    if (!response || response.accepted) {
      clearConnectionInteraction();
      return response;
    }

    if (result.intent) {
      setConnectionIntent(result.intent);
    }

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
    if (!shouldClearWorkflowConnectionInteractionAfterConnectEnd({
      session: horseshoeSession,
      feedback: horseshoeInsertFeedback,
    })) return;
    clearConnectionInteraction();
  }

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

  async function handleDelete({ nodes: deletedNodes, edges: deletedEdges }: { nodes: Node[]; edges: Edge[] }) {
    if (!canEdit) return;
    clearConnectionInteraction();

    await removeWorkflowGraphEdges(
      deletedEdges.map((edge) => edge.id),
      '[WorkflowGraph] Failed to remove edge:',
    );

    for (const node of deletedNodes) {
      removeNode(node.id);
    }
  }

  async function handleDrop(event: DragEvent) {
    await handleWorkflowGraphPaletteDrop({
      canEdit,
      clearConnectionInteraction,
      clearEdgeInsertPreview,
      commitEdgeInsertDrop,
      currentViewport,
      edgeInsertPreview,
      event,
      getRelativePointerPosition,
      onAddNode: addNode,
      refreshEdgeInsertPreview,
    });
  }

  async function handleDragOver(event: DragEvent) {
    await handleWorkflowGraphPaletteDragOver({
      canEdit,
      clearConnectionInteraction,
      clearEdgeInsertPreview,
      commitEdgeInsertDrop,
      currentViewport,
      edgeInsertPreview,
      event,
      getRelativePointerPosition,
      onAddNode: addNode,
      refreshEdgeInsertPreview,
    });
  }

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
    const anchors = resolveWorkflowConnectionAnchors(newConnection);
    if (!anchors) {
      clearConnectionInteraction();
      return;
    }

    connectionDragState = markConnectionDragFinalizing(connectionDragState);

    const result = await commitWorkflowReconnect({
      anchors,
      oldEdge,
      fallbackRevision: getGraphRevision(),
    });

    if (result.type === 'accepted' || result.type === 'stale') {
      clearConnectionInteraction();
      return;
    }

    if (result.type === 'rejected' && result.rejection) {
      setConnectionIntent(preserveConnectionIntentState({
        sourceAnchor:
          connectionDragState.reconnectingSourceAnchor ??
          result.sourceAnchor,
        graphRevision: result.graphRevision,
        currentIntent: $connectionIntent,
        rejection: result.rejection,
      }));
      console.warn('[WorkflowGraph] Reconnection rejected:', result.rejection.message);
      return;
    }

    if (result.type === 'failed') {
      console.error('[WorkflowGraph] Failed to reconnect edge:', result.error);
    }
  }

  async function handleReconnectEnd(_event: MouseEvent | TouchEvent, _edge: Edge, _handleType: unknown, connectionState: { isValid: boolean }) {
    if (!canEdit) return;

    const reconnectingEdgeId = shouldRemoveReconnectedEdge(connectionDragState, connectionState);
    if (reconnectingEdgeId) {
      await removeWorkflowGraphEdge(
        reconnectingEdgeId,
        '[WorkflowGraph] Failed to remove edge on reconnect end:',
      );
    }

    clearConnectionInteraction();
  }

  let isCutting = $state(false);
  let ctrlPressed = $state(false);

  function handleKeyDown(e: KeyboardEvent) {
    containerSelected = handleWorkflowGraphContainerKeyDown({
      event: e,
      containerSelected,
      horseshoeDisplayState: horseshoeSession.displayState,
      onClearConnectionInteraction: clearConnectionInteraction,
      onCloseHorseshoeSelector: closeHorseshoeSelector,
      onZoomToOrchestration: zoomToOrchestration,
    });
  }

  function handleWindowKeyDown(e: KeyboardEvent) {
    handleWorkflowGraphWindowKeyDown({
      event: e,
      session: horseshoeSession,
      feedback: horseshoeInsertFeedback,
      items: $connectionIntent?.insertableNodeTypes,
      selectedIndex: horseshoeSelectedIndex,
      query: horseshoeQuery,
      onClose: closeHorseshoeSelector,
      onConfirmSelection: (candidate) => void commitInsertSelection(candidate),
      onQueryUpdate: updateInsertQuery,
      onRequestOpen: requestHorseshoeOpen,
      onRotateSelection: rotateInsertSelection,
      onTrace: (trace) => {
        horseshoeLastTrace = trace;
      },
    });
  }

  async function handleEdgesCut(edgeIds: string[]) {
    clearConnectionInteraction();
    await removeWorkflowGraphEdges(edgeIds, '[WorkflowGraph] Failed to remove edge via cut:');
  }

</script>

<svelte:window onmousemove={updateDragCursorFromMouseEvent} />

<WorkflowGraphCanvas
  bind:nodes
  bind:edges
  bind:containerElement
  bind:ctrlPressed
  bind:isCutting
  {canEdit}
  {checkValidConnection}
  {containerBounds}
  {containerSelected}
  {currentViewport}
  {edgeInsertPreview}
  {externalPaletteDragActive}
  {graphInteractionState}
  {handleConnect}
  {handleConnectEnd}
  {handleConnectStart}
  {handleDelete}
  {handleDragOver}
  {handleDrop}
  {handleEdgesCut}
  {handleKeyDown}
  {handleMove}
  {handleMoveEnd}
  handleNodeClick={onNodeClick}
  handleNodeDragStop={onNodeDragStop}
  {handlePaneClick}
  handlePaneMouseMove={updateDragCursorFromMouseEvent}
  {handleReconnect}
  {handleReconnectEnd}
  {handleReconnectStart}
  {handleSelectionChange}
  {horseshoeInsertFeedback}
  {horseshoeLastTrace}
  {horseshoeQuery}
  {horseshoeSelectedIndex}
  {horseshoeSession}
  insertableNodeTypes={$connectionIntent?.insertableNodeTypes ?? []}
  onCancelHorseshoe={closeHorseshoeSelector}
  onRotateInsertSelection={rotateInsertSelection}
  onSelectInsertCandidate={(candidate) => void commitInsertSelection(candidate)}
  onToggleContainerSelection={toggleContainerSelection}
/>
