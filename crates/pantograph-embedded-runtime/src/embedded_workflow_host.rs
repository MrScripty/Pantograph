use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use node_engine::{CoreTaskExecutor, NullEventSink, WorkflowExecutor};
use pantograph_runtime_identity::canonical_runtime_backend_key;
use pantograph_runtime_registry::RuntimeRegistryError;
use pantograph_workflow_service::capabilities;
use pantograph_workflow_service::graph::WorkflowGraphSessionStateView;
use pantograph_workflow_service::{
    WorkflowHost, WorkflowHostModelDescriptor, WorkflowOutputTarget, WorkflowPortBinding,
    WorkflowRunOptions, WorkflowRuntimeCapability, WorkflowServiceError,
    WorkflowSessionRetentionHint, WorkflowSessionRuntimeSelectionTarget,
    WorkflowSessionRuntimeUnloadCandidate, WorkflowTechnicalFitDecision,
    WorkflowTechnicalFitRequest,
};
use uuid::Uuid;

use crate::{
    EmbeddedWorkflowHost, HostRuntimeModeSnapshot, RuntimeExtensionsSnapshot,
    apply_runtime_extensions_for_execution, python_runtime, runtime_capabilities, runtime_registry,
    runtime_registry_errors, task_executor, technical_fit, workflow_session_execution,
};

#[async_trait::async_trait]
impl WorkflowHost for EmbeddedWorkflowHost {
    fn workflow_roots(&self) -> Vec<PathBuf> {
        self.workflow_roots.clone()
    }

    fn max_input_bindings(&self) -> usize {
        capabilities::DEFAULT_MAX_INPUT_BINDINGS
    }

    fn max_output_targets(&self) -> usize {
        capabilities::DEFAULT_MAX_OUTPUT_TARGETS
    }

    fn max_value_bytes(&self) -> usize {
        capabilities::DEFAULT_MAX_VALUE_BYTES
    }

    async fn default_backend_name(&self) -> Result<String, WorkflowServiceError> {
        Ok(canonical_runtime_backend_key(
            &self.gateway.current_backend_name().await,
        ))
    }

    async fn model_metadata(
        &self,
        model_id: &str,
    ) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
        let Some(api) = self.pumas_api().await else {
            return Ok(None);
        };

