use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use inference::BackendConfig;
use node_engine::WorkflowGraph;
use pantograph_runtime_identity::canonical_engine_backend_key;
use pantograph_runtime_registry::{RuntimeReservationRequirements, RuntimeRetentionHint};
use pantograph_workflow_service::{
    WorkflowExecutionSessionRetentionHint, WorkflowExecutionSessionRuntimeSelectionTarget,
    WorkflowExecutionSessionRuntimeUnloadCandidate, WorkflowExecutionSessionState, WorkflowHost,
    WorkflowOutputTarget, WorkflowPortBinding, WorkflowRuntimeDiagnosticPhaseHint,
    WorkflowRuntimeRequirements, WorkflowServiceError,
};
use workflow_nodes::setup::{PumasSelectorAccess, PUMAS_SELECTOR_ACCESS};

use crate::{
    runtime_registry, runtime_registry_errors, task_executor, EmbeddedWorkflowHost,
    HostRuntimeModeSnapshot, RUNTIME_WARMUP_POLL_INTERVAL_MS, RUNTIME_WARMUP_WAIT_TIMEOUT_MS,
};

impl EmbeddedWorkflowHost {
    pub(crate) async fn pumas_api(&self) -> Option<Arc<pumas_library::PumasApi>> {
        let guard = self.extensions.read().await;
        guard
            .get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
            .cloned()
    }

    pub(crate) async fn pumas_selector_access(&self) -> Option<Arc<PumasSelectorAccess>> {
        let guard = self.extensions.read().await;
        guard
            .get::<Arc<PumasSelectorAccess>>(PUMAS_SELECTOR_ACCESS)
            .cloned()
    }

    pub(crate) fn observe_python_runtime_execution_metadata(
        &self,
        metadata: &[task_executor::PythonRuntimeExecutionMetadata],
    ) -> Result<(), WorkflowServiceError> {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(());
        };
        for metadata in metadata {
            runtime_registry::reconcile_runtime_registry_snapshot_override_with_health_assessment(
                runtime_registry.as_ref(),
                &metadata.snapshot,
                metadata.model_target.as_deref(),
                metadata.health_assessment.as_ref(),
            );
        }

