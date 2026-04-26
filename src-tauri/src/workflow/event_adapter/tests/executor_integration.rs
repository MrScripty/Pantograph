use super::*;

#[tokio::test]
async fn workflow_executor_parallel_run_emits_consumer_visible_events_through_adapter() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    diagnostics_store.set_execution_metadata("exec-parallel", Some("parallel".to_string()));
    diagnostics_store.set_execution_graph("exec-parallel", &sample_parallel_graph());

    let emitted = Arc::new(Mutex::new(Vec::<Value>::new()));
    let captured = emitted.clone();
    let channel: Channel<TauriWorkflowEvent> = Channel::new(move |body| {
        let value = match body {
            InvokeResponseBody::Json(json) => {
                serde_json::from_str::<Value>(&json).expect("channel event json")
            }
            InvokeResponseBody::Raw(bytes) => {
                serde_json::from_slice::<Value>(&bytes).expect("channel event raw json")
            }
        };
        captured.lock().expect("captured events lock").push(value);
        Ok(())
    });
    let adapter = Arc::new(TauriEventAdapter::new(
        channel,
        "parallel",
        diagnostics_store,
    ));
    let workflow_executor = WorkflowExecutor::new(
        "exec-parallel",
        make_parallel_roots_graph(),
        adapter.clone(),
    );
    let executor_impl = YieldingAcceptanceExecutor::new();

    let outputs = workflow_executor
        .demand_multiple(&["left".to_string(), "right".to_string()], &executor_impl)
        .await
        .expect("parallel workflow should succeed");

    assert_eq!(executor_impl.max_in_flight(), 2);
    assert_eq!(outputs.len(), 2);

    let events = emitted.lock().expect("captured events lock");
    let event_types = events
        .iter()
        .map(|event| event["type"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert!(event_types.contains(&"IncrementalExecutionStarted".to_string()));
    assert!(event_types.contains(&"NodeStarted".to_string()));
    assert!(event_types.contains(&"NodeCompleted".to_string()));
    assert!(event_types.contains(&"DiagnosticsSnapshot".to_string()));

    let snapshot = events
        .iter()
        .rev()
        .find(|event| event["type"] == "DiagnosticsSnapshot")
        .expect("final diagnostics snapshot");
    let run = &snapshot["data"]["snapshot"]["runsById"]["exec-parallel"];
    assert_eq!(run["workflowId"], "parallel");
    assert_eq!(run["graphFingerprintAtStart"], "graph-parallel");
    assert_eq!(run["lastIncrementalTaskIds"][0], "left");
    assert_eq!(run["lastIncrementalTaskIds"][1], "right");
    assert_eq!(run["nodes"]["left"]["status"], "completed");
    assert_eq!(run["nodes"]["right"]["status"], "completed");
}
