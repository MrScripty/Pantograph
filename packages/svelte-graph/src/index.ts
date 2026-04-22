// @pantograph/svelte-graph — Reusable node graph editor for Svelte 5

// --- Types ---
export type {
  PortDataType,
  PortDefinition,
  NodeCategory,
  ExecutionMode,
  NodeDefinition,
  GraphNode,
  GraphEdge,
  ConnectionAnchor,
  ConnectionTargetAnchorCandidate,
  ConnectionTargetNodeCandidate,
  InsertableNodeTypeCandidate,
  InsertNodePositionHint,
  ConnectionCandidatesResponse,
  ConnectionRejectionReason,
  ConnectionRejection,
  ConnectionCommitResponse,
  EdgeInsertionBridge,
  EdgeInsertionPreviewResponse,
  InsertNodeConnectionResponse,
  InsertNodeOnEdgeResponse,
  ConnectionIntentState,
  WorkflowGraph,
  WorkflowMetadata,
  WorkflowFile,
  WorkflowRuntimeRequirements,
  WorkflowRuntimeInstallState,
  WorkflowRuntimeCapability,
  WorkflowCapabilityModel,
  WorkflowCapabilitiesResponse,
  WorkflowSessionHandle,
  WorkflowSessionKind,
  WorkflowSessionState,
  WorkflowSessionSummary,
  WorkflowSessionQueueItemStatus,
  WorkflowSessionQueueItem,
  WorkflowEventType,
  WorkflowEventData,
  WorkflowEvent,
  WorkflowGraphMutationResponse,
  NodeExecutionState,
  NodeExecutionInfo,
} from './types/workflow.js';

export type {
  PortMapping,
  NodeGroup,
  CreateGroupResult,
  ExpandGroupResult,
  GroupNavigationState,
  GroupBreadcrumbItem,
} from './types/groups.js';

export type {
  ViewLevel,
  BreadcrumbItem,
  ViewportState,
  ZoomTarget,
  AnimationConfig,
} from './types/view.js';

export type { NodeTypeRegistry } from './types/registry.js';

export type {
  WorkflowBackend,
  UndoRedoState,
  PortOption,
  PortOptionsResult,
  PortOptionsQuery,
} from './types/backend.js';

// --- Constants ---
export { PORT_TYPE_COLORS, getPortColor } from './constants/portColors.js';
export { DEFAULT_ANIMATION } from './types/view.js';

// --- Store Factories ---
export { createWorkflowStores } from './stores/createWorkflowStores.js';
export type { WorkflowStores } from './stores/createWorkflowStores.js';
export { applyWorkflowGraphMutationResponse } from './stores/workflowGraphMutationResponse.js';

export { createViewStores } from './stores/createViewStores.js';
export type { ViewStores, ViewStoreOptions } from './stores/createViewStores.js';

export { createSessionStores } from './stores/createSessionStores.js';
export type { SessionStores, SessionStoreOptions, GraphType, GraphInfo, SessionKind } from './stores/createSessionStores.js';
export {
  claimWorkflowExecutionIdFromEvent,
  getWorkflowEventExecutionId,
  isWorkflowEventRelevantToExecution,
  projectWorkflowEventOwnership,
} from './workflowEventOwnership.js';
export type { WorkflowEventOwnershipProjection } from './workflowEventOwnership.js';
export type { ExecutionScopedWorkflowEvent } from './workflowEventOwnership.js';

// --- Context ---
export { createGraphContext, createGraphContextFromStores } from './context/createGraphContext.js';
export type { GraphContextOptions } from './context/createGraphContext.js';

export { useGraphContext } from './context/useGraphContext.js';

export type { GraphContext } from './context/types.js';

// --- Backends ---
export { MockWorkflowBackend, MOCK_NODE_DEFINITIONS } from './backends/MockWorkflowBackend.js';

