use super::*;

#[tokio::test]
async fn workflow_execution_session_run_admits_cold_start_without_legacy_runtime_admission_gate() {
    let admission_open = Arc::new(AtomicBool::new(false));
    let host = AdmissionGatedHost::new(admission_open.clone());
    let service = WorkflowService::with_capacity_limits(1, 1)
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-gated".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create gated session");

    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: Some(1),
            },
        )
        .await
        .expect("cold-start admission should not depend on legacy runtime gate");
    assert_eq!(response.outputs.len(), 1);
    assert!(!admission_open.load(Ordering::SeqCst));

    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: created.session_id.clone(),
        })
        .await
        .expect("scheduler snapshot after cold-start run");
    assert!(snapshot.items.is_empty());

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 20,
        )
        .expect("diagnostic events")
    };
    let started_event = diagnostic_events
        .iter()
        .find(|event| {
            event.event_kind == pantograph_diagnostics_ledger::DiagnosticEventKind::RunStarted
        })
        .expect("run started event");
    assert_eq!(
        started_event.workflow_run_id.as_ref().map(|id| id.as_str()),
        Some(response.workflow_run_id.as_str())
    );
    assert!(started_event
        .payload_json
        .contains("\"scheduler_decision_reason\":\"cold_start_required\""));
    let delay_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerRunDelayed
        })
        .collect::<Vec<_>>();
    assert!(delay_events.is_empty());
}
