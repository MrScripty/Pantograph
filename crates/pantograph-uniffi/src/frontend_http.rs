use std::sync::{Arc, LazyLock};

use pantograph_frontend_http_adapter::FrontendHttpWorkflowHost;
use pantograph_workflow_service::{
    BucketCreateRequest, BucketDeleteRequest, ClientRegistrationRequest, ClientSessionOpenRequest,
    ClientSessionResumeRequest, WorkflowAttributedRunRequest, WorkflowCapabilitiesRequest,
    WorkflowErrorCode, WorkflowErrorEnvelope, WorkflowPreflightRequest, WorkflowRunRequest,
    WorkflowService, WorkflowServiceError, WorkflowSessionCloseRequest,
    WorkflowSessionCreateRequest, WorkflowSessionKeepAliveRequest,
    WorkflowSessionQueueCancelRequest, WorkflowSessionQueueListRequest,
    WorkflowSessionQueueReprioritizeRequest, WorkflowSessionRunRequest,
    WorkflowSessionStatusRequest,
};

use super::{FfiError, FfiPumasApi};

static WORKFLOW_SERVICE: LazyLock<WorkflowService> = LazyLock::new(|| {
    WorkflowService::with_ephemeral_attribution_store()
        .expect("frontend HTTP attribution store should initialize")
});

fn map_workflow_service_error(err: WorkflowServiceError) -> FfiError {
    FfiError::Other {
        message: err.to_envelope_json(),
    }
}

fn workflow_error_json(code: WorkflowErrorCode, message: impl Into<String>) -> String {
    let envelope = WorkflowErrorEnvelope {
        code,
        message: message.into(),
        details: None,
    };
    serde_json::to_string(&envelope).unwrap_or_else(|_| {
        r#"{"code":"internal_error","message":"failed to serialize workflow error envelope"}"#
            .to_string()
    })
}

fn workflow_adapter_error(code: WorkflowErrorCode, message: impl Into<String>) -> FfiError {
    FfiError::Other {
        message: workflow_error_json(code, message),
    }
}

fn workflow_parse_request<T: serde::de::DeserializeOwned>(
    request_json: &str,
) -> Result<T, FfiError> {
    serde_json::from_str(request_json).map_err(|e| {
        workflow_adapter_error(
            WorkflowErrorCode::InvalidRequest,
            format!("invalid request: {}", e),
        )
    })
}

fn workflow_serialize_response<T: serde::Serialize>(value: &T) -> Result<String, FfiError> {
    serde_json::to_string(value).map_err(|e| {
        workflow_adapter_error(
            WorkflowErrorCode::InternalError,
            format!("response serialization error: {}", e),
        )
    })
}

fn build_frontend_http_host(
    base_url: String,
    pumas_api: Option<Arc<FfiPumasApi>>,
) -> Result<FrontendHttpWorkflowHost, FfiError> {
    FrontendHttpWorkflowHost::with_defaults(
        base_url,
        pumas_api.as_ref().map(|api| api.api_arc()),
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
    )
    .map_err(|e| {
        workflow_adapter_error(
            WorkflowErrorCode::InvalidRequest,
            format!("frontend HTTP host config error: {}", e),
        )
    })
}

