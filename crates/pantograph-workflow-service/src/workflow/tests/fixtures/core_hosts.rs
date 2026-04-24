use super::*;

pub(in crate::workflow::tests) struct MockWorkflowHost {
    pub(in crate::workflow::tests) capabilities: WorkflowHostCapabilities,
    pub(in crate::workflow::tests) omit_requested_target_output: bool,
    pub(in crate::workflow::tests) emit_invalid_output_binding: bool,
    pub(in crate::workflow::tests) technical_fit_decision: Option<WorkflowTechnicalFitDecision>,
    pub(in crate::workflow::tests) recorded_run_options: Arc<Mutex<Vec<WorkflowRunOptions>>>,
}

impl MockWorkflowHost {
    pub(in crate::workflow::tests) fn new(
        max_input_bindings: usize,
        max_value_bytes: usize,
    ) -> Self {
        Self {
            capabilities: WorkflowHostCapabilities {
                max_input_bindings,
                max_output_targets: 16,
                max_value_bytes,
                runtime_requirements: WorkflowRuntimeRequirements {
                    estimated_peak_vram_mb: Some(1024),
                    estimated_peak_ram_mb: Some(2048),
                    estimated_min_vram_mb: Some(512),
                    estimated_min_ram_mb: Some(1024),
                    estimation_confidence: "estimated".to_string(),
                    required_models: vec!["model-a".to_string()],
                    required_backends: vec!["llama_cpp".to_string()],
                    required_extensions: vec!["inference_gateway".to_string()],
                },
                models: vec![WorkflowCapabilityModel {
                    model_id: "model-a".to_string(),
                    model_revision_or_hash: Some("sha256:hash-model-a".to_string()),
                    model_type: Some("embedding".to_string()),
                    node_ids: vec!["node-1".to_string()],
                    roles: vec!["embedding".to_string(), "inference".to_string()],
                }],
                runtime_capabilities: vec![ready_runtime_capability()],
            },
            omit_requested_target_output: false,
            emit_invalid_output_binding: false,
            technical_fit_decision: None,
            recorded_run_options: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(in crate::workflow::tests) fn with_missing_requested_output(
        max_input_bindings: usize,
        max_value_bytes: usize,
    ) -> Self {
        Self {
            omit_requested_target_output: true,
            ..Self::new(max_input_bindings, max_value_bytes)
        }
    }

    pub(in crate::workflow::tests) fn with_invalid_output_binding(
        max_input_bindings: usize,
        max_value_bytes: usize,
    ) -> Self {
        Self {
            emit_invalid_output_binding: true,
            ..Self::new(max_input_bindings, max_value_bytes)
        }
    }

    pub(in crate::workflow::tests) fn with_technical_fit_decision(
        max_input_bindings: usize,
        max_value_bytes: usize,
        technical_fit_decision: WorkflowTechnicalFitDecision,
    ) -> Self {
        Self {
            technical_fit_decision: Some(technical_fit_decision),
            ..Self::new(max_input_bindings, max_value_bytes)
        }
    }
}

pub(in crate::workflow::tests) struct InspectionHost {
    pub(in crate::workflow::tests) calls: Arc<Mutex<Vec<(String, String)>>>,
    pub(in crate::workflow::tests) state: Option<WorkflowGraphSessionStateView>,
}

#[async_trait]
impl WorkflowHost for InspectionHost {
    async fn workflow_execution_session_inspection_state(
        &self,
        session_id: &str,
        workflow_id: &str,
    ) -> Result<Option<WorkflowGraphSessionStateView>, WorkflowServiceError> {
        self.calls
            .lock()
            .expect("inspection host calls lock poisoned")
            .push((session_id.to_string(), workflow_id.to_string()));
        Ok(self.state.clone())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        unreachable!("inspection host does not execute workflow runs")
    }
}

pub(in crate::workflow::tests) fn ready_runtime_capability() -> WorkflowRuntimeCapability {
    WorkflowRuntimeCapability {
        runtime_id: "llama_cpp".to_string(),
        display_name: "llama.cpp".to_string(),
        install_state: WorkflowRuntimeInstallState::Installed,
        available: true,
        configured: true,
        can_install: false,
        can_remove: true,
        source_kind: WorkflowRuntimeSourceKind::Managed,
        selected: true,
        readiness_state: Some(WorkflowRuntimeReadinessState::Ready),
        selected_version: Some("b8248".to_string()),
        supports_external_connection: true,
        backend_keys: vec!["llamacpp".to_string(), "llama.cpp".to_string()],
        missing_files: Vec::new(),
        unavailable_reason: None,
    }
}

pub(in crate::workflow::tests) fn ready_pytorch_runtime_capability() -> WorkflowRuntimeCapability {
    WorkflowRuntimeCapability {
        runtime_id: "pytorch".to_string(),
        display_name: "PyTorch".to_string(),
        install_state: WorkflowRuntimeInstallState::Installed,
        available: true,
        configured: true,
        can_install: false,
        can_remove: true,
        source_kind: WorkflowRuntimeSourceKind::Managed,
        selected: true,
        readiness_state: Some(WorkflowRuntimeReadinessState::Ready),
        selected_version: Some("2.6.0".to_string()),
        supports_external_connection: true,
        backend_keys: vec!["pytorch".to_string(), "torch".to_string()],
        missing_files: Vec::new(),
        unavailable_reason: None,
    }
}

#[async_trait]
impl WorkflowHost for MockWorkflowHost {
    fn max_input_bindings(&self) -> usize {
        self.capabilities.max_input_bindings
    }

    fn max_output_targets(&self) -> usize {
        self.capabilities.max_output_targets
    }

    fn max_value_bytes(&self) -> usize {
        self.capabilities.max_value_bytes
    }

    async fn validate_workflow(&self, workflow_id: &str) -> Result<(), WorkflowServiceError> {
        if workflow_id == "wf-missing" {
            return Err(WorkflowServiceError::WorkflowNotFound(
                workflow_id.to_string(),
            ));
        }
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("mock-graph".to_string())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(self.capabilities.clone())
    }

    async fn workflow_io(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        Ok(WorkflowIoResponse {
            inputs: vec![WorkflowIoNode {
                node_id: "text-input-1".to_string(),
                node_type: "text-input".to_string(),
                name: Some("Input".to_string()),
                description: None,
                ports: vec![WorkflowIoPort {
                    port_id: "text".to_string(),
                    name: Some("Text".to_string()),
                    description: None,
                    data_type: Some("string".to_string()),
                    required: Some(false),
                    multiple: Some(false),
                }],
            }],
            outputs: vec![WorkflowIoNode {
                node_id: "text-output-1".to_string(),
                node_type: "text-output".to_string(),
                name: Some("Output".to_string()),
                description: None,
                ports: vec![WorkflowIoPort {
                    port_id: "text".to_string(),
                    name: Some("Text".to_string()),
                    description: None,
                    data_type: Some("string".to_string()),
                    required: Some(false),
                    multiple: Some(false),
                }],
            }],
        })
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        Ok(self.capabilities.runtime_capabilities.clone())
    }

    async fn workflow_technical_fit_decision(
        &self,
        _request: &WorkflowTechnicalFitRequest,
    ) -> Result<Option<WorkflowTechnicalFitDecision>, WorkflowServiceError> {
        Ok(self.technical_fit_decision.clone())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        self.recorded_run_options
            .lock()
            .expect("run options lock poisoned")
            .push(run_options);

        if inputs.iter().any(|binding| {
            binding
                .value
                .as_str()
                .map(|value| value.contains("runtime-error"))
                .unwrap_or(false)
        }) {
            return Err(WorkflowServiceError::RuntimeNotReady(
                "backend not ready".to_string(),
            ));
        }

        if let Some(targets) = output_targets {
            if self.omit_requested_target_output && !targets.is_empty() {
                return Ok(Vec::new());
            }
            let mut outputs = Vec::with_capacity(targets.len());
            for target in targets {
                let value = inputs
                    .iter()
                    .find(|binding| {
                        binding.node_id == target.node_id && binding.port_id == target.port_id
                    })
                    .map(|binding| binding.value.clone())
                    .unwrap_or(serde_json::Value::Null);

                outputs.push(WorkflowPortBinding {
                    node_id: target.node_id.clone(),
                    port_id: target.port_id.clone(),
                    value,
                });
            }
            return Ok(outputs);
        }

        if self.emit_invalid_output_binding {
            return Ok(vec![WorkflowPortBinding {
                node_id: "text-output-1".to_string(),
                port_id: String::new(),
                value: serde_json::json!("invalid"),
            }]);
        }

        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("default output"),
        }])
    }
}
