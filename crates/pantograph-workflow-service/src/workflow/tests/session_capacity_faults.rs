use super::*;
use async_trait::async_trait;

struct CapacityFaultHost {
    inner: MockWorkflowHost,
    loads: Mutex<Vec<String>>,
    unloads: Mutex<Vec<String>>,
    unload_error: Option<String>,
}

impl CapacityFaultHost {
    fn new(unload_error: Option<String>) -> Self {
        Self {
            inner: MockWorkflowHost::new(16, 4096),
            loads: Mutex::new(Vec::new()),
            unloads: Mutex::new(Vec::new()),
            unload_error,
        }
    }
}

#[async_trait]
impl WorkflowHost for CapacityFaultHost {
    async fn validate_workflow(&self, id: &str) -> Result<(), WorkflowServiceError> {
        self.inner.validate_workflow(id).await
    }
    async fn workflow_graph(&self, id: &str) -> Result<WorkflowGraph, WorkflowServiceError> {
        self.inner.workflow_graph(id).await
    }
    async fn workflow_graph_fingerprint(&self, id: &str) -> Result<String, WorkflowServiceError> {
        self.inner.workflow_graph_fingerprint(id).await
    }
    async fn workflow_capabilities(
        &self,
        id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        self.inner.workflow_capabilities(id).await
    }
    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        self.inner.runtime_capabilities().await
    }
    async fn load_session_runtime(
        &self,
        session_id: &str,
        _id: &str,
        _usage: Option<&str>,
        _hint: WorkflowExecutionSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        self.loads
            .lock()
            .expect("loads")
            .push(session_id.to_string());
        Ok(())
    }
    async fn unload_session_runtime(
        &self,
        session_id: &str,
        _id: &str,
        _reason: WorkflowExecutionSessionUnloadReason,
    ) -> Result<(), WorkflowServiceError> {
        if let Some(error) = &self.unload_error {
            return Err(WorkflowServiceError::RuntimeNotReady(error.clone()));
        }
        self.unloads
            .lock()
            .expect("unloads")
            .push(session_id.to_string());
        Ok(())
    }
    async fn run_workflow(
        &self,
        _id: &str,
        _inputs: &[WorkflowPortBinding],
        _targets: Option<&[WorkflowOutputTarget]>,
        _options: WorkflowRunOptions,
        _handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        panic!("explicit keep-alive must not execute a workflow")
    }
}

async fn assert_completed_append_failure_preserves_residency(enable_existing: bool) {
    let temp = tempfile::tempdir().expect("temp ledger");
    let path = temp.path().join("diagnostics.sqlite");
    let service = WorkflowService::with_capacity_limits(2, 1)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open(&path).expect("ledger"));
    let host = CapacityFaultHost::new(None);
    let victim = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: true,
            },
        )
        .await
        .expect("loaded victim");
    let connection = rusqlite::Connection::open(&path).expect("fault connection");
    connection
        .execute_batch(
            "CREATE TRIGGER reject_terminal BEFORE INSERT ON diagnostic_events
        WHEN json_extract(NEW.payload_json, '$.transition') = 'unload_completed'
        BEGIN SELECT RAISE(FAIL, 'injected terminal append failure'); END;",
        )
        .expect("fault trigger");
    let target_id;
    let error = if enable_existing {
        let target = service
            .create_workflow_execution_session(
                &host,
                WorkflowExecutionSessionCreateRequest {
                    workflow_id: "wf-1".to_string(),
                    usage_profile: None,
                    keep_alive: false,
                },
            )
            .await
            .expect("unloaded target");
        target_id = Some(target.session_id.clone());
        service
            .workflow_set_execution_session_keep_alive(
                &host,
                WorkflowExecutionSessionKeepAliveRequest {
                    session_id: target.session_id,
                    keep_alive: true,
                },
            )
            .await
            .expect_err("terminal telemetry failed")
    } else {
        target_id = None;
        service
            .create_workflow_execution_session(
                &host,
                WorkflowExecutionSessionCreateRequest {
                    workflow_id: "wf-1".to_string(),
                    usage_profile: None,
                    keep_alive: true,
                },
            )
            .await
            .expect_err("terminal telemetry failed")
    };
    let link = error.diagnostics().expect("explicit telemetry failure");
    assert!(
        link.diagnostic_event_id.is_some(),
        "canonical error is independently recorded"
    );
    assert!(link
        .diagnostics_unavailable
        .as_deref()
        .expect("lifecycle gap")
        .contains("injected terminal append failure"));
    assert!(link.workflow_run_id.is_none());
    {
        let store = service.session_store_guard().expect("store");
        assert_eq!(store.loaded_session_count(), 0);
        assert_eq!(
            store
                .session_summary(&victim.session_id)
                .expect("victim")
                .state,
            WorkflowExecutionSessionState::IdleUnloaded
        );
        assert_eq!(store.active.len(), if enable_existing { 2 } else { 1 });
        if let Some(id) = target_id {
            let target = store.session_summary(&id).expect("target");
            assert!(!target.keep_alive);
            assert_eq!(target.state, WorkflowExecutionSessionState::IdleUnloaded);
        }
    }
    assert_eq!(
        *host.unloads.lock().expect("unloads"),
        vec![victim.session_id.clone()]
    );
    connection
        .execute_batch("DROP TRIGGER reject_terminal")
        .expect("remove fault");
    service
        .workflow_set_execution_session_keep_alive(
            &host,
            WorkflowExecutionSessionKeepAliveRequest {
                session_id: victim.session_id.clone(),
                keep_alive: true,
            },
        )
        .await
        .expect("recovery reloads unloaded victim");
    assert_eq!(
        *host.loads.lock().expect("loads"),
        vec![victim.session_id.clone(), victim.session_id]
    );
    assert_eq!(
        service
            .session_store_guard()
            .expect("store")
            .loaded_session_count(),
        1
    );
}

