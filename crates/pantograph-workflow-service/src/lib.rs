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
pub mod workflow;

pub use graph::{
    convert_graph_to_node_engine, validate_workflow_connection, ConnectionAnchor,
    ConnectionCandidatesResponse, ConnectionCommitResponse, ConnectionRejection,
    ConnectionRejectionReason, ConnectionTargetAnchorCandidate, ConnectionTargetNodeCandidate,
    ExecutionMode, FileSystemWorkflowGraphStore, GraphEdge, GraphNode,
    InsertNodeConnectionResponse, InsertNodePositionHint, InsertableNodeTypeCandidate,
    IoBindingOrigin, NodeCategory, NodeDefinition, NodeRegistry, PortDataType, PortDefinition,
    Position, UndoRedoState, Viewport, WorkflowFile, WorkflowGraph, WorkflowGraphAddEdgeRequest,
    WorkflowGraphAddNodeRequest, WorkflowGraphConnectRequest, WorkflowGraphEditSessionCloseRequest,
    WorkflowGraphEditSessionCloseResponse, WorkflowGraphEditSessionCreateRequest,
    WorkflowGraphEditSessionCreateResponse, WorkflowGraphEditSessionGraphRequest,
    WorkflowGraphEditSessionGraphResponse, WorkflowGraphGetConnectionCandidatesRequest,
    WorkflowGraphInsertNodeAndConnectRequest, WorkflowGraphListResponse, WorkflowGraphLoadRequest,
    WorkflowGraphMetadata, WorkflowGraphRemoveEdgeRequest, WorkflowGraphRemoveNodeRequest,
    WorkflowGraphSaveRequest, WorkflowGraphSaveResponse, WorkflowGraphStore,
    WorkflowGraphUndoRedoStateRequest, WorkflowGraphUndoRedoStateResponse,
    WorkflowGraphUpdateNodeDataRequest, WorkflowGraphUpdateNodePositionRequest,
};
pub use workflow::{
    WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse, WorkflowCapabilityModel,
    WorkflowErrorCode, WorkflowErrorEnvelope, WorkflowHost, WorkflowHostCapabilities,
    WorkflowHostModelDescriptor, WorkflowInputTarget, WorkflowIoNode, WorkflowIoPort,
    WorkflowIoRequest, WorkflowIoResponse, WorkflowOutputTarget, WorkflowPortBinding,
    WorkflowPreflightRequest, WorkflowPreflightResponse, WorkflowRunHandle, WorkflowRunOptions,
    WorkflowRunRequest, WorkflowRunResponse, WorkflowRuntimeRequirements, WorkflowService,
    WorkflowServiceError, WorkflowSessionCloseRequest, WorkflowSessionCloseResponse,
    WorkflowSessionCreateRequest, WorkflowSessionCreateResponse, WorkflowSessionKeepAliveRequest,
    WorkflowSessionKeepAliveResponse, WorkflowSessionQueueCancelRequest,
    WorkflowSessionQueueCancelResponse, WorkflowSessionQueueItem, WorkflowSessionQueueItemStatus,
    WorkflowSessionQueueListRequest, WorkflowSessionQueueListResponse,
    WorkflowSessionQueueReprioritizeRequest, WorkflowSessionQueueReprioritizeResponse,
    WorkflowSessionRunRequest, WorkflowSessionState, WorkflowSessionStatusRequest,
    WorkflowSessionStatusResponse, WorkflowSessionSummary, WorkflowSessionUnloadReason,
};
