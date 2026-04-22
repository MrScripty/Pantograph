use super::*;

pub(in crate::workflow::tests) struct TimeoutAwareHost {
    pub(in crate::workflow::tests) cancelled: Arc<AtomicBool>,
    pub(in crate::workflow::tests) capabilities: WorkflowHostCapabilities,
}

impl TimeoutAwareHost {
    pub(in crate::workflow::tests) fn new(cancelled: Arc<AtomicBool>) -> Self {
        Self {
            cancelled,
            capabilities: WorkflowHostCapabilities {
                max_input_bindings: 16,
                max_output_targets: 16,
                max_value_bytes: 4096,
                runtime_requirements: WorkflowRuntimeRequirements::default(),
                models: Vec::new(),
                runtime_capabilities: Vec::new(),
            },
        }
    }
}

#[derive(Clone)]
pub(in crate::workflow::tests) struct BlockingRunHost {
    pub(in crate::workflow::tests) capabilities: WorkflowHostCapabilities,
    pub(in crate::workflow::tests) started_runs: Arc<AtomicUsize>,
    pub(in crate::workflow::tests) first_run_started: Arc<Notify>,
    pub(in crate::workflow::tests) release_first_run: Arc<Notify>,
}

impl BlockingRunHost {
    pub(in crate::workflow::tests) fn new() -> Self {
        Self {
            capabilities: MockWorkflowHost::new(8, 1024).capabilities,
            started_runs: Arc::new(AtomicUsize::new(0)),
            first_run_started: Arc::new(Notify::new()),
            release_first_run: Arc::new(Notify::new()),
        }
    }

    pub(in crate::workflow::tests) async fn wait_for_first_run_started(&self) {
        if self.started_runs.load(Ordering::SeqCst) > 0 {
            return;
        }
        self.first_run_started.notified().await;
    }

    pub(in crate::workflow::tests) fn release_first_run(&self) {
        self.release_first_run.notify_waiters();
    }
}

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
    pub(in crate::workflow::tests) retention_hints: Arc<Mutex<Vec<WorkflowSessionRetentionHint>>>,
    pub(in crate::workflow::tests) capabilities: WorkflowHostCapabilities,
}

impl RecordingRuntimeHost {
    pub(in crate::workflow::tests) fn new(
        retention_hints: Arc<Mutex<Vec<WorkflowSessionRetentionHint>>>,
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

#[async_trait]
impl WorkflowHost for TimeoutAwareHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("timeout-graph".to_string())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(self.capabilities.clone())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        loop {
            if run_handle.is_cancelled() {
                self.cancelled.store(true, Ordering::SeqCst);
                return Err(WorkflowServiceError::Cancelled(
                    "workflow run cancelled".to_string(),
                ));
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

#[async_trait]
impl WorkflowHost for BlockingRunHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("blocking-run-graph".to_string())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(self.capabilities.clone())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if self.started_runs.fetch_add(1, Ordering::SeqCst) == 0 {
            self.first_run_started.notify_waiters();
            self.release_first_run.notified().await;
        }

        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("ok"),
        }])
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

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(self.capabilities.clone())
    }

    async fn can_load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        _retention_hint: WorkflowSessionRetentionHint,
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

    async fn load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        retention_hint: WorkflowSessionRetentionHint,
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
