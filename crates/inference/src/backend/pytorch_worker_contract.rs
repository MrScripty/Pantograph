use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model_contracts::{
    InferenceTaskId, ModelArtifactKind, OptionCompatibilityDiagnostic, PumasModelRef,
};

pub(super) const PYTORCH_WORKER_CONTRACT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub(super) struct PyTorchWorkerEnvelope<T> {
    pub contract_version: u32,
    pub request_id: String,
    pub operation: PyTorchWorkerOperation,
    #[serde(default)]
    pub cancellation: PyTorchWorkerCancellation,
    pub payload: T,
}

impl<T> PyTorchWorkerEnvelope<T> {
    #[must_use]
    pub(super) fn new(
        request_id: impl Into<String>,
        operation: PyTorchWorkerOperation,
        payload: T,
    ) -> Self {
        Self {
            contract_version: PYTORCH_WORKER_CONTRACT_VERSION,
            request_id: request_id.into(),
            operation,
            cancellation: PyTorchWorkerCancellation::default(),
            payload,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum PyTorchWorkerOperation {
    InitWorker,
    ShutdownWorker,
    GetLoadedInfo,
    LoadTransformersModel,
    GenerateText,
    GenerateTextStream,
    SaveKvCache,
    RestoreKvCache,
    ClearKvCache,
    TruncateKvCache,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchWorkerCancellation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default)]
    pub drop_stream_cancels: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchTransformersLoadRequest {
    pub model_ref: PumasModelRef,
    pub artifact_kind: ModelArtifactKind,
    pub entry_path: String,
    pub task_id: InferenceTaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_type_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(default)]
    pub trust_policy: PyTorchTransformersTrustPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_defaults: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchTransformersTrustPolicy {
    #[serde(default)]
    pub allow_remote_code: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accepted_sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "status")]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub(super) enum PyTorchWorkerResponse<T> {
    Ok(PyTorchWorkerSuccess<T>),
    Error(PyTorchWorkerFailure),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub(super) struct PyTorchWorkerSuccess<T> {
    pub request_id: String,
    pub result: T,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchWorkerFailure {
    pub request_id: String,
    pub error: PyTorchWorkerError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchWorkerError {
    pub kind: PyTorchWorkerErrorKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_code: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum PyTorchWorkerErrorKind {
    InvalidRequest,
    UnsupportedTask,
    TrustPolicyRejected,
    ModelLoadFailed,
    GenerationFailed,
    Cancelled,
    RuntimeUnavailable,
    Internal,
}
