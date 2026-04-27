//! Durable client, session, bucket, and workflow-run attribution.
//!
//! This crate validates caller attribution at backend boundaries and persists
//! complete attribution state transitions before runtime execution begins.

mod error;
mod ids;
mod records;
mod repository;
mod schema;
mod sqlite;
mod sqlite_rows;
mod util;

pub use error::AttributionError;
pub use ids::{
    BucketId, ClientCredentialId, ClientId, ClientSessionId, UsageEventId, WorkflowId,
    WorkflowRunId, WorkflowVersionId,
};
pub use records::{
    BucketCreateRequest, BucketDeleteRequest, BucketRecord, BucketSelection, BucketStatus,
    ClientCredential, ClientCredentialStatus, ClientRegistrationRequest,
    ClientRegistrationResponse, ClientSessionDisconnectRequest, ClientSessionExpireRequest,
    ClientSessionLifecycleState, ClientSessionOpenRequest, ClientSessionOpenResponse,
    ClientSessionRecord, ClientSessionResumeRequest, ClientStatus, CredentialProofRequest,
    CredentialSecret, DefaultBucketAssignment, SessionLifecycleRecord, WorkflowRunAttribution,
    WorkflowRunRecord, WorkflowRunStartRequest, WorkflowRunStatus, WorkflowVersionRecord,
    WorkflowVersionResolveRequest,
};
pub use repository::AttributionRepository;
pub use sqlite::SqliteAttributionStore;

#[cfg(test)]
mod tests;
