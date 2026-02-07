// Node type registry for injectable component mapping

import type { Component } from 'svelte';

/**
 * Registry mapping node type strings to Svelte components.
 * Consumers provide this when creating a graph context to control
 * which components render each node type.
 */
export interface NodeTypeRegistry {
  /** Map of node_type string to Svelte component */
  nodeTypes: Record<string, Component<any>>;
  /** Fallback component for unknown node types */
  fallbackNode: Component<any>;
  /** Optional custom edge types (defaults to ReconnectableEdge) */
  edgeTypes?: Record<string, Component<any>>;
}
