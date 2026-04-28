export type ProjectionStatus = 'current' | 'rebuilding' | 'needs_rebuild' | 'failed';

export interface ProjectionStateRecord {
  projection_name: string;
  projection_version: number;
  last_applied_event_seq: number;
  status: ProjectionStatus;
  rebuilt_at_ms?: number | null;
  updated_at_ms: number;
}

export type DiagnosticEventKind =
  | 'scheduler_estimate_produced'
  | 'scheduler_queue_placement'
  | 'scheduler_run_delayed'
  | 'scheduler_model_lifecycle_changed'
  | 'run_started'
  | 'run_terminal'
  | 'run_snapshot_accepted'
  | 'io_artifact_observed'
  | 'retention_artifact_state_changed'
  | 'library_asset_accessed'
  | 'retention_policy_changed'
  | 'runtime_capability_observed'
  | 'node_execution_status';

export type DiagnosticEventSourceComponent =
  | 'scheduler'
  | 'workflow_service'
  | 'runtime'
  | 'node_execution'
  | 'retention'
  | 'library'
  | 'local_observer';

export interface SchedulerTimelineProjectionRecord {
  event_seq: number;
  event_id: string;
  event_kind: DiagnosticEventKind;
  source_component: DiagnosticEventSourceComponent;
  occurred_at_ms: number;
  recorded_at_ms: number;
  workflow_run_id: string;
  workflow_id: string;
  workflow_version_id?: string | null;
  workflow_semantic_version?: string | null;
  scheduler_policy_id?: string | null;
  retention_policy_id?: string | null;
  summary: string;
  detail?: string | null;
  payload_json: string;
}

export type RunListProjectionStatus =
  | 'accepted'
  | 'future'
  | 'scheduled'
  | 'queued'
  | 'delayed'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled';

export interface RunListProjectionRecord {
  workflow_run_id: string;
  workflow_id: string;
  workflow_version_id?: string | null;
  workflow_semantic_version?: string | null;
  status: RunListProjectionStatus;
  accepted_at_ms?: number | null;
  enqueued_at_ms?: number | null;
  started_at_ms?: number | null;
  completed_at_ms?: number | null;
  duration_ms?: number | null;
  scheduler_policy_id?: string | null;
  retention_policy_id?: string | null;
  client_id?: string | null;
  client_session_id?: string | null;
  bucket_id?: string | null;
  workflow_execution_session_id?: string | null;
  scheduler_queue_position?: number | null;
  scheduler_priority?: number | null;
  estimate_confidence?: string | null;
  estimated_queue_wait_ms?: number | null;
  estimated_duration_ms?: number | null;
  scheduler_reason?: string | null;
  last_event_seq: number;
  last_updated_at_ms: number;
}

export type RunListFacetKind =
  | 'workflow_version'
  | 'status'
  | 'scheduler_policy'
  | 'retention_policy';

export interface RunListFacetRecord {
  facet_kind: RunListFacetKind;
  facet_value: string;
  run_count: number;
}

export interface RunDetailProjectionRecord extends RunListProjectionRecord {
  client_id?: string | null;
  client_session_id?: string | null;
  bucket_id?: string | null;
  workflow_run_snapshot_id?: string | null;
  workflow_presentation_revision_id?: string | null;
  latest_estimate_json?: string | null;
  latest_queue_placement_json?: string | null;
  started_payload_json?: string | null;
  terminal_payload_json?: string | null;
  terminal_error?: string | null;
  timeline_event_count: number;
}

export interface IoArtifactProjectionRecord {
  event_seq: number;
  event_id: string;
  occurred_at_ms: number;
  recorded_at_ms: number;
  workflow_run_id: string;
  workflow_id: string;
  workflow_version_id?: string | null;
  workflow_semantic_version?: string | null;
  node_id?: string | null;
  node_type?: string | null;
  node_version?: string | null;
  runtime_id?: string | null;
  runtime_version?: string | null;
  model_id?: string | null;
  model_version?: string | null;
  artifact_id: string;
  artifact_role: string;
  producer_node_id?: string | null;
  producer_port_id?: string | null;
  consumer_node_id?: string | null;
  consumer_port_id?: string | null;
  media_type?: string | null;
  size_bytes?: number | null;
  content_hash?: string | null;
  payload_ref?: string | null;
  retention_state: IoArtifactRetentionState;
  retention_reason?: string | null;
  retention_policy_id?: string | null;
}

