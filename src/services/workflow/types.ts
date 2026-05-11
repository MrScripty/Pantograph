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

export type InferenceTaskId =
  | 'text_generation'
  | 'chat_completion'
  | 'embedding'
  | 'rerank'
  | 'image_generation'
  | 'image_understanding'
  | 'depth_estimation'
  | 'audio_transcription'
  | 'video_understanding'
  | 'multimodal_generation'
  | 'unknown';

export type InferencePortPayloadRole =
  | 'task_input'
  | 'task_output'
  | 'model_reference'
  | 'options'
  | 'diagnostics'
  | 'usage'
  | 'cache_handle';

export type InferenceExecutionInputKind =
  | 'text_generation'
  | 'embedding'
  | 'rerank'
  | 'image_generation'
  | 'image_understanding'
  | 'depth_estimation'
  | 'audio_transcription'
  | 'video_understanding'
  | 'multimodal_generation';

export type InferenceExecutionResultKind = InferenceExecutionInputKind;

export interface InferencePortPayloadContract {
  task_id: InferenceTaskId;
  role: InferencePortPayloadRole;
  input_kind?: InferenceExecutionInputKind;
  result_kind?: InferenceExecutionResultKind;
}

export interface PortDefinition {
  id: string;
  label: string;
  data_type: PortDataType;
  required: boolean;
  multiple: boolean;
  inference_payloads?: InferencePortPayloadContract[];
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
  graph_diagnostics?: WorkflowGraphDiagnostic[];
  executable_topology: WorkflowExecutableTopology;
  presentation_metadata: WorkflowPresentationMetadata;
  graph_settings: WorkflowGraphRunSettings;
}

export interface WorkflowGraphInspectionRequest {
  path: string;
  selected_node_id?: string | null;
}

export interface WorkflowGraphInspectionRunContext {
  workflow_run_id: string;
  workflow_id?: string | null;
}

export interface WorkflowGraphInspectionSelectedNode {
  node: GraphNode;
  diagnostics: WorkflowGraphDiagnostic[];
}

export interface WorkflowGraphInspectionProjection {
  graph: WorkflowGraph;
  selected_node?: WorkflowGraphInspectionSelectedNode | null;
  diagnostics: WorkflowGraphDiagnostic[];
  run_context?: WorkflowGraphInspectionRunContext | null;
}

export type WorkflowGraphDiagnosticSeverity = 'error' | 'warning' | 'info';
export type WorkflowGraphDiagnosticScope = 'graph' | 'node' | 'edge';
export type WorkflowGraphDiagnosticCode =
  | 'duplicate_node_id'
  | 'duplicate_edge_id'
  | 'unknown_node_type'
  | 'retired_node_type'
  | 'invalid_node_id'
  | 'invalid_node_type'
  | 'invalid_dynamic_definition'
  | 'missing_edge_source_node'
  | 'missing_edge_target_node'
  | 'self_connection'
  | 'missing_source_contract'
  | 'missing_target_contract'
  | 'missing_source_output'
  | 'missing_target_input'
  | 'target_input_capacity_reached'
  | 'incompatible_port_types'
  | 'compatibility_check_failed'
  | 'cycle_detected';

