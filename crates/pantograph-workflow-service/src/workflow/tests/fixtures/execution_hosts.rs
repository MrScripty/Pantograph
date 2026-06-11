use super::*;
use crate::{GraphNode, Position};

#[derive(Clone)]
pub(in crate::workflow::tests) struct AdmissionGatedHost {
    pub(in crate::workflow::tests) capabilities: WorkflowHostCapabilities,
    pub(in crate::workflow::tests) admission_open: Arc<AtomicBool>,
}

impl AdmissionGatedHost {
    pub(in crate::workflow::tests) fn new(admission_open: Arc<AtomicBool>) -> Self {
        Self {
            capabilities: MockWorkflowHost::new(8, 1024).capabilities,
            admission_open,
        }
    }
}

pub(in crate::workflow::tests) struct RecordingRuntimeHost {
    pub(in crate::workflow::tests) retention_hints:
        Arc<Mutex<Vec<WorkflowExecutionSessionRetentionHint>>>,
    pub(in crate::workflow::tests) capabilities: WorkflowHostCapabilities,
}

pub(in crate::workflow::tests) struct FailingRunSnapshotHost {
    pub(in crate::workflow::tests) inner: MockWorkflowHost,
}

pub(in crate::workflow::tests) struct FailingUnloadWithPoisonedDiagnosticsHost {
    pub(in crate::workflow::tests) inner: MockWorkflowHost,
    pub(in crate::workflow::tests) diagnostics_ledger: Arc<Mutex<SqliteDiagnosticsLedger>>,
}

pub(in crate::workflow::tests) struct StaleWorkflowGraphHost {
    pub(in crate::workflow::tests) inner: MockWorkflowHost,
    pub(in crate::workflow::tests) run_attempts: Arc<AtomicUsize>,
}

fn poison_diagnostics_ledger(diagnostics_ledger: &Arc<Mutex<SqliteDiagnosticsLedger>>) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = diagnostics_ledger
            .lock()
            .expect("diagnostics ledger lock should be available before poisoning");
        panic!("poison diagnostics ledger for test");
    }));
}

