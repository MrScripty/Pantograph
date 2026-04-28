use super::*;

#[tokio::test]
async fn local_network_status_reports_local_node_and_scheduler_load() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_capacity_limits(3, 2);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-local-network".to_string(),
                usage_profile: None,
                keep_alive: true,
            },
        )
        .await
        .expect("create session");
    let queued_run_id = {
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
                    priority: Some(1),
                },
            )
            .expect("enqueue run")
    };

    let response = service
        .workflow_local_network_status_query(WorkflowLocalNetworkStatusQueryRequest {
            include_network_interfaces: false,
            include_disks: false,
        })
        .expect("query local network status");

    assert_eq!(response.local_node.node_id, "local");
    assert_eq!(
        response.local_node.transport_state,
        WorkflowNetworkTransportState::LocalOnly
    );
    assert_eq!(response.local_node.scheduler_load.max_sessions, 3);
    assert_eq!(response.local_node.scheduler_load.max_loaded_sessions, 2);
    assert_eq!(response.local_node.scheduler_load.active_session_count, 1);
    assert_eq!(response.local_node.scheduler_load.queued_run_count, 1);
    assert_eq!(
        response.local_node.scheduler_load.queued_workflow_run_ids,
        vec![queued_run_id]
    );
    assert!(response
        .local_node
        .scheduler_load
        .active_workflow_run_ids
        .is_empty());
    assert!(response.local_node.system.disks.is_empty());
    assert!(response.local_node.system.network_interfaces.is_empty());
    assert!(!response.local_node.system.gpu.available);
    assert!(!response.local_node.degradation_warnings.is_empty());
    assert!(response.peer_nodes.is_empty());
}
