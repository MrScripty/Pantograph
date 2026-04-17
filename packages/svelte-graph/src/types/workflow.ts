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

export interface InsertNodePositionHint {
  position: { x: number; y: number };
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
  | 'unknown_edge'
  | 'duplicate_connection'
  | 'target_capacity_reached'
  | 'self_connection'
  | 'cycle_detected'
  | 'incompatible_types'
  | 'unknown_insert_node_type'
  | 'no_compatible_insert_input'
  | 'no_compatible_insert_path';

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

export interface InsertNodeConnectionResponse {
  accepted: boolean;
  graph_revision: string;
  inserted_node_id?: string;
  graph?: WorkflowGraph;
  rejection?: ConnectionRejection;
}

export interface EdgeInsertionBridge {
  input_port_id: string;
  output_port_id: string;
}

export interface EdgeInsertionPreviewResponse {
  accepted: boolean;
  graph_revision: string;
  bridge?: EdgeInsertionBridge;
  rejection?: ConnectionRejection;
}

export interface InsertNodeOnEdgeResponse {
  accepted: boolean;
  graph_revision: string;
  inserted_node_id?: string;
  bridge?: EdgeInsertionBridge;
  graph?: WorkflowGraph;
  rejection?: ConnectionRejection;
}

export interface ConnectionIntentState {
  sourceAnchor: ConnectionAnchor;
  graphRevision: string;
  compatibleNodeIds: string[];
  compatibleTargetKeys: string[];
  insertableNodeTypes: InsertableNodeTypeCandidate[];
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

export interface WorkflowRuntimeRequirements {
  estimated_peak_vram_mb?: number | null;
  estimated_peak_ram_mb?: number | null;
  estimated_min_vram_mb?: number | null;
  estimated_min_ram_mb?: number | null;
  estimation_confidence: string;
  required_models: string[];
  required_backends: string[];
  required_extensions: string[];
}

export type WorkflowRuntimeInstallState =
  | 'installed'
  | 'system_provided'
  | 'missing'
  | 'unsupported';

export interface WorkflowRuntimeCapability {
  runtime_id: string;
  display_name: string;
  install_state: WorkflowRuntimeInstallState;
  available: boolean;
  configured: boolean;
  can_install: boolean;
  can_remove: boolean;
  backend_keys: string[];
  missing_files: string[];
  unavailable_reason?: string | null;
}

export interface WorkflowCapabilityModel {
  model_id: string;
  model_revision_or_hash?: string | null;
  model_type?: string | null;
  node_ids: string[];
  roles: string[];
}

export interface WorkflowCapabilitiesResponse {
  max_input_bindings: number;
  max_output_targets: number;
  max_value_bytes: number;
  runtime_requirements: WorkflowRuntimeRequirements;
  models: WorkflowCapabilityModel[];
  runtime_capabilities: WorkflowRuntimeCapability[];
}

export type WorkflowSessionKind = 'edit' | 'workflow';

export interface WorkflowSessionHandle {
  session_id: string;
  session_kind: WorkflowSessionKind;
}

export type WorkflowSessionState = 'idle_loaded' | 'idle_unloaded' | 'running';

export interface WorkflowSessionSummary {
  session_id: string;
  workflow_id: string;
  session_kind: WorkflowSessionKind;
  usage_profile?: string | null;
  keep_alive: boolean;
  state: WorkflowSessionState;
  queued_runs: number;
  run_count: number;
}

export type WorkflowSessionQueueItemStatus = 'pending' | 'running';

export interface WorkflowSessionQueueItem {
  queue_id: string;
  run_id?: string | null;
  priority: number;
  status: WorkflowSessionQueueItemStatus;
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
  | 'Cancelled'
  | 'GraphModified'
  | 'WaitingForInput'
  | 'IncrementalExecutionStarted'
  | 'RuntimeSnapshot'
  | 'SchedulerSnapshot'
  | 'DiagnosticsSnapshot';

export interface WorkflowEventData {
  Started: { workflow_id: string; node_count: number; execution_id?: string };
  NodeStarted: { node_id: string; node_type: string; execution_id?: string };
  NodeProgress: { node_id: string; progress: number; message?: string; execution_id?: string };
  NodeStream: { node_id: string; port: string; chunk: unknown; execution_id?: string };
  NodeCompleted: { node_id: string; outputs: Record<string, unknown>; execution_id?: string };
  NodeError: { node_id: string; error: string; execution_id?: string };
  Completed: {
    workflow_id?: string;
    outputs: Record<string, unknown>;
    execution_id?: string;
  };
  Failed: { workflow_id?: string; error: string; execution_id?: string };
  Cancelled: { workflow_id?: string; error: string; execution_id?: string };
  GraphModified: {
    workflow_id?: string;
    execution_id?: string;
    graph?: WorkflowGraph | null;
    dirty_tasks?: string[];
  };
  WaitingForInput: {
    workflow_id?: string;
    execution_id?: string;
    node_id: string;
    message?: string | null;
  };
  IncrementalExecutionStarted: {
    workflow_id?: string;
    execution_id?: string;
    task_ids: string[];
  };
  RuntimeSnapshot: {
    workflow_id?: string;
    execution_id?: string;
    captured_at_ms: number;
    capabilities?: WorkflowCapabilitiesResponse | null;
    error?: string | null;
  };
  SchedulerSnapshot: {
    workflow_id?: string;
    execution_id?: string;
    session_id: string;
    captured_at_ms: number;
    session?: WorkflowSessionSummary | null;
    items: WorkflowSessionQueueItem[];
    error?: string | null;
  };
  DiagnosticsSnapshot: {
    execution_id?: string;
    snapshot: unknown;
  };
}

export interface WorkflowEvent<T extends WorkflowEventType = WorkflowEventType> {
  type: T;
  data: WorkflowEventData[T];
}

// --- Execution State ---

export type NodeExecutionState = 'idle' | 'running' | 'waiting' | 'success' | 'error';

export interface NodeExecutionInfo {
  state: NodeExecutionState;
  message?: string;
}
