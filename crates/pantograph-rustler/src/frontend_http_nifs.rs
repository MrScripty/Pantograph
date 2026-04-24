use std::sync::LazyLock;

use pantograph_workflow_service::{
    BucketCreateRequest, BucketDeleteRequest, ClientRegistrationRequest, ClientSessionOpenRequest,
    ClientSessionResumeRequest, WorkflowAttributedRunRequest, WorkflowCapabilitiesRequest,
    WorkflowPreflightRequest, WorkflowRunRequest, WorkflowService,
};
use rustler::{NifResult, ResourceArc};

use crate::resources::PumasApiResource;
use crate::workflow_host_contract::{
    build_frontend_http_host, map_workflow_service_error, workflow_parse_request,
    workflow_run_host_request, workflow_runtime, workflow_serialize_response,
};

static WORKFLOW_SERVICE: LazyLock<WorkflowService> = LazyLock::new(|| {
    WorkflowService::with_ephemeral_attribution_store()
        .expect("frontend HTTP attribution store should initialize")
});

pub(crate) fn workflow_run(
    base_url: String,
    request_json: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
) -> NifResult<String> {
    workflow_run_host_request::<WorkflowRunRequest, _, _>(
        base_url,
        request_json,
        pumas_resource,
        |host, request| async move { WORKFLOW_SERVICE.workflow_run(&host, request).await },
    )
}

pub(crate) fn workflow_get_capabilities(
    base_url: String,
    request_json: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
) -> NifResult<String> {
    workflow_run_host_request::<WorkflowCapabilitiesRequest, _, _>(
        base_url,
        request_json,
        pumas_resource,
        |host, request| async move {
            WORKFLOW_SERVICE
                .workflow_get_capabilities(&host, request)
                .await
        },
    )
}

pub(crate) fn workflow_preflight(
    base_url: String,
    request_json: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
) -> NifResult<String> {
    workflow_run_host_request::<WorkflowPreflightRequest, _, _>(
        base_url,
        request_json,
        pumas_resource,
        |host, request| async move { WORKFLOW_SERVICE.workflow_preflight(&host, request).await },
    )
}

pub(crate) fn workflow_register_attribution_client(request_json: String) -> NifResult<String> {
    let request: ClientRegistrationRequest = workflow_parse_request(&request_json)?;
    let response = WORKFLOW_SERVICE
        .register_attribution_client(request)
        .map_err(map_workflow_service_error)?;
    workflow_serialize_response(&response)
}

pub(crate) fn workflow_open_client_session(request_json: String) -> NifResult<String> {
    let request: ClientSessionOpenRequest = workflow_parse_request(&request_json)?;
    let response = WORKFLOW_SERVICE
        .open_client_session(request)
        .map_err(map_workflow_service_error)?;
    workflow_serialize_response(&response)
}

pub(crate) fn workflow_resume_client_session(request_json: String) -> NifResult<String> {
    let request: ClientSessionResumeRequest = workflow_parse_request(&request_json)?;
    let response = WORKFLOW_SERVICE
        .resume_client_session(request)
        .map_err(map_workflow_service_error)?;
    workflow_serialize_response(&response)
}

pub(crate) fn workflow_create_client_bucket(request_json: String) -> NifResult<String> {
    let request: BucketCreateRequest = workflow_parse_request(&request_json)?;
    let response = WORKFLOW_SERVICE
        .create_client_bucket(request)
        .map_err(map_workflow_service_error)?;
    workflow_serialize_response(&response)
}

pub(crate) fn workflow_delete_client_bucket(request_json: String) -> NifResult<String> {
    let request: BucketDeleteRequest = workflow_parse_request(&request_json)?;
    let response = WORKFLOW_SERVICE
        .delete_client_bucket(request)
        .map_err(map_workflow_service_error)?;
    workflow_serialize_response(&response)
}

pub(crate) fn workflow_run_attributed(
    base_url: String,
    request_json: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
) -> NifResult<String> {
    let request: WorkflowAttributedRunRequest = workflow_parse_request(&request_json)?;
    let runtime = workflow_runtime()?;
    let host = build_frontend_http_host(base_url, pumas_resource)?;
    let response = runtime
        .block_on(WORKFLOW_SERVICE.workflow_run_attributed(&host, request))
        .map_err(map_workflow_service_error)?;
    workflow_serialize_response(&response)
}