// --- Utilities ---
export { linesIntersect } from './utils/geometry.js';
export {
  buildDerivedGraph,
  computeConsumerCountMap,
  computeGraphFingerprint,
  withDerivedGraph,
} from './graphRevision.js';
export {
  buildConnectionIntentState,
  edgeToGraphEdge,
  isWorkflowConnectionValid,
  preserveConnectionIntentState,
} from './workflowConnections.js';
export {
  HORSESHOE_VISIBLE_COUNT,
  HORSESHOE_OUTER_RADIUS,
  HORSESHOE_SELECTION_RADIUS,
  HORSESHOE_START_ANGLE,
  HORSESHOE_END_ANGLE,
  clampHorseshoeIndex,
  findNearestVisibleHorseshoeIndex,
  rotateHorseshoeIndex,
  getHorseshoeItemPosition,
  getHorseshoeWindow,
  findBestInsertableMatchIndex,
} from './horseshoeSelector.js';
export {
  isSpaceKey,
  resolveHorseshoeOpenRequest,
  resolveHorseshoeSpaceKeyAction,
  shouldUpdateHorseshoeAnchorFromPointer,
  formatHorseshoeBlockedReason,
} from './horseshoeInvocation.js';
export type {
  HorseshoeBlockedReason,
  HorseshoeOpenContext,
  HorseshoeOpenResolution,
  HorseshoeSpaceKeyAction,
  HorseshoeSpaceKeyContext,
} from './horseshoeInvocation.js';
export {
  clearHorseshoeInsertFeedback,
  createHorseshoeInsertFeedbackState,
  rejectHorseshoeInsertFeedback,
  resolveHorseshoeSessionStatusLabel,
  resolveHorseshoeStatusLabel,
  startHorseshoeInsertFeedback,
} from './horseshoeInsertFeedback.js';
export type {
  HorseshoeInsertFeedbackState,
  HorseshoeStatusContext,
} from './horseshoeInsertFeedback.js';
export {
  clearConnectionDragState,
  createConnectionDragState,
  markConnectionDragFinalizing,
  shouldRemoveReconnectedEdge,
  startConnectionDrag,
  startReconnectDrag,
  supportsInsertFromConnectionDrag,
} from './connectionDragState.js';
export type {
  ConnectionDragMode,
  ConnectionDragState,
} from './connectionDragState.js';
export {
  clearWorkflowConnectionDragInteraction,
  shouldClearWorkflowConnectionInteractionAfterConnectEnd,
} from './workflowConnectionInteraction.js';
export type {
  WorkflowConnectionDragInteractionState,
} from './workflowConnectionInteraction.js';
export {
  closeHorseshoeDisplay,
  clearHorseshoeDragSession,
  createHorseshoeDragSessionState,
  requestHorseshoeDisplay,
  startHorseshoeDrag,
  syncHorseshoeDisplay,
  updateHorseshoeAnchor,
} from './horseshoeDragSession.js';
export type {
  HorseshoeAnchorPosition,
  HorseshoeDisplayState,
  HorseshoeDragSessionState,
} from './horseshoeDragSession.js';
export {
  resolveWorkflowHorseshoeSessionUpdate,
} from './workflowHorseshoeSessionUpdate.js';
export type {
  WorkflowHorseshoeSessionUpdate,
  WorkflowHorseshoeSessionViewState,
} from './workflowHorseshoeSessionUpdate.js';
export {
  isEditableKeyboardTarget,
  resolveHorseshoeKeyboardAction,
} from './workflowHorseshoeKeyboard.js';
export type {
  HorseshoeKeyboardAction,
  HorseshoeKeyboardContext,
} from './workflowHorseshoeKeyboard.js';
export {
  normalizeWorkflowHorseshoeSelectedIndex,
  resolveWorkflowHorseshoeQueryUpdate,
  resolveWorkflowHorseshoeSelectionSnapshot,
  rotateWorkflowHorseshoeSelection,
} from './workflowHorseshoeSelection.js';
export type {
  WorkflowHorseshoeQueryUpdate,
  WorkflowHorseshoeSelectionSnapshot,
} from './workflowHorseshoeSelection.js';
export {
  formatWorkflowHorseshoeOpenRequestTrace,
  formatWorkflowHorseshoeSessionTrace,
} from './workflowHorseshoeTrace.js';
export { requestWorkflowHorseshoeOpen } from './workflowHorseshoeOpenRequest.js';
export type { WorkflowHorseshoeOpenRequestResult } from './workflowHorseshoeOpenRequest.js';
export { buildWorkflowHorseshoeOpenContext } from './workflowHorseshoeOpenContext.js';
export type { WorkflowHorseshoeOpenContextInput } from './workflowHorseshoeOpenContext.js';
export {
  WORKFLOW_NODE_DOUBLE_CLICK_THRESHOLD_MS,
  isWorkflowGroupNode,
  resolveWorkflowGroupZoomTarget,
  resolveWorkflowNodeClick,
} from './workflowNodeActivation.js';
export type {
  WorkflowGroupZoomTarget,
  WorkflowNodeActivationLike,
  WorkflowNodeClickDecision,
  WorkflowNodeClickState,
} from './workflowNodeActivation.js';
export { isPortTypeCompatible } from './portTypeCompatibility.js';
export {
  applySelectedNodeIds,
  collectSelectedNodeIds,
} from './workflowSelection.js';
export {
  WORKFLOW_PALETTE_DRAG_END_EVENT,
  WORKFLOW_PALETTE_DRAG_START_EVENT,
  dispatchWorkflowPaletteDragEnd,
  dispatchWorkflowPaletteDragStart,
} from './paletteDragState.js';
export { resolveWorkflowDragCursorUpdate } from './workflowDragCursor.js';
export type {
  WorkflowDragCursorDecision,
  WorkflowDragCursorPosition,
} from './workflowDragCursor.js';
export {
  WORKFLOW_PALETTE_DROP_NODE_OFFSET,
  readWorkflowPaletteDragDefinition,
  resolveWorkflowPaletteDropPosition,
} from './workflowPaletteDrag.js';
export type {
  WorkflowPaletteContainerBounds,
  WorkflowPaletteDragDataTransfer,
  WorkflowPaletteDragEvent,
  WorkflowPalettePointerPosition,
} from './workflowPaletteDrag.js';
export {
  resolveWorkflowPointerClientPosition,
  resolveWorkflowRelativePointerPosition,
} from './workflowPointerPosition.js';
export type {
  WorkflowPointerBounds,
  WorkflowPointerClientPosition,
  WorkflowPointerEventLike,
  WorkflowPointerPosition,
  WorkflowPointerTouchListLike,
} from './workflowPointerPosition.js';
export { WORKFLOW_GRAPH_DEFAULT_EDGE_OPTIONS } from './workflowGraphEdgeOptions.js';
export { resolveWorkflowInsertPositionHint } from './workflowInsertPosition.js';
export type {
  WorkflowInsertAnchorPosition,
  WorkflowInsertViewport,
} from './workflowInsertPosition.js';