export interface WorkflowGraphDiagnostic {
  code: WorkflowGraphDiagnosticCode;
  severity: WorkflowGraphDiagnosticSeverity;
  scope: WorkflowGraphDiagnosticScope;
  node_id?: string | null;
  node_type?: string | null;
  message: string;
  blocking_submission: boolean;
  details?: Record<string, string>;
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

export type WorkflowLocalRunPlacementState = 'running' | 'queued';
export type WorkflowSchedulerModelCacheState =
  | 'unknown'
  | 'not_required'
  | 'cache_hit'
  | 'cache_miss'
  | 'load_requested'
  | 'loaded'
  | 'unload_requested'
  | 'unloaded'
  | 'failed';

export interface WorkflowLocalRunPlacementRecord {
  workflow_run_id: string;
  workflow_execution_session_id: string;
  workflow_id: string;
  state: WorkflowLocalRunPlacementState;
  runtime_loaded: boolean;
  model_cache_state: WorkflowSchedulerModelCacheState;
  required_backends: string[];
  required_models: string[];
}

export interface WorkflowLocalSchedulerLoad {
  max_sessions: number;
  active_session_count: number;
  max_loaded_sessions: number;
  loaded_session_count: number;
  active_run_count: number;
  queued_run_count: number;
  active_workflow_run_ids: string[];
  queued_workflow_run_ids: string[];
  run_placements: WorkflowLocalRunPlacementRecord[];
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

export type WorkflowInferenceModality =
  | 'text'
  | 'image'
  | 'audio'
  | 'video'
  | 'embedding'
  | 'tokens'
  | 'json'
  | 'point_cloud'
  | 'mesh'
  | 'other';

export type WorkflowInferenceTaskId =
  | 'text_generation'
  | 'chat_completion'
  | 'embedding'
  | 'rerank'
  | 'image_generation'
  | 'image_understanding'
  | 'depth_estimation'
  | 'audio_transcription'
  | 'video_understanding'
  | 'multimodal_generation'
  | 'unknown';

export type WorkflowSupportTier =
  | 'stable'
  | 'experimental'
  | 'roadmap'
  | 'unsupported'
  | 'unknown';

export type WorkflowInferenceExecutionInputKind = InferenceExecutionInputKind;
export type WorkflowInferenceExecutionResultKind = InferenceExecutionResultKind;

export type WorkflowTaskStreamingSupport =
  | 'supported'
  | 'unsupported'
  | 'backend_dependent'
  | 'unknown';

export type WorkflowBackendComponentCapability =
  | 'unknown'
  | 'not_required'
  | 'backend_managed'
  | 'requires_package_component'
  | 'unsupported';

export interface WorkflowTaskModalitySignature {
  inputs: WorkflowInferenceModality[];
  outputs: WorkflowInferenceModality[];
}

export interface WorkflowBackendTaskCapability {
  task_id: WorkflowInferenceTaskId;
  support_tier: WorkflowSupportTier;
  modality_signature: WorkflowTaskModalitySignature;
  request_contract?: WorkflowTaskRequestContract;
}

export type WorkflowInferenceDeviceClass =
  | 'unknown'
  | 'cpu'
  | 'cuda'
  | 'metal'
  | 'mps';

export type WorkflowDeviceResolutionDiagnosticSeverity =
  | 'unknown'
  | 'advisory'
  | 'warning'
  | 'error';

export type WorkflowDeviceResolutionDiagnosticCode =
  | 'unknown'
  | 'invalid_device_policy'
  | 'invalid_device_id'
  | 'invalid_runtime_variant_id'
  | 'invalid_backend_id'
  | 'candidate_unavailable'
  | 'explicit_device_unavailable'
  | 'no_valid_candidate'
  | 'ambiguous_auto_resolution'
  | 'backend_incompatible'
  | 'unsupported_device_class'
  | 'missing_runtime_variant'
  | 'legacy_device_rejected';

export interface WorkflowDeviceResolutionDiagnostic {
  code: WorkflowDeviceResolutionDiagnosticCode;
  severity: WorkflowDeviceResolutionDiagnosticSeverity;
  message: string;
  device_class?: WorkflowInferenceDeviceClass | null;
  device_id?: string | null;
  runtime_variant_id?: string | null;
  backend_id?: string | null;
}

export interface WorkflowRuntimeVariantCapability {
  runtime_variant_id: string;
  device_class: WorkflowInferenceDeviceClass;
  available: boolean;
  diagnostics: WorkflowDeviceResolutionDiagnostic[];
}

export interface WorkflowTaskRequestContract {
  task_id: WorkflowInferenceTaskId;
  input_kind: WorkflowInferenceExecutionInputKind;
  result_kind: WorkflowInferenceExecutionResultKind;
  execution_supported: boolean;
  streaming_support: WorkflowTaskStreamingSupport;
  required_input_modalities: WorkflowInferenceModality[];
  output_modalities: WorkflowInferenceModality[];
}

export type WorkflowBackendFeatureSupport =
  | 'supported'
  | 'unsupported'
  | 'unknown';

export interface WorkflowBackendFeatureCapabilityFacts {
  streaming: WorkflowBackendFeatureSupport;
  device_selection: WorkflowBackendFeatureSupport;
  external_connection: WorkflowBackendFeatureSupport;
  kv_cache: WorkflowBackendFeatureSupport;
}

export type WorkflowModelArtifactKind =
  | 'gguf'
  | 'hf_compatible_directory'
  | 'safetensors'
  | 'diffusers_bundle'
  | 'onnx'
  | 'adapter'
  | 'shard'
  | 'unknown';

export type WorkflowBackendHintLabel =
  | 'transformers'
  | 'llama.cpp'
  | 'vllm'
  | 'mlx'
  | 'candle'
  | 'diffusers'
  | 'onnx-runtime';

export interface WorkflowBackendModelSourceCapabilityFacts {
  artifact_kinds: WorkflowModelArtifactKind[];
  backend_hints: WorkflowBackendHintLabel[];
  custom_code: WorkflowBackendFeatureSupport;
}

export type WorkflowInferenceLifecyclePhase =
  | 'model_package_resolution'
  | 'task_validation'
  | 'preprocessing'
  | 'backend_execution'
  | 'postprocessing'
  | 'result_projection';

export type WorkflowBackendRequestCancellationSemantics =
  | 'unknown'
  | 'not_applicable'
  | 'not_supported'
  | 'adapter_managed'
  | 'drop_consumer';

export type WorkflowBackendRequestCleanupSemantics =
  | 'unknown'
  | 'not_applicable'
  | 'not_required'
  | 'adapter_managed'
  | 'drop_stream'
  | 'rollback_publication';

export interface WorkflowBackendRequestLifecyclePhaseFacts {
  phase: WorkflowInferenceLifecyclePhase;
  component: WorkflowBackendComponentCapability;
  cancellation: WorkflowBackendRequestCancellationSemantics;
  cleanup: WorkflowBackendRequestCleanupSemantics;
}

export interface WorkflowBackendRequestLifecycleFacts {
  phases: WorkflowBackendRequestLifecyclePhaseFacts[];
  kv_cache_publication_cleanup: WorkflowBackendRequestCleanupSemantics;
}

export interface WorkflowBackendCapabilityFacts {
  tasks: WorkflowBackendTaskCapability[];
  runtime_variants?: WorkflowRuntimeVariantCapability[];
  preprocessing: WorkflowBackendComponentCapability;
  postprocessing: WorkflowBackendComponentCapability;
  model_sources: WorkflowBackendModelSourceCapabilityFacts;
  features: WorkflowBackendFeatureCapabilityFacts;
  request_lifecycle: WorkflowBackendRequestLifecycleFacts;
}

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
  backend_capability_facts?: WorkflowBackendCapabilityFacts | null;
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

export interface WorkflowPortBinding {
  node_id: string;
  port_id: string;
  value: unknown;
}

export interface WorkflowOutputTarget {
  node_id: string;
  port_id: string;
}

export interface WorkflowTechnicalFitOverride {
  runtime_id?: string | null;
  runtime_variant_id?: string | null;
  model_id?: string | null;
  backend_key?: string | null;
}

export type WorkflowTechnicalFitDeviceClass = 'cpu' | 'cuda' | 'metal' | 'mps';

export type WorkflowTechnicalFitDevicePolicy =
  | { policy: 'auto' }
  | {
      policy: 'explicit';
      device_class: WorkflowTechnicalFitDeviceClass;
      device_id?: string | null;
    };

export interface WorkflowTechnicalFitResourceEstimate {
  estimated_peak_vram_mb?: number | null;
  estimated_peak_ram_mb?: number | null;
  estimated_min_vram_mb?: number | null;
  estimated_min_ram_mb?: number | null;
}

export interface WorkflowTechnicalFitObservedThroughputHint {
  tokens_per_second_milli?: number | null;
  images_per_second_milli?: number | null;
  sample_count?: number | null;
}

export type WorkflowTechnicalFitDeviceDiagnosticSeverity =
  | 'advisory'
  | 'warning'
  | 'error';

export type WorkflowTechnicalFitDeviceDiagnosticCode =
  | 'invalid_device_policy'
  | 'invalid_device_id'
  | 'invalid_runtime_variant_id'
  | 'invalid_backend_id'
  | 'candidate_unavailable'
  | 'explicit_device_unavailable'
  | 'no_valid_candidate'
  | 'ambiguous_auto_resolution'
  | 'backend_incompatible'
  | 'unsupported_device_class'
  | 'missing_runtime_variant'
  | 'legacy_device_rejected';

export interface WorkflowTechnicalFitDeviceDiagnostic {
  code: WorkflowTechnicalFitDeviceDiagnosticCode;
  severity: WorkflowTechnicalFitDeviceDiagnosticSeverity;
  message: string;
  device_class?: WorkflowTechnicalFitDeviceClass | null;
  device_id?: string | null;
  runtime_variant_id?: string | null;
  backend_key?: string | null;
}

export type WorkflowTechnicalFitSelectionMode = 'automatic' | 'explicit_override';

export type WorkflowTechnicalFitReasonCode =
  | 'explicit_runtime_override'
  | 'explicit_runtime_variant_override'
  | 'explicit_model_override'
  | 'explicit_backend_override'
  | 'required_context_length'
  | 'runtime_requirements'
  | 'residency_reuse'
  | 'warmup_cost'
  | 'budget_pressure'
  | 'queue_pressure'
  | 'missing_candidate_data'
  | 'missing_runtime_state'
  | 'deterministic_tie_break';

export interface WorkflowTechnicalFitReason {
  code: WorkflowTechnicalFitReasonCode;
  candidate_id?: string | null;
}

export interface WorkflowTechnicalFitCompatibilityReport {
  status: string;
  compatible: boolean;
  task: string;
  model_source: string;
  preprocessing: string;
  postprocessing: string;
}

export interface WorkflowTechnicalFitCompatibilityIssue {
  kind: string;
  phase: string;
  message: string;
  model_id?: string | null;
  path?: string | null;
}

export interface WorkflowTechnicalFitDecision {
  selection_mode?: WorkflowTechnicalFitSelectionMode;
  selected_candidate_id?: string | null;
  selected_runtime_id?: string | null;
  selected_runtime_variant_id?: string | null;
  selected_backend_key?: string | null;
  selected_model_id?: string | null;
  selected_device_class?: WorkflowTechnicalFitDeviceClass | null;
  selected_device_id?: string | null;
  resource_estimate?: WorkflowTechnicalFitResourceEstimate | null;
  observed_throughput_hint?: WorkflowTechnicalFitObservedThroughputHint | null;
  device_diagnostics?: WorkflowTechnicalFitDeviceDiagnostic[];
  reasons?: WorkflowTechnicalFitReason[];
  compatibility_report?: WorkflowTechnicalFitCompatibilityReport | null;
  compatibility_issue_count?: number;
  compatibility_issues?: WorkflowTechnicalFitCompatibilityIssue[];
}

export interface WorkflowTechnicalFitQueuePressure {
  current_session_queue_depth?: number | null;
  total_queued_run_count?: number | null;
  loaded_runtime_count?: number | null;
  loaded_runtime_capacity?: number | null;
}

export interface WorkflowTechnicalFitRequest {
  workflow_id: string;
  runtime_requirements: WorkflowRuntimeRequirements;
  override_selection?: WorkflowTechnicalFitOverride | null;
  device_policy?: WorkflowTechnicalFitDevicePolicy | null;
  session_id?: string | null;
  usage_profile?: string | null;
  queue_pressure?: WorkflowTechnicalFitQueuePressure | null;
}

export interface WorkflowExecutionSessionCreateRequest {
  workflow_id: string;
  usage_profile?: string | null;
  keep_alive?: boolean;
}

export interface WorkflowExecutionSessionCreateResponse {
  session_id: string;
  attribution?: unknown | null;
  runtime_capabilities: WorkflowRuntimeCapability[];
}

export interface WorkflowExecutionSessionRunRequest {
  session_id: string;
  workflow_semantic_version: string;
  inputs?: WorkflowPortBinding[];
  output_targets?: WorkflowOutputTarget[] | null;
  override_selection?: WorkflowTechnicalFitOverride | null;
  timeout_ms?: number | null;
  priority?: number | null;
}

export interface WorkflowExecutionSessionCloseRequest {
  session_id: string;
}

export interface WorkflowExecutionSessionCloseResponse {
  ok: boolean;
}

export interface WorkflowRunResponse {
  workflow_run_id: string;
  outputs: WorkflowPortBinding[];
  timing_ms: number;
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

export interface WorkflowSessionQueueCancelRequest {
  session_id: string;
  workflow_run_id: string;
}

export interface WorkflowSessionQueueCancelResponse {
  ok: boolean;
}

export interface WorkflowAdminQueueCancelRequest {
  workflow_run_id: string;
}

export interface WorkflowAdminQueueCancelResponse {
  ok: boolean;
  session_id: string;
}

export interface WorkflowAdminQueueReprioritizeRequest {
  workflow_run_id: string;
  priority: number;
}

export interface WorkflowAdminQueueReprioritizeResponse {
  ok: boolean;
  session_id: string;
}

export interface WorkflowAdminQueuePushFrontRequest {
  workflow_run_id: string;
}

export interface WorkflowAdminQueuePushFrontResponse {
  ok: boolean;
  session_id: string;
  priority: number;
}

export interface WorkflowSessionQueuePushFrontRequest {
  session_id: string;
  workflow_run_id: string;
}

export interface WorkflowSessionQueuePushFrontResponse {
  ok: boolean;
  priority: number;
}

export interface WorkflowSessionQueueReprioritizeRequest {
  session_id: string;
  workflow_run_id: string;
  priority: number;
}

export interface WorkflowSessionQueueReprioritizeResponse {
  ok: boolean;
}

export interface WorkflowSchedulerSnapshotResponse {
  workflow_id?: string | null;
  session_id: string;
  workflow_run_id?: string | null;
  session: WorkflowSessionSummary;
  items: WorkflowSessionQueueItem[];
}

export type WorkflowArtifactPayloadKind =
  | 'text'
  | 'image'
  | 'audio'
  | 'video'
  | '3d'
  | 'large_table'
  | 'generic_binary'
  | 'structured';

export type WorkflowArtifactLifecycleState =
  | 'declared'
  | 'writing'
  | 'streaming'
  | 'finalizing'
  | 'retained'
  | 'failed'
  | 'expired'
  | 'deleted';

export type WorkflowArtifactRetentionState =
  | 'retained'
  | 'metadata_only'
  | 'external'
  | 'truncated'
  | 'too_large'
  | 'expired'
  | 'deleted';

export type WorkflowArtifactAccessMode = 'read' | 'download' | 'stream';
export type WorkflowArtifactBodyTransport = 'binary_body' | 'redirect_url' | 'stream_handle';

export interface WorkflowArtifactAttribution {
  workflow_run_id: string;
  workflow_id?: string | null;
  workflow_version_id?: string | null;
  node_id?: string | null;
  port_id?: string | null;
  model_id?: string | null;
  runtime_id?: string | null;
}

export interface WorkflowArtifactFormatMetadata {
  format_id: string;
  media_type: string;
  codec_id?: string | null;
  quality_percent?: number | null;
  bitrate_kbps?: number | null;
  crf?: number | null;
  bit_depth?: string | null;
  color_profile_id?: string | null;
  converter_id?: string | null;
  converter_version?: string | null;
  library_version?: string | null;
  conversion_id?: string | null;
  conversion_status?: WorkflowArtifactConversionStatus | null;
  conversion_command_id?: string | null;
  conversion_dependencies?: WorkflowArtifactConversionDependency[] | null;
}

export type WorkflowArtifactConversionStatus = 'converted' | 'passed_through' | 'failed';

export interface WorkflowArtifactConversionDependency {
  dependency_id: string;
  active_version: string;
  lease_id: string;
  lease_holder: string;
}

export interface WorkflowArtifactDescriptor {
  artifact_id: string;
  payload_kind: WorkflowArtifactPayloadKind;
  lifecycle_state: WorkflowArtifactLifecycleState;
  retention_state: WorkflowArtifactRetentionState;
  byte_length?: number | null;
  content_hash?: string | null;
  format?: WorkflowArtifactFormatMetadata | null;
  attribution: WorkflowArtifactAttribution;
  access_modes: WorkflowArtifactAccessMode[];
  read_handle?: string | null;
  stream_handle?: string | null;
  retention_reason?: string | null;
}

export interface WorkflowArtifactDescriptorQueryRequest {
  artifact_id: string;
}

export interface WorkflowArtifactDescriptorQueryResponse {
  artifact?: WorkflowArtifactDescriptor | null;
}

export interface WorkflowArtifactReadRequest {
  artifact_id: string;
  byte_range_start?: number | null;
  byte_range_end_exclusive?: number | null;
}

export interface WorkflowArtifactReadResponse {
  artifact_id: string;
  media_type: string;
  body_transport: WorkflowArtifactBodyTransport;
  read_handle: string;
  byte_length: number;
  content_hash?: string | null;
  complete: boolean;
}

export interface WorkflowArtifactBodyRead {
  response: WorkflowArtifactReadResponse;
  body: number[];
}

export interface WorkflowArtifactStreamReadRequest {
  artifact_id: string;
  byte_range_start?: number | null;
  byte_range_end_exclusive?: number | null;
}

export interface WorkflowArtifactStreamReadResponse {
  artifact_id: string;
  stream_handle: string;
  media_type: string;
  body_transport: WorkflowArtifactBodyTransport;
  byte_length: number;
  available_byte_length: number;
  lifecycle_state: WorkflowArtifactLifecycleState;
  complete: boolean;
}

export interface WorkflowArtifactStreamBodyRead {
  response: WorkflowArtifactStreamReadResponse;
  body: number[];
}

export interface WorkflowArtifactConsumeAcknowledgementRequest {
  artifact_id: string;
  consumer_id: string;
}

export interface WorkflowArtifactConsumeAcknowledgementResponse {
  artifact_id: string;
  retained_after_consume: boolean;
}

export interface WorkflowArtifactPolicy {
  policy_id: string;
  policy_version: number;
  ttl_seconds?: number | null;
  max_disk_bytes?: number | null;
  max_memory_bytes?: number | null;
  max_single_artifact_bytes?: number | null;
  spill_threshold_bytes?: number | null;
  delete_on_consume: boolean;
}

export interface WorkflowArtifactFormatSettings {
  image: WorkflowImageArtifactFormatSettings;
  audio: WorkflowAudioArtifactFormatSettings;
  video: WorkflowVideoArtifactFormatSettings;
  three_d: WorkflowThreeDArtifactFormatSettings;
}

export type WorkflowArtifactFormatSettingsQueryRequest = Record<string, never>;

export interface WorkflowArtifactFormatSettingsQueryResponse {
  settings: WorkflowArtifactFormatSettings;
}

export interface WorkflowArtifactFormatSettingsUpdateRequest {
  settings: WorkflowArtifactFormatSettings;
  reason?: string | null;
}

export interface WorkflowArtifactFormatSettingsUpdateResponse {
  settings: WorkflowArtifactFormatSettings;
}

export interface WorkflowImageArtifactFormatSettings {
  format_id: string;
  quality_percent: number;
  color_profile_id: string;
}

export interface WorkflowAudioArtifactFormatSettings {
  container_id: string;
  codec_id: string;
  bitrate_kbps: number;
}

export interface WorkflowVideoArtifactFormatSettings {
  container_id: string;
  codec_id: string;
  crf: number;
  bit_depth: string;
}

export interface WorkflowThreeDArtifactFormatSettings {
  format_id: string;
}

export interface WorkflowMediaFormatOption {
  format_id: string;
  display_name: string;
  media_type: string;
  codec_ids: string[];
  quality_min_percent?: number | null;
  quality_max_percent?: number | null;
  bitrate_min_kbps?: number | null;
  bitrate_max_kbps?: number | null;
  crf_min?: number | null;
  crf_max?: number | null;
  bit_depths: string[];
  color_profile_ids: string[];
  provided_by_dependency_id: string;
  provided_by_version?: string | null;
}

export interface WorkflowArtifactFormatCapabilities {
  image_formats: WorkflowMediaFormatOption[];
  audio_formats: WorkflowMediaFormatOption[];
  video_formats: WorkflowMediaFormatOption[];
  three_d_formats: WorkflowMediaFormatOption[];
}

export type WorkflowManagedMediaDependencyId =
  | 'ffmpeg'
  | 'ocioconvert'
  | 'oiiotool'
  | 'open_color_io';

export type WorkflowManagedMediaDependencyCategory =
  | 'tool_binary'
  | 'native_library_artifact';

export type WorkflowManagedMediaDependencyInstallState =
  | 'installed'
  | 'missing'
  | 'unsupported';

export type WorkflowManagedMediaDependencyReadiness =
  | 'missing'
  | 'ready'
  | 'unsupported';

export type WorkflowManagedMediaDependencyPackageKind =
  | 'archive'
  | 'native_package';

export type WorkflowManagedMediaDependencyArchiveKind =
  | 'tar_gz'
  | 'tar_xz'
  | 'zip';

export interface WorkflowManagedMediaDependencySource {
  owner: string;
  project: string;
}

export interface WorkflowManagedMediaDependencyCatalogEntry {
  id: WorkflowManagedMediaDependencyId;
  display_name: string;
  category: WorkflowManagedMediaDependencyCategory;
  source: WorkflowManagedMediaDependencySource;
  license_redistribution: string;
  platform_key: string;
  version: string;
  package_kind: WorkflowManagedMediaDependencyPackageKind;
  archive_kind?: WorkflowManagedMediaDependencyArchiveKind | null;
  archive_name?: string | null;
  download_url?: string | null;
  expected_files: string[];
  checksum_sha256?: string | null;
  signature?: string | null;
}

export interface WorkflowManagedMediaDependencySelection {
  selected_version?: string | null;
  active_version?: string | null;
  default_version?: string | null;
}

export interface WorkflowManagedMediaDependencyVersionStatus {
  version: string;
  platform_key: string;
  install_root: string;
  expected_files: string[];
  missing_files: string[];
  install_state: WorkflowManagedMediaDependencyInstallState;
  readiness: WorkflowManagedMediaDependencyReadiness;
  selected: boolean;
  active: boolean;
}

export interface WorkflowManagedMediaDependencyStatus {
  id: WorkflowManagedMediaDependencyId;
  display_name: string;
  category: WorkflowManagedMediaDependencyCategory;
  install_state: WorkflowManagedMediaDependencyInstallState;
  readiness: WorkflowManagedMediaDependencyReadiness;
  available: boolean;
  missing_files: string[];
  catalog: WorkflowManagedMediaDependencyCatalogEntry;
  selection: WorkflowManagedMediaDependencySelection;
  versions: WorkflowManagedMediaDependencyVersionStatus[];
}

export interface WorkflowManagedMediaDependencyInstallFromStagingRequest {
  id: WorkflowManagedMediaDependencyId;
  version: string;
  staging_dir: string;
}

export interface WorkflowManagedMediaDependencyVersionSelectionRequest {
  id: WorkflowManagedMediaDependencyId;
  version?: string | null;
}

export interface WorkflowManagedMediaDependencyVersionActionRequest {
  id: WorkflowManagedMediaDependencyId;
  version: string;
}

export interface WorkflowArtifactStoreStats {
  artifact_count: number;
  retained_body_count: number;
  retained_body_bytes: number;
  memory_cache_body_count: number;
  memory_cache_body_bytes: number;
  streaming_body_count: number;
  streaming_body_bytes: number;
  metadata_only_count: number;
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
  | 'NodeInputsResolved'
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
  NodeInputsResolved: WorkflowEventOwnershipData & {
    node_id: string;
    inputs: Record<string, unknown>;
    cache_status?: 'fresh_execution' | 'cache_hit' | 'cache_invalidated' | null;
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
  metadata?: Record<string, unknown>;
}
