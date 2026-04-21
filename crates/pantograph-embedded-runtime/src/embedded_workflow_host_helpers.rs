use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use node_engine::WorkflowGraph;
use pantograph_runtime_registry::{RuntimeReservationRequirements, RuntimeRetentionHint};
use pantograph_workflow_service::{
    WorkflowHost, WorkflowOutputTarget, WorkflowPortBinding, WorkflowRuntimeRequirements,
    WorkflowServiceError, WorkflowSessionRetentionHint, WorkflowSessionRuntimeSelectionTarget,
    WorkflowSessionRuntimeUnloadCandidate, WorkflowSessionState,
};

use crate::{
    EmbeddedWorkflowHost, HostRuntimeModeSnapshot, RUNTIME_WARMUP_POLL_INTERVAL_MS,
    RUNTIME_WARMUP_WAIT_TIMEOUT_MS, runtime_registry, runtime_registry_errors, task_executor,
};

impl EmbeddedWorkflowHost {
    pub(crate) async fn pumas_api(&self) -> Option<Arc<pumas_library::PumasApi>> {
        let guard = self.extensions.read().await;
        guard
            .get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
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
        retention_hint: WorkflowSessionRetentionHint,
    ) -> RuntimeRetentionHint {
        match retention_hint {
            WorkflowSessionRetentionHint::Ephemeral => RuntimeRetentionHint::Ephemeral,
            WorkflowSessionRetentionHint::KeepAlive => RuntimeRetentionHint::KeepAlive,
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
        session_state: WorkflowSessionState,
    ) -> Result<(), WorkflowServiceError> {
        if session_state == WorkflowSessionState::IdleUnloaded {
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
                WorkflowSessionRetentionHint::KeepAlive
            } else {
                WorkflowSessionRetentionHint::Ephemeral
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
        retention_hint: WorkflowSessionRetentionHint,
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
        target: &WorkflowSessionRuntimeSelectionTarget,
        candidates: &[WorkflowSessionRuntimeUnloadCandidate],
    ) -> Option<WorkflowSessionRuntimeUnloadCandidate> {
        pantograph_workflow_service::select_runtime_unload_candidate_by_affinity(target, candidates)
    }
}
