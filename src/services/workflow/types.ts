// Workflow type definitions for the node-based visual programming system
// NOTE: These types must match the Rust types in src-tauri/src/workflow/types.rs
// Rust uses snake_case serialization for enums

import type { WorkflowEventOwnershipProjection } from '@pantograph/svelte-graph';

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
  | 'kv_cache'
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
  workflow_event?: WorkflowEvent<'GraphModified'> | null;
  workflow_session_state?: WorkflowGraphSessionStateView | null;
  rejection?: ConnectionRejection;
}

export interface InsertNodeConnectionResponse {
  accepted: boolean;
  graph_revision: string;
  inserted_node_id?: string;
  graph?: WorkflowGraph;
  workflow_event?: WorkflowEvent<'GraphModified'> | null;
  workflow_session_state?: WorkflowGraphSessionStateView | null;
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
  workflow_event?: WorkflowEvent<'GraphModified'> | null;
  workflow_session_state?: WorkflowGraphSessionStateView | null;
  rejection?: ConnectionRejection;
}

export interface WorkflowGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  derived_graph?: WorkflowDerivedGraph;
}

export interface WorkflowRunGraphQueryRequest {
  workflow_run_id: string;
}

export interface WorkflowRunGraphQueryResponse {
  run_graph?: WorkflowRunGraphProjection | null;
}

export interface WorkflowRunGraphProjection {
  workflow_run_id: string;
  workflow_id: string;
  workflow_version_id: string;
  workflow_presentation_revision_id: string;
  workflow_semantic_version: string;
  workflow_execution_fingerprint: string;
  snapshot_created_at_ms: number;
  workflow_version_created_at_ms: number;
  presentation_revision_created_at_ms: number;
  graph: WorkflowGraph;
  executable_topology: WorkflowExecutableTopology;
  presentation_metadata: WorkflowPresentationMetadata;
  graph_settings: WorkflowGraphRunSettings;
}

export interface WorkflowExecutableTopology {
  schema_version: number;
  nodes: WorkflowExecutableTopologyNode[];
  edges: WorkflowExecutableTopologyEdge[];
}

export interface WorkflowExecutableTopologyNode {
  node_id: string;
  node_type: string;
  contract_version: string;
  behavior_digest: string;
}

export interface WorkflowExecutableTopologyEdge {
  source_node_id: string;
  source_port_id: string;
  target_node_id: string;
  target_port_id: string;
}

export interface WorkflowPresentationMetadata {
  schema_version: number;
  nodes: WorkflowPresentationNode[];
  edges: WorkflowPresentationEdge[];
}

export interface WorkflowPresentationNode {
  node_id: string;
  position: { x: number; y: number };
}

export interface WorkflowPresentationEdge {
  edge_id: string;
  source_node_id: string;
  source_port_id: string;
  target_node_id: string;
  target_port_id: string;
}

export interface WorkflowGraphRunSettings {
  schema_version: number;
  nodes: WorkflowGraphRunSettingsNode[];
}

export interface WorkflowGraphRunSettingsNode {
  node_id: string;
  node_type: string;
  data: unknown;
}

export interface WorkflowLocalNetworkStatusQueryRequest {
  include_network_interfaces?: boolean;
  include_disks?: boolean;
}

export interface WorkflowLocalNetworkStatusQueryResponse {
  local_node: WorkflowLocalNetworkNodeStatus;
  peer_nodes: WorkflowPeerNetworkNodeStatus[];
}

export type WorkflowNetworkTransportState =
  | 'local_only'
  | 'peer_networking_unavailable'
  | 'pairing_required'
  | 'connected'
  | 'degraded';

export interface WorkflowLocalNetworkNodeStatus {
  node_id: string;
  display_name: string;
  captured_at_ms: number;
  transport_state: WorkflowNetworkTransportState;
  system: WorkflowLocalSystemMetrics;
  scheduler_load: WorkflowLocalSchedulerLoad;
  degradation_warnings: string[];
}

export interface WorkflowPeerNetworkNodeStatus {
  node_id: string;
  display_name: string;
  transport_state: WorkflowNetworkTransportState;
  last_seen_at_ms?: number | null;
}

export interface WorkflowLocalSystemMetrics {
  hostname?: string | null;
  os_name?: string | null;
  os_version?: string | null;
  kernel_version?: string | null;
  cpu: WorkflowLocalCpuMetrics;
  memory: WorkflowLocalMemoryMetrics;
  disks: WorkflowLocalDiskMetrics[];
  network_interfaces: WorkflowLocalNetworkInterfaceMetrics[];
  gpu: WorkflowLocalGpuMetrics;
}

export interface WorkflowLocalCpuMetrics {
  logical_core_count: number;
  average_usage_percent?: number | null;
}

