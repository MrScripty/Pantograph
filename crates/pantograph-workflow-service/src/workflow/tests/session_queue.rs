use super::*;

#[tokio::test]
async fn workflow_execution_session_queue_items_include_authoritative_timestamps() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow execution session");

    let request = WorkflowExecutionSessionRunRequest {
        session_id: created.session_id.clone(),
        workflow_semantic_version: "0.1.0".to_string(),
        inputs: Vec::new(),
        output_targets: None,
        override_selection: None,
        timeout_ms: None,
        priority: None,
    };

    let queue_id = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .enqueue_run(&created.session_id, &request)
            .expect("enqueue run")
    };

    let pending_items = {
        let store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .list_queue(&created.session_id)
            .expect("list pending queue items")
    };
    assert_eq!(pending_items.len(), 1);
    assert_eq!(pending_items[0].workflow_run_id, queue_id);
    assert!(pending_items[0].enqueued_at_ms.is_some());
    assert!(pending_items[0].dequeued_at_ms.is_none());
    assert_eq!(pending_items[0].queue_position, Some(0));
    assert_eq!(
        pending_items[0].scheduler_admission_outcome,
        Some(WorkflowSchedulerAdmissionOutcome::Queued)
    );
    assert_eq!(
        pending_items[0].status,
        WorkflowExecutionSessionQueueItemStatus::Pending
    );
    assert_eq!(
        pending_items[0].scheduler_decision_reason,
        Some(WorkflowSchedulerDecisionReason::HighestPriorityFirst)
    );

    let running_items = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .begin_queued_run(&created.session_id, &queue_id)
            .expect("begin queued run");
        store
            .list_queue(&created.session_id)
            .expect("list running queue items")
    };
    assert_eq!(running_items.len(), 1);
    assert_eq!(running_items[0].workflow_run_id, queue_id);
    assert_eq!(
        running_items[0].status,
        WorkflowExecutionSessionQueueItemStatus::Running
    );
    assert_eq!(
        running_items[0].enqueued_at_ms,
        pending_items[0].enqueued_at_ms
    );
    assert_eq!(running_items[0].queue_position, Some(0));
    assert_eq!(
        running_items[0].scheduler_admission_outcome,
        Some(WorkflowSchedulerAdmissionOutcome::Admitted)
    );
    assert!(running_items[0].dequeued_at_ms.is_some());
    assert!(
        running_items[0]
            .dequeued_at_ms
            .expect("dequeued timestamp present")
            >= running_items[0]
                .enqueued_at_ms
                .expect("enqueued timestamp present")
    );
    assert_eq!(
        running_items[0].scheduler_decision_reason,
        Some(WorkflowSchedulerDecisionReason::ColdStartRequired)
    );
}

#[tokio::test]
async fn workflow_execution_session_queue_control_records_typed_events() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new()
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().expect("ledger"));
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow execution session");

    let request = WorkflowExecutionSessionRunRequest {
        session_id: created.session_id.clone(),
        workflow_semantic_version: "0.1.0".to_string(),
        inputs: Vec::new(),
        output_targets: None,
        override_selection: None,
        timeout_ms: None,
        priority: Some(2),
    };

    let cancel_id = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .enqueue_run(&created.session_id, &request)
            .expect("enqueue cancel run")
    };
    service
        .workflow_cancel_execution_session_queue_item(WorkflowExecutionSessionQueueCancelRequest {
            session_id: created.session_id.clone(),
            workflow_run_id: cancel_id.clone(),
        })
        .await
        .expect("cancel queue item");

    let reprioritize_id = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .enqueue_run(&created.session_id, &request)
            .expect("enqueue reprioritize run")
    };
    service
        .workflow_reprioritize_execution_session_queue_item(
            WorkflowExecutionSessionQueueReprioritizeRequest {
                session_id: created.session_id.clone(),
                workflow_run_id: reprioritize_id.clone(),
                priority: 9,
            },
        )
        .await
        .expect("reprioritize queue item");

    service
        .workflow_cancel_execution_session_queue_item(WorkflowExecutionSessionQueueCancelRequest {
            session_id: created.session_id.clone(),
            workflow_run_id: cancel_id.clone(),
        })
        .await
        .expect_err("already cancelled queue item should be denied");
    service
        .workflow_reprioritize_execution_session_queue_item(
            WorkflowExecutionSessionQueueReprioritizeRequest {
                session_id: created.session_id.clone(),
                workflow_run_id: "run_missing_queue_item".to_string(),
                priority: 3,
            },
        )
        .await
        .expect_err("missing queue item reprioritize should be denied");

    let diagnostic_events = {
        let ledger = service
            .diagnostics_ledger_guard()
            .expect("diagnostics ledger");
        pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::diagnostic_events_after(
            &*ledger, 0, 10,
        )
        .expect("diagnostic events")
    };
    let queue_control_events = diagnostic_events
        .iter()
        .filter(|event| {
            event.event_kind
                == pantograph_diagnostics_ledger::DiagnosticEventKind::SchedulerQueueControl
        })
        .collect::<Vec<_>>();
    assert_eq!(queue_control_events.len(), 4);
    assert_eq!(
        queue_control_events[0]
            .workflow_run_id
            .as_ref()
            .map(|id| id.as_str()),
        Some(cancel_id.as_str())
    );
    assert!(queue_control_events[0]
        .payload_json
        .contains("\"action\":\"cancel\""));
    assert!(queue_control_events[0]
        .payload_json
        .contains("\"outcome\":\"accepted\""));
    assert!(queue_control_events[1]
        .payload_json
        .contains("\"action\":\"reprioritize\""));
    assert!(queue_control_events[1]
        .payload_json
        .contains("\"outcome\":\"accepted\""));
    assert!(queue_control_events[1]
        .payload_json
        .contains("\"new_priority\":9"));
    assert!(queue_control_events[2]
        .payload_json
        .contains("\"action\":\"cancel\""));
    assert!(queue_control_events[2]
        .payload_json
        .contains("\"outcome\":\"denied\""));
    assert!(queue_control_events[2].payload_json.contains("not found"));
    assert!(queue_control_events[3]
        .payload_json
        .contains("\"action\":\"reprioritize\""));
    assert!(queue_control_events[3]
        .payload_json
        .contains("\"outcome\":\"denied\""));
    assert!(queue_control_events[3]
        .payload_json
        .contains("\"new_priority\":3"));
}

