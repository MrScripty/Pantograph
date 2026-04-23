use super::*;

#[tokio::test]
async fn test_workflow_executor_human_input_emits_waiting_for_input() {
    let graph = WorkflowGraph {
        id: "interactive-workflow".to_string(),
        name: "Interactive Workflow".to_string(),
        nodes: vec![GraphNode {
            id: "approval".to_string(),
            node_type: "human-input".to_string(),
            data: serde_json::json!({
                "node_type": "human-input",
                "prompt": "Approve deployment?"
            }),
            position: (0.0, 0.0),
        }],
        edges: Vec::new(),
        groups: Vec::new(),
    };
    let event_sink = Arc::new(VecEventSink::new());
    let workflow_executor = WorkflowExecutor::new("exec_human_input", graph, event_sink.clone());
    let executor_impl = crate::core_executor::CoreTaskExecutor::new();

    let error = workflow_executor
        .demand(&"approval".to_string(), &executor_impl)
        .await
        .expect_err("human input should pause execution");
    assert!(matches!(
        error,
        NodeEngineError::WaitingForInput {
            task_id,
            prompt: Some(prompt)
        } if task_id == "approval" && prompt == "Approve deployment?"
    ));

    let events = event_sink.events();
    assert!(matches!(
        events.as_slice(),
        [
            WorkflowEvent::TaskStarted { task_id, .. },
            WorkflowEvent::WaitingForInput {
                workflow_id,
                task_id: waiting_task_id,
                prompt: Some(prompt),
                ..
            }
        ] if task_id == "approval"
            && workflow_id == "interactive-workflow"
            && waiting_task_id == "approval"
            && prompt == "Approve deployment?"
    ));
}

#[tokio::test]
async fn test_workflow_executor_human_input_continues_with_response() {
    let graph = WorkflowGraph {
        id: "interactive-workflow".to_string(),
        name: "Interactive Workflow".to_string(),
        nodes: vec![GraphNode {
            id: "approval".to_string(),
            node_type: "human-input".to_string(),
            data: serde_json::json!({
                "node_type": "human-input",
                "prompt": "Approve deployment?",
                "user_response": "approved"
            }),
            position: (0.0, 0.0),
        }],
        edges: Vec::new(),
        groups: Vec::new(),
    };
    let event_sink = Arc::new(VecEventSink::new());
    let workflow_executor = WorkflowExecutor::new("exec_human_input", graph, event_sink.clone());
    let executor_impl = crate::core_executor::CoreTaskExecutor::new();

    let outputs = workflow_executor
        .demand(&"approval".to_string(), &executor_impl)
        .await
        .expect("human input should continue once a response is present");
    assert_eq!(outputs.get("value"), Some(&serde_json::json!("approved")));

    let events = event_sink.events();
    assert!(matches!(
        events.as_slice(),
        [
            WorkflowEvent::TaskStarted { task_id, .. },
            WorkflowEvent::TaskCompleted {
                task_id: completed_task_id,
                ..
            }
        ] if task_id == "approval" && completed_task_id == "approval"
    ));
}