        Ok(())
    }

    pub(crate) fn trimmed_optional(value: Option<&str>) -> Option<String> {
        value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    }

    pub(crate) fn reservation_requirements(
        runtime_requirements: &WorkflowRuntimeRequirements,
    ) -> Option<RuntimeReservationRequirements> {
        let requirements = RuntimeReservationRequirements {
            estimated_peak_vram_mb: runtime_requirements.estimated_peak_vram_mb,
            estimated_peak_ram_mb: runtime_requirements.estimated_peak_ram_mb,
            estimated_min_vram_mb: runtime_requirements.estimated_min_vram_mb,
            estimated_min_ram_mb: runtime_requirements.estimated_min_ram_mb,
        };

        if requirements.estimated_peak_vram_mb.is_none()
            && requirements.estimated_peak_ram_mb.is_none()
            && requirements.estimated_min_vram_mb.is_none()
            && requirements.estimated_min_ram_mb.is_none()
        {
            return None;
        }

        Some(requirements)
    }

    pub(crate) fn runtime_retention_hint(
        retention_hint: WorkflowExecutionSessionRetentionHint,
    ) -> RuntimeRetentionHint {
        match retention_hint {
            WorkflowExecutionSessionRetentionHint::Ephemeral => RuntimeRetentionHint::Ephemeral,
            WorkflowExecutionSessionRetentionHint::KeepAlive => RuntimeRetentionHint::KeepAlive,
        }
    }

    pub(crate) async fn ensure_workflow_runtime_ready_for_session_load(
        &self,
        workflow_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let capabilities = WorkflowHost::workflow_capabilities(self, workflow_id).await?;
        let (_, blocking_runtime_issues) = pantograph_workflow_service::evaluate_runtime_preflight(
            &capabilities.runtime_requirements.required_backends,
            &capabilities.runtime_capabilities,
        );

        if blocking_runtime_issues.is_empty() {
            return Ok(());
        }

        Err(WorkflowServiceError::RuntimeNotReady(
            pantograph_workflow_service::format_runtime_not_ready_message(&blocking_runtime_issues),
        ))
    }

    pub(crate) async fn ensure_workflow_inference_model_loaded(
        &self,
        workflow_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let Some(model_path) = self
            .resolve_llamacpp_workflow_model_path(workflow_id)
            .await?
        else {
            return Ok(());
        };

        if !self
            .llamacpp_gateway_matches_requested_model(&model_path)
            .await
        {
            let backend_key = canonical_engine_backend_key(Some(
                self.gateway.current_backend_name().await.as_str(),
            ));
            if backend_key.as_deref() != Some("llamacpp") {
                self.gateway
                    .switch_backend("llama.cpp")
                    .await
                    .map_err(|error| {
                        WorkflowServiceError::RuntimeNotReady(error.to_string())
                            .with_runtime_diagnostic_phase(
                                WorkflowRuntimeDiagnosticPhaseHint::RuntimeLaunch,
                            )
                    })?;
            }

            self.gateway
                .start(&BackendConfig {
                    model_path: Some(model_path.clone()),
                    device: Some("auto".to_string()),
                    gpu_layers: Some(-1),
                    embedding_mode: false,
                    reranking_mode: false,
                    ..BackendConfig::default()
                })
                .await
                .map_err(|error| {
                    let phase = runtime_start_diagnostic_phase(&error);
                    WorkflowServiceError::RuntimeNotReady(format!(
                        "failed to load llama.cpp model '{}': {error}",
                        model_path.display()
                    ))
                    .with_runtime_diagnostic_phase(phase)
                })?;
        }

        if self
            .llamacpp_gateway_matches_requested_model(&model_path)
            .await
        {
            Ok(())
        } else {
            Err(WorkflowServiceError::RuntimeNotReady(format!(
                "llama.cpp reported ready but active model does not match '{}'",
                model_path.display()
            ))
            .with_runtime_diagnostic_phase(WorkflowRuntimeDiagnosticPhaseHint::RuntimeModelLoad))
        }
    }

    async fn llamacpp_gateway_matches_requested_model(&self, model_path: &Path) -> bool {
        if !self.gateway.is_ready().await
            || self.gateway.is_embedding_mode().await
            || self.gateway.is_reranking_mode().await
        {
            return false;
        }

        let backend_key =
            canonical_engine_backend_key(Some(self.gateway.current_backend_name().await.as_str()));
        if backend_key.as_deref() != Some("llamacpp") {
            return false;
        }

        let Some(config) = self.gateway.restart_runtime_config().await else {
            return false;
        };
        if config.external_url.is_some() {
            return true;
        }
        let Some(active_model_path) = config.model_path.as_deref() else {
            return false;
        };
        paths_refer_to_same_file(active_model_path, model_path)
    }

    async fn resolve_llamacpp_workflow_model_path(
        &self,
        workflow_id: &str,
    ) -> Result<Option<PathBuf>, WorkflowServiceError> {
        let stored = pantograph_workflow_service::capabilities::load_and_validate_workflow(
            workflow_id,
            &self.workflow_roots,
        )?;
        let graph = stored.to_workflow_graph(workflow_id);
        let Some(llamacpp_node) = graph
            .nodes
            .iter()
            .find(|node| is_canonical_llamacpp_inference_node(&node.node_type, &node.data))
        else {
            return Ok(None);
        };

        if let Some(model_path) = model_path_from_node_data(&llamacpp_node.data) {
            return resolve_gguf_path(&model_path).map(Some);
        }

        let Some(model_edge) = graph.edges.iter().find(|edge| {
            edge.target == llamacpp_node.id
                && matches!(
                    edge.target_handle.as_str(),
                    "pumas_model_ref" | "model_path"
                )
        }) else {
            return Err(WorkflowServiceError::RuntimeNotReady(format!(
                "llama.cpp workflow '{}' has an inference node without a pumas_model_ref input",
                workflow_id
            )));
        };
        let Some(source_node) = graph.find_node(&model_edge.source) else {
            return Err(WorkflowServiceError::RuntimeNotReady(format!(
                "llama.cpp workflow '{}' references missing model source node '{}'",
                workflow_id, model_edge.source
            )));
        };

        let model_path = if source_node.node_type == "puma-lib" {
            self.resolve_puma_lib_node_model_path(&source_node.data)
                .await?
        } else {
            model_path_from_node_data(&source_node.data)
        };

        let Some(model_path) = model_path else {
            return Err(WorkflowServiceError::RuntimeNotReady(format!(
                "llama.cpp workflow '{}' could not resolve a model path from node '{}'",
                workflow_id, source_node.id
            )));
        };

        resolve_gguf_path(&model_path).map(Some)
    }

    async fn resolve_puma_lib_node_model_path(
        &self,
        data: &serde_json::Value,
    ) -> Result<Option<String>, WorkflowServiceError> {
        let mut model_path = model_path_from_node_data(data);
        let model_id = read_optional_string_aliases(data, &["model_id", "modelId"]);
        let Some(api) = self.pumas_api().await else {
            return Ok(model_path);
        };

        let model = if let Some(model_id) = model_id.as_deref() {
            api.get_model(model_id).await.map_err(|error| {
                WorkflowServiceError::RuntimeNotReady(format!(
                    "failed to query Puma-Lib model '{model_id}': {error}"
                ))
                .with_runtime_diagnostic_phase(WorkflowRuntimeDiagnosticPhaseHint::ModelDependency)
            })?
        } else {
            None
        };

        if let Some(model) = model {
            if !model.path.trim().is_empty() {
                model_path = Some(model.path.clone());
            }
            match api.resolve_model_execution_descriptor(&model.id).await {
                Ok(descriptor) if !descriptor.entry_path.trim().is_empty() => {
                    model_path = Some(descriptor.entry_path);
                }
                Ok(_) => {}
                Err(error) => {
                    log::warn!(
                        "Puma-Lib execution descriptor lookup failed during model preload for '{}': {}",
                        model.id,
                        error
                    );
                }
            }
        }

        Ok(model_path)
    }

    pub(crate) fn record_session_runtime_reservation(
        &self,
        session_id: &str,
        reservation_id: u64,
    ) -> Result<Option<u64>, WorkflowServiceError> {
        let mut reservations = self.session_runtime_reservations.lock().map_err(|_| {
            WorkflowServiceError::Internal("session runtime reservation lock poisoned".to_string())
        })?;

        Ok(reservations.insert(session_id.to_string(), reservation_id))
    }

    pub(crate) fn restore_session_runtime_reservation(
        &self,
        session_id: &str,
        previous_reservation_id: Option<u64>,
    ) -> Result<(), WorkflowServiceError> {
        let mut reservations = self.session_runtime_reservations.lock().map_err(|_| {
            WorkflowServiceError::Internal("session runtime reservation lock poisoned".to_string())
        })?;

        if let Some(previous_reservation_id) = previous_reservation_id {
            reservations.insert(session_id.to_string(), previous_reservation_id);
        } else {
            reservations.remove(session_id);
        }

        Ok(())
    }

    pub(crate) fn sync_loaded_session_runtime_retention_hint(
        &self,
        session_id: &str,
        keep_alive: bool,
        session_state: WorkflowExecutionSessionState,
    ) -> Result<(), WorkflowServiceError> {
        if session_state == WorkflowExecutionSessionState::IdleUnloaded {
            return Ok(());
        }

        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(());
        };

        let reservation_id = {
            let reservations = self.session_runtime_reservations.lock().map_err(|_| {
                WorkflowServiceError::Internal(
                    "session runtime reservation lock poisoned".to_string(),
                )
            })?;
            reservations.get(session_id).copied()
        };

        let Some(reservation_id) = reservation_id else {
            return Ok(());
        };

        runtime_registry::sync_runtime_reservation_retention_hint(
            runtime_registry.as_ref(),
            reservation_id,
            Self::runtime_retention_hint(if keep_alive {
                WorkflowExecutionSessionRetentionHint::KeepAlive
            } else {
                WorkflowExecutionSessionRetentionHint::Ephemeral
            }),
        )
        .map_err(runtime_registry_errors::workflow_service_error_from_runtime_registry)?;

        Ok(())
    }

    pub(crate) async fn consume_runtime_warmup_disposition(
        &self,
        runtime_registry: &pantograph_runtime_registry::RuntimeRegistry,
        runtime_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        runtime_registry::consume_active_runtime_warmup_disposition(
            self.gateway.as_ref(),
            runtime_registry,
            runtime_id,
            Duration::from_millis(RUNTIME_WARMUP_POLL_INTERVAL_MS),
            Duration::from_millis(RUNTIME_WARMUP_WAIT_TIMEOUT_MS),
        )
        .await
        .map_err(runtime_registry_errors::workflow_service_error_from_runtime_warmup_coordination)
    }

    pub(crate) async fn reserve_loaded_session_runtime(
        &self,
        session_id: &str,
        workflow_id: &str,
        usage_profile: Option<&str>,
        retention_hint: WorkflowExecutionSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(());
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
        let descriptor = runtime_registry::active_runtime_descriptor(&host_runtime_mode_info);
        let lease = runtime_registry
            .acquire_reservation(reservation_request)
            .map_err(runtime_registry_errors::workflow_service_error_from_runtime_registry)?;

        let previous_reservation_id =
            self.record_session_runtime_reservation(session_id, lease.reservation_id)?;
        if let Err(error) = self
            .consume_runtime_warmup_disposition(runtime_registry.as_ref(), &descriptor.runtime_id)
            .await
        {
            self.restore_session_runtime_reservation(session_id, previous_reservation_id)?;
            if previous_reservation_id != Some(lease.reservation_id) {
                runtime_registry::release_reservation_and_reconcile_runtime_registry(
                    self.gateway.as_ref(),
                    runtime_registry.as_ref(),
                    lease.reservation_id,
                )
                .await
                .map_err(runtime_registry_errors::workflow_service_error_from_runtime_registry)?;
            }
            return Err(error);
        }

        Ok(())
    }

    pub(crate) async fn release_loaded_session_runtime(
        &self,
        session_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(());
        };

        let reservation_id = {
            let mut reservations = self.session_runtime_reservations.lock().map_err(|_| {
                WorkflowServiceError::Internal(
                    "session runtime reservation lock poisoned".to_string(),
                )
            })?;
            reservations.remove(session_id)
        };

        if let Some(reservation_id) = reservation_id {
            runtime_registry::release_reservation_and_reconcile_runtime_registry(
                self.gateway.as_ref(),
                runtime_registry.as_ref(),
                reservation_id,
            )
            .await
            .map_err(runtime_registry_errors::workflow_service_error_from_runtime_registry)?;
        }

        Ok(())
    }

    pub(crate) fn apply_input_bindings(
        graph: &mut WorkflowGraph,
        inputs: &[WorkflowPortBinding],
    ) -> Result<(), WorkflowServiceError> {
        for binding in inputs {
            let node = graph
                .nodes
                .iter_mut()
                .find(|node| node.id == binding.node_id)
                .ok_or_else(|| {
                    WorkflowServiceError::InvalidRequest(format!(
                        "input binding references unknown node_id '{}'",
                        binding.node_id
                    ))
                })?;

            if node.data.is_null() {
                node.data = serde_json::json!({});
            }

            let map = node.data.as_object_mut().ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(format!(
                    "input node '{}' has non-object data payload",
                    binding.node_id
                ))
            })?;
            map.insert(binding.port_id.clone(), binding.value.clone());
        }

        Ok(())
    }

    pub(crate) fn resolve_output_node_ids(
        graph: &WorkflowGraph,
        output_targets: Option<&[WorkflowOutputTarget]>,
    ) -> Result<Vec<String>, WorkflowServiceError> {
        if let Some(targets) = output_targets {
            let known_nodes = graph
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<HashSet<_>>();
            let mut dedup = HashSet::new();
            let mut node_ids = Vec::new();

            for target in targets {
                if !known_nodes.contains(target.node_id.as_str()) {
                    return Err(WorkflowServiceError::InvalidRequest(format!(
                        "output target references unknown node_id '{}'",
                        target.node_id
                    )));
                }
                if dedup.insert(target.node_id.clone()) {
                    node_ids.push(target.node_id.clone());
                }
            }
            return Ok(node_ids);
        }

        let output_node_ids = graph
            .nodes
            .iter()
            .filter(|node| node.node_type.ends_with("-output"))
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        if output_node_ids.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow has no output nodes; add explicit `*-output` nodes or provide output_targets"
                    .to_string(),
            ));
        }

        Ok(output_node_ids)
    }

    pub(crate) fn collect_run_outputs(
        node_outputs: &HashMap<String, HashMap<String, serde_json::Value>>,
        output_node_ids: &[String],
        output_targets: Option<&[WorkflowOutputTarget]>,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if let Some(targets) = output_targets {
            let mut outputs = Vec::with_capacity(targets.len());
            for target in targets {
                let Some(value) = node_outputs
                    .get(&target.node_id)
                    .and_then(|ports| ports.get(&target.port_id))
                    .cloned()
                else {
                    continue;
                };

                outputs.push(WorkflowPortBinding {
                    node_id: target.node_id.clone(),
                    port_id: target.port_id.clone(),
                    value,
                });
            }
            return Ok(outputs);
        }

        let mut outputs = Vec::new();
        for node_id in output_node_ids {
            let Some(ports) = node_outputs.get(node_id) else {
                continue;
            };

            let mut keys = ports.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            for port_id in keys {
                if let Some(value) = ports.get(&port_id) {
                    outputs.push(WorkflowPortBinding {
                        node_id: node_id.clone(),
                        port_id,
                        value: value.clone(),
                    });
                }
            }
        }

        Ok(outputs)
    }

    pub(crate) fn apply_data_graph_inputs(
        graph: &mut WorkflowGraph,
        inputs: &HashMap<String, serde_json::Value>,
    ) {
        for (port_name, value) in inputs {
            for node in &mut graph.nodes {
                if node.node_type == "text-input" && (port_name == "text" || port_name == "input") {
                    if let Some(obj) = node.data.as_object_mut() {
                        obj.insert("text".to_string(), value.clone());
                    } else {
                        node.data = serde_json::json!({ "text": value });
                    }
                }

                if let Some(obj) = node.data.as_object_mut() {
                    obj.insert(format!("_input_{}", port_name), value.clone());
                }
            }
        }
    }

    pub(crate) fn terminal_data_graph_node_ids(graph: &WorkflowGraph) -> Vec<String> {
        graph
            .nodes
            .iter()
            .filter(|node| !graph.edges.iter().any(|edge| edge.source == node.id))
            .map(|node| node.id.clone())
            .collect()
    }

    pub(crate) fn collect_data_graph_outputs(
        graph_id: &str,
        terminal_nodes: &[String],
        node_outputs: &HashMap<String, HashMap<String, serde_json::Value>>,
    ) -> HashMap<String, serde_json::Value> {
        let mut outputs = HashMap::new();

        for terminal_id in terminal_nodes {
            let Some(terminal_outputs) = node_outputs.get(terminal_id) else {
                continue;
            };

            for (output_port, output_value) in terminal_outputs {
                outputs.insert(
                    format!("{}.{}", terminal_id, output_port),
                    output_value.clone(),
                );
                outputs.insert(output_port.clone(), output_value.clone());
            }
        }

        outputs.insert(
            "_graph_id".to_string(),
            serde_json::Value::String(graph_id.to_string()),
        );
        outputs.insert(
            "_terminal_nodes".to_string(),
            serde_json::Value::Array(
                terminal_nodes
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );

        outputs
    }

    pub(crate) fn fallback_runtime_unload_candidate(
        target: &WorkflowExecutionSessionRuntimeSelectionTarget,
        candidates: &[WorkflowExecutionSessionRuntimeUnloadCandidate],
    ) -> Option<WorkflowExecutionSessionRuntimeUnloadCandidate> {
        pantograph_workflow_service::select_runtime_unload_candidate_by_affinity(target, candidates)
    }
}

