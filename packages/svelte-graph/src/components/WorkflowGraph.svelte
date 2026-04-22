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
  import { get } from 'svelte/store';

  import { useGraphContext } from '../context/useGraphContext.js';
  import { applyWorkflowGraphMutationResponse } from '../stores/workflowGraphMutationResponse.js';
  import {
    buildConnectionIntentState,
    edgeToGraphEdge,
    isWorkflowConnectionValid,
  } from '../workflowConnections.js';
  import { computeWorkflowGraphSyncDecision } from '../workflowGraphSync.js';
  import type {
    ConnectionAnchor,
    ConnectionCandidatesResponse,
    ConnectionCommitResponse,
    InsertableNodeTypeCandidate,
  } from '../types/workflow.js';
  import {
    findBestInsertableMatchIndex,
    rotateHorseshoeIndex,
  } from '../horseshoeSelector.js';
  import { formatHorseshoeBlockedReason } from '../horseshoeInvocation.js';
  import {
    isEditableKeyboardTarget,
    resolveHorseshoeKeyboardAction,
  } from '../workflowHorseshoeKeyboard.js';
  import {
    clearHorseshoeInsertFeedback,
    createHorseshoeInsertFeedbackState,
    rejectHorseshoeInsertFeedback,
    resolveHorseshoeStatusLabel,
    startHorseshoeInsertFeedback,
    type HorseshoeInsertFeedbackState,
  } from '../horseshoeInsertFeedback.js';
  import {
    clearHorseshoeDragSession,
    createHorseshoeDragSessionState,
    requestHorseshoeDisplay,
    startHorseshoeDrag,
    syncHorseshoeDisplay,
    type HorseshoeBlockedReason,
    type HorseshoeDragSessionState,
  } from '../horseshoeDragSession.js';
  import {
    clearConnectionDragState,
    createConnectionDragState,
    markConnectionDragFinalizing,
    shouldRemoveReconnectedEdge,
    startConnectionDrag,
    startReconnectDrag,
    supportsInsertFromConnectionDrag,
    type ConnectionDragState,
  } from '../connectionDragState.js';
  import {
    WORKFLOW_PALETTE_DRAG_END_EVENT,
    WORKFLOW_PALETTE_DRAG_START_EVENT,
  } from '../paletteDragState.js';
  import { resolveReconnectSourceAnchor } from '../reconnectInteraction.js';
  import { resolveWorkflowDragCursorUpdate } from '../workflowDragCursor.js';
  import {
    resolveWorkflowGroupZoomTarget,
    resolveWorkflowNodeClick,
    type WorkflowNodeClickState,
  } from '../workflowNodeActivation.js';
  import {
    readWorkflowPaletteDragDefinition,
    resolveWorkflowPaletteDropPosition,
  } from '../workflowPaletteDrag.js';
  import { resolveWorkflowInsertPositionHint } from '../workflowInsertPosition.js';
  import { getWorkflowMiniMapNodeColor } from '../workflowMiniMap.js';
  import CutTool from './CutTool.svelte';
  import ContainerBorder from './ContainerBorder.svelte';
  import HorseshoeInsertSelector from './HorseshoeInsertSelector.svelte';
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
  const {
    isEditing,
    nodeDefinitions: nodeDefsStore,
    selectedNodeIds: selectedNodeIdsStore,
    workflowGraph: workflowGraphStore,
    workflowMetadata: workflowMetadataStore,
  } =
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
  let nodeClickState = $state<WorkflowNodeClickState>({
    lastClickTime: 0,
    lastClickNodeId: null,
  });

  // Container element reference for size calculations
  let containerElement = $state<HTMLElement | null>(null);

  // Current viewport state for container border rendering
  let currentViewport = $state<{ x: number; y: number; zoom: number } | null>(null);

  // CutTool ref and bindable state
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

  // Reset container border transition when returning to data-graph view
  $effect(() => {
    if ($viewLevel === 'data-graph') {
      containerBorderRef?.resetTransition();
    }
  });

  // Initialize node definitions on mount
  onMount(async () => {
    window.addEventListener('keydown', handleWindowKeyDown, true);
    window.addEventListener(WORKFLOW_PALETTE_DRAG_START_EVENT, handleWorkflowPaletteDragStart);
    window.addEventListener(WORKFLOW_PALETTE_DRAG_END_EVENT, handleWorkflowPaletteDragEnd);
    window.addEventListener('blur', handleWorkflowPaletteDragEnd);

    const definitions = await backend.getNodeDefinitions();
    nodeDefsStore.set(definitions);

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

    if ($connectionIntentStore.insertableNodeTypes.length > 0) {
      horseshoeSelectedIndex = Math.max(
        0,
        Math.min(horseshoeSelectedIndex, $connectionIntentStore.insertableNodeTypes.length - 1),
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
    selectedNodeIdsStore.set(selectedNodes.map((node) => node.id));
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
      hasConnectionIntent: Boolean($connectionIntentStore),
      insertableCount: $connectionIntentStore?.insertableNodeTypes.length ?? 0,
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
    horseshoeLastTrace = [
      'request-open',
      horseshoeSession.dragActive ? 'drag' : 'idle',
      connectionDragState.mode,
      $connectionIntentStore ? 'intent' : 'no-intent',
      `${$connectionIntentStore?.insertableNodeTypes.length ?? 0}-insertables`,
      horseshoeSession.anchorPosition ? 'anchor' : 'no-anchor',
    ].join(':');
    applyHorseshoeSession(requestHorseshoeDisplay(horseshoeSession, getHorseshoeOpenContext()));
  }

  function rotateInsertSelection(delta: number) {
    if (!$connectionIntentStore || $connectionIntentStore.insertableNodeTypes.length === 0) return;

    horseshoeInsertFeedback = clearHorseshoeInsertFeedback();
    horseshoeSelectedIndex = rotateHorseshoeIndex(
      horseshoeSelectedIndex,
      delta,
      $connectionIntentStore.insertableNodeTypes.length,
    );
  }

  function updateInsertQuery(nextQuery: string) {
    horseshoeInsertFeedback = clearHorseshoeInsertFeedback();
    horseshoeQuery = nextQuery;
    horseshoeSelectedIndex = findBestInsertableMatchIndex(
      $connectionIntentStore?.insertableNodeTypes ?? [],
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
      const response = await backend.insertNodeAndConnect(
        connectionIntent.sourceAnchor,
        candidate.node_type,
        sessionId,
        connectionIntent.graphRevision || getGraphRevision(),
        positionHint,
        candidate.matching_input_port_ids[0],
      );

      if (response.accepted && response.graph) {
        stores.workflow.loadWorkflow(response.graph, get(workflowMetadataStore) ?? undefined);
        applyWorkflowGraphMutationResponse(response, {
          setNodeExecutionState: stores.workflow.setNodeExecutionState,
        });
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

  function getHorseshoeStatusLabel(): string | null {
    return resolveHorseshoeStatusLabel({
      pending: horseshoeInsertFeedback.pending,
      rejectionMessage: horseshoeInsertFeedback.rejectionMessage,
      displayState: horseshoeSession.displayState,
      blockedReason: horseshoeSession.blockedReason,
    });
  }

  function handleWindowKeyDown(event: KeyboardEvent) {
    if (isEditableKeyboardTarget(event.target as HTMLElement | null)) {
      return;
    }

    const action = resolveHorseshoeKeyboardAction(event, {
      displayState: horseshoeSession.displayState,
      dragActive: horseshoeSession.dragActive,
      pending: horseshoeInsertFeedback.pending,
      hasSelection: Boolean($connectionIntentStore?.insertableNodeTypes[horseshoeSelectedIndex]),
    });

    if (action.preventDefault) {
      event.preventDefault();
    }

    switch (action.type) {
      case 'request-open':
        horseshoeLastTrace = 'keydown:space';
        requestHorseshoeOpen();
        return;
      case 'confirm-selection': {
        horseshoeLastTrace = event.key === 'Enter' ? 'keydown:enter' : 'keydown:space';
        const candidate = $connectionIntentStore?.insertableNodeTypes[horseshoeSelectedIndex];
        if (candidate) {
          void commitInsertSelection(candidate);
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

  function checkValidConnection(connection: Edge | Connection): boolean {
    return isWorkflowConnectionValid(connection, nodes, $connectionIntentStore);
  }

  function getGraphRevision(): string {
    return get(workflowGraphStore).derived_graph?.graph_fingerprint ?? '';
  }

  function setConnectionIntentState(
    candidates: ConnectionCandidatesResponse,
    rejection?: ConnectionCommitResponse['rejection'],
  ) {
    stores.workflow.setConnectionIntent(buildConnectionIntentState(candidates, rejection));
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
      const candidates = await backend.getConnectionCandidates(
        sourceAnchor,
        sessionId,
        options?.graphRevision ?? getGraphRevision(),
      );

      if (requestId !== connectionIntentRequestId) return;
      setConnectionIntentState(candidates, options?.rejection);
    } catch (error) {
      if (requestId === connectionIntentRequestId) {
        if (options?.preserveDisplay) {
          stores.workflow.setConnectionIntent({
            sourceAnchor,
            graphRevision: options?.graphRevision ?? getGraphRevision(),
            compatibleNodeIds: $connectionIntentStore?.compatibleNodeIds ?? [],
            compatibleTargetKeys: $connectionIntentStore?.compatibleTargetKeys ?? [],
            insertableNodeTypes: $connectionIntentStore?.insertableNodeTypes ?? [],
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
      applyWorkflowGraphMutationResponse(response, {
        setNodeExecutionState: stores.workflow.setNodeExecutionState,
      });
      clearConnectionInteraction();
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
    if (
      horseshoeSession.displayState === 'open' ||
      horseshoeInsertFeedback.pending ||
      horseshoeSession.openRequested
    ) return;
    clearConnectionInteraction();
  }

  async function handleConnect(connection: Connection) {
    if (!canEdit) return;

    const response = await commitConnection(connection);
    if (!response?.accepted) return;
  }

  async function handleDelete({ nodes: deletedNodes, edges: deletedEdges }: { nodes: Node[]; edges: Edge[] }) {
    if (!canEdit) return;

    const sessionId = get(currentSessionId) || '';
    clearConnectionInteraction();

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

  // --- Edge Reconnection ---

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
        applyWorkflowGraphMutationResponse(response, {
          setNodeExecutionState: stores.workflow.setNodeExecutionState,
        });
        clearConnectionInteraction();
        return;
      }

      const restoredGraph = await backend.addEdge(edgeToGraphEdge(oldEdge), sessionId);
      stores.workflow.syncEdgesFromBackend(restoredGraph);

      if (response.rejection) {
        stores.workflow.setConnectionIntent({
          sourceAnchor:
            connectionDragState.reconnectingSourceAnchor ??
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

    const reconnectingEdgeId = shouldRemoveReconnectedEdge(connectionDragState, connectionState);
    if (reconnectingEdgeId) {
      try {
        const sessionId = get(currentSessionId) || '';
        const updatedGraph = await backend.removeEdge(reconnectingEdgeId, sessionId);
        stores.workflow.syncEdgesFromBackend(updatedGraph);
      } catch (error) {
        console.error('[WorkflowGraph] Failed to notify backend of edge removal:', error);
      }
    }

    clearConnectionInteraction();
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
    clearConnectionInteraction();
  }

  // --- Cut tool edge removal ---

  async function handleEdgesCut(edgeIds: string[]) {
    const sessionId = get(currentSessionId) || '';
    clearConnectionInteraction();
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

<svelte:window onmousemove={updateDragCursorFromMouseEvent} />

<!-- a11y-reviewed: SvelteFlow graph canvas owns pointer interaction while keyboard graph commands are handled on this focusable container. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
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

  <HorseshoeInsertSelector
    displayState={horseshoeSession.displayState}
    anchorPosition={horseshoeSession.anchorPosition}
    items={$connectionIntentStore?.insertableNodeTypes ?? []}
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
