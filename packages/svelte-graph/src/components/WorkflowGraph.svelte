<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
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
  import './WorkflowGraph.css';
  import { get } from 'svelte/store';

  import { useGraphContext } from '../context/useGraphContext.js';
  import { isWorkflowConnectionValid } from '../workflowConnections.js';
  import { computeWorkflowGraphSyncDecision } from '../workflowGraphSync.js';
  import type {
    ConnectionAnchor,
    ConnectionCommitResponse,
    InsertableNodeTypeCandidate,
  } from '../types/workflow.js';
  import {
    dispatchWorkflowHorseshoeKeyboardAction,
    isEditableKeyboardTarget,
  } from '../workflowHorseshoeKeyboard.js';
  import {
    normalizeWorkflowHorseshoeSelectedIndex,
    resolveWorkflowHorseshoeQueryUpdate,
    resolveWorkflowHorseshoeSelectionSnapshot,
    rotateWorkflowHorseshoeSelection,
  } from '../workflowHorseshoeSelection.js';
  import { resolveWorkflowHorseshoeSessionUpdate } from '../workflowHorseshoeSessionUpdate.js';
  import { requestWorkflowHorseshoeOpen } from '../workflowHorseshoeOpenRequest.js';
  import { buildWorkflowHorseshoeOpenContext } from '../workflowHorseshoeOpenContext.js';
  import {
    clearHorseshoeInsertFeedback,
    createHorseshoeInsertFeedbackState,
    rejectHorseshoeInsertFeedback,
    startHorseshoeInsertFeedback,
    type HorseshoeInsertFeedbackState,
  } from '../horseshoeInsertFeedback.js';
  import {
    closeHorseshoeDisplay,
    createHorseshoeDragSessionState,
    startHorseshoeDrag,
    syncHorseshoeDisplay,
    type HorseshoeBlockedReason,
    type HorseshoeDragSessionState,
  } from '../horseshoeDragSession.js';
  import {
    createConnectionDragState,
    markConnectionDragFinalizing,
    shouldRemoveReconnectedEdge,
    startConnectionDrag,
    startReconnectDrag,
    supportsInsertFromConnectionDrag,
    type ConnectionDragState,
  } from '../connectionDragState.js';
  import {
    clearWorkflowConnectionDragInteraction,
    shouldClearWorkflowConnectionInteractionAfterConnectEnd,
  } from '../workflowConnectionInteraction.js';
  import { collectSelectedNodeIds } from '../workflowSelection.js';
  import { resolveReconnectSourceAnchor } from '../reconnectInteraction.js';
  import { resolveWorkflowDragCursorUpdate } from '../workflowDragCursor.js';
  import { resolveWorkflowGraphInteractionState } from '../workflowGraphInteraction.js';
  import { registerWorkflowGraphWindowListeners } from '../workflowGraphWindowListeners.js';
  import { resolveWorkflowHorseshoeBlockedReasonLog } from '../workflowHorseshoeTrace.js';
  import {
    resolveWorkflowGroupZoomTarget,
    resolveWorkflowNodeClick,
    type WorkflowNodeClickState,
  } from '../workflowNodeActivation.js';
  import {
    readWorkflowPaletteDragDefinition,
    resolveWorkflowPaletteDropPosition,
  } from '../workflowPaletteDrag.js';
  import {
    resolveWorkflowPointerClientPosition,
    resolveWorkflowRelativePointerPosition,
  } from '../workflowPointerPosition.js';
  import { WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS } from '../workflowGraphEdgeOptions.js';
  import {
    WORKFLOW_GRAPH_FIT_VIEW_OPTIONS,
    WORKFLOW_GRAPH_MAX_ZOOM,
    WORKFLOW_GRAPH_MINIMAP_MASK_COLOR,
    WORKFLOW_GRAPH_MIN_ZOOM,
    WORKFLOW_GRAPH_PAN_ACTIVATION_KEY,
  } from '../workflowGraphViewport.js';
  import { resolveWorkflowInsertPositionHint } from '../workflowInsertPosition.js';
  import { getWorkflowMiniMapNodeColor } from '../workflowMiniMap.js';
  import {
    commitWorkflowConnection as commitWorkflowConnectionMutation,
    commitWorkflowInsertCandidate as commitWorkflowInsertCandidateMutation,
    commitWorkflowReconnect as commitWorkflowReconnectMutation,
    loadWorkflowConnectionIntentState as loadWorkflowConnectionIntentStateMutation,
    removeWorkflowGraphEdges as removeWorkflowGraphEdgesMutation,
  } from './workflowGraphBackendActions.js';
  import CutTool from './CutTool.svelte';
  import ContainerBorder from './ContainerBorder.svelte';
  import WorkflowGraphHorseshoeLayer from './WorkflowGraphHorseshoeLayer.svelte';
  import ReconnectableEdge from './edges/ReconnectableEdge.svelte';

  const { backend, registry, stores } = useGraphContext();

  interface Props {
    showContainerBorder?: boolean;
    onContainerZoomOut?: () => void;
  }

  let { showContainerBorder = false, onContainerZoomOut }: Props = $props();
  const nodesStore = stores.workflow.nodes;
  const edgesStore = stores.workflow.edges;
  const connectionIntentStore = stores.workflow.connectionIntent;
  const {
    isEditing,
    nodeDefinitions: nodeDefsStore,
    selectedNodeIds: selectedNodeIdsStore,
    workflowGraph: workflowGraphStore,
  } =
    stores.workflow;
  const { isReadOnly, currentSessionId } = stores.session;
  const { viewLevel } = stores.view;

  const nodeTypes: NodeTypes = registry.nodeTypes as unknown as NodeTypes;
  const edgeTypes: EdgeTypes = (registry.edgeTypes || { reconnectable: ReconnectableEdge }) as unknown as EdgeTypes;

  let nodes = $state.raw<Node[]>([]);
  let edges = $state.raw<Edge[]>([]);

  let canEdit = $derived($isEditing && !$isReadOnly);
  let nodeClickState = $state<WorkflowNodeClickState>({
    lastClickTime: 0,
    lastClickNodeId: null,
  });
  let containerElement = $state<HTMLElement | null>(null);
  let currentViewport = $state<{ x: number; y: number; zoom: number } | null>(null);
  let cutToolRef: CutTool;
  let ctrlPressed = $state(false);
  let isCutting = $state(false);
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
  let containerBorderRef: ContainerBorder;
  let _prevNodesRef: Node[] | null = null;
  let _prevEdgesRef: Edge[] | null = null;
  let _skipNextNodeSync = false;

  $effect(() => {
    const syncDecision = computeWorkflowGraphSyncDecision({
      storeNodes: $nodesStore,
      storeEdges: $edgesStore,
      prevNodesRef: _prevNodesRef,
      prevEdgesRef: _prevEdgesRef,
      skipNextNodeSync: _skipNextNodeSync,
    });

    _prevNodesRef = syncDecision.nextPrevNodesRef;
    _prevEdgesRef = syncDecision.nextPrevEdgesRef;
    _skipNextNodeSync = syncDecision.nextSkipNextNodeSync;

    if (syncDecision.applyNodes) {
      nodes = $nodesStore;
    }

    if (syncDecision.applyEdges) {
      edges = $edgesStore;
    }
  });

  $effect(() => {
    if ($viewLevel === 'data-graph') {
      containerBorderRef?.resetTransition();
    }
  });

  onMount(async () => {
    const removeWindowListeners = registerWorkflowGraphWindowListeners(window, {
      onKeyDown: handleWindowKeyDown,
      onPaletteDragEnd: handleWorkflowPaletteDragEnd,
      onPaletteDragStart: handleWorkflowPaletteDragStart,
    });

    const definitions = await backend.getNodeDefinitions();
    nodeDefsStore.set(definitions);

    return removeWindowListeners;
  });

  onDestroy(() => {
    if (horseshoeQueryResetTimer) {
      clearTimeout(horseshoeQueryResetTimer);
    }
  });

  $effect(() => {
    if (!$connectionIntentStore) {
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
      itemCount: $connectionIntentStore.insertableNodeTypes.length,
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
    clearConnectionDragTracking();
    stores.workflow.clearConnectionIntent();
  }

  function handleWorkflowPaletteDragStart() {
    externalPaletteDragActive = true;
    clearConnectionInteraction();
  }

  function handleWorkflowPaletteDragEnd() {
    externalPaletteDragActive = false;
  }

  function handleSelectionChange({ nodes: selectedNodes }: { nodes: Node[]; edges: Edge[] }) {
    selectedNodeIdsStore.set(collectSelectedNodeIds(selectedNodes));
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
      hasConnectionIntent: Boolean($connectionIntentStore),
      insertableCount: $connectionIntentStore?.insertableNodeTypes.length ?? 0,
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

  function updateDragCursorFromMouseEvent(event: MouseEvent) {
    const nextPosition = getRelativePointerPosition(event.clientX, event.clientY);
    const decision = resolveWorkflowDragCursorUpdate({
      pointerPosition: nextPosition,
      session: horseshoeSession,
      insertableNodeTypes: $connectionIntentStore?.insertableNodeTypes ?? [],
      selectedIndex: horseshoeSelectedIndex,
    });

    if (decision.type === 'select-index') {
      horseshoeSelectedIndex = decision.selectedIndex;
    } else if (decision.type === 'update-anchor') {
      applyHorseshoeSession(decision.session);
    }
  }

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
      itemCount: $connectionIntentStore?.insertableNodeTypes.length ?? 0,
    });
    if (selectedIndex === null) return;

    horseshoeInsertFeedback = clearHorseshoeInsertFeedback();
    horseshoeSelectedIndex = selectedIndex;
  }

  function updateInsertQuery(nextQuery: string) {
    const queryUpdate = resolveWorkflowHorseshoeQueryUpdate({
      items: $connectionIntentStore?.insertableNodeTypes,
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
    const connectionIntent = $connectionIntentStore;
    if (
      !connectionIntent ||
      horseshoeInsertFeedback.pending ||
      !supportsInsertFromConnectionDrag(connectionDragState)
    ) {
      return;
    }

    const sessionId = get(currentSessionId);
    const positionHint = resolveWorkflowInsertPositionHint({
      anchorPosition: horseshoeSession.anchorPosition,
      viewport: currentViewport,
    });
    if (!sessionId || !positionHint) return;

    horseshoeInsertFeedback = startHorseshoeInsertFeedback();

    try {
      const response = await commitWorkflowInsertCandidateMutation({
        backend,
        candidateNodeType: candidate.node_type,
        graphRevision: connectionIntent.graphRevision || getGraphRevision(),
        positionHint,
        preferredInputPortId: candidate.matching_input_port_ids[0],
        sessionId,
        sourceAnchor: connectionIntent.sourceAnchor,
        workflowStores: stores.workflow,
      });

      if (response.accepted && response.graph) {
        clearConnectionInteraction();
        return;
      }

      horseshoeInsertFeedback = rejectHorseshoeInsertFeedback(response.rejection);
      horseshoeLastTrace = `insert-rejected:${response.rejection?.reason ?? 'unknown'}`;
      await loadConnectionIntent(connectionIntent.sourceAnchor, {
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

  function handleWindowKeyDown(event: KeyboardEvent) {
    if (isEditableKeyboardTarget(event.target as HTMLElement | null)) {
      return;
    }

    const selection = resolveWorkflowHorseshoeSelectionSnapshot({
      session: horseshoeSession,
      feedback: horseshoeInsertFeedback,
      items: $connectionIntentStore?.insertableNodeTypes,
      selectedIndex: horseshoeSelectedIndex,
    });
    dispatchWorkflowHorseshoeKeyboardAction({
      event,
      query: horseshoeQuery,
      selection,
      handlers: {
        onClose: closeHorseshoeSelector,
        onConfirmSelection: (candidate) => void commitInsertSelection(candidate),
        onQueryUpdate: updateInsertQuery,
        onRequestOpen: requestHorseshoeOpen,
        onRotateSelection: rotateInsertSelection,
        onTrace: (trace) => {
          horseshoeLastTrace = trace;
        },
      },
    });
  }

  function checkValidConnection(connection: Edge | Connection): boolean {
    return isWorkflowConnectionValid(connection, nodes, $connectionIntentStore);
  }

  function getGraphRevision(): string {
    return get(workflowGraphStore).derived_graph?.graph_fingerprint ?? '';
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
    const sessionId = get(currentSessionId);
    if (!canEdit || !sessionId) {
      clearConnectionInteraction();
      return;
    }

    const requestId = ++connectionIntentRequestId;
    if (!options?.preserveDisplay) {
      horseshoeInsertFeedback = clearHorseshoeInsertFeedback();
      closeHorseshoeSelector();
    }

    try {
      const result = await loadWorkflowConnectionIntentStateMutation({
        backend,
        currentIntent: $connectionIntentStore,
        graphRevision: options?.graphRevision ?? getGraphRevision(),
        preserveDisplay: options?.preserveDisplay,
        rejection: options?.rejection,
        sessionId,
        sourceAnchor,
        workflowStores: stores.workflow,
      });

      if (requestId !== connectionIntentRequestId) return;
      if (result.type === 'set') {
        stores.workflow.setConnectionIntent(result.intent);
      } else {
        clearConnectionInteraction();
      }
    } catch (error) {
      if (requestId === connectionIntentRequestId) {
        clearConnectionInteraction();
      }
      console.error('[WorkflowGraph] Failed to load connection candidates:', error);
    }
  }

  async function commitConnection(connection: Connection): Promise<ConnectionCommitResponse | null> {
    const sessionId = get(currentSessionId);
    if (!sessionId) return null;

    const result = await commitWorkflowConnectionMutation({
      backend,
      connection,
      currentGraphRevision: getGraphRevision(),
      currentIntent: $connectionIntentStore,
      sessionId,
      workflowStores: stores.workflow,
    });
    const response = result.response;

    if (response?.accepted) {
      clearConnectionInteraction();
      return response;
    }

    if (result.intent) {
      stores.workflow.setConnectionIntent(result.intent);
    }

    if (response?.rejection) {
      console.warn('[WorkflowGraph] Connection rejected:', response.rejection.message);
    }

    return response;
  }

  function onNodeDragStop({
    targetNode,
  }: {
    targetNode: Node | null;
    nodes: Node[];
    event: MouseEvent | TouchEvent;
  }) {
    if (!canEdit || !targetNode) return;
    _skipNextNodeSync = true;
    stores.workflow.updateNodePosition(targetNode.id, targetNode.position);
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

    stores.view.zoomTarget.set(target);
    await stores.view.tabIntoGroup(node.id);
  }

  async function handleConnectStart(
    _event: MouseEvent | TouchEvent,
    params: { nodeId: string; handleId: string | null; handleType: 'source' | 'target' },
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
    _connectionState: { isValid: boolean },
  ) {
    if (!shouldClearWorkflowConnectionInteractionAfterConnectEnd({
      session: horseshoeSession,
      feedback: horseshoeInsertFeedback,
    })) return;
    clearConnectionInteraction();
  }

  async function handleConnect(connection: Connection) {
    if (!canEdit) return;

    const response = await commitConnection(connection);
    if (!response?.accepted) return;
  }

  async function handleDelete({ nodes: deletedNodes, edges: deletedEdges }: { nodes: Node[]; edges: Edge[] }) {
    if (!canEdit) return;

    clearConnectionInteraction();
    await stores.workflow.deleteSelection(
      deletedNodes.map((node) => node.id),
      deletedEdges.map((edge) => edge.id),
    );
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    if (!canEdit) return;
    clearConnectionInteraction();

    const definition = readWorkflowPaletteDragDefinition(event, (error) => {
      console.warn('[WorkflowGraph] Failed to parse palette drag data:', error);
    });
    if (!definition) return;

    const container = event.currentTarget as HTMLElement;
    const position = resolveWorkflowPaletteDropPosition({
      clientPosition: { x: event.clientX, y: event.clientY },
      containerBounds: container.getBoundingClientRect(),
    });

    stores.workflow.addNode(definition, position);
  }

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
    if (!canEdit) return;
    event.dataTransfer!.dropEffect = 'copy';
  }

  async function handleReconnectStart(
    _event: MouseEvent | TouchEvent,
    edge: Edge,
    handleType: 'source' | 'target',
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

    const sessionId = get(currentSessionId);
    if (!sessionId) return;

    const result = await commitWorkflowReconnectMutation({
      backend,
      currentIntent: $connectionIntentStore,
      fallbackRevision: getGraphRevision(),
      newConnection,
      oldEdge,
      reconnectingSourceAnchor: connectionDragState.reconnectingSourceAnchor,
      sessionId,
      workflowStores: stores.workflow,
    });

    if (result.type === 'invalid') {
      clearConnectionInteraction();
      return;
    }

    if (result.type === 'accepted' || result.type === 'stale') {
      clearConnectionInteraction();
      return;
    }

    if (result.type === 'rejected') {
      stores.workflow.setConnectionIntent(result.intent);
      console.warn('[WorkflowGraph] Reconnection rejected:', result.intent.rejection?.message);
      return;
    }

    if (result.type === 'failed') {
      console.error('[WorkflowGraph] Failed to notify backend of reconnection:', result.error);
    }
  }

  async function handleReconnectEnd(
    _event: MouseEvent | TouchEvent,
    _edge: Edge,
    _handleType: unknown,
    connectionState: { isValid: boolean },
  ) {
    if (!canEdit) return;

    const reconnectingEdgeId = shouldRemoveReconnectedEdge(connectionDragState, connectionState);
    const sessionId = get(currentSessionId);
    if (reconnectingEdgeId && sessionId) {
      await removeWorkflowGraphEdgesMutation({
        backend,
        edgeIds: [reconnectingEdgeId],
        errorMessage: '[WorkflowGraph] Failed to notify backend of edge removal:',
        sessionId,
        workflowStores: stores.workflow,
      });
    }

    clearConnectionInteraction();
  }

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
    clearConnectionInteraction();
  }

  async function handleEdgesCut(edgeIds: string[]) {
    const sessionId = get(currentSessionId);
    clearConnectionInteraction();
    if (sessionId) {
      await removeWorkflowGraphEdgesMutation({
        backend,
        edgeIds,
        errorMessage: '[WorkflowGraph] Failed to notify backend of edge cut:',
        sessionId,
        workflowStores: stores.workflow,
      });
    }
  }
</script>

<svelte:window onmousemove={updateDragCursorFromMouseEvent} />

<!-- a11y-reviewed: SvelteFlow graph canvas owns pointer interaction while keyboard graph commands are handled on this focusable container. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- a11y-reviewed: The same keyboard-enabled container intentionally carries tabindex so graph shortcuts and focus management remain on one stable surface. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="workflow-graph-container"
  class:cutting={isCutting}
  bind:this={containerElement}
  tabindex={canEdit ? 0 : -1}
  data-horseshoe-blocked-reason={horseshoeSession.blockedReason ?? undefined}
  data-horseshoe-display-state={horseshoeSession.displayState}
  data-horseshoe-last-trace={horseshoeLastTrace}
  ondrop={handleDrop}
  ondragover={handleDragOver}
  onmousedown={(e) => cutToolRef?.onPaneMouseDown(e)}
  onmousemove={(e) => {
    updateDragCursorFromMouseEvent(e);
    cutToolRef?.onPaneMouseMove(e);
  }}
  onmouseup={(e) => cutToolRef?.onPaneMouseUp(e)}
  role="application"
>

  <SvelteFlow
    bind:nodes
    bind:edges
    {nodeTypes}
    {edgeTypes}
    fitViewOptions={WORKFLOW_GRAPH_FIT_VIEW_OPTIONS}
    nodesConnectable={graphInteractionState.nodesConnectable}
    elementsSelectable={graphInteractionState.elementsSelectable}
    nodesDraggable={graphInteractionState.nodesDraggable}
    panOnDrag={graphInteractionState.panOnDrag}
    panActivationKey={WORKFLOW_GRAPH_PAN_ACTIVATION_KEY}
    zoomOnScroll={true}
    minZoom={WORKFLOW_GRAPH_MIN_ZOOM}
    maxZoom={WORKFLOW_GRAPH_MAX_ZOOM}
    deleteKey={graphInteractionState.deleteKey}
    edgesReconnectable={graphInteractionState.edgesReconnectable}
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
    defaultEdgeOptions={WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS}
  >
    <Controls />
    <MiniMap nodeColor={getWorkflowMiniMapNodeColor} maskColor={WORKFLOW_GRAPH_MINIMAP_MASK_COLOR} />
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

  <WorkflowGraphHorseshoeLayer
    session={horseshoeSession}
    feedback={horseshoeInsertFeedback}
    insertableNodeTypes={$connectionIntentStore?.insertableNodeTypes ?? []}
    selectedIndex={horseshoeSelectedIndex}
    query={horseshoeQuery}
    trace={horseshoeLastTrace}
    onSelect={(candidate) => void commitInsertSelection(candidate)}
    onRotate={rotateInsertSelection}
    onCancel={closeHorseshoeSelector}
  />
</div>