#[tokio::test]
async fn keep_alive_creation_terminal_append_failure_preserves_residency_and_recovery() {
    assert_completed_append_failure_preserves_residency(false).await;
}

#[tokio::test]
async fn keep_alive_enablement_terminal_append_failure_preserves_residency_and_recovery() {
    assert_completed_append_failure_preserves_residency(true).await;
}

async fn assert_failed_unload_reports_terminal_diagnostics(reject_terminal: bool) {
    let temp = tempfile::tempdir().expect("temp ledger");
    let path = temp.path().join("diagnostics.sqlite");
    let service = WorkflowService::with_capacity_limits(2, 1)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open(&path).expect("ledger"));
    let message = "unload refused\nworker still busy";
    let host = CapacityFaultHost::new(Some(message.to_string()));
    let victim = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: true,
            },
        )
        .await
        .expect("loaded victim");
    let target = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("unloaded target");
    if reject_terminal {
        rusqlite::Connection::open(&path)
            .expect("fault connection")
            .execute_batch(
                "CREATE TRIGGER reject_failed BEFORE INSERT ON diagnostic_events
             WHEN json_extract(NEW.payload_json, '$.transition') = 'unload_failed'
             BEGIN SELECT RAISE(FAIL, 'injected failed-event append failure'); END;",
            )
            .expect("fault trigger");
    }
    let error = service
        .workflow_set_execution_session_keep_alive(
            &host,
            WorkflowExecutionSessionKeepAliveRequest {
                session_id: target.session_id.clone(),
                keep_alive: true,
            },
        )
        .await
        .expect_err("host unload failure");
    assert_eq!(error.code(), WorkflowErrorCode::RuntimeNotReady);
    assert_eq!(error.message(), message);
    let link = error.diagnostics().expect("canonical error link");
    assert!(link.workflow_run_id.is_none());
    assert!(link.diagnostic_event_id.is_some());
    if reject_terminal {
        assert!(link
            .diagnostics_unavailable
            .as_deref()
            .expect("terminal event gap")
            .contains("injected failed-event append failure"));
    } else {
        assert!(link.diagnostics_unavailable.is_none());
    }
    let store = service.session_store_guard().expect("store");
    assert_eq!(store.loaded_session_count(), 1);
    assert_eq!(
        store
            .session_summary(&victim.session_id)
            .expect("victim")
            .state,
        WorkflowExecutionSessionState::IdleLoaded
    );
    assert!(
        !store
            .session_summary(&target.session_id)
            .expect("target")
            .keep_alive
    );
    drop(store);
    let ledger = service.diagnostics_ledger_guard().expect("ledger");
    let events =
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 20,
        )
        .expect("events");
    let lifecycle = events.iter().filter(|event| event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerModelLifecycleChanged).collect::<Vec<_>>();
    assert_eq!(lifecycle.len(), if reject_terminal { 2 } else { 3 });
    let payloads = lifecycle
        .iter()
        .map(|event| {
            assert!(event.workflow_run_id.is_none());
            serde_json::from_str::<serde_json::Value>(&event.payload_json).expect("payload")
        })
        .collect::<Vec<_>>();
    assert_eq!(payloads[0]["transition"], "unload_scheduled");
    assert_eq!(payloads[1]["transition"], "unload_started");
    assert_eq!(
        payloads[0]["timing_attempt_id"],
        payloads[1]["timing_attempt_id"]
    );
    if !reject_terminal {
        assert_eq!(payloads[2]["transition"], "unload_failed");
        assert_eq!(
            payloads[2]["timing_attempt_id"],
            payloads[0]["timing_attempt_id"]
        );
        assert!(!payloads[2]["error"]
            .as_str()
            .expect("sanitized host error")
            .contains('\n'));
        assert!(payloads[2]["error"]
            .as_str()
            .expect("error")
            .contains("worker still busy"));
    }
}

#[tokio::test]
async fn multiline_unload_error_records_complete_lifecycle_with_healthy_ledger() {
    assert_failed_unload_reports_terminal_diagnostics(false).await;
}

#[tokio::test]
async fn terminal_unload_failure_is_reported_even_when_canonical_error_records() {
    assert_failed_unload_reports_terminal_diagnostics(true).await;
}
