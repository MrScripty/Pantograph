// Type definitions for Node Groups
// These types must match the Rust types in src-tauri/src/workflow/groups.rs

import type { PortDataType, GraphNode, GraphEdge } from './types';

/**
 * Mapping from a group-level port to an internal node's port
 */
export interface PortMapping {
  /** The ID of the internal node that has the actual port */
  internal_node_id: string;
  /** The port ID on the internal node */
  internal_port_id: string;
  /** The port ID as it appears on the collapsed group node */
  group_port_id: string;
  /** Human-readable label for the group port */
  group_port_label: string;
  /** Data type of the port */
  data_type: PortDataType;
}

/**
 * A node group containing multiple nodes
 */
export interface NodeGroup {
  /** Unique identifier for this group */
  id: string;
  /** Human-readable name for the group */
  name: string;
  /** Nodes contained within this group */
  nodes: GraphNode[];
  /** Edges connecting nodes within this group */
  edges: GraphEdge[];
  /** Input ports exposed at the group level */
  exposed_inputs: PortMapping[];
  /** Output ports exposed at the group level */
  exposed_outputs: PortMapping[];
  /** Position of the collapsed group node on the canvas */
  position: { x: number; y: number };
  /** Whether the group is currently collapsed */
  collapsed: boolean;
  /** Optional description */
  description?: string;
  /** Optional color/theme */
  color?: string;
}

/**
 * Result of creating a group from selected nodes
 */
export interface CreateGroupResult {
  /** The created group */
  group: NodeGroup;
  /** IDs of edges that were internalized (moved into the group) */
  internalized_edge_ids: string[];
  /** IDs of edges that cross the group boundary */
  boundary_edge_ids: string[];
  /** Suggested input port mappings */
  suggested_inputs: PortMapping[];
  /** Suggested output port mappings */
  suggested_outputs: PortMapping[];
}

/**
 * Result of expanding a group
 */
export interface ExpandGroupResult {
  /** Nodes that were inside the group */
  nodes: GraphNode[];
  /** Edges that were inside the group */
  edges: GraphEdge[];
  /** The group ID that was expanded */
  group_id: string;
}

/**
 * State for tracking group navigation (for breadcrumb)
 */
export interface GroupNavigationState {
  /** Stack of group IDs we've "tabbed into" */
  groupStack: string[];
  /** Currently expanded group ID (top of stack), or null if at root */
  currentGroupId: string | null;
}

/**
 * Breadcrumb item for group navigation
 */
export interface GroupBreadcrumbItem {
  id: string;
  name: string;
  isRoot: boolean;
}
