// Type definitions for Node Groups
// These types must match the Rust types in src-tauri/src/workflow/groups.rs

import type { PortDataType, GraphNode, GraphEdge } from './workflow.js';

/** Mapping from a group-level port to an internal node's port */
export interface PortMapping {
  internal_node_id: string;
  internal_port_id: string;
  group_port_id: string;
  group_port_label: string;
  data_type: PortDataType;
}

/** A node group containing multiple nodes */
export interface NodeGroup {
  id: string;
  name: string;
  nodes: GraphNode[];
  edges: GraphEdge[];
  exposed_inputs: PortMapping[];
  exposed_outputs: PortMapping[];
  position: { x: number; y: number };
  collapsed: boolean;
  description?: string;
  color?: string;
}

/** Result of creating a group from selected nodes */
export interface CreateGroupResult {
  group: NodeGroup;
  internalized_edge_ids: string[];
  boundary_edge_ids: string[];
  suggested_inputs: PortMapping[];
  suggested_outputs: PortMapping[];
}

/** Result of expanding a group */
export interface ExpandGroupResult {
  nodes: GraphNode[];
  edges: GraphEdge[];
  group_id: string;
}

/** State for tracking group navigation (for breadcrumb) */
export interface GroupNavigationState {
  groupStack: string[];
  currentGroupId: string | null;
}

/** Breadcrumb item for group navigation */
export interface GroupBreadcrumbItem {
  id: string;
  name: string;
  isRoot: boolean;
}
