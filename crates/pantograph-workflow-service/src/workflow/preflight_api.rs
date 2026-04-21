use std::collections::HashSet;

use crate::technical_fit::WorkflowTechnicalFitOverride;

use super::io_contract::validate_workflow_io;
use super::runtime_preflight::collect_preflight_warnings;
use super::validation::{
    collect_invalid_output_targets, derive_required_external_inputs, validate_bindings,
    validate_output_targets, validate_payload_size, validate_workflow_id,
};
use super::{
    WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse, WorkflowHost, WorkflowIoRequest,
    WorkflowIoResponse, WorkflowPreflightRequest, WorkflowPreflightResponse, WorkflowService,
    WorkflowServiceError,
};

impl WorkflowService {
    pub async fn workflow_get_capabilities<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowCapabilitiesRequest,
    ) -> Result<WorkflowCapabilitiesResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        host.validate_workflow(&request.workflow_id).await?;
        let capabilities = host.workflow_capabilities(&request.workflow_id).await?;
        Ok(WorkflowCapabilitiesResponse {
            max_input_bindings: capabilities.max_input_bindings,
            max_output_targets: capabilities.max_output_targets,
            max_value_bytes: capabilities.max_value_bytes,
            runtime_requirements: capabilities.runtime_requirements,
            models: capabilities.models,
            runtime_capabilities: capabilities.runtime_capabilities,
        })
    }

    pub async fn workflow_get_io<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowIoRequest,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        host.validate_workflow(&request.workflow_id).await?;
        let io = host.workflow_io(&request.workflow_id).await?;
        validate_workflow_io(&io)?;
        Ok(io)
    }

    pub async fn workflow_preflight<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowPreflightRequest,
    ) -> Result<WorkflowPreflightResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        validate_bindings(&request.inputs, "inputs")?;
        if let Some(targets) = request.output_targets.as_ref() {
            validate_output_targets(targets)?;
        }

        host.validate_workflow(&request.workflow_id).await?;
        let capabilities = host.workflow_capabilities(&request.workflow_id).await?;
        let graph_fingerprint = host
            .workflow_graph_fingerprint(&request.workflow_id)
            .await?;
        if request.inputs.len() > capabilities.max_input_bindings {
            return Err(WorkflowServiceError::CapabilityViolation(format!(
                "input binding count {} exceeds max_input_bindings {}",
                request.inputs.len(),
                capabilities.max_input_bindings
            )));
        }
        if let Some(targets) = request.output_targets.as_ref() {
            if targets.len() > capabilities.max_output_targets {
                return Err(WorkflowServiceError::CapabilityViolation(format!(
                    "output target count {} exceeds max_output_targets {}",
                    targets.len(),
                    capabilities.max_output_targets
                )));
            }
        }
        for binding in &request.inputs {
            validate_payload_size(binding, capabilities.max_value_bytes)?;
        }

        let io = host.workflow_io(&request.workflow_id).await?;
        validate_workflow_io(&io)?;

        let supplied_inputs = request
            .inputs
            .iter()
            .map(|binding| (binding.node_id.as_str(), binding.port_id.as_str()))
            .collect::<HashSet<_>>();
        let required_inputs = derive_required_external_inputs(&io);
        let mut missing_required_inputs = required_inputs
            .iter()
            .filter(|required| {
                !supplied_inputs.contains(&(required.node_id.as_str(), required.port_id.as_str()))
            })
            .cloned()
            .collect::<Vec<_>>();
        missing_required_inputs.sort_by(|a, b| {
            a.node_id
                .cmp(&b.node_id)
                .then_with(|| a.port_id.cmp(&b.port_id))
        });

        let invalid_targets = request
            .output_targets
            .as_deref()
            .map(|targets| collect_invalid_output_targets(targets, &io))
            .unwrap_or_default();

        let runtime_preflight = self
            .workflow_runtime_preflight_assessment(
                host,
                &request.workflow_id,
                &capabilities,
                request
                    .override_selection
                    .as_ref()
                    .and_then(WorkflowTechnicalFitOverride::normalized),
            )
            .await?;
        let warnings = collect_preflight_warnings(
            &io,
            &runtime_preflight.runtime_warnings,
            &runtime_preflight.blocking_runtime_issues,
        );
        let can_run = missing_required_inputs.is_empty()
            && invalid_targets.is_empty()
            && runtime_preflight.blocking_runtime_issues.is_empty();

        Ok(WorkflowPreflightResponse {
            missing_required_inputs,
            invalid_targets,
            warnings,
            graph_fingerprint,
            technical_fit_decision: runtime_preflight.technical_fit_decision,
            runtime_warnings: runtime_preflight.runtime_warnings,
            blocking_runtime_issues: runtime_preflight.blocking_runtime_issues,
            can_run,
        })
    }
}
