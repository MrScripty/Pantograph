use std::time::{Duration, Instant};

use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use uuid::Uuid;

use crate::graph::WorkflowGraphRunSettings;
use crate::scheduler::WorkflowExecutionSessionPreflightCache;
use crate::technical_fit::WorkflowTechnicalFitOverride;

use super::artifact_output_conversion::convert_media_outputs_to_artifacts;
use super::diagnostic_errors::{
    WorkflowDiagnosticArtifactScope, WorkflowDiagnosticErrorRecordRequest,
    WorkflowDiagnosticNodeScope, WorkflowDiagnosticRunContext, WorkflowDiagnosticRunScope,
};
use super::io_contract::validate_workflow_io;
use super::runtime_preflight::format_runtime_not_ready_message;
use super::validation::{
    validate_bindings, validate_host_output_bindings, validate_output_targets,
    validate_output_targets_against_io, validate_payload_size, validate_requested_outputs_produced,
    validate_timeout_ms, validate_workflow_id, validate_workflow_semantic_version,
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
        workflow_run_id: Option<String>,
        graph_run_settings: Option<WorkflowGraphRunSettings>,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        validate_workflow_semantic_version(&request.workflow_semantic_version)?;
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

        let workflow_run_id_value = workflow_run_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let diagnostic_run_context =
            workflow_run_diagnostic_context(&request, &workflow_run_id_value)?;
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
                result = &mut run_future => match result {
                    Ok(outputs) => outputs,
                    Err(error) => {
                        self.record_workflow_diagnostic_error_if_configured(
                            WorkflowDiagnosticErrorRecordRequest::node_execution_failed(
                                workflow_node_diagnostic_scope(&diagnostic_run_context, &request),
                                &error,
                            )
                            .with_source_instance_id("workflow-run-host"),
                        )?;
                        return Err(error);
                    }
                },
                _ = &mut timeout => {
                    run_handle.cancel();
                    let cancel_grace = tokio::time::sleep(Duration::from_millis(WORKFLOW_CANCEL_GRACE_WINDOW_MS));
                    tokio::pin!(cancel_grace);
                    tokio::select! {
                        _ = &mut run_future => {}
                        _ = &mut cancel_grace => {}
                    }
                    let error = WorkflowServiceError::RuntimeTimeout(format!(
                        "workflow run exceeded timeout_ms {}",
                        timeout_ms
                    ));
                    self.record_workflow_diagnostic_error_if_configured(
                        WorkflowDiagnosticErrorRecordRequest::run_timeout(
                            WorkflowDiagnosticRunScope {
                                run: diagnostic_run_context.clone(),
                            },
                            &error,
                        )
                        .with_source_instance_id("workflow-run-host"),
                    )?;
                    return Err(error);
                }
            }
        } else {
            match run_future.await {
                Ok(outputs) => outputs,
                Err(error) => {
                    self.record_workflow_diagnostic_error_if_configured(
                        WorkflowDiagnosticErrorRecordRequest::node_execution_failed(
                            workflow_node_diagnostic_scope(&diagnostic_run_context, &request),
                            &error,
                        )
                        .with_source_instance_id("workflow-run-host"),
                    )?;
                    return Err(error);
                }
            }
        };

        if let Some(targets) = request.output_targets.as_ref() {
            if let Err(error) = validate_requested_outputs_produced(targets, &outputs) {
                self.record_workflow_diagnostic_error_if_configured(
                    WorkflowDiagnosticErrorRecordRequest::output_validation_failed(
                        workflow_node_diagnostic_scope(&diagnostic_run_context, &request),
                        &error,
                    )
                    .with_source_instance_id("workflow-run-host"),
                )?;
                return Err(error);
            }
        } else if outputs.is_empty() {
            let error = WorkflowServiceError::Internal(
                "workflow execution returned zero outputs".to_string(),
            );
            self.record_workflow_diagnostic_error_if_configured(
                WorkflowDiagnosticErrorRecordRequest::output_validation_failed(
                    workflow_node_diagnostic_scope(&diagnostic_run_context, &request),
                    &error,
                )
                .with_source_instance_id("workflow-run-host"),
            )?;
            return Err(error);
        }

        if let Err(error) = validate_host_output_bindings(&outputs, "outputs") {
            self.record_workflow_diagnostic_error_if_configured(
                WorkflowDiagnosticErrorRecordRequest::output_validation_failed(
                    workflow_node_diagnostic_scope(&diagnostic_run_context, &request),
                    &error,
                )
                .with_source_instance_id("workflow-run-host"),
            )?;
            return Err(error);
        }
        let outputs = match convert_media_outputs_to_artifacts(
            self,
            &request.workflow_id,
            &request.workflow_semantic_version,
            &workflow_run_id_value,
            graph_run_settings.as_ref(),
            outputs,
        )
        .await
        {
            Ok(outputs) => outputs,
            Err(error) => {
                self.record_workflow_diagnostic_error_if_configured(
                    WorkflowDiagnosticErrorRecordRequest::artifact_failed(
                        WorkflowDiagnosticArtifactScope {
                            run: diagnostic_run_context.clone(),
                            node_id: workflow_diagnostic_node_id(&request),
                            payload_ref: None,
                        },
                        &error,
                    )
                    .with_source_instance_id("workflow-run-host"),
                )?;
                return Err(error);
            }
        };
        for binding in &outputs {
            validate_payload_size(binding, max_value_bytes)?;
        }

        Ok(WorkflowRunResponse {
            workflow_run_id: workflow_run_id_value,
            outputs,
            timing_ms: started.elapsed().as_millis(),
        })
    }
}

fn workflow_run_diagnostic_context(
    request: &WorkflowRunRequest,
    workflow_run_id: &str,
) -> Result<WorkflowDiagnosticRunContext, WorkflowServiceError> {
    Ok(WorkflowDiagnosticRunContext {
        workflow_run_id: WorkflowRunId::try_from(workflow_run_id.to_string())?,
        workflow_id: WorkflowId::try_from(request.workflow_id.clone())?,
        workflow_version_id: None,
        workflow_semantic_version: Some(request.workflow_semantic_version.clone()),
        client_id: None,
        client_session_id: None,
        bucket_id: None,
        scheduler_policy_id: None,
        retention_policy_id: None,
    })
}

fn workflow_node_diagnostic_scope(
    run: &WorkflowDiagnosticRunContext,
    request: &WorkflowRunRequest,
) -> WorkflowDiagnosticNodeScope {
    WorkflowDiagnosticNodeScope {
        run: run.clone(),
        node_id: workflow_diagnostic_node_id(request).unwrap_or_else(|| "workflow".to_string()),
        node_type: None,
        node_version: None,
        runtime_id: request
            .override_selection
            .as_ref()
            .and_then(|selection| selection.backend_key.clone()),
        model_id: None,
    }
}

fn workflow_diagnostic_node_id(request: &WorkflowRunRequest) -> Option<String> {
    request
        .output_targets
        .as_ref()
        .and_then(|targets| targets.first())
        .map(|target| target.node_id.clone())
        .or_else(|| {
            request
                .inputs
                .first()
                .map(|binding| binding.node_id.clone())
        })
}
