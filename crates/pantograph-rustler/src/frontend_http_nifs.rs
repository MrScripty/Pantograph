use std::sync::LazyLock;

use pantograph_workflow_service::{
    WorkflowCapabilitiesRequest, WorkflowPreflightRequest, WorkflowRunRequest, WorkflowService,
    WorkflowSessionCloseRequest, WorkflowSessionCreateRequest, WorkflowSessionKeepAliveRequest,
    WorkflowSessionQueueCancelRequest, WorkflowSessionQueueListRequest,
    WorkflowSessionQueueReprioritizeRequest, WorkflowSessionRunRequest,
    WorkflowSessionStatusRequest,
};
use rustler::{NifResult, ResourceArc};

use crate::resources::PumasApiResource;
use crate::workflow_host_contract::{workflow_run_host_request, workflow_run_scheduler_request};

static WORKFLOW_SERVICE: LazyLock<WorkflowService> = LazyLock::new(WorkflowService::new);

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

pub(crate) fn workflow_create_session(
    base_url: String,
    request_json: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
) -> NifResult<String> {
    workflow_run_host_request::<WorkflowSessionCreateRequest, _, _>(
        base_url,
        request_json,
        pumas_resource,
        |host, request| async move {
            WORKFLOW_SERVICE
                .create_workflow_session(&host, request)
                .await
        },
    )
}

pub(crate) fn workflow_run_session(
    base_url: String,
    request_json: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
) -> NifResult<String> {
    workflow_run_host_request::<WorkflowSessionRunRequest, _, _>(
        base_url,
        request_json,
        pumas_resource,
        |host, request| async move { WORKFLOW_SERVICE.run_workflow_session(&host, request).await },
    )
}

pub(crate) fn workflow_close_session(
    base_url: String,
    request_json: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
) -> NifResult<String> {
    workflow_run_host_request::<WorkflowSessionCloseRequest, _, _>(
        base_url,
        request_json,
        pumas_resource,
        |host, request| async move {
            WORKFLOW_SERVICE
                .close_workflow_session(&host, request)
                .await
        },
    )
}

pub(crate) fn workflow_get_session_status(request_json: String) -> NifResult<String> {
    workflow_run_scheduler_request::<WorkflowSessionStatusRequest, _, _>(
        request_json,
        |request| async move { WORKFLOW_SERVICE.workflow_get_session_status(request).await },
    )
}

pub(crate) fn workflow_list_session_queue(request_json: String) -> NifResult<String> {
    workflow_run_scheduler_request::<WorkflowSessionQueueListRequest, _, _>(
        request_json,
        |request| async move { WORKFLOW_SERVICE.workflow_list_session_queue(request).await },
    )
}

pub(crate) fn workflow_cancel_session_queue_item(request_json: String) -> NifResult<String> {
    workflow_run_scheduler_request::<WorkflowSessionQueueCancelRequest, _, _>(
        request_json,
        |request| async move {
            WORKFLOW_SERVICE
                .workflow_cancel_session_queue_item(request)
                .await
        },
    )
}

pub(crate) fn workflow_reprioritize_session_queue_item(request_json: String) -> NifResult<String> {
    workflow_run_scheduler_request::<WorkflowSessionQueueReprioritizeRequest, _, _>(
        request_json,
        |request| async move {
            WORKFLOW_SERVICE
                .workflow_reprioritize_session_queue_item(request)
                .await
        },
    )
}

pub(crate) fn workflow_set_session_keep_alive(
    base_url: String,
    request_json: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
) -> NifResult<String> {
    workflow_run_host_request::<WorkflowSessionKeepAliveRequest, _, _>(
        base_url,
        request_json,
        pumas_resource,
        |host, request| async move {
            WORKFLOW_SERVICE
                .workflow_set_session_keep_alive(&host, request)
                .await
        },
    )
}