        let model = api
            .get_model(model_id)
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?;
        Ok(model.map(|m| m.metadata))
    }

    async fn model_descriptor(
        &self,
        model_id: &str,
    ) -> Result<Option<WorkflowHostModelDescriptor>, WorkflowServiceError> {
        let Some(api) = self.pumas_api().await else {
            return Ok(None);
        };

        let model = api
            .get_model(model_id)
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?;
        Ok(model.map(|m| WorkflowHostModelDescriptor {
            model_type: Some(m.model_type.trim().to_string()).filter(|v| !v.is_empty()),
            hashes: m.hashes,
        }))
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        let selected_backend_key =
            canonical_runtime_backend_key(&self.gateway.current_backend_name().await);
        let available_backends = self.gateway.available_backends();
        let managed_runtimes = inference::list_managed_runtime_snapshots(&self.app_data_dir)
            .map_err(WorkflowServiceError::RuntimeNotReady)?;
        let mut runtimes = runtime_capabilities::managed_runtime_capabilities(
            &managed_runtimes,
            &available_backends,
            &selected_backend_key,
        );
        runtimes.extend(runtime_capabilities::host_runtime_capabilities(
            &available_backends,
            &selected_backend_key,
        ));
        runtimes.extend(runtime_capabilities::python_runtime_capabilities(
            python_runtime::resolve_python_executable_for_env_ids(&[]),
            &selected_backend_key,
        ));
        runtimes.extend(self.additional_runtime_capabilities.clone());
        runtimes.sort_by(|left, right| left.runtime_id.cmp(&right.runtime_id));
        Ok(runtimes)
    }

    async fn can_load_session_runtime(
        &self,
        session_id: &str,
        workflow_id: &str,
        usage_profile: Option<&str>,
        retention_hint: WorkflowSessionRetentionHint,
    ) -> Result<bool, WorkflowServiceError> {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(true);
        };

        let mode_info = self.gateway.mode_info().await;
        let host_runtime_mode_info = HostRuntimeModeSnapshot::from_mode_info(&mode_info);
        let requirements = Self::reservation_requirements(
            &WorkflowHost::workflow_capabilities(self, workflow_id)
                .await?
                .runtime_requirements,
        );
        let trimmed_usage_profile = Self::trimmed_optional(usage_profile);
        let reservation_request = runtime_registry::active_runtime_reservation_request(
            runtime_registry,
            &host_runtime_mode_info,
            workflow_id,
            Some(session_id),
            trimmed_usage_profile.as_deref(),
            requirements,
            Self::runtime_retention_hint(retention_hint),
        );

        match runtime_registry.can_acquire_reservation(&reservation_request) {
            Ok(()) => Ok(true),
            Err(RuntimeRegistryError::AdmissionRejected { .. })
            | Err(RuntimeRegistryError::ReservationRejected(_)) => Ok(false),
            Err(error) => {
                Err(runtime_registry_errors::workflow_service_error_from_runtime_registry(error))
            }
        }
    }

    async fn load_session_runtime(
        &self,
        session_id: &str,
        workflow_id: &str,
        usage_profile: Option<&str>,
        retention_hint: WorkflowSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        self.ensure_workflow_runtime_ready_for_session_load(workflow_id)
            .await?;
        self.reserve_loaded_session_runtime(session_id, workflow_id, usage_profile, retention_hint)
            .await
    }

    async fn unload_session_runtime(
        &self,
        session_id: &str,
        _workflow_id: &str,
        reason: pantograph_workflow_service::WorkflowSessionUnloadReason,
    ) -> Result<(), WorkflowServiceError> {
        self.release_loaded_session_runtime(session_id).await?;
        workflow_session_execution::apply_session_execution_unload_transition(
            &self.session_executions,
            session_id,
            reason,
        )
        .await
    }

    async fn select_runtime_unload_candidate(
        &self,
        target: &WorkflowSessionRuntimeSelectionTarget,
        candidates: &[WorkflowSessionRuntimeUnloadCandidate],
    ) -> Result<Option<WorkflowSessionRuntimeUnloadCandidate>, WorkflowServiceError> {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(Self::fallback_runtime_unload_candidate(target, candidates));
        };

        if let Some((session_id, _runtime_id)) =
            runtime_registry::runtime_registry_reclaim_candidate_for_sessions(
                runtime_registry,
                candidates,
            )
        {
            if let Some(candidate) = candidates
                .iter()
                .find(|candidate| candidate.session_id == session_id)
            {
                return Ok(Some(candidate.clone()));
            }
        }

        Ok(Self::fallback_runtime_unload_candidate(target, candidates))
    }

    async fn workflow_technical_fit_decision(
        &self,
        request: &WorkflowTechnicalFitRequest,
    ) -> Result<Option<WorkflowTechnicalFitDecision>, WorkflowServiceError> {
        technical_fit::workflow_technical_fit_decision(self, request).await
    }

    async fn workflow_session_inspection_state(
        &self,
        session_id: &str,
        workflow_id: &str,
    ) -> Result<Option<WorkflowGraphSessionStateView>, WorkflowServiceError> {
        let Some(entry) = self.session_executions.get(session_id)? else {
            return Ok(None);
        };
        if entry.workflow_id != workflow_id {
            return Ok(None);
        }

        let executor = entry.executor.lock().await;
        let residency = executor.workflow_session_residency().await;
        let node_memory = executor
            .workflow_session_node_memory_snapshots(session_id)
            .await;
        let checkpoint = Some(
            executor
                .workflow_session_checkpoint_summary(session_id)
                .await,
        );
        Ok(Some(WorkflowGraphSessionStateView::new(
            residency,
            node_memory,
            None,
            checkpoint,
        )))
    }

    async fn run_workflow(
        &self,
        workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        run_options: WorkflowRunOptions,
        run_handle: pantograph_workflow_service::WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if let Some(workflow_session_id) = run_options.workflow_session_id.as_deref() {
            return workflow_session_execution::run_session_workflow(
                self,
                workflow_id,
                workflow_session_id,
                inputs,
                output_targets,
                run_handle,
            )
            .await;
        }

        if run_handle.is_cancelled() {
            return Err(WorkflowServiceError::Cancelled(
                "workflow run cancelled before execution started".to_string(),
            ));
        }

        let stored = capabilities::load_and_validate_workflow(workflow_id, &self.workflow_roots)?;
        let mut graph = stored.to_workflow_graph(workflow_id);
        Self::apply_input_bindings(&mut graph, inputs)?;

        let output_node_ids = Self::resolve_output_node_ids(&graph, output_targets)?;
        let runtime_ext = RuntimeExtensionsSnapshot::from_shared(&self.extensions).await;

        let execution_id = Uuid::new_v4().to_string();
        let core = Arc::new(
            CoreTaskExecutor::new()
                .with_project_root(self.project_root.clone())
                .with_gateway(self.gateway.clone())
                .with_execution_id(execution_id.clone()),
        );
        let host = Arc::new(task_executor::TauriTaskExecutor::with_python_runtime(
            self.rag_backend.clone(),
            self.python_runtime.clone(),
        ));
        let task_executor = node_engine::CompositeTaskExecutor::new(
            Some(host as Arc<dyn node_engine::TaskExecutor>),
            core,
        );
        let python_runtime_execution_recorder =
            Arc::new(task_executor::PythonRuntimeExecutionRecorder::default());

        let mut executor =
            WorkflowExecutor::new(execution_id.clone(), graph, Arc::new(NullEventSink));
        apply_runtime_extensions_for_execution(
            &mut executor,
            &runtime_ext,
            None,
            Some(execution_id.clone()),
            Some(python_runtime_execution_recorder.clone()),
        );

        let mut node_outputs = HashMap::new();
        let mut run_result = Ok(());
        for node_id in &output_node_ids {
            if run_handle.is_cancelled() {
                run_result = Err(WorkflowServiceError::Cancelled(
                    "workflow run cancelled during execution".to_string(),
                ));
                break;
            }
            match executor.demand(node_id, &task_executor).await {
                Ok(outputs) => {
                    node_outputs.insert(node_id.clone(), outputs);
                }
                Err(error) => {
                    run_result = Err(match error {
                        node_engine::NodeEngineError::WaitingForInput { task_id, .. } => {
                            WorkflowServiceError::InvalidRequest(format!(
                                "workflow '{}' requires interactive input at node '{}'",
                                workflow_id, task_id
                            ))
                        }
                        other => WorkflowServiceError::Internal(other.to_string()),
                    });
                    break;
                }
            }
        }

        let python_runtime_execution_metadata = python_runtime_execution_recorder.snapshots();
        self.observe_python_runtime_execution_metadata(&python_runtime_execution_metadata)?;

        run_result?;
        Self::collect_run_outputs(&node_outputs, &output_node_ids, output_targets)
    }
}
