use super::*;

#[test]
fn adapter_send_emits_primary_and_diagnostics_events() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
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
    let adapter = TauriEventAdapter::new(channel, "adapter-workflow", diagnostics_store);

    EventSink::send(
        &adapter,
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(55),
        },
    )
    .expect("send should succeed");

    let events = emitted.lock().expect("captured events lock");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["type"], "Started");
    assert_eq!(events[0]["data"]["workflow_id"], "adapter-workflow");
    assert_eq!(events[0]["data"]["workflow_run_id"], "exec-1");
    assert_eq!(events[1]["type"], "DiagnosticsSnapshot");
    assert_eq!(events[1]["data"]["workflow_run_id"], "exec-1");
    assert_eq!(events[1]["data"]["snapshot"]["runOrder"][0], "exec-1");
    assert_eq!(
        events[1]["data"]["snapshot"]["runsById"]["exec-1"]["workflowId"],
        "adapter-workflow"
    );
}