impl RecordingRuntimeHost {
    pub(in crate::workflow::tests) fn new(
        retention_hints: Arc<Mutex<Vec<WorkflowExecutionSessionRetentionHint>>>,
    ) -> Self {
        Self {
            retention_hints,
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

impl FailingRunSnapshotHost {
    pub(in crate::workflow::tests) fn new() -> Self {
        Self {
            inner: MockWorkflowHost::new(8, 1024),
        }
    }
}

impl FailingUnloadWithPoisonedDiagnosticsHost {
    pub(in crate::workflow::tests) fn new(
        diagnostics_ledger: Arc<Mutex<SqliteDiagnosticsLedger>>,
    ) -> Self {
        Self {
            inner: MockWorkflowHost::new(8, 1024),
            diagnostics_ledger,
        }
    }

    fn poison_diagnostics_ledger(&self) {
        poison_diagnostics_ledger(&self.diagnostics_ledger);
    }
}

impl StaleWorkflowGraphHost {
    pub(in crate::workflow::tests) fn new() -> Self {
        Self {
            inner: MockWorkflowHost::new(8, 1024),
            run_attempts: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl WorkflowHost for AdmissionGatedHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("admission-gated-graph".to_string())
    }

    async fn workflow_graph(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowGraph, WorkflowServiceError> {
        Ok(mock_workflow_graph())
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

    async fn can_load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        _retention_hint: WorkflowExecutionSessionRetentionHint,
    ) -> Result<bool, WorkflowServiceError> {
        Ok(self.admission_open.load(Ordering::SeqCst))
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
impl WorkflowHost for RecordingRuntimeHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("recording-graph".to_string())
    }

    async fn workflow_graph(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowGraph, WorkflowServiceError> {
        Ok(mock_workflow_graph())
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
                name: None,
                description: None,
                ports: vec![WorkflowIoPort {
                    port_id: "text".to_string(),
                    name: None,
                    description: None,
                    data_type: Some("string".to_string()),
                    required: Some(true),
                    multiple: Some(false),
                }],
            }],
            outputs: vec![WorkflowIoNode {
                node_id: "text-output-1".to_string(),
                node_type: "text-output".to_string(),
                name: None,
                description: None,
                ports: vec![WorkflowIoPort {
                    port_id: "text".to_string(),
                    name: None,
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

    async fn load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        retention_hint: WorkflowExecutionSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        self.retention_hints
            .lock()
            .expect("retention hints lock poisoned")
            .push(retention_hint);
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
impl WorkflowHost for FailingRunSnapshotHost {
    async fn validate_workflow(&self, workflow_id: &str) -> Result<(), WorkflowServiceError> {
        self.inner.validate_workflow(workflow_id).await
    }

    async fn workflow_graph_fingerprint(
        &self,
        workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        self.inner.workflow_graph_fingerprint(workflow_id).await
    }

    async fn workflow_graph(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowGraph, WorkflowServiceError> {
        Err(WorkflowServiceError::Internal(
            "snapshot graph read failed".to_string(),
        ))
    }

    async fn workflow_capabilities(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        self.inner.workflow_capabilities(workflow_id).await
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        self.inner.runtime_capabilities().await
    }

    async fn load_session_runtime(
        &self,
        session_id: &str,
        workflow_id: &str,
        usage_profile: Option<&str>,
        retention_hint: WorkflowExecutionSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        self.inner
            .load_session_runtime(session_id, workflow_id, usage_profile, retention_hint)
            .await
    }

    async fn run_workflow(
        &self,
        workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        run_options: WorkflowRunOptions,
        run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        self.inner
            .run_workflow(workflow_id, inputs, output_targets, run_options, run_handle)
            .await
    }
}

#[async_trait]
impl WorkflowHost for FailingUnloadWithPoisonedDiagnosticsHost {
    async fn validate_workflow(&self, workflow_id: &str) -> Result<(), WorkflowServiceError> {
        self.inner.validate_workflow(workflow_id).await
    }

    async fn workflow_graph_fingerprint(
        &self,
        workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        self.inner.workflow_graph_fingerprint(workflow_id).await
    }

    async fn workflow_graph(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowGraph, WorkflowServiceError> {
        self.inner.workflow_graph(workflow_id).await
    }

    async fn workflow_capabilities(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        self.inner.workflow_capabilities(workflow_id).await
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        self.inner.runtime_capabilities().await
    }

    async fn load_session_runtime(
        &self,
        session_id: &str,
        workflow_id: &str,
        usage_profile: Option<&str>,
        retention_hint: WorkflowExecutionSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        self.inner
            .load_session_runtime(session_id, workflow_id, usage_profile, retention_hint)
            .await
    }

    async fn run_workflow(
        &self,
        workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        run_options: WorkflowRunOptions,
        run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        self.inner
            .run_workflow(workflow_id, inputs, output_targets, run_options, run_handle)
            .await
    }

    async fn unload_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _reason: WorkflowExecutionSessionUnloadReason,
    ) -> Result<(), WorkflowServiceError> {
        self.poison_diagnostics_ledger();
        Err(WorkflowServiceError::RuntimeNotReady(
            "runtime unload failed".to_string(),
        ))
    }
}

#[async_trait]
impl WorkflowHost for StaleWorkflowGraphHost {
    async fn validate_workflow(&self, workflow_id: &str) -> Result<(), WorkflowServiceError> {
        self.inner.validate_workflow(workflow_id).await
    }

    async fn workflow_graph_fingerprint(
        &self,
        workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        self.inner.workflow_graph_fingerprint(workflow_id).await
    }

    async fn workflow_graph(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowGraph, WorkflowServiceError> {
        Ok(WorkflowGraph {
            nodes: vec![GraphNode {
                id: "diffusion".to_string(),
                node_type: "diffusion-inference".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({}),
            }],
            edges: Vec::new(),
            derived_graph: None,
        })
    }

    async fn workflow_capabilities(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        self.inner.workflow_capabilities(workflow_id).await
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        self.inner.runtime_capabilities().await
    }

    async fn run_workflow(
        &self,
        workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        run_options: WorkflowRunOptions,
        run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        self.run_attempts.fetch_add(1, Ordering::SeqCst);
        self.inner
            .run_workflow(workflow_id, inputs, output_targets, run_options, run_handle)
            .await
    }
}
