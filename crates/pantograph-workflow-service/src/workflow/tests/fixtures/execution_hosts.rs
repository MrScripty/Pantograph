use super::*;
use crate::{GraphNode, Position};

#[derive(Clone)]
pub(in crate::workflow::tests) struct BlockingRunHost {
    pub(in crate::workflow::tests) capabilities: WorkflowHostCapabilities,
    pub(in crate::workflow::tests) started_runs: Arc<AtomicUsize>,
    pub(in crate::workflow::tests) first_run_started: Arc<Notify>,
    pub(in crate::workflow::tests) first_run_released: Arc<AtomicBool>,
    pub(in crate::workflow::tests) release_first_run: Arc<Notify>,
}

impl BlockingRunHost {
    pub(in crate::workflow::tests) fn new() -> Self {
        Self {
            capabilities: MockWorkflowHost::new(8, 1024).capabilities,
            started_runs: Arc::new(AtomicUsize::new(0)),
            first_run_started: Arc::new(Notify::new()),
            first_run_released: Arc::new(AtomicBool::new(false)),
            release_first_run: Arc::new(Notify::new()),
        }
    }

    pub(in crate::workflow::tests) async fn wait_for_first_run_started(&self) {
        while self.started_runs.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    pub(in crate::workflow::tests) fn release_first_run(&self) {
        self.first_run_released.store(true, Ordering::SeqCst);
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
    pub(in crate::workflow::tests) retention_hints:
        Arc<Mutex<Vec<WorkflowExecutionSessionRetentionHint>>>,
    pub(in crate::workflow::tests) capabilities: WorkflowHostCapabilities,
}

pub(in crate::workflow::tests) struct FailingRuntimeLoadHost {
    pub(in crate::workflow::tests) capabilities: WorkflowHostCapabilities,
    pub(in crate::workflow::tests) phase_hint: Option<WorkflowRuntimeDiagnosticPhaseHint>,
}

pub(in crate::workflow::tests) struct FailingRunSnapshotHost {
    pub(in crate::workflow::tests) inner: MockWorkflowHost,
}

pub(in crate::workflow::tests) struct FailingRunWithPoisonedDiagnosticsHost {
    pub(in crate::workflow::tests) inner: MockWorkflowHost,
    pub(in crate::workflow::tests) diagnostics_ledger: Arc<Mutex<SqliteDiagnosticsLedger>>,
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

impl FailingRuntimeLoadHost {
    pub(in crate::workflow::tests) fn new() -> Self {
        Self {
            capabilities: MockWorkflowHost::new(8, 1024).capabilities,
            phase_hint: None,
        }
    }

    pub(in crate::workflow::tests) fn with_phase_hint(
        phase_hint: WorkflowRuntimeDiagnosticPhaseHint,
    ) -> Self {
        Self {
            capabilities: MockWorkflowHost::new(8, 1024).capabilities,
            phase_hint: Some(phase_hint),
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

impl FailingRunWithPoisonedDiagnosticsHost {
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
        MockWorkflowHost::new(8, 1024)
            .workflow_io(_workflow_id)
            .await
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        Ok(self.capabilities.runtime_capabilities.clone())
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
            while !self.first_run_released.load(Ordering::SeqCst) {
                self.release_first_run.notified().await;
            }
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
impl WorkflowHost for FailingRuntimeLoadHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("failing-runtime-load-graph".to_string())
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
        _retention_hint: WorkflowExecutionSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        let error = WorkflowServiceError::RuntimeNotReady(
            "llama.cpp spawn failed\u{0000}\nmissing server".to_string(),
        );
        Err(match self.phase_hint {
            Some(phase_hint) => error.with_runtime_diagnostic_phase(phase_hint),
            None => error,
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
        unreachable!("runtime load failure prevents workflow execution")
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
impl WorkflowHost for FailingRunWithPoisonedDiagnosticsHost {
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

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        self.poison_diagnostics_ledger();
        Err(WorkflowServiceError::InvalidRequest(
            "workflow execution failed".to_string(),
        ))
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
