use std::path::Path;

use rustler::{NifResult, ResourceArc};

use pantograph_frontend_http_adapter::FrontendHttpWorkflowHost;
use pantograph_workflow_service::{WorkflowErrorCode, WorkflowErrorEnvelope, WorkflowServiceError};

use crate::PumasApiResource;

pub(crate) fn map_workflow_service_error(err: WorkflowServiceError) -> rustler::Error {
    rustler::Error::Term(Box::new(err.to_envelope_json()))
}

pub(crate) fn workflow_error_json(code: WorkflowErrorCode, message: impl Into<String>) -> String {
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

pub(crate) fn workflow_error_term(
    code: WorkflowErrorCode,
    message: impl Into<String>,
) -> rustler::Error {
    rustler::Error::Term(Box::new(workflow_error_json(code, message)))
}

pub(crate) fn workflow_runtime() -> NifResult<tokio::runtime::Runtime> {
    tokio::runtime::Runtime::new().map_err(|e| {
        workflow_error_term(
            WorkflowErrorCode::InternalError,
            format!("runtime initialization error: {}", e),
        )
    })
}

pub(crate) fn workflow_serialize_response<T: serde::Serialize>(value: &T) -> NifResult<String> {
    serde_json::to_string(value).map_err(|e| {
        workflow_error_term(
            WorkflowErrorCode::InternalError,
            format!("response serialization error: {}", e),
        )
    })
}

pub(crate) fn workflow_parse_request<T: serde::de::DeserializeOwned>(
    request_json: &str,
) -> NifResult<T> {
    serde_json::from_str(request_json).map_err(|e| {
        workflow_error_term(
            WorkflowErrorCode::InvalidRequest,
            format!("invalid request: {}", e),
        )
    })
}

pub(crate) fn build_frontend_http_host(
    base_url: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
) -> NifResult<FrontendHttpWorkflowHost> {
    FrontendHttpWorkflowHost::with_defaults(
        base_url,
        pumas_resource.as_ref().map(|resource| resource.api.clone()),
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
    .map_err(|e| {
        workflow_error_term(
            WorkflowErrorCode::InvalidRequest,
            format!("frontend HTTP host config error: {}", e),
        )
    })
}

pub(crate) fn workflow_run_host_request<Request, Response, Fut>(
    base_url: String,
    request_json: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
    execute: impl FnOnce(FrontendHttpWorkflowHost, Request) -> Fut,
) -> NifResult<String>
where
    Request: serde::de::DeserializeOwned,
    Response: serde::Serialize,
    Fut: std::future::Future<Output = Result<Response, WorkflowServiceError>>,
{
    let request: Request = workflow_parse_request(&request_json)?;
    let runtime = workflow_runtime()?;
    let host = build_frontend_http_host(base_url, pumas_resource)?;
    let response = runtime
        .block_on(execute(host, request))
        .map_err(map_workflow_service_error)?;
    workflow_serialize_response(&response)
}

#[cfg(test)]
mod tests {
    use pantograph_workflow_service::{WorkflowErrorCode, WorkflowErrorEnvelope};

    use super::workflow_error_json;

    #[test]
    fn preserves_cancelled_envelope() {
        let json = workflow_error_json(WorkflowErrorCode::Cancelled, "workflow run cancelled");
        let envelope: WorkflowErrorEnvelope =
            serde_json::from_str(&json).expect("parse cancelled envelope");

        assert_eq!(envelope.code, WorkflowErrorCode::Cancelled);
        assert_eq!(envelope.message, "workflow run cancelled");
    }

    #[test]
    fn preserves_invalid_request_envelope() {
        let json = workflow_error_json(
            WorkflowErrorCode::InvalidRequest,
            "workflow 'interactive-human-input' requires interactive input at node 'human-input-1'",
        );
        let envelope: WorkflowErrorEnvelope =
            serde_json::from_str(&json).expect("parse invalid-request envelope");

        assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
        assert_eq!(
            envelope.message,
            "workflow 'interactive-human-input' requires interactive input at node 'human-input-1'"
        );
    }
}