// --- Components ---
export { default as WorkflowGraphEditor } from './components/WorkflowGraph.svelte';
export { default as WorkflowToolbar } from './components/WorkflowToolbar.svelte';
export { default as NodePalette } from './components/NodePalette.svelte';
export { default as NavigationBreadcrumb } from './components/NavigationBreadcrumb.svelte';
export { default as ZoomTransition } from './components/ZoomTransition.svelte';
export { default as NodeGroupEditor } from './components/NodeGroupEditor.svelte';
export { default as GroupPortMapper } from './components/GroupPortMapper.svelte';
export { default as ContainerBorder } from './components/ContainerBorder.svelte';
export { default as CutTool } from './components/CutTool.svelte';
export { default as HorseshoeDebugOverlay } from './components/HorseshoeDebugOverlay.svelte';
export { default as HorseshoeInsertSelector } from './components/HorseshoeInsertSelector.svelte';

// --- Node/Edge Components ---
export { default as BaseNode } from './components/nodes/BaseNode.svelte';
export { default as GenericNode } from './components/nodes/GenericNode.svelte';
export { default as TextInputNode } from './components/nodes/TextInputNode.svelte';
export { default as TextOutputNode } from './components/nodes/TextOutputNode.svelte';
export { default as LlamaCppInferenceNode } from './components/nodes/LlamaCppInferenceNode.svelte';
export { default as PumaLibNode } from './components/nodes/PumaLibNode.svelte';
export { default as ReconnectableEdge } from './components/edges/ReconnectableEdge.svelte';

// --- Registry Builder ---
export { buildRegistry } from './utils/buildRegistry.js';
