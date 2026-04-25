use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::scheduler::WorkflowExecutionSessionPreflightCache;
use crate::technical_fit::WorkflowTechnicalFitOverride;

use super::io_contract::validate_workflow_io;
use super::runtime_preflight::format_runtime_not_ready_message;
use super::validation::{
    validate_bindings, validate_host_output_bindings, validate_output_targets,
    validate_output_targets_against_io, validate_payload_size, validate_requested_outputs_produced,
    validate_timeout_ms, validate_workflow_id,
};
use super::{
    WorkflowHost, WorkflowRunHandle, WorkflowRunOptions, WorkflowRunRequest, WorkflowRunResponse,
    WorkflowService, WorkflowServiceError,
};

const WORKFLOW_CANCEL_GRACE_WINDOW_MS: u64 = 250;

impl WorkflowService {
    pub(super) async fn workflow_run_internal<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowRunRequest,
        cached_preflight: Option<WorkflowExecutionSessionPreflightCache>,
        workflow_execution_session_id: Option<String>,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        validate_timeout_ms(request.timeout_ms)?;
        validate_bindings(&request.inputs, "inputs")?;
        if let Some(targets) = request.output_targets.as_ref() {
            validate_output_targets(targets)?;
        }
        let override_selection = request
            .override_selection
            .as_ref()
            .and_then(WorkflowTechnicalFitOverride::normalized);

        let max_input_bindings = host.max_input_bindings();
        let max_output_targets = host.max_output_targets();
        let max_value_bytes = host.max_value_bytes();

        host.validate_workflow(&request.workflow_id).await?;
        if let Some(targets) = request.output_targets.as_ref() {
            let io = host.workflow_io(&request.workflow_id).await?;
            validate_workflow_io(&io)?;
            validate_output_targets_against_io(targets, &io)?;
        }
        let blocking_runtime_issues = if let Some(cache) = cached_preflight.as_ref() {
            cache.blocking_runtime_issues.clone()
        } else {
            let capabilities = host.workflow_capabilities(&request.workflow_id).await?;
            self.workflow_runtime_preflight_assessment(
                host,
                &request.workflow_id,
                &capabilities,
                override_selection,
            )
            .await?
            .blocking_runtime_issues
        };

        if !blocking_runtime_issues.is_empty() {
            return Err(WorkflowServiceError::RuntimeNotReady(
                format_runtime_not_ready_message(&blocking_runtime_issues),
            ));
        }

        if request.inputs.len() > max_input_bindings {
            return Err(WorkflowServiceError::CapabilityViolation(format!(
                "input binding count {} exceeds max_input_bindings {}",
                request.inputs.len(),
                max_input_bindings
            )));
        }

        if let Some(targets) = request.output_targets.as_ref() {
            if targets.len() > max_output_targets {
                return Err(WorkflowServiceError::CapabilityViolation(format!(
                    "output target count {} exceeds max_output_targets {}",
                    targets.len(),
                    max_output_targets
                )));
            }
        }

        for binding in &request.inputs {
            validate_payload_size(binding, max_value_bytes)?;
        }

        let started = Instant::now();
        let run_options = WorkflowRunOptions {
            timeout_ms: request.timeout_ms,
            workflow_execution_session_id,
        };
        let run_handle = WorkflowRunHandle::new();
        let mut run_future = Box::pin(host.run_workflow(
            &request.workflow_id,
            &request.inputs,
            request.output_targets.as_deref(),
            run_options,
            run_handle.clone(),
        ));
        let outputs = if let Some(timeout_ms) = request.timeout_ms {
            let timeout = tokio::time::sleep(Duration::from_millis(timeout_ms));
            tokio::pin!(timeout);
            tokio::select! {
                result = &mut run_future => result?,
                _ = &mut timeout => {
                    run_handle.cancel();
                    let cancel_grace = tokio::time::sleep(Duration::from_millis(WORKFLOW_CANCEL_GRACE_WINDOW_MS));
                    tokio::pin!(cancel_grace);
                    tokio::select! {
                        _ = &mut run_future => {}
                        _ = &mut cancel_grace => {}
                    }
                    return Err(WorkflowServiceError::RuntimeTimeout(format!(
                        "workflow run exceeded timeout_ms {}",
                        timeout_ms
                    )));
                }
            }
        } else {
            run_future.await?
        };

        if let Some(targets) = request.output_targets.as_ref() {
            validate_requested_outputs_produced(targets, &outputs)?;
        } else if outputs.is_empty() {
            return Err(WorkflowServiceError::Internal(
                "workflow execution returned zero outputs".to_string(),
            ));
        }

        validate_host_output_bindings(&outputs, "outputs")?;
        for binding in &outputs {
            validate_payload_size(binding, max_value_bytes)?;
        }

        let run_id = request
            .run_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        Ok(WorkflowRunResponse {
            run_id,
            outputs,
            timing_ms: started.elapsed().as_millis(),
        })
    }
}
