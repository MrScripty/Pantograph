<script lang="ts">
  import { SvelteFlow, Controls, MiniMap, type Node, type Edge, type Connection } from '@xyflow/svelte';
  import {
    CutTool,
    WorkflowGraphHorseshoeLayer,
    WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS,
    WORKFLOW_GRAPH_FIT_VIEW_OPTIONS,
    WORKFLOW_GRAPH_MAX_ZOOM,
    WORKFLOW_GRAPH_MINIMAP_MASK_COLOR,
    WORKFLOW_GRAPH_MIN_ZOOM,
    WORKFLOW_GRAPH_PAN_ACTIVATION_KEY,
    type HorseshoeDragSessionState,
    type HorseshoeInsertFeedbackState,
  } from '@pantograph/svelte-graph';
  import './WorkflowGraph.css';
  import type { InsertableNodeTypeCandidate } from '../services/workflow/types';
  import WorkflowContainerBoundary from './WorkflowContainerBoundary.svelte';
  import WorkflowEdgeInsertPreviewMarker from './WorkflowEdgeInsertPreviewMarker.svelte';
  import type {
    WorkflowContainerBounds,
    WorkflowContainerViewport,
  } from './workflowContainerBoundary';
  import type { EdgeInsertPreviewState } from './edgeInsertInteraction';
  import { getWorkflowMiniMapNodeColor } from './workflowMiniMap';
  import { workflowEdgeTypes, workflowNodeTypes } from './workflowGraphTypes';

  interface GraphInteractionState {
    deleteKey: string | null;
    edgesReconnectable: boolean;
    elementsSelectable: boolean;
    nodesConnectable: boolean;
    nodesDraggable: boolean;
    panOnDrag: boolean | number[];
  }

  interface Props {
    canEdit: boolean;
    checkValidConnection: (connection: Edge | Connection) => boolean;
    containerBounds: WorkflowContainerBounds | null;
    containerElement: HTMLElement | undefined;
    containerSelected: boolean;
    ctrlPressed: boolean;
    currentViewport: WorkflowContainerViewport | null;
    edgeInsertPreview: EdgeInsertPreviewState;
    edges: Edge[];
    externalPaletteDragActive: boolean;
    graphInteractionState: GraphInteractionState;
    handleConnect: (connection: Connection) => void;
    handleConnectEnd: (
      event: MouseEvent | TouchEvent,
      connectionState: { isValid: boolean },
    ) => void;
    handleConnectStart: (
      event: MouseEvent | TouchEvent,
      params: { nodeId: string; handleId: string | null; handleType: 'source' | 'target' },
    ) => void;
    handleDelete: (deleted: { nodes: Node[]; edges: Edge[] }) => void;
    handleDragOver: (event: DragEvent) => void;
    handleDrop: (event: DragEvent) => void;
    handleEdgesCut: (edgeIds: string[]) => void;
    handleKeyDown: (event: KeyboardEvent) => void;
    handleMove: (
      event: MouseEvent | TouchEvent | null,
      viewport: WorkflowContainerViewport,
    ) => void;
    handleMoveEnd: (
      event: MouseEvent | TouchEvent | null,
      viewport: WorkflowContainerViewport,
    ) => void;
    handleNodeClick: (event: { node: Node }) => void;
    handleNodeDragStop: (event: {
      targetNode: Node | null;
      nodes: Node[];
      event: MouseEvent | TouchEvent;
    }) => void;
    handlePaneClick: () => void;
    handlePaneMouseMove: (event: MouseEvent) => void;
    handleReconnect: (oldEdge: Edge, newConnection: Connection) => void;
    handleReconnectEnd: (
      event: MouseEvent | TouchEvent,
      edge: Edge,
      handleType: unknown,
      connectionState: { isValid: boolean },
    ) => void;
    handleReconnectStart: (
      event: MouseEvent | TouchEvent,
      edge: Edge,
      handleType: 'source' | 'target',
    ) => void;
    handleSelectionChange: (event: { nodes: Node[]; edges: Edge[] }) => void;
    horseshoeInsertFeedback: HorseshoeInsertFeedbackState;
    horseshoeLastTrace: string;
    horseshoeQuery: string;
    horseshoeSelectedIndex: number;
    horseshoeSession: HorseshoeDragSessionState;
    insertableNodeTypes: InsertableNodeTypeCandidate[];
    isCutting: boolean;
    nodes: Node[];
    onCancelHorseshoe: () => void;
    onRotateInsertSelection: (delta: number) => void;
    onSelectInsertCandidate: (candidate: InsertableNodeTypeCandidate) => void;
    onToggleContainerSelection: () => void;
  }

  let {
    canEdit,
    checkValidConnection,
    containerBounds,
    containerElement = $bindable(),
    containerSelected,
    ctrlPressed = $bindable(),
    currentViewport,
    edgeInsertPreview,
    edges = $bindable(),
    externalPaletteDragActive,
    graphInteractionState,
    handleConnect,
    handleConnectEnd,
    handleConnectStart,
    handleDelete,
    handleDragOver,
    handleDrop,
    handleEdgesCut,
    handleKeyDown,
    handleMove,
    handleMoveEnd,
    handleNodeClick,
    handleNodeDragStop,
    handlePaneClick,
    handlePaneMouseMove,
    handleReconnect,
    handleReconnectEnd,
    handleReconnectStart,
    handleSelectionChange,
    horseshoeInsertFeedback,
    horseshoeLastTrace,
    horseshoeQuery,
    horseshoeSelectedIndex,
    horseshoeSession,
    insertableNodeTypes,
    isCutting = $bindable(),
    nodes = $bindable(),
    onCancelHorseshoe,
    onRotateInsertSelection,
    onSelectInsertCandidate,
    onToggleContainerSelection,
  }: Props = $props();

  let cutToolRef: CutTool;

  function handlePaneMouseDown(event: MouseEvent) {
    if (externalPaletteDragActive) {
      return;
    }

    cutToolRef?.onPaneMouseDown(event);
  }

  function handleCanvasPaneMouseMove(event: MouseEvent) {
    handlePaneMouseMove(event);
    if (externalPaletteDragActive) {
      return;
    }

    cutToolRef?.onPaneMouseMove(event);
  }

  function handlePaneMouseUp(event: MouseEvent) {
    if (externalPaletteDragActive) {
      return;
    }

    cutToolRef?.onPaneMouseUp(event);
  }
