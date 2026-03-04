//! Host-agnostic application services for Pantograph workflow use-cases.
//!
//! This crate owns service-level request/response contracts and orchestration
//! interfaces. Transport adapters (Tauri/UniFFI/Rustler) should delegate into
//! this crate rather than duplicate business logic.

pub mod capabilities;
pub mod workflow;

pub use workflow::{
    WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse, WorkflowCapabilityModel,
    WorkflowHost, WorkflowHostCapabilities, WorkflowHostModelDescriptor, WorkflowOutputTarget,
    WorkflowPortBinding, WorkflowRunRequest, WorkflowRunResponse, WorkflowRuntimeRequirements,
    WorkflowService, WorkflowServiceError, WorkflowSessionCloseRequest,
    WorkflowSessionCloseResponse, WorkflowSessionCreateRequest, WorkflowSessionCreateResponse,
    WorkflowSessionRunRequest,
};
