use async_trait::async_trait;
use pantograph_runtime_identity::canonical_runtime_backend_key;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::capabilities;
use crate::graph::WorkflowGraphSessionStateView;
use crate::scheduler::{
    select_runtime_unload_candidate_by_affinity, WorkflowExecutionSessionRetentionHint,
    WorkflowExecutionSessionRuntimeSelectionTarget, WorkflowExecutionSessionRuntimeUnloadCandidate,
    WorkflowExecutionSessionUnloadReason, WorkflowSchedulerRuntimeRegistryDiagnostics,
};
use crate::technical_fit::{WorkflowTechnicalFitDecision, WorkflowTechnicalFitRequest};

use super::io_contract::derive_workflow_io;
use super::{
    WorkflowCapabilityModel, WorkflowHostCapabilities, WorkflowHostModelDescriptor,
    WorkflowIoResponse, WorkflowOutputTarget, WorkflowPortBinding, WorkflowRunHandle,
    WorkflowRunOptions, WorkflowRuntimeCapability, WorkflowRuntimeRequirements,
    WorkflowServiceError,
};

/// Trait boundary for host/runtime concerns needed by workflow service.
#[async_trait]
pub trait WorkflowHost: Send + Sync {
    /// Candidate roots that may contain `.pantograph/workflows/<workflow_id>.json`.
    fn workflow_roots(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    /// Upper bound for request input bindings.
    fn max_input_bindings(&self) -> usize {
        capabilities::DEFAULT_MAX_INPUT_BINDINGS
    }

    /// Upper bound for explicit output target count.
    fn max_output_targets(&self) -> usize {
        capabilities::DEFAULT_MAX_OUTPUT_TARGETS
    }

    /// Upper bound for each bound value payload, in bytes.
    fn max_value_bytes(&self) -> usize {
        capabilities::DEFAULT_MAX_VALUE_BYTES
    }

    /// Backend identifier used when workflow data does not declare one.
    async fn default_backend_name(&self) -> Result<String, WorkflowServiceError> {
        Ok("unknown".to_string())
    }

    /// Optional model metadata for runtime requirement estimation.
    async fn model_metadata(
        &self,
        _model_id: &str,
    ) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
        Ok(None)
    }

    /// Optional model descriptor for capability inventory.
    async fn model_descriptor(
        &self,
        _model_id: &str,
    ) -> Result<Option<WorkflowHostModelDescriptor>, WorkflowServiceError> {
        Ok(None)
    }