#[tokio::test]
async fn workflow_execution_session_queue_marks_loaded_compatible_admission_as_warm_reuse() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create workflow execution session");

    let queue_id = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .mark_runtime_loaded(&created.session_id, true)
            .expect("mark runtime loaded");
        store
            .enqueue_run(
                &created.session_id,
                &WorkflowExecutionSessionRunRequest {
                    session_id: created.session_id.clone(),
                    workflow_semantic_version: "0.1.0".to_string(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    priority: Some(1),
                },
            )
            .expect("enqueue run")
    };

    let running_items = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .begin_queued_run(&created.session_id, &queue_id)
            .expect("begin queued run");
        store
            .list_queue(&created.session_id)
            .expect("list running queue items")
    };

    assert_eq!(running_items.len(), 1);
    assert_eq!(running_items[0].workflow_run_id, queue_id);
    assert_eq!(
        running_items[0].scheduler_admission_outcome,
        Some(WorkflowSchedulerAdmissionOutcome::Admitted)
    );
    assert_eq!(
        running_items[0].scheduler_decision_reason,
        Some(WorkflowSchedulerDecisionReason::WarmSessionReused)
    );
}

#[tokio::test]
async fn workflow_execution_session_queue_prefers_bounded_warm_reuse_over_same_priority_cold_head()
{
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create workflow execution session");

    let (cold_head_queue_id, warm_queue_id) = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .update_runtime_affinity_basis(
                &created.session_id,
                vec!["llama_cpp".to_string()],
                vec!["model-a".to_string()],
            )
            .expect("update runtime affinity basis");
        store
            .mark_runtime_loaded(&created.session_id, true)
            .expect("mark runtime loaded");
        let cold_head_queue_id = store
            .enqueue_run(
                &created.session_id,
                &WorkflowExecutionSessionRunRequest {
                    session_id: created.session_id.clone(),
                    workflow_semantic_version: "0.1.0".to_string(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: Some(WorkflowTechnicalFitOverride {
                        model_id: Some("model-b".to_string()),
                        backend_key: Some("pytorch".to_string()),
                    }),
                    timeout_ms: None,
                    priority: Some(1),
                },
            )
            .expect("enqueue cold head");
        let warm_queue_id = store
            .enqueue_run(
                &created.session_id,
                &WorkflowExecutionSessionRunRequest {
                    session_id: created.session_id.clone(),
                    workflow_semantic_version: "0.1.0".to_string(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    priority: Some(1),
                },
            )
            .expect("enqueue warm follow");
        (cold_head_queue_id, warm_queue_id)
    };

    let running_items = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .begin_queued_run(&created.session_id, &warm_queue_id)
            .expect("begin queued run");
        store
            .list_queue(&created.session_id)
            .expect("list running queue items")
    };

    assert_eq!(running_items.len(), 2);
    assert_eq!(running_items[0].workflow_run_id, warm_queue_id);
    assert_eq!(
        running_items[0].scheduler_decision_reason,
        Some(WorkflowSchedulerDecisionReason::WarmSessionReused)
    );
    assert_eq!(running_items[1].workflow_run_id, cold_head_queue_id);
    assert_eq!(
        running_items[1].scheduler_decision_reason,
        Some(WorkflowSchedulerDecisionReason::HighestPriorityFirst)
    );
}

