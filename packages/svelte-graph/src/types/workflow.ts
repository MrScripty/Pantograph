// Workflow type definitions for the node-based visual programming system
// NOTE: These types must match the Rust types in src-tauri/src/workflow/types.rs

export type PortDataType =
  | 'any'
  | 'string'
  | 'image'
  | 'audio'
  | 'audio_stream'
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
  description?: string;
  default_value?: unknown;
  constraints?: {
    min?: number;
    max?: number;
    allowed_values?: unknown[];
  };
}

export type NodeCategory = 'input' | 'processing' | 'tool' | 'output' | 'control';
export type IoBindingOrigin = 'client_session' | 'integrated';
export type ExecutionMode = 'reactive' | 'manual' | 'stream';

export interface NodeDefinition {
  node_type: string;
  category: NodeCategory;
  label: string;
  description: string;
  io_binding_origin: IoBindingOrigin;
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

export interface ConnectionAnchor {
  node_id: string;
  port_id: string;
}

export interface ConnectionTargetAnchorCandidate {
  port_id: string;
  port_label: string;
  data_type: PortDataType;
  multiple: boolean;
}

export interface ConnectionTargetNodeCandidate {
  node_id: string;
  node_type: string;
  node_label: string;
  position: { x: number; y: number };
  anchors: ConnectionTargetAnchorCandidate[];
}

export interface InsertableNodeTypeCandidate {
  node_type: string;
  category: NodeCategory;
  label: string;
  description: string;
  matching_input_port_ids: string[];
}

export interface ConnectionCandidatesResponse {
  graph_revision: string;
  revision_matches: boolean;
  source_anchor: ConnectionAnchor;
  compatible_nodes: ConnectionTargetNodeCandidate[];
  insertable_node_types: InsertableNodeTypeCandidate[];
}

export type ConnectionRejectionReason =
  | 'stale_revision'
  | 'unknown_source_anchor'
  | 'unknown_target_anchor'
  | 'duplicate_connection'
  | 'target_capacity_reached'
  | 'self_connection'
  | 'cycle_detected'
  | 'incompatible_types';

export interface ConnectionRejection {
  reason: ConnectionRejectionReason;
  message: string;
}

export interface ConnectionCommitResponse {
  accepted: boolean;
  graph_revision: string;
  graph?: WorkflowGraph;
  rejection?: ConnectionRejection;
}

export interface WorkflowGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  derived_graph?: WorkflowDerivedGraph;
}

export interface WorkflowDerivedGraph {
  schema_version: number;
  graph_fingerprint: string;
  consumer_count_map: Record<string, number>;
}

export interface WorkflowMetadata {
  id?: string;
  name: string;
  description?: string;
  created: string;
  modified: string;
  orchestrationId?: string;
}

export interface WorkflowFile {
  version: string;
  metadata: WorkflowMetadata;
  graph: WorkflowGraph;
  viewport?: { x: number; y: number; zoom: number };
  linkMappings?: unknown[];
}

// --- Event Types ---

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
  NodeStream: { node_id: string; port: string; chunk: unknown };
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

// --- Execution State ---

export type NodeExecutionState = 'idle' | 'running' | 'success' | 'error';

export interface NodeExecutionInfo {
  state: NodeExecutionState;
  errorMessage?: string;
}
