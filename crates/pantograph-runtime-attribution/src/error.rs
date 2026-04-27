use thiserror::Error;

use crate::{BucketId, ClientId, ClientSessionLifecycleState, WorkflowId};

#[derive(Debug, Error)]
pub enum AttributionError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
    #[error("{field} is too long")]
    FieldTooLong { field: &'static str, max_len: usize },
    #[error("{field} contains control characters")]
    InvalidField { field: &'static str },
    #[error("credential is revoked")]
    CredentialRevoked,
    #[error("credential does not match persisted client identity")]
    CredentialMismatch,
    #[error("client is disabled")]
    ClientDisabled,
    #[error("client already has an active session")]
    DuplicateActiveSession { client_id: ClientId },
    #[error("client session does not belong to the credential client")]
    SessionClientMismatch,
    #[error("client session is not resumable")]
    SessionNotResumable { state: ClientSessionLifecycleState },
    #[error("client session is not active")]
    SessionNotActive { state: ClientSessionLifecycleState },
    #[error("invalid client session lifecycle transition")]
    InvalidSessionTransition {
        from: ClientSessionLifecycleState,
        to: ClientSessionLifecycleState,
    },
    #[error("bucket name already exists for this client")]
    BucketNameCollision { name: String },
    #[error("bucket name is reserved")]
    BucketNameReserved { name: String },
    #[error("bucket does not belong to the credential client")]
    BucketClientMismatch,
    #[error("default bucket cannot be deleted")]
    DefaultBucketProtected,
    #[error("bucket is protected by active workflow runs")]
    BucketDeletionProtected { bucket_id: BucketId },
    #[error("bucket rename is unsupported because bucket names are immutable")]
    BucketRenameUnsupported,
    #[error("workflow semantic version is invalid")]
    InvalidWorkflowSemanticVersion { value: String },
    #[error("workflow semantic version already points at a different execution fingerprint")]
    WorkflowSemanticVersionConflict {
        workflow_id: WorkflowId,
        semantic_version: String,
    },
    #[error("workflow execution fingerprint already points at a different semantic version")]
    WorkflowFingerprintVersionConflict {
        workflow_id: WorkflowId,
        execution_fingerprint: String,
    },
    #[error("record was not found")]
    NotFound { entity: &'static str },
    #[error("unsupported attribution schema version {found}")]
    UnsupportedSchemaVersion { found: i64 },
    #[error("attribution storage error: {0}")]
    Storage(#[from] rusqlite::Error),
}