#[tokio::test]
async fn workflow_execution_session_queue_items_expose_authoritative_queue_positions() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow execution session");

    let first_queue_id = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .enqueue_run(
                &created.session_id,
                &WorkflowExecutionSessionRunRequest {
                    session_id: created.session_id.clone(),
                    workflow_semantic_version: "0.1.0".to_string(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    priority: Some(10),
                },
            )
            .expect("enqueue first run")
    };
    let second_queue_id = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .enqueue_run(
                &created.session_id,
                &WorkflowExecutionSessionRunRequest {
                    session_id: created.session_id.clone(),
                    workflow_semantic_version: "0.1.0".to_string(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    priority: Some(5),
                },
            )
            .expect("enqueue second run")
    };

    let pending_items = {
        let store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .list_queue(&created.session_id)
            .expect("list pending queue items")
    };
    assert_eq!(pending_items.len(), 2);
    assert_eq!(pending_items[0].workflow_run_id, first_queue_id);
    assert_eq!(pending_items[0].queue_position, Some(0));
    assert_eq!(pending_items[1].workflow_run_id, second_queue_id);
    assert_eq!(pending_items[1].queue_position, Some(1));

    let running_items = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .begin_queued_run(&created.session_id, &first_queue_id)
            .expect("begin first run");
        store
            .list_queue(&created.session_id)
            .expect("list queue after begin")
    };
    assert_eq!(running_items.len(), 2);
    assert_eq!(running_items[0].workflow_run_id, first_queue_id);
    assert_eq!(running_items[0].queue_position, Some(0));
    assert_eq!(running_items[1].workflow_run_id, second_queue_id);
    assert_eq!(running_items[1].queue_position, Some(1));
}

#[tokio::test]
async fn workflow_execution_session_queue_promotes_starved_runs_before_newer_higher_priority_runs()
{
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow execution session");

    let low_priority_queue_id = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .enqueue_run(
                &created.session_id,
                &WorkflowExecutionSessionRunRequest {
                    session_id: created.session_id.clone(),
                    workflow_semantic_version: "0.1.0".to_string(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    priority: Some(0),
                },
            )
            .expect("enqueue low priority run")
    };

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        for _ in [
            "newer-high-priority-1",
            "newer-high-priority-2",
            "newer-high-priority-3",
            "newer-high-priority-4",
        ] {
            store
                .enqueue_run(
                    &created.session_id,
                    &WorkflowExecutionSessionRunRequest {
                        session_id: created.session_id.clone(),
                        workflow_semantic_version: "0.1.0".to_string(),
                        inputs: Vec::new(),
                        output_targets: None,
                        override_selection: None,
                        timeout_ms: None,
                        priority: Some(2),
                    },
                )
                .expect("enqueue higher priority run");
        }
    }

    let pending_items = {
        let store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .list_queue(&created.session_id)
            .expect("list starved queue items")
    };
    assert_eq!(pending_items.len(), 5);
    assert_eq!(pending_items[0].workflow_run_id, low_priority_queue_id);
    assert_eq!(pending_items[0].queue_position, Some(0));
    assert_eq!(
        pending_items[0].scheduler_admission_outcome,
        Some(WorkflowSchedulerAdmissionOutcome::Queued)
    );
    assert_eq!(
        pending_items[0].scheduler_decision_reason,
        Some(WorkflowSchedulerDecisionReason::StarvationProtection)
    );
    assert_eq!(
        pending_items[1].scheduler_decision_reason,
        Some(WorkflowSchedulerDecisionReason::FifoPriorityTieBreak)
    );

    let running_items = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .begin_queued_run(&created.session_id, &low_priority_queue_id)
            .expect("admit starved queue item");
        store
            .list_queue(&created.session_id)
            .expect("list running queue items")
    };
    assert_eq!(running_items[0].workflow_run_id, low_priority_queue_id);
    assert_eq!(
        running_items[0].scheduler_admission_outcome,
        Some(WorkflowSchedulerAdmissionOutcome::Admitted)
    );
    assert_eq!(
        running_items[0].scheduler_decision_reason,
        Some(WorkflowSchedulerDecisionReason::ColdStartRequired)
    );
}
