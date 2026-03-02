//! Host-agnostic application services for Pantograph workflow use-cases.
//!
//! This crate owns service-level request/response contracts and orchestration
//! interfaces. Transport adapters (Tauri/UniFFI/Rustler) should delegate into
//! this crate rather than duplicate business logic.

pub mod embedding;

pub use embedding::{
    EmbedInputObject, EmbedObjectError, EmbedObjectResult, EmbedObjectsV1Request,
    EmbedObjectsV1Response, EmbeddingHost, EmbeddingHostCapabilities, EmbeddingService,
    EmbeddingServiceError, EmbeddingStatus, GetEmbeddingWorkflowCapabilitiesV1Request,
    GetEmbeddingWorkflowCapabilitiesV1Response, ModelSignature,
};