export type IoArtifactRetentionState =
  | 'retained'
  | 'metadata_only'
  | 'external'
  | 'truncated'
  | 'too_large'
  | 'expired'
  | 'deleted';

export interface IoArtifactRetentionSummaryRecord {
  retention_state: IoArtifactRetentionState;
  artifact_count: number;
}

export type NodeExecutionProjectionStatus =
  | 'queued'
  | 'running'
  | 'waiting'
  | 'completed'
  | 'failed'
  | 'cancelled';

export interface NodeStatusProjectionRecord {
  workflow_run_id: string;
  workflow_id: string;
  workflow_version_id?: string | null;
  workflow_semantic_version?: string | null;
  node_id: string;
  node_type?: string | null;
  node_version?: string | null;
  runtime_id?: string | null;
  runtime_version?: string | null;
  model_id?: string | null;
  model_version?: string | null;
  status: NodeExecutionProjectionStatus;
  started_at_ms?: number | null;
  completed_at_ms?: number | null;
  duration_ms?: number | null;
  error?: string | null;
  last_event_seq: number;
  last_updated_at_ms: number;
}

export interface WorkflowRunListQueryRequest {
  workflow_id?: string | null;
  workflow_version_id?: string | null;
  workflow_semantic_version?: string | null;
  status?: RunListProjectionStatus | null;
  scheduler_policy_id?: string | null;
  retention_policy_id?: string | null;
  client_id?: string | null;
  client_session_id?: string | null;
  bucket_id?: string | null;
  accepted_at_from_ms?: number | null;
  accepted_at_to_ms?: number | null;
  after_event_seq?: number | null;
  limit?: number | null;
  projection_batch_size?: number | null;
}

export interface WorkflowRunListQueryResponse {
  runs: RunListProjectionRecord[];
  facets: RunListFacetRecord[];
  projection_state: ProjectionStateRecord;
}

export interface WorkflowRunDetailQueryRequest {
  workflow_run_id: string;
  projection_batch_size?: number | null;
}

export interface WorkflowRunDetailQueryResponse {
  run?: RunDetailProjectionRecord | null;
  projection_state: ProjectionStateRecord;
}

export interface WorkflowSchedulerEstimateQueryRequest {
  workflow_run_id: string;
  projection_batch_size?: number | null;
}

export interface WorkflowSchedulerEstimateRecord {
  workflow_run_id: string;
  workflow_id: string;
  workflow_version_id?: string | null;
  workflow_semantic_version?: string | null;
  scheduler_policy_id?: string | null;
  latest_estimate_json?: string | null;
  estimate_confidence?: string | null;
  estimated_queue_wait_ms?: number | null;
  estimated_duration_ms?: number | null;
  last_event_seq: number;
  last_updated_at_ms: number;
}

export interface WorkflowSchedulerEstimateQueryResponse {
  estimate?: WorkflowSchedulerEstimateRecord | null;
  projection_state: ProjectionStateRecord;
}

export interface WorkflowIoArtifactQueryRequest {
  workflow_run_id?: string | null;
  node_id?: string | null;
  producer_node_id?: string | null;
  consumer_node_id?: string | null;
  artifact_role?: string | null;
  media_type?: string | null;
  retention_state?: IoArtifactRetentionState | null;
  retention_policy_id?: string | null;
  runtime_id?: string | null;
  model_id?: string | null;
  after_event_seq?: number | null;
  limit?: number | null;
  projection_batch_size?: number | null;
}

export interface WorkflowIoArtifactQueryResponse {
  artifacts: IoArtifactProjectionRecord[];
  retention_summary: IoArtifactRetentionSummaryRecord[];
  projection_state: ProjectionStateRecord;
}

export interface WorkflowNodeStatusQueryRequest {
  workflow_run_id?: string | null;
  node_id?: string | null;
  status?: NodeExecutionProjectionStatus | null;
  after_event_seq?: number | null;
  limit?: number | null;
  projection_batch_size?: number | null;
}