</script>

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
  onmousemove={handleCanvasPaneMouseMove}
  onmouseup={handlePaneMouseUp}
  role="application"
>
  <SvelteFlow
    bind:nodes
    bind:edges
    nodeTypes={workflowNodeTypes}
    edgeTypes={workflowEdgeTypes}
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
    onnodedragstop={handleNodeDragStop}
    onnodeclick={handleNodeClick}
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

  <WorkflowContainerBoundary
    bounds={containerBounds}
    viewport={currentViewport}
    selected={containerSelected}
    onToggleSelected={onToggleContainerSelection}
  />

  {#if edgeInsertPreview.bridge && edgeInsertPreview.hitPoint}
    <WorkflowEdgeInsertPreviewMarker hitPoint={edgeInsertPreview.hitPoint} />
  {/if}

  <WorkflowGraphHorseshoeLayer
    session={horseshoeSession}
    feedback={horseshoeInsertFeedback}
    insertableNodeTypes={insertableNodeTypes}
    selectedIndex={horseshoeSelectedIndex}
    query={horseshoeQuery}
    trace={horseshoeLastTrace}
    onSelect={onSelectInsertCandidate}
    onRotate={onRotateInsertSelection}
    onCancel={onCancelHorseshoe}
  />

  <CutTool
    bind:this={cutToolRef}
    edges={edges}
    enabled={canEdit && !externalPaletteDragActive}
    bind:ctrlPressed
    bind:isCutting
    onEdgesCut={handleEdgesCut}
  />
</div>
