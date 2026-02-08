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
  WorkflowGraph,
  WorkflowMetadata,
  WorkflowFile,
  WorkflowEventType,
  WorkflowEventData,
  WorkflowEvent,
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

export type { WorkflowBackend, UndoRedoState } from './types/backend.js';

// --- Constants ---
export { PORT_TYPE_COLORS, getPortColor } from './constants/portColors.js';
export { DEFAULT_ANIMATION } from './types/view.js';

// --- Store Factories ---
export { createWorkflowStores } from './stores/createWorkflowStores.js';
export type { WorkflowStores } from './stores/createWorkflowStores.js';

export { createViewStores } from './stores/createViewStores.js';
export type { ViewStores, ViewStoreOptions } from './stores/createViewStores.js';

export { createSessionStores } from './stores/createSessionStores.js';
export type { SessionStores, SessionStoreOptions, GraphType, GraphInfo } from './stores/createSessionStores.js';

// --- Context ---
export { createGraphContext, createGraphContextFromStores } from './context/createGraphContext.js';
export type { GraphContextOptions } from './context/createGraphContext.js';

export { useGraphContext } from './context/useGraphContext.js';

export type { GraphContext } from './context/types.js';

// --- Backends ---
export { MockWorkflowBackend, MOCK_NODE_DEFINITIONS } from './backends/MockWorkflowBackend.js';

// --- Utilities ---
export { linesIntersect } from './utils/geometry.js';

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

// --- Node/Edge Components ---
export { default as BaseNode } from './components/nodes/BaseNode.svelte';
export { default as GenericNode } from './components/nodes/GenericNode.svelte';
export { default as TextInputNode } from './components/nodes/TextInputNode.svelte';
export { default as TextOutputNode } from './components/nodes/TextOutputNode.svelte';
export { default as LlamaCppInferenceNode } from './components/nodes/LlamaCppInferenceNode.svelte';
export { default as ReconnectableEdge } from './components/edges/ReconnectableEdge.svelte';
