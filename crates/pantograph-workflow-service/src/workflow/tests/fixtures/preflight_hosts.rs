use super::*;

pub(in crate::workflow::tests) struct PreflightHost {
    pub(in crate::workflow::tests) capabilities: WorkflowHostCapabilities,
    pub(in crate::workflow::tests) technical_fit_decision: Option<WorkflowTechnicalFitDecision>,
}

impl PreflightHost {
    pub(in crate::workflow::tests) fn new() -> Self {
        Self {
            capabilities: WorkflowHostCapabilities {
                max_input_bindings: 16,
                max_output_targets: 16,
                max_value_bytes: 4096,
                runtime_requirements: WorkflowRuntimeRequirements::default(),
                models: Vec::new(),
                runtime_capabilities: Vec::new(),
            },
            technical_fit_decision: None,
        }
    }

    pub(in crate::workflow::tests) fn with_technical_fit_decision(
        capabilities: WorkflowHostCapabilities,
        technical_fit_decision: WorkflowTechnicalFitDecision,
    ) -> Self {
        Self {
            capabilities,
            technical_fit_decision: Some(technical_fit_decision),
        }
    }
}

pub(in crate::workflow::tests) struct DefaultCapabilitiesHost {
    pub(in crate::workflow::tests) workflow_root: PathBuf,
}

pub(in crate::workflow::tests) struct CountingPreflightHost {
    pub(in crate::workflow::tests) workflow_capabilities_calls: Arc<AtomicUsize>,
    pub(in crate::workflow::tests) runtime_capabilities_calls: Arc<AtomicUsize>,
    pub(in crate::workflow::tests) graph_fingerprint: Arc<Mutex<String>>,
    pub(in crate::workflow::tests) technical_fit_requests:
        Arc<Mutex<Vec<WorkflowTechnicalFitRequest>>>,
}

#[async_trait]
impl WorkflowHost for DefaultCapabilitiesHost {
    fn workflow_roots(&self) -> Vec<PathBuf> {
        vec![self.workflow_root.clone()]
    }

    async fn default_backend_name(&self) -> Result<String, WorkflowServiceError> {
        Ok("fallback-backend".to_string())
    }

    async fn model_metadata(
        &self,
        model_id: &str,
    ) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
        if model_id == "model-a" {
            Ok(Some(serde_json::json!({
                "size_bytes": 2_u64 * 1024_u64 * 1024_u64
            })))
        } else {
            Ok(None)
        }
    }

    async fn model_descriptor(
        &self,
        model_id: &str,
    ) -> Result<Option<WorkflowHostModelDescriptor>, WorkflowServiceError> {
        if model_id == "model-a" {
            Ok(Some(WorkflowHostModelDescriptor {
                model_type: Some("embedding".to_string()),
                hashes: HashMap::from([
                    ("blake3".to_string(), "bbb".to_string()),
                    ("sha256".to_string(), "abc123".to_string()),
                ]),
            }))
        } else {
            Ok(None)
        }
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if let Some(targets) = output_targets {
            return Ok(targets
                .iter()
                .map(|target| WorkflowPortBinding {
                    node_id: target.node_id.clone(),
                    port_id: target.port_id.clone(),
                    value: serde_json::json!("ok"),
                })
                .collect());
        }

        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("ok"),
        }])
    }
}

#[async_trait]
impl WorkflowHost for CountingPreflightHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok(self
            .graph_fingerprint
            .lock()
            .expect("graph fingerprint lock poisoned")
            .clone())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        self.workflow_capabilities_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(WorkflowHostCapabilities {
            max_input_bindings: 8,
            max_output_targets: 8,
            max_value_bytes: 4096,
            runtime_requirements: WorkflowRuntimeRequirements {
                required_backends: vec!["llama_cpp".to_string()],
                ..WorkflowRuntimeRequirements::default()
            },
            models: Vec::new(),
            runtime_capabilities: vec![ready_runtime_capability()],
        })
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        self.runtime_capabilities_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(vec![ready_runtime_capability()])
    }

    async fn workflow_io(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        Ok(WorkflowIoResponse {
            inputs: Vec::new(),
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

    async fn workflow_technical_fit_decision(
        &self,
        request: &WorkflowTechnicalFitRequest,
    ) -> Result<Option<WorkflowTechnicalFitDecision>, WorkflowServiceError> {
        self.technical_fit_requests
            .lock()
            .expect("technical-fit requests lock poisoned")
            .push(request.clone());
        Ok(None)
    }
}

#[async_trait]
impl WorkflowHost for PreflightHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("preflight-graph".to_string())
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
                name: Some("Prompt".to_string()),
                description: None,
                ports: vec![
                    WorkflowIoPort {
                        port_id: "text".to_string(),
                        name: Some("Text".to_string()),
                        description: None,
                        data_type: Some("string".to_string()),
                        required: Some(true),
                        multiple: Some(false),
                    },
                    WorkflowIoPort {
                        port_id: "tone".to_string(),
                        name: Some("Tone".to_string()),
                        description: None,
                        data_type: Some("string".to_string()),
                        required: None,
                        multiple: Some(false),
                    },
                ],
            }],
            outputs: vec![WorkflowIoNode {
                node_id: "text-output-1".to_string(),
                node_type: "text-output".to_string(),
                name: Some("Answer".to_string()),
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

    async fn workflow_technical_fit_decision(
        &self,
        _request: &WorkflowTechnicalFitRequest,
    ) -> Result<Option<WorkflowTechnicalFitDecision>, WorkflowServiceError> {
        Ok(self.technical_fit_decision.clone())
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