fn read_optional_string_aliases(data: &serde_json::Value, aliases: &[&str]) -> Option<String> {
    aliases.iter().find_map(|key| {
        data.get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn is_canonical_llamacpp_inference_node(node_type: &str, data: &serde_json::Value) -> bool {
    if node_type != "llm-inference" {
        return false;
    }

    read_inference_backend_hint(data).as_deref() == Some("llamacpp")
}

fn read_inference_backend_hint(data: &serde_json::Value) -> Option<String> {
    read_optional_string_aliases(
        data,
        &[
            "backend_key",
            "backendKey",
            "runtime_hint",
            "runtimeHint",
            "recommended_backend",
            "recommendedBackend",
        ],
    )
    .or_else(|| {
        data.get("pumas_model_ref").and_then(|model_ref| {
            read_optional_string_aliases(
                model_ref,
                &[
                    "backend_key",
                    "backendKey",
                    "recommended_backend",
                    "recommendedBackend",
                ],
            )
        })
    })
    .and_then(|value| canonical_engine_backend_key(Some(&value)))
}

fn model_path_from_node_data(data: &serde_json::Value) -> Option<String> {
    read_optional_string_aliases(data, &["model_path", "modelPath"]).or_else(|| {
        data.get("pumas_model_ref").and_then(|model_ref| {
            read_optional_string_aliases(
                model_ref,
                &[
                    "model_path",
                    "modelPath",
                    "selected_artifact_path",
                    "selectedArtifactPath",
                    "entry_path",
                    "entryPath",
                ],
            )
        })
    })
}

fn resolve_gguf_path(path: &str) -> Result<PathBuf, WorkflowServiceError> {
    let path = PathBuf::from(path);
    if !path.is_dir() {
        return Ok(path);
    }

    std::fs::read_dir(&path)
        .map_err(|error| {
            WorkflowServiceError::RuntimeNotReady(format!(
                "cannot read model directory '{}': {error}",
                path.display()
            ))
            .with_runtime_diagnostic_phase(WorkflowRuntimeDiagnosticPhaseHint::ModelDependency)
        })?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("gguf"))
        })
        .ok_or_else(|| {
            WorkflowServiceError::RuntimeNotReady(format!(
                "no .gguf file found in model directory '{}'",
                path.display()
            ))
            .with_runtime_diagnostic_phase(WorkflowRuntimeDiagnosticPhaseHint::ModelDependency)
        })
}

fn runtime_start_diagnostic_phase(
    error: &inference::GatewayError,
) -> WorkflowRuntimeDiagnosticPhaseHint {
    match error {
        inference::GatewayError::Backend(inference::BackendError::ManagedBinary(_)) => {
            WorkflowRuntimeDiagnosticPhaseHint::ManagedBinary
        }
        _ => WorkflowRuntimeDiagnosticPhaseHint::RuntimeLaunch,
    }
}

fn paths_refer_to_same_file(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}
