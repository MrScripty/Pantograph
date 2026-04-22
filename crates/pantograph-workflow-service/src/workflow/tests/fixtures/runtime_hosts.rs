use super::*;

pub(in crate::workflow::tests) struct SelectingRuntimeHost {
    pub(in crate::workflow::tests) selected_session_id: String,
    pub(in crate::workflow::tests) unloads: Arc<Mutex<Vec<(String, WorkflowSessionUnloadReason)>>>,
    pub(in crate::workflow::tests) capabilities: WorkflowHostCapabilities,
}

impl SelectingRuntimeHost {
    pub(in crate::workflow::tests) fn new(
        selected_session_id: String,
        unloads: Arc<Mutex<Vec<(String, WorkflowSessionUnloadReason)>>>,
    ) -> Self {
        Self {
            selected_session_id,
            unloads,
            capabilities: WorkflowHostCapabilities {
                max_input_bindings: 16,
                max_output_targets: 16,
                max_value_bytes: 4096,
                runtime_requirements: WorkflowRuntimeRequirements::default(),
                models: Vec::new(),
                runtime_capabilities: vec![ready_runtime_capability()],
            },
        }
    }
}

pub(in crate::workflow::tests) struct AffinityRuntimeHost {
    pub(in crate::workflow::tests) unloads: Arc<Mutex<Vec<String>>>,
    pub(in crate::workflow::tests) capabilities: WorkflowHostCapabilities,
    pub(in crate::workflow::tests) required_backends_by_workflow: HashMap<String, Vec<String>>,
    pub(in crate::workflow::tests) required_models_by_workflow: HashMap<String, Vec<String>>,
}

impl AffinityRuntimeHost {
    pub(in crate::workflow::tests) fn new(unloads: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            unloads,
            capabilities: WorkflowHostCapabilities {
                max_input_bindings: 16,
                max_output_targets: 16,
                max_value_bytes: 4096,
                runtime_requirements: WorkflowRuntimeRequirements::default(),
                models: Vec::new(),
                runtime_capabilities: vec![
                    ready_runtime_capability(),
                    ready_pytorch_runtime_capability(),
                ],
            },
            required_backends_by_workflow: HashMap::new(),
            required_models_by_workflow: HashMap::new(),
        }
    }

    pub(in crate::workflow::tests) fn with_runtime_affinity(
        unloads: Arc<Mutex<Vec<String>>>,
        required_backends_by_workflow: HashMap<String, Vec<String>>,
        required_models_by_workflow: HashMap<String, Vec<String>>,
    ) -> Self {
        Self {
            unloads,
            capabilities: WorkflowHostCapabilities {
                max_input_bindings: 16,
                max_output_targets: 16,
                max_value_bytes: 4096,
                runtime_requirements: WorkflowRuntimeRequirements::default(),
                models: Vec::new(),
                runtime_capabilities: vec![
                    ready_runtime_capability(),
                    ready_pytorch_runtime_capability(),
                ],
            },
            required_backends_by_workflow,
            required_models_by_workflow,
        }
    }
}

#[async_trait]
impl WorkflowHost for SelectingRuntimeHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("selection-graph".to_string())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(self.capabilities.clone())
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        Ok(self.capabilities.runtime_capabilities.clone())
    }

    async fn select_runtime_unload_candidate(
        &self,
        _target: &WorkflowSessionRuntimeSelectionTarget,
        candidates: &[WorkflowSessionRuntimeUnloadCandidate],
    ) -> Result<Option<WorkflowSessionRuntimeUnloadCandidate>, WorkflowServiceError> {
        Ok(candidates
            .iter()
            .find(|candidate| candidate.session_id == self.selected_session_id)
            .cloned())
    }

    async fn unload_session_runtime(
        &self,
        session_id: &str,
        _workflow_id: &str,
        reason: WorkflowSessionUnloadReason,
    ) -> Result<(), WorkflowServiceError> {
        self.unloads
            .lock()
            .expect("unloads lock poisoned")
            .push((session_id.to_string(), reason));
        Ok(())
    }

    async fn load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        _retention_hint: WorkflowSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("ok"),
        }])
    }
}

#[async_trait]
impl WorkflowHost for AffinityRuntimeHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("affinity-graph".to_string())
    }

    async fn workflow_capabilities(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        let mut capabilities = self.capabilities.clone();
        capabilities.runtime_requirements.required_backends = self
            .required_backends_by_workflow
            .get(workflow_id)
            .cloned()
            .unwrap_or_default();
        capabilities.runtime_requirements.required_models = self
            .required_models_by_workflow
            .get(workflow_id)
            .cloned()
            .unwrap_or_default();
        Ok(capabilities)
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        Ok(self.capabilities.runtime_capabilities.clone())
    }

    async fn unload_session_runtime(
        &self,
        session_id: &str,
        _workflow_id: &str,
        _reason: WorkflowSessionUnloadReason,
    ) -> Result<(), WorkflowServiceError> {
        self.unloads
            .lock()
            .expect("unloads lock poisoned")
            .push(session_id.to_string());
        Ok(())
    }

    async fn load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        _retention_hint: WorkflowSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("ok"),
        }])
    }
}
