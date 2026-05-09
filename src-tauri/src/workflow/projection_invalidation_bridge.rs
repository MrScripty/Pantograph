use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};

use pantograph_workflow_service::{
    WorkflowDiagnosticsProjectionKind, WorkflowDiagnosticsProjectionRefreshReason,
    WorkflowDiagnosticsProjectionRefreshRequest, WorkflowDiagnosticsProjectionRefreshResponse,
    WorkflowDiagnosticsProjectionRefreshSink, WorkflowService,
};
use tauri::AppHandle;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::app_tasks::AppTaskRegistry;

const REFRESH_COALESCE_WINDOW: Duration = Duration::from_millis(25);

pub struct WorkflowDiagnosticsProjectionInvalidationBridge {
    sender: UnboundedSender<WorkflowDiagnosticsProjectionRefreshRequest>,
}

impl WorkflowDiagnosticsProjectionInvalidationBridge {
    pub fn start(
        app: AppHandle,
        workflow_service: Arc<WorkflowService>,
        app_task_registry: &AppTaskRegistry,
    ) -> Arc<Self> {
        let (sender, receiver) = unbounded_channel();
        let task = tauri::async_runtime::spawn(run_projection_refresh_worker(
            app,
            workflow_service,
            receiver,
        ));
        app_task_registry.track("workflow-diagnostics-projection-invalidation-bridge", task);
        Arc::new(Self { sender })
    }
}

impl WorkflowDiagnosticsProjectionRefreshSink for WorkflowDiagnosticsProjectionInvalidationBridge {
    fn request_projection_refresh(&self, request: WorkflowDiagnosticsProjectionRefreshRequest) {
        if self.sender.send(request).is_err() {
            log::warn!("diagnostics projection refresh bridge is not running");
        }
    }
}

async fn run_projection_refresh_worker(
    app: AppHandle,
    workflow_service: Arc<WorkflowService>,
    mut receiver: UnboundedReceiver<WorkflowDiagnosticsProjectionRefreshRequest>,
) {
    while let Some(first_request) = receiver.recv().await {
        tokio::time::sleep(REFRESH_COALESCE_WINDOW).await;

        let mut requests = vec![first_request];
        while let Ok(request) = receiver.try_recv() {
            requests.push(request);
        }

        for request in coalesce_projection_refresh_requests(requests) {
            let workflow_service = workflow_service.clone();
            let refresh_result = tokio::task::spawn_blocking(move || {
                workflow_service.workflow_diagnostics_projection_refresh(request)
            })
            .await;

            match refresh_result {
                Ok(Ok(response)) => emit_refresh_response(&app, response),
                Ok(Err(error)) => {
                    log::warn!("diagnostics projection refresh failed: {error}");
                }
                Err(error) => {
                    log::warn!("diagnostics projection refresh worker failed: {error}");
                }
            }
        }
    }
}

fn emit_refresh_response(app: &AppHandle, response: WorkflowDiagnosticsProjectionRefreshResponse) {
    for failure in response.failed {
        log::warn!(
            "diagnostics projection '{:?}' refresh failed: {}",
            failure.projection_kind,
            failure.error
        );
    }

    if let Err(error) = super::projection_invalidation_transport::emit_projection_invalidations(
        app,
        &response.invalidations,
    ) {
        log::warn!("failed to emit diagnostics projection invalidations: {error}");
    }
}

fn coalesce_projection_refresh_requests(
    requests: impl IntoIterator<Item = WorkflowDiagnosticsProjectionRefreshRequest>,
) -> Vec<WorkflowDiagnosticsProjectionRefreshRequest> {
    let mut by_scope = BTreeMap::new();
    for request in requests {
        let key = (
            request.workflow_run_id.clone(),
            request.workflow_id.clone(),
            request.batch_size,
        );
        by_scope
            .entry(key)
            .and_modify(|pending: &mut PendingRefreshRequest| pending.merge(&request))
            .or_insert_with(|| PendingRefreshRequest::from_request(request));
    }
    by_scope
        .into_values()
        .map(PendingRefreshRequest::into_request)
        .collect()
}

struct PendingRefreshRequest {
    projections: BTreeSet<WorkflowDiagnosticsProjectionKind>,
    workflow_run_id: Option<String>,
    workflow_id: Option<String>,
    reason: WorkflowDiagnosticsProjectionRefreshReason,
    batch_size: u32,
}

impl PendingRefreshRequest {
    fn from_request(request: WorkflowDiagnosticsProjectionRefreshRequest) -> Self {
        Self {
            projections: request.projections.into_iter().collect(),
            workflow_run_id: request.workflow_run_id,
            workflow_id: request.workflow_id,
            reason: request.reason,
            batch_size: request.batch_size,
        }
    }

    fn merge(&mut self, request: &WorkflowDiagnosticsProjectionRefreshRequest) {
        self.projections.extend(request.projections.iter().copied());
        self.reason = request.reason;
    }

    fn into_request(self) -> WorkflowDiagnosticsProjectionRefreshRequest {
        WorkflowDiagnosticsProjectionRefreshRequest {
            projections: self.projections.into_iter().collect(),
            workflow_run_id: self.workflow_run_id,
            workflow_id: self.workflow_id,
            reason: self.reason,
            batch_size: self.batch_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(
        projections: Vec<WorkflowDiagnosticsProjectionKind>,
        workflow_run_id: Option<&str>,
        workflow_id: Option<&str>,
    ) -> WorkflowDiagnosticsProjectionRefreshRequest {
        WorkflowDiagnosticsProjectionRefreshRequest {
            projections,
            workflow_run_id: workflow_run_id.map(ToOwned::to_owned),
            workflow_id: workflow_id.map(ToOwned::to_owned),
            reason: WorkflowDiagnosticsProjectionRefreshReason::DiagnosticEventAppended,
            batch_size: 500,
        }
    }

    #[test]
    fn coalesce_projection_refresh_requests_merges_same_scope() {
        let requests = coalesce_projection_refresh_requests([
            request(
                vec![WorkflowDiagnosticsProjectionKind::RunDetail],
                Some("run-a"),
                Some("wf-a"),
            ),
            request(
                vec![WorkflowDiagnosticsProjectionKind::NodeStatus],
                Some("run-a"),
                Some("wf-a"),
            ),
            request(
                vec![WorkflowDiagnosticsProjectionKind::RunList],
                Some("run-b"),
                Some("wf-a"),
            ),
        ]);

        assert_eq!(requests.len(), 2);
        let run_a = requests
            .iter()
            .find(|request| request.workflow_run_id.as_deref() == Some("run-a"))
            .expect("run-a request");
        assert_eq!(
            run_a.projections,
            vec![
                WorkflowDiagnosticsProjectionKind::RunDetail,
                WorkflowDiagnosticsProjectionKind::NodeStatus
            ]
        );
        let run_b = requests
            .iter()
            .find(|request| request.workflow_run_id.as_deref() == Some("run-b"))
            .expect("run-b request");
        assert_eq!(
            run_b.projections,
            vec![WorkflowDiagnosticsProjectionKind::RunList]
        );
    }
}