    /// Report runtime capability state for the current host.
    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        Ok(Vec::new())
    }

    /// Resolve workflow identity and fail if it is unknown to the host.
    async fn validate_workflow(&self, workflow_id: &str) -> Result<(), WorkflowServiceError> {
        capabilities::load_and_validate_workflow(workflow_id, &self.workflow_roots()).map(|_| ())
    }

    /// Return the current graph fingerprint for session-scoped preflight caching.
    async fn workflow_graph_fingerprint(
        &self,
        workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        capabilities::workflow_graph_fingerprint(workflow_id, &self.workflow_roots())
    }

    /// Return capability limits and model support metadata.
    async fn workflow_capabilities(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        let stored = capabilities::load_and_validate_workflow(workflow_id, &self.workflow_roots())?;
        let required_models = capabilities::extract_required_models(stored.nodes());
        let mut required_backends = capabilities::extract_required_backends(stored.nodes());
        if required_backends.is_empty() {
            required_backends.push(canonical_runtime_backend_key(
                &self.default_backend_name().await?,
            ));
        }
        required_backends.sort();
        required_backends.dedup();

        let required_extensions = capabilities::extract_required_extensions(
            stored.nodes(),
            stored.edges(),
            !required_models.is_empty(),
        );
        let mut model_metadata = HashMap::new();
        for model_id in &required_models {
            if let Some(metadata) = self.model_metadata(model_id).await? {
                model_metadata.insert(model_id.clone(), metadata);
            }
        }

        let (
            estimated_peak_vram_mb,
            estimated_peak_ram_mb,
            estimated_min_vram_mb,
            estimated_min_ram_mb,
            estimation_confidence,
        ) = capabilities::estimate_memory_requirements(&required_models, &model_metadata);
        let model_usages = capabilities::extract_model_usages(stored.nodes());
        let mut models = Vec::with_capacity(model_usages.len());
        for usage in model_usages {
            let descriptor = self.model_descriptor(&usage.model_id).await?;
            let model_revision_or_hash = descriptor
                .as_ref()
                .and_then(|record| capabilities::select_preferred_hash(&record.hashes));
            let model_type = descriptor.and_then(|record| {
                record
                    .model_type
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
            });

            models.push(WorkflowCapabilityModel {
                model_id: usage.model_id,
                model_revision_or_hash,
                model_type,
                node_ids: usage.node_ids,
                roles: usage.roles,
            });
        }

        Ok(WorkflowHostCapabilities {
            max_input_bindings: self.max_input_bindings(),
            max_output_targets: self.max_output_targets(),
            max_value_bytes: self.max_value_bytes(),
            runtime_requirements: WorkflowRuntimeRequirements {
                estimated_peak_vram_mb,
                estimated_peak_ram_mb,
                estimated_min_vram_mb,
                estimated_min_ram_mb,
                estimation_confidence,
                required_models,
                required_backends,
                required_extensions,
            },
            models,
            runtime_capabilities: self.runtime_capabilities().await?,
        })
    }

    /// Discover externally bindable input and output nodes for a workflow.
    async fn workflow_io(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        let stored = capabilities::load_and_validate_workflow(workflow_id, &self.workflow_roots())?;
        derive_workflow_io(stored.nodes())
    }

    /// Execute one workflow run and return output port bindings.
    async fn run_workflow(
        &self,
        workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        run_options: WorkflowRunOptions,
        run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError>;

    /// Load session runtime resources (model runtime, worker state) when needed.
    async fn can_load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        _retention_hint: WorkflowExecutionSessionRetentionHint,
    ) -> Result<bool, WorkflowServiceError> {
        Ok(true)
    }

    async fn load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        _retention_hint: WorkflowExecutionSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    /// Unload session runtime resources when scheduler rebalances or session closes.
    async fn unload_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _reason: WorkflowExecutionSessionUnloadReason,
    ) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn select_runtime_unload_candidate(
        &self,
        target: &WorkflowExecutionSessionRuntimeSelectionTarget,
        candidates: &[WorkflowExecutionSessionRuntimeUnloadCandidate],
    ) -> Result<Option<WorkflowExecutionSessionRuntimeUnloadCandidate>, WorkflowServiceError> {
        Ok(select_runtime_unload_candidate_by_affinity(
            target, candidates,
        ))
    }

    async fn workflow_technical_fit_decision(
        &self,
        _request: &WorkflowTechnicalFitRequest,
    ) -> Result<Option<WorkflowTechnicalFitDecision>, WorkflowServiceError> {
        Ok(None)
    }

    /// Optional backend-owned live workflow execution session inspection surface for
    /// node memory, checkpoint, and residency state.
    async fn workflow_execution_session_inspection_state(
        &self,
        _session_id: &str,
        _workflow_id: &str,
    ) -> Result<Option<WorkflowGraphSessionStateView>, WorkflowServiceError> {
        Ok(None)
    }
}

/// Backend-owned request for additive scheduler diagnostics that depend on a
/// runtime-registry or another host-specific runtime fact source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSchedulerRuntimeDiagnosticsRequest {
    pub session_id: String,
    pub workflow_id: String,
    pub usage_profile: Option<String>,
    pub keep_alive: bool,
    pub runtime_loaded: bool,
    pub next_admission_queue_id: Option<String>,
    pub reclaim_candidates: Vec<WorkflowExecutionSessionRuntimeUnloadCandidate>,
}

/// Optional backend provider for additive scheduler diagnostics that require
/// host/runtime state outside the canonical queue store.
#[async_trait]
pub trait WorkflowSchedulerDiagnosticsProvider: Send + Sync {
    async fn scheduler_runtime_registry_diagnostics(
        &self,
        _request: &WorkflowSchedulerRuntimeDiagnosticsRequest,
    ) -> Result<Option<WorkflowSchedulerRuntimeRegistryDiagnostics>, WorkflowServiceError> {
        Ok(None)
    }
}