/// Execute frontend HTTP workflow contract (`workflow_run`) and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_run(
    base_url: String,
    request_json: String,
    pumas_api: Option<Arc<FfiPumasApi>>,
) -> Result<String, FfiError> {
    let request: WorkflowRunRequest = workflow_parse_request(&request_json)?;

    let host = build_frontend_http_host(base_url, pumas_api)?;
    let response = WORKFLOW_SERVICE
        .workflow_run(&host, request)
        .await
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Register a frontend HTTP attribution client and return ClientRegistrationResponse JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_register_attribution_client(
    request_json: String,
) -> Result<String, FfiError> {
    let request: ClientRegistrationRequest = workflow_parse_request(&request_json)?;
    let response = WORKFLOW_SERVICE
        .register_attribution_client(request)
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Open a durable frontend HTTP client session and return ClientSessionOpenResponse JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_open_client_session(
    request_json: String,
) -> Result<String, FfiError> {
    let request: ClientSessionOpenRequest = workflow_parse_request(&request_json)?;
    let response = WORKFLOW_SERVICE
        .open_client_session(request)
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Resume a durable frontend HTTP client session and return ClientSessionRecord JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_resume_client_session(
    request_json: String,
) -> Result<String, FfiError> {
    let request: ClientSessionResumeRequest = workflow_parse_request(&request_json)?;
    let response = WORKFLOW_SERVICE
        .resume_client_session(request)
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Create a durable frontend HTTP client bucket and return BucketRecord JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_create_client_bucket(
    request_json: String,
) -> Result<String, FfiError> {
    let request: BucketCreateRequest = workflow_parse_request(&request_json)?;
    let response = WORKFLOW_SERVICE
        .create_client_bucket(request)
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Delete a durable frontend HTTP client bucket and return BucketRecord JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_delete_client_bucket(
    request_json: String,
) -> Result<String, FfiError> {
    let request: BucketDeleteRequest = workflow_parse_request(&request_json)?;
    let response = WORKFLOW_SERVICE
        .delete_client_bucket(request)
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Execute an attributed frontend HTTP workflow run and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_run_attributed(
    base_url: String,
    request_json: String,
    pumas_api: Option<Arc<FfiPumasApi>>,
) -> Result<String, FfiError> {
    let request: WorkflowAttributedRunRequest = workflow_parse_request(&request_json)?;

    let host = build_frontend_http_host(base_url, pumas_api)?;
    let response = WORKFLOW_SERVICE
        .workflow_run_attributed(&host, request)
        .await
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Execute frontend HTTP workflow capabilities contract and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_get_capabilities(
    base_url: String,
    request_json: String,
    pumas_api: Option<Arc<FfiPumasApi>>,
) -> Result<String, FfiError> {
    let request: WorkflowCapabilitiesRequest = workflow_parse_request(&request_json)?;

    let host = build_frontend_http_host(base_url, pumas_api)?;
    let response = WORKFLOW_SERVICE
        .workflow_get_capabilities(&host, request)
        .await
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Execute frontend HTTP workflow preflight contract and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_preflight(
    base_url: String,
    request_json: String,
    pumas_api: Option<Arc<FfiPumasApi>>,
) -> Result<String, FfiError> {
    let request: WorkflowPreflightRequest = workflow_parse_request(&request_json)?;

    let host = build_frontend_http_host(base_url, pumas_api)?;
    let response = WORKFLOW_SERVICE
        .workflow_preflight(&host, request)
        .await
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Create scheduler-managed frontend HTTP workflow session and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_create_session(
    base_url: String,
    request_json: String,
    pumas_api: Option<Arc<FfiPumasApi>>,
) -> Result<String, FfiError> {
    let request: WorkflowSessionCreateRequest = workflow_parse_request(&request_json)?;

    let host = build_frontend_http_host(base_url, pumas_api)?;
    let response = WORKFLOW_SERVICE
        .create_workflow_session(&host, request)
        .await
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Run scheduler-managed frontend HTTP workflow session and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_run_session(
    base_url: String,
    request_json: String,
    pumas_api: Option<Arc<FfiPumasApi>>,
) -> Result<String, FfiError> {
    let request: WorkflowSessionRunRequest = workflow_parse_request(&request_json)?;

    let host = build_frontend_http_host(base_url, pumas_api)?;
    let response = WORKFLOW_SERVICE
        .run_workflow_session(&host, request)
        .await
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Close scheduler-managed workflow session and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_close_session(
    base_url: String,
    request_json: String,
    pumas_api: Option<Arc<FfiPumasApi>>,
) -> Result<String, FfiError> {
    let request: WorkflowSessionCloseRequest = workflow_parse_request(&request_json)?;

    let host = build_frontend_http_host(base_url, pumas_api)?;
    let response = WORKFLOW_SERVICE
        .close_workflow_session(&host, request)
        .await
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Get scheduler-managed workflow session status and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_get_session_status(
    request_json: String,
) -> Result<String, FfiError> {
    let request: WorkflowSessionStatusRequest = workflow_parse_request(&request_json)?;

    let response = WORKFLOW_SERVICE
        .workflow_get_session_status(request)
        .await
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// List scheduler-managed workflow session queue and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_list_session_queue(
    request_json: String,
) -> Result<String, FfiError> {
    let request: WorkflowSessionQueueListRequest = workflow_parse_request(&request_json)?;

    let response = WORKFLOW_SERVICE
        .workflow_list_session_queue(request)
        .await
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Cancel a queued workflow session run and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_cancel_session_queue_item(
    request_json: String,
) -> Result<String, FfiError> {
    let request: WorkflowSessionQueueCancelRequest = workflow_parse_request(&request_json)?;

    let response = WORKFLOW_SERVICE
        .workflow_cancel_session_queue_item(request)
        .await
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Reprioritize a queued workflow session run and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_reprioritize_session_queue_item(
    request_json: String,
) -> Result<String, FfiError> {
    let request: WorkflowSessionQueueReprioritizeRequest = workflow_parse_request(&request_json)?;

    let response = WORKFLOW_SERVICE
        .workflow_reprioritize_session_queue_item(request)
        .await
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}

/// Set scheduler-managed workflow session keep-alive state and return response JSON.
#[uniffi::export(async_runtime = "tokio")]
pub async fn frontend_http_workflow_set_session_keep_alive(
    base_url: String,
    request_json: String,
    pumas_api: Option<Arc<FfiPumasApi>>,
) -> Result<String, FfiError> {
    let request: WorkflowSessionKeepAliveRequest = workflow_parse_request(&request_json)?;

    let host = build_frontend_http_host(base_url, pumas_api)?;
    let response = WORKFLOW_SERVICE
        .workflow_set_session_keep_alive(&host, request)
        .await
        .map_err(map_workflow_service_error)?;

    workflow_serialize_response(&response)
}