export interface WorkflowNodeStatusQueryResponse {
  nodes: NodeStatusProjectionRecord[];
  projection_state: ProjectionStateRecord;
}

export interface WorkflowProjectionRebuildRequest {
  projection_name: string;
  batch_size?: number | null;
}

export interface WorkflowProjectionRebuildResponse {
  projection_state: ProjectionStateRecord;
}

export interface LibraryUsageProjectionRecord {
  asset_id: string;
  total_access_count: number;
  run_access_count: number;
  total_network_bytes: number;
  last_accessed_at_ms: number;
  last_operation: string;
  last_cache_status?: string | null;
  last_workflow_run_id?: string | null;
  last_workflow_id?: string | null;
  last_workflow_version_id?: string | null;
  last_workflow_semantic_version?: string | null;
  last_client_id?: string | null;
  last_client_session_id?: string | null;
  last_bucket_id?: string | null;
  last_event_seq: number;
  last_updated_at_ms: number;
}

export interface WorkflowLibraryUsageQueryRequest {
  asset_id?: string | null;
  workflow_run_id?: string | null;
  workflow_id?: string | null;
  workflow_version_id?: string | null;
  after_event_seq?: number | null;
  limit?: number | null;
  projection_batch_size?: number | null;
}

export interface WorkflowLibraryUsageQueryResponse {
  assets: LibraryUsageProjectionRecord[];
  projection_state: ProjectionStateRecord;
}

export type DiagnosticsRetentionClass = 'standard';

export interface DiagnosticsRetentionPolicy {
  policy_id: string;
  policy_version: number;
  retention_class: DiagnosticsRetentionClass;
  retention_days: number;
  applied_at_ms: number;
  explanation: string;
}

export type WorkflowRetentionPolicyQueryRequest = Record<string, never>;

export interface WorkflowRetentionPolicyQueryResponse {
  retention_policy: DiagnosticsRetentionPolicy;
}

export interface WorkflowRetentionPolicyUpdateRequest {
  retention_days: number;
  explanation: string;
  reason: string;
}

export interface WorkflowRetentionPolicyUpdateResponse {
  retention_policy: DiagnosticsRetentionPolicy;
}

export interface WorkflowRetentionCleanupRequest {
  limit?: number | null;
  reason: string;
}

export interface WorkflowRetentionCleanupResult {
  policy_id: string;
  policy_version: number;
  retention_class: DiagnosticsRetentionClass;
  cutoff_occurred_before_ms: number;
  expired_artifact_count: number;
  last_event_seq?: number | null;
}

export interface WorkflowRetentionCleanupResponse {
  cleanup: WorkflowRetentionCleanupResult;
}

export interface PumasModelDeleteAuditResponse {
  success: boolean;
  error?: string | null;
  auditEventSeq?: number | null;
}

export interface PumasHfModelSearchAuditRequest {
  query: string;
  kind?: string | null;
  limit?: number | null;
  hydrateLimit?: number | null;
}

export interface PumasHfModelSearchResult {
  id?: string | null;
  [key: string]: unknown;
}

export interface PumasHfModelSearchAuditResponse {
  models: PumasHfModelSearchResult[];
  auditEventSeq?: number | null;
}

export interface PumasHfDownloadRequest {
  repo_id: string;
  family: string;
  official_name: string;
  model_type?: string | null;
  quant?: string | null;
  filename?: string | null;
  filenames?: string[] | null;
  pipeline_tag?: string | null;
  bundle_format?: unknown;
  pipeline_class?: string | null;
  release_date?: string | null;
  download_url?: string | null;
  model_card_json?: string | null;
  license_status?: string | null;
}

export interface PumasHfDownloadStartAuditResponse {
  downloadId: string;
  auditEventSeq?: number | null;
}

export interface WorkflowSchedulerTimelineQueryRequest {
  workflow_run_id?: string | null;
  workflow_id?: string | null;
  scheduler_policy_id?: string | null;
  after_event_seq?: number | null;
  limit?: number | null;
  projection_batch_size?: number | null;
}

export interface WorkflowSchedulerTimelineQueryResponse {
  events: SchedulerTimelineProjectionRecord[];
  projection_state: ProjectionStateRecord;
}
