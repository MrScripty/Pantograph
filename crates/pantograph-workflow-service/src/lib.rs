//! Host-agnostic application services for Pantograph workflow use-cases.
//!
//! This crate owns service-level request/response contracts and orchestration
//! interfaces. Transport adapters (Tauri/UniFFI/Rustler) should delegate into
//! this crate rather than duplicate business logic.

// Force linker to include workflow-nodes inventory::submit!() statics so the
// graph registry can discover built-in descriptors in headless consumers too.
extern crate workflow_nodes;

pub mod capabilities;
pub mod graph;
mod scheduler;
pub mod technical_fit;
pub mod trace;
pub mod workflow;

pub use graph::{
    convert_graph_from_node_engine, convert_graph_to_node_engine,
    graph_memory_impact_from_node_engine_graph_change, validate_workflow_connection,
    validate_workflow_graph_contract, workflow_executable_topology,
    workflow_executable_topology_with_node_versions, workflow_execution_fingerprint,
    workflow_execution_fingerprint_for_topology, workflow_graph_run_settings,
    workflow_graph_run_settings_json, workflow_presentation_fingerprint,
    workflow_presentation_fingerprint_for_metadata, workflow_presentation_metadata,
    workflow_presentation_metadata_json, ConnectionAnchor, ConnectionCandidatesResponse,
    ConnectionCommitResponse, ConnectionRejection, ConnectionRejectionReason,
    ConnectionTargetAnchorCandidate, ConnectionTargetNodeCandidate, EdgeInsertionBridge,
    EdgeInsertionPreviewResponse, ExecutionMode, FileSystemWorkflowGraphStore, GraphEdge,
    GraphNode, InsertNodeConnectionResponse, InsertNodeOnEdgeResponse, InsertNodePositionHint,
    InsertableNodeTypeCandidate, IoBindingOrigin, NodeCategory, NodeDefinition, NodeGroup,
    NodeRegistry, PortDataType, PortDefinition, PortMapping, Position, UndoRedoState, Viewport,
    WorkflowExecutableTopology, WorkflowExecutableTopologyEdge, WorkflowExecutableTopologyNode,
    WorkflowFile, WorkflowGraph, WorkflowGraphAddEdgeRequest, WorkflowGraphAddNodeRequest,
    WorkflowGraphConnectRequest, WorkflowGraphCreateGroupRequest, WorkflowGraphDeleteRequest,
    WorkflowGraphDeleteResponse, WorkflowGraphDeleteSelectionRequest,
    WorkflowGraphEditSessionCloseRequest, WorkflowGraphEditSessionCloseResponse,
    WorkflowGraphEditSessionCreateRequest, WorkflowGraphEditSessionCreateResponse,
    WorkflowGraphEditSessionGraphRequest, WorkflowGraphEditSessionGraphResponse,
    WorkflowGraphGetConnectionCandidatesRequest, WorkflowGraphInsertNodeAndConnectRequest,
    WorkflowGraphInsertNodeOnEdgeRequest, WorkflowGraphListResponse, WorkflowGraphLoadRequest,
    WorkflowGraphMetadata, WorkflowGraphPreviewNodeInsertOnEdgeRequest,
    WorkflowGraphRemoveEdgeRequest, WorkflowGraphRemoveEdgesRequest,
    WorkflowGraphRemoveNodeRequest, WorkflowGraphRunSettings, WorkflowGraphRunSettingsNode,
    WorkflowGraphSaveRequest, WorkflowGraphSaveResponse, WorkflowGraphStore,
    WorkflowGraphUndoRedoStateRequest, WorkflowGraphUndoRedoStateResponse,
    WorkflowGraphUngroupRequest, WorkflowGraphUpdateGroupPortsRequest,
    WorkflowGraphUpdateNodeDataRequest, WorkflowGraphUpdateNodePositionRequest,
    WorkflowPresentationEdge, WorkflowPresentationMetadata, WorkflowPresentationNode,
};
pub use pantograph_runtime_attribution::{ClientSessionId, WorkflowId, WorkflowRunId};
pub use scheduler::{
    select_runtime_unload_candidate_by_affinity, WorkflowSchedulerRuntimeCapacityPressure,
    WorkflowSchedulerRuntimeRegistryDiagnostics, WorkflowSchedulerRuntimeWarmupDecision,
    WorkflowSchedulerRuntimeWarmupReason, WorkflowSchedulerSnapshotDiagnostics,
};
pub use technical_fit::{
    build_workflow_technical_fit_request, WorkflowTechnicalFitDecision,
    WorkflowTechnicalFitOverride, WorkflowTechnicalFitQueuePressure, WorkflowTechnicalFitReason,
    WorkflowTechnicalFitReasonCode, WorkflowTechnicalFitRequest, WorkflowTechnicalFitSelectionMode,
};
pub use trace::{
    WorkflowTraceEvent, WorkflowTraceGraphContext, WorkflowTraceGraphTimingExpectations,
    WorkflowTraceNodeRecord, WorkflowTraceNodeStatus, WorkflowTraceNodeTimingExpectation,
    WorkflowTraceQueueMetrics, WorkflowTraceRecordResult, WorkflowTraceRuntimeMetrics,
    WorkflowTraceRuntimeSelection, WorkflowTraceSnapshotRequest, WorkflowTraceSnapshotResponse,
    WorkflowTraceStatus, WorkflowTraceStore, WorkflowTraceSummary,
};
pub use workflow::{
    evaluate_runtime_preflight, format_runtime_not_ready_message, BucketCreateRequest,
    BucketDeleteRequest, BucketRecord, BucketSelection, ClientRegistrationRequest,
    ClientRegistrationResponse, ClientSessionOpenRequest, ClientSessionOpenResponse,
    ClientSessionRecord, ClientSessionResumeRequest, CredentialProofRequest, CredentialSecret,
    WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse, WorkflowCapabilityModel,
    WorkflowDiagnosticsUsageQueryRequest, WorkflowDiagnosticsUsageQueryResponse,
    WorkflowDiagnosticsUsageSummary, WorkflowErrorCode, WorkflowErrorDetails,
    WorkflowErrorEnvelope, WorkflowExecutionSessionCloseRequest,
    WorkflowExecutionSessionCloseResponse, WorkflowExecutionSessionCreateRequest,
    WorkflowExecutionSessionCreateResponse, WorkflowExecutionSessionInspectionRequest,
    WorkflowExecutionSessionInspectionResponse, WorkflowExecutionSessionKeepAliveRequest,
    WorkflowExecutionSessionKeepAliveResponse, WorkflowExecutionSessionQueueCancelRequest,
    WorkflowExecutionSessionQueueCancelResponse, WorkflowExecutionSessionQueueItem,
    WorkflowExecutionSessionQueueItemStatus, WorkflowExecutionSessionQueueListRequest,
    WorkflowExecutionSessionQueueListResponse, WorkflowExecutionSessionQueueReprioritizeRequest,
    WorkflowExecutionSessionQueueReprioritizeResponse, WorkflowExecutionSessionRetentionHint,
    WorkflowExecutionSessionRunRequest, WorkflowExecutionSessionRuntimeSelectionTarget,
    WorkflowExecutionSessionRuntimeUnloadCandidate, WorkflowExecutionSessionStaleCleanupRequest,
    WorkflowExecutionSessionStaleCleanupResponse, WorkflowExecutionSessionStaleCleanupWorker,
    WorkflowExecutionSessionStaleCleanupWorkerConfig, WorkflowExecutionSessionState,
    WorkflowExecutionSessionStatusRequest, WorkflowExecutionSessionStatusResponse,
    WorkflowExecutionSessionSummary, WorkflowExecutionSessionUnloadReason, WorkflowHost,
    WorkflowHostCapabilities, WorkflowHostModelDescriptor, WorkflowIdentity, WorkflowIdentityError,
    WorkflowInputTarget, WorkflowIoNode, WorkflowIoPort, WorkflowIoRequest, WorkflowIoResponse,
    WorkflowOutputTarget, WorkflowPortBinding, WorkflowPreflightRequest, WorkflowPreflightResponse,
    WorkflowPresentationRevisionRecord, WorkflowPresentationRevisionResolveRequest,
    WorkflowRunAttribution, WorkflowRunHandle, WorkflowRunOptions, WorkflowRunRecord,
    WorkflowRunRequest, WorkflowRunResponse, WorkflowRunSnapshotRecord, WorkflowRunSnapshotRequest,
    WorkflowRunVersionProjection, WorkflowRuntimeCapability, WorkflowRuntimeInstallState,
    WorkflowRuntimeIssue, WorkflowRuntimeReadinessState, WorkflowRuntimeRequirements,
    WorkflowRuntimeSourceKind, WorkflowSchedulerAdmissionOutcome, WorkflowSchedulerDecisionReason,
    WorkflowSchedulerDiagnosticsProvider, WorkflowSchedulerErrorDetails,
    WorkflowSchedulerErrorReason, WorkflowSchedulerRuntimeDiagnosticsRequest,
    WorkflowSchedulerSnapshotRequest, WorkflowSchedulerSnapshotResponse,
    WorkflowSchedulerTimelineQueryRequest, WorkflowSchedulerTimelineQueryResponse, WorkflowService,
    WorkflowServiceError, WorkflowVersionRecord, WorkflowVersionResolveRequest,
};
pub use workflow::{
    ProjectionStateRecord, SchedulerTimelineProjectionRecord, SqliteDiagnosticsLedger,
    WorkflowTimingExpectation, WorkflowTimingExpectationComparison,
};
