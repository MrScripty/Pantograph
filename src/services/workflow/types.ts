// Workflow type definitions for the node-based visual programming system
// NOTE: These types must match the Rust types in src-tauri/src/workflow/types.rs
// Rust uses snake_case serialization for enums

export type PortDataType =
  | 'any'
  | 'string'
  | 'image'
  | 'audio'
  | 'component'
  | 'stream'
  | 'prompt'
  | 'tools'
  | 'embedding'
  | 'document'
  | 'json'
  | 'boolean'
  | 'number'
  | 'vector_db';

export interface PortDefinition {
  id: string;
  label: string;
  data_type: PortDataType;
  required: boolean;
  multiple: boolean;
}

export type NodeCategory = 'input' | 'processing' | 'tool' | 'output' | 'control';
export type ExecutionMode = 'reactive' | 'manual' | 'stream';

export interface NodeDefinition {
  node_type: string;
  category: NodeCategory;
  label: string;
  description: string;
  inputs: PortDefinition[];
  outputs: PortDefinition[];
  execution_mode: ExecutionMode;
}

export interface GraphNode {
  id: string;
  node_type: string;
  position: { x: number; y: number };
  data: Record<string, unknown>;
}

export interface GraphEdge {
  id: string;
  source: string;
  source_handle: string;
  target: string;
  target_handle: string;
}

export interface WorkflowGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface WorkflowMetadata {
  id?: string; // Filename stem (e.g., "coding-agent") for loading, populated by list_workflows
  name: string;
  description?: string;
  created: string;
  modified: string;
  orchestrationId?: string; // Optional link to parent orchestration for zoom-out navigation
}

// Link mapping types for GUI element linking
export type LinkStatus = 'linked' | 'unlinked' | 'error';

export interface LinkMapping {
  nodeId: string;
  elementId: string;
  elementLabel: string;
  status: LinkStatus;
  errorMessage?: string;
  lastValue?: string;
}

export interface WorkflowFile {
  version: string;
  metadata: WorkflowMetadata;
  graph: WorkflowGraph;
  viewport?: { x: number; y: number; zoom: number };
  linkMappings?: LinkMapping[]; // Persisted link mappings for linked-input nodes
}

export type WorkflowEventType =
  | 'Started'
  | 'NodeStarted'
  | 'NodeProgress'
  | 'NodeStream'
  | 'NodeCompleted'
  | 'NodeError'
  | 'Completed'
  | 'Failed'
  | 'GraphModified';

export interface WorkflowEventData {
  Started: { workflow_id: string; node_count: number };
  NodeStarted: { node_id: string; node_type: string };
  NodeProgress: { node_id: string; progress: number; message: string };
  NodeStream: { node_id: string; port: string; chunk: { type: string; content: string } };
  NodeCompleted: { node_id: string; outputs: Record<string, unknown> };
  NodeError: { node_id: string; error: string };
  Completed: { outputs: Record<string, unknown> };
  Failed: { error: string };
  GraphModified: { graph: WorkflowGraph };
}

export interface WorkflowEvent<T extends WorkflowEventType = WorkflowEventType> {
  type: T;
  data: WorkflowEventData[T];
}

export type NodeExecutionState = 'idle' | 'running' | 'success' | 'error';

/** Extended execution info including error messages */
export interface NodeExecutionInfo {
  state: NodeExecutionState;
  errorMessage?: string;
}

// Port options query types (matches node-engine PortOption/PortOptionsResult)

/** A selectable option for a port value */
export interface PortOption {
  value: unknown;
  label: string;
  description?: string;
  metadata?: Record<string, unknown>;
}

/** Result of a port options query */
export interface PortOptionsResult {
  options: PortOption[];
  totalCount: number;
  searchable: boolean;
}