export interface WorkflowLocalMemoryMetrics {
  total_bytes: number;
  used_bytes: number;
  available_bytes: number;
}

export interface WorkflowLocalDiskMetrics {
  name: string;
  mount_point: string;
  total_bytes: number;
  available_bytes: number;
}

export interface WorkflowLocalNetworkInterfaceMetrics {
  name: string;
  total_received_bytes: number;
  total_transmitted_bytes: number;
}

export interface WorkflowLocalGpuMetrics {
  available: boolean;
  reason?: string | null;
}

export interface WorkflowLocalSchedulerLoad {
  max_sessions: number;
  active_session_count: number;
  max_loaded_sessions: number;
  loaded_session_count: number;
  active_run_count: number;
  queued_run_count: number;
}

export interface WorkflowDerivedGraph {
  schema_version: number;
  graph_fingerprint: string;
  consumer_count_map: Record<string, number>;
}

export interface WorkflowMetadata {
  id?: string; // Filename stem (e.g., "coding-agent") for loading, populated by list_workflows
  name: string;
  description?: string;
  created: string;
  modified: string;
  orchestrationId?: string; // Optional link to parent orchestration for zoom-out navigation
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

export type WorkflowRuntimeSourceKind = 'unknown' | 'managed' | 'system' | 'host';

export interface WorkflowRuntimeCapability {
  runtime_id: string;
  display_name: string;
  install_state: WorkflowRuntimeInstallState;
  available: boolean;
  configured: boolean;
  can_install: boolean;
  can_remove: boolean;
  source_kind: WorkflowRuntimeSourceKind;
  selected: boolean;
  supports_external_connection: boolean;
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

export interface RuntimeLifecycleSnapshot {
  runtime_id?: string | null;
  runtime_instance_id?: string | null;
  warmup_started_at_ms?: number | null;
  warmup_completed_at_ms?: number | null;
  warmup_duration_ms?: number | null;
  runtime_reused?: boolean | null;
  lifecycle_decision_reason?: string | null;
  active: boolean;
  last_error?: string | null;
}

export type WorkflowSessionKind = 'edit' | 'workflow';

export interface WorkflowSessionHandle {
  session_id: string;
  session_kind: WorkflowSessionKind;
}

export interface WorkflowEditSessionRunResponse {
  workflow_run_id: string;
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

export interface WorkflowSessionStatusResponse {
  session: WorkflowSessionSummary;
}

export type WorkflowSessionQueueItemStatus = 'pending' | 'running';

export interface WorkflowSessionQueueItem {
  workflow_run_id: string;
  enqueued_at_ms?: number | null;
  dequeued_at_ms?: number | null;
  priority: number;
  status: WorkflowSessionQueueItemStatus;
}

export interface WorkflowSessionQueueListResponse {
  session_id: string;
  items: WorkflowSessionQueueItem[];
}

export interface WorkflowSchedulerSnapshotResponse {
  workflow_id?: string | null;
  session_id: string;
  workflow_run_id?: string | null;
  session: WorkflowSessionSummary;
  items: WorkflowSessionQueueItem[];
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
  | 'Cancelled'
  | 'GraphModified'
  | 'WaitingForInput'
  | 'IncrementalExecutionStarted'
  | 'RuntimeSnapshot'
  | 'SchedulerSnapshot'
  | 'DiagnosticsSnapshot';

export type WorkflowEventOwnershipData = {
  ownership?: WorkflowEventOwnershipProjection | null;
};

export interface WorkflowEventData {
  Started: WorkflowEventOwnershipData & {
    workflow_id: string;
    node_count: number;
    workflow_run_id?: string;
  };
  NodeStarted: WorkflowEventOwnershipData & {
    node_id: string;
    node_type: string;
    workflow_run_id?: string;
  };
  NodeProgress: WorkflowEventOwnershipData & {
    node_id: string;
    progress: number;
    message?: string;
    workflow_run_id?: string;
  };
  NodeStream: WorkflowEventOwnershipData & {
    node_id: string;
    port: string;
    chunk: unknown;
    workflow_run_id?: string;
  };
  NodeCompleted: WorkflowEventOwnershipData & {
    node_id: string;
    outputs: Record<string, unknown>;
    workflow_run_id?: string;
  };
  NodeError: WorkflowEventOwnershipData & {
    node_id: string;
    error: string;
    workflow_run_id?: string;
  };
  Completed: WorkflowEventOwnershipData & {
    workflow_id?: string;
    outputs: Record<string, unknown>;
    workflow_run_id?: string;
  };
  Failed: WorkflowEventOwnershipData & {
    workflow_id?: string;
    error: string;
    workflow_run_id?: string;
  };
  Cancelled: WorkflowEventOwnershipData & {
    workflow_id?: string;
    error: string;
    workflow_run_id?: string;
  };
  GraphModified: WorkflowEventOwnershipData & {
    workflow_id?: string;
    workflow_run_id?: string;
    graph?: WorkflowGraph | null;
    dirty_tasks?: string[];
    memory_impact?: GraphMemoryImpactSummary | null;
  };
  WaitingForInput: WorkflowEventOwnershipData & {
    workflow_id?: string;
    workflow_run_id?: string;
    node_id: string;
    message?: string | null;
  };
  IncrementalExecutionStarted: WorkflowEventOwnershipData & {
    workflow_id?: string;
    workflow_run_id?: string;
    task_ids: string[];
  };
  RuntimeSnapshot: WorkflowEventOwnershipData & {
    workflow_id?: string;
    workflow_run_id?: string;
    captured_at_ms: number;
    capabilities?: WorkflowCapabilitiesResponse | null;
    active_model_target?: string | null;
    embedding_model_target?: string | null;
    active_runtime_snapshot?: RuntimeLifecycleSnapshot | null;
    embedding_runtime_snapshot?: RuntimeLifecycleSnapshot | null;
    error?: string | null;
  };
  SchedulerSnapshot: WorkflowEventOwnershipData & {
    workflow_id?: string;
    workflow_run_id?: string;
    session_id: string;
    captured_at_ms: number;
    session?: WorkflowSessionSummary | null;
    items: WorkflowSessionQueueItem[];
    error?: string | null;
  };
  DiagnosticsSnapshot: WorkflowEventOwnershipData & {
    workflow_run_id?: string;
    snapshot: unknown;
  };
}

export type WorkflowEvent<T extends WorkflowEventType = WorkflowEventType> =
  T extends WorkflowEventType
    ? {
      type: T;
      data: WorkflowEventData[T];
    }
    : never;

export interface WorkflowGraphMutationResponse {
  graph: WorkflowGraph;
  workflow_event?: WorkflowEvent<'GraphModified'> | null;
  workflow_session_state?: WorkflowGraphSessionStateView | null;
}

export type WorkflowSessionResidencyState =
  | 'active'
  | 'warm'
  | 'checkpointed_but_unloaded'
  | 'restored';

export type NodeMemoryCompatibility =
  | 'preserve_as_is'
  | 'preserve_with_input_refresh'
  | 'drop_on_identity_change'
  | 'drop_on_schema_incompatibility'
  | 'fallback_full_invalidation';

export type NodeMemoryStatus = 'empty' | 'ready' | 'invalidated';

export type NodeMemoryRestoreStrategy =
  | 'rehydrate_before_resume'
  | 'rebind_host_resource'
  | 'drop_if_unavailable';

export interface NodeMemoryIdentity {
  session_id: string;
  node_id: string;
  node_type: string;
  schema_version?: string | null;
}

export interface NodeMemoryIndirectStateReference {
  reference_kind: string;
  reference_id: string;
  restore_strategy: NodeMemoryRestoreStrategy;
  inspection_metadata?: unknown;
}

export interface NodeMemorySnapshot {
  identity: NodeMemoryIdentity;
  status: NodeMemoryStatus;
  input_fingerprint?: string | null;
  output_snapshot?: unknown;
  private_state?: unknown;
  indirect_state_reference?: NodeMemoryIndirectStateReference | null;
  inspection_metadata?: unknown;
}

export interface NodeMemoryCompatibilitySnapshot {
  node_id: string;
  compatibility: NodeMemoryCompatibility;
  reason?: string | null;
}

export interface GraphMemoryImpactSummary {
  node_decisions?: NodeMemoryCompatibilitySnapshot[];
  fallback_to_full_invalidation: boolean;
}

export interface WorkflowSessionCheckpointSummary {
  session_id: string;
  graph_revision: string;
  residency: WorkflowSessionResidencyState;
  checkpoint_available: boolean;
  preserved_node_count: number;
  checkpointed_at_ms?: number | null;
}

export interface WorkflowGraphSessionStateView {
  contract_version: number;
  residency: WorkflowSessionResidencyState;
  node_memory?: NodeMemorySnapshot[];
  memory_impact?: GraphMemoryImpactSummary | null;
  checkpoint?: WorkflowSessionCheckpointSummary | null;
}

export type NodeExecutionState = 'idle' | 'running' | 'waiting' | 'success' | 'error';

/** Extended execution info including error messages */
export interface NodeExecutionInfo {
  state: NodeExecutionState;
  errorMessage?: string;
}

// Masked text input types for selective dLLM regeneration

export interface PromptSegment {
  text: string;
  masked: boolean;
}

export interface MaskedPrompt {
  type: 'masked_prompt';
  segments: PromptSegment[];
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
