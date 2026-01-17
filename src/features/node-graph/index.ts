/**
 * Node Graph Feature Module
 *
 * Blender-style dataflow node editor for visual programming.
 */

// Components
export { default as NodeGraph } from '../../components/NodeGraph.svelte';

// Stores
export { nodes, edges, updateNodePosition, updateNodeData } from '../../stores/nodeGraphStore';

// View mode management
export { viewMode, toggleViewMode } from '../../stores/viewModeStore';
export type { ViewMode } from '../../stores/viewModeStore';
