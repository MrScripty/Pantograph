use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::{
    BucketId, ClientCredentialId, ClientId, ClientSessionId, WorkflowId,
    WorkflowPresentationRevisionId, WorkflowRunId, WorkflowRunSnapshotId, WorkflowVersionId,
};

const MAX_CREDENTIAL_SECRET_LEN: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientStatus {
    Active,
    Disabled,
}

impl ClientStatus {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientCredentialStatus {
    Active,
    Revoked,
}

impl ClientCredentialStatus {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientSessionLifecycleState {
    Opening,
    Connected,
    DisconnectedGrace,
    Expired,
    TakenOver,
    Closed,
}

impl ClientSessionLifecycleState {
    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Opening | Self::Connected | Self::DisconnectedGrace
        )
    }

    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Opening => "opening",
            Self::Connected => "connected",
            Self::DisconnectedGrace => "disconnected_grace",
            Self::Expired => "expired",
            Self::TakenOver => "taken_over",
            Self::Closed => "closed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BucketStatus {
    Active,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl WorkflowRunStatus {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRecord {
    pub client_id: ClientId,
    pub display_name: Option<String>,
    pub metadata_json: Option<String>,
    pub status: ClientStatus,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientCredential {
    pub client_credential_id: ClientCredentialId,
    pub client_id: ClientId,
    pub status: ClientCredentialStatus,
    pub created_at_ms: i64,
    pub revoked_at_ms: Option<i64>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct CredentialSecret(String);

impl CredentialSecret {
    #[must_use]
    pub fn generate() -> Self {
        Self(format!(
            "pcta_{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        ))
    }

    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    pub fn from_boundary_secret(value: impl Into<String>) -> Result<Self, crate::AttributionError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(crate::AttributionError::MissingField {
                field: "credential.secret",
            });
        }
        if trimmed.len() > MAX_CREDENTIAL_SECRET_LEN {
            return Err(crate::AttributionError::FieldTooLong {
                field: "credential.secret",
                max_len: MAX_CREDENTIAL_SECRET_LEN,
            });
        }
        if trimmed.chars().any(char::is_control) {
            return Err(crate::AttributionError::InvalidField {
                field: "credential.secret",
            });
        }
        Ok(Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn proof_request(&self, credential_id: ClientCredentialId) -> CredentialProofRequest {
        CredentialProofRequest {
            credential_id,
            secret: self.clone(),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_raw_for_test(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl Serialize for CredentialSecret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.expose_secret())
    }
}

impl<'de> Deserialize<'de> for CredentialSecret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_boundary_secret(value).map_err(serde::de::Error::custom)
    }
}

impl fmt::Debug for CredentialSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CredentialSecret(<redacted>)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialProofRequest {
    pub credential_id: ClientCredentialId,
    pub secret: CredentialSecret,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRegistrationRequest {
    pub display_name: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRegistrationResponse {
    pub client: ClientRecord,
    pub credential: ClientCredential,
    pub credential_secret: CredentialSecret,
}

impl ClientRegistrationResponse {
    #[must_use]
    pub fn credential_proof_request(&self) -> CredentialProofRequest {
        self.credential_secret
            .proof_request(self.credential.client_credential_id.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientSessionRecord {
    pub client_session_id: ClientSessionId,
    pub client_id: ClientId,
    pub opened_at_ms: i64,
    pub latest_lifecycle_state: ClientSessionLifecycleState,
    pub grace_deadline_ms: Option<i64>,
    pub superseded_by_session_id: Option<ClientSessionId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionLifecycleRecord {
    pub event_id: String,
    pub client_session_id: ClientSessionId,
    pub lifecycle_state: ClientSessionLifecycleState,
    pub occurred_at_ms: i64,
    pub reason: Option<String>,
    pub related_session_id: Option<ClientSessionId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BucketRecord {
    pub bucket_id: BucketId,
    pub client_id: ClientId,
    pub name: String,
    pub metadata_json: Option<String>,
    pub created_at_ms: i64,
    pub deleted_at_ms: Option<i64>,
    pub deletion_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefaultBucketAssignment {
    pub client_session_id: ClientSessionId,
    pub bucket_id: BucketId,
    pub assigned_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRunRecord {
    pub workflow_run_id: WorkflowRunId,
    pub workflow_id: WorkflowId,
    pub client_id: ClientId,
    pub client_session_id: ClientSessionId,
    pub bucket_id: BucketId,
    pub status: WorkflowRunStatus,
    pub started_at_ms: i64,
    pub completed_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowVersionRecord {
    pub workflow_version_id: WorkflowVersionId,
    pub workflow_id: WorkflowId,
    pub semantic_version: String,
    pub execution_fingerprint: String,
    pub executable_topology_json: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowPresentationRevisionRecord {
    pub workflow_presentation_revision_id: WorkflowPresentationRevisionId,
    pub workflow_id: WorkflowId,
    pub workflow_version_id: WorkflowVersionId,
    pub presentation_fingerprint: String,
    pub presentation_metadata_json: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRunSnapshotRecord {
    pub workflow_run_snapshot_id: WorkflowRunSnapshotId,
    pub workflow_run_id: WorkflowRunId,
    pub workflow_id: WorkflowId,
    pub workflow_version_id: WorkflowVersionId,
    pub workflow_presentation_revision_id: WorkflowPresentationRevisionId,
    pub workflow_semantic_version: String,
    pub workflow_execution_fingerprint: String,
    pub workflow_execution_session_id: String,
    pub workflow_execution_session_kind: String,
    pub usage_profile: Option<String>,
    pub keep_alive: bool,
    pub retention_policy: String,
    pub scheduler_policy: String,
    pub priority: i32,
    pub timeout_ms: Option<u64>,
    pub inputs_json: String,
    pub output_targets_json: Option<String>,
    pub override_selection_json: Option<String>,
    pub graph_settings_json: String,
    pub runtime_requirements_json: String,
    pub capability_models_json: String,
    pub runtime_capabilities_json: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRunVersionProjection {
    pub snapshot: WorkflowRunSnapshotRecord,
    pub workflow_version: WorkflowVersionRecord,
    pub presentation_revision: WorkflowPresentationRevisionRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRunAttribution {
    pub client_id: ClientId,
    pub client_session_id: ClientSessionId,
    pub bucket_id: BucketId,
    pub workflow_run_id: WorkflowRunId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientSessionOpenRequest {
    pub credential: CredentialProofRequest,
    pub takeover: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientSessionOpenResponse {
    pub session: ClientSessionRecord,
    pub default_bucket: BucketRecord,
    pub default_bucket_assignment: DefaultBucketAssignment,
    pub superseded_session_id: Option<ClientSessionId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientSessionResumeRequest {
    pub credential: CredentialProofRequest,
    pub client_session_id: ClientSessionId,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientSessionDisconnectRequest {
    pub credential: CredentialProofRequest,
    pub client_session_id: ClientSessionId,
    pub grace_deadline_ms: i64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientSessionExpireRequest {
    pub client_session_id: ClientSessionId,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BucketCreateRequest {
    pub credential: CredentialProofRequest,
    pub name: String,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BucketDeleteRequest {
    pub credential: CredentialProofRequest,
    pub bucket_id: BucketId,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "bucket_id")]
pub enum BucketSelection {
    Default,
    Explicit(BucketId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRunStartRequest {
    pub credential: CredentialProofRequest,
    pub client_session_id: ClientSessionId,
    pub workflow_id: WorkflowId,
    pub bucket_selection: BucketSelection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowVersionResolveRequest {
    pub workflow_id: WorkflowId,
    pub semantic_version: String,
    pub execution_fingerprint: String,
    pub executable_topology_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowPresentationRevisionResolveRequest {
    pub workflow_id: WorkflowId,
    pub workflow_version_id: WorkflowVersionId,
    pub presentation_fingerprint: String,
    pub presentation_metadata_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRunSnapshotRequest {
    pub workflow_run_id: WorkflowRunId,
    pub workflow_id: WorkflowId,
    pub workflow_version_id: WorkflowVersionId,
    pub workflow_presentation_revision_id: WorkflowPresentationRevisionId,
    pub workflow_semantic_version: String,
    pub workflow_execution_fingerprint: String,
    pub workflow_execution_session_id: String,
    pub workflow_execution_session_kind: String,
    pub usage_profile: Option<String>,
    pub keep_alive: bool,
    pub retention_policy: String,
    pub scheduler_policy: String,
    pub priority: i32,
    pub timeout_ms: Option<u64>,
    pub inputs_json: String,
    pub output_targets_json: Option<String>,
    pub override_selection_json: Option<String>,
    pub graph_settings_json: String,
    pub runtime_requirements_json: String,
    pub capability_models_json: String,
    pub runtime_capabilities_json: String,
}
