use super::*;

#[tokio::test]
async fn test_workflow_executor_demand_multiple_preserves_parallel_failure_event_attribution() {
    let graph = make_parallel_roots_graph();
    let event_sink = Arc::new(VecEventSink::new());
    let executor_impl = YieldingFailingExecutor::new("right");
    let workflow_executor = WorkflowExecutor::new("exec_parallel", graph, event_sink.clone());

    let error = workflow_executor
        .demand_multiple(&["left".to_string(), "right".to_string()], &executor_impl)
        .await
        .expect_err("parallel incremental demand should fail");

    assert!(
        matches!(error, NodeEngineError::ExecutionFailed(message) if message.contains("forced failure at right"))
    );
    assert_eq!(executor_impl.max_in_flight(), 2);

    let events = event_sink.events();
    let started_tasks = events
        .iter()
        .filter_map(|event| match event {
            WorkflowEvent::TaskStarted { task_id, .. } => Some(task_id.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    let completed_tasks = events
        .iter()
        .filter_map(|event| match event {
            WorkflowEvent::TaskCompleted { task_id, .. } => Some(task_id.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    assert_eq!(
        started_tasks,
        HashSet::from(["left".to_string(), "right".to_string()])
    );
    assert_eq!(completed_tasks, HashSet::from(["left".to_string()]));
}

#[tokio::test]
async fn test_workflow_executor_demand_multiple_preserves_parallel_waiting_event_attribution() {
    let graph = make_parallel_roots_graph();
    let event_sink = Arc::new(VecEventSink::new());
    let executor_impl = YieldingWaitingExecutor::new("right");
    let workflow_executor = WorkflowExecutor::new("exec_parallel", graph, event_sink.clone());

    let error = workflow_executor
        .demand_multiple(&["left".to_string(), "right".to_string()], &executor_impl)
        .await
        .expect_err("parallel incremental demand should pause");

    assert!(matches!(
        error,
        NodeEngineError::WaitingForInput {
            task_id,
            prompt: Some(prompt)
        } if task_id == "right" && prompt == "waiting at right"
    ));
    assert_eq!(executor_impl.max_in_flight(), 2);

    let events = event_sink.events();
    let started_tasks = events
        .iter()
        .filter_map(|event| match event {
            WorkflowEvent::TaskStarted { task_id, .. } => Some(task_id.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    let completed_tasks = events
        .iter()
        .filter_map(|event| match event {
            WorkflowEvent::TaskCompleted { task_id, .. } => Some(task_id.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    let waiting_tasks = events
        .iter()
        .filter_map(|event| match event {
            WorkflowEvent::WaitingForInput { task_id, .. } => Some(task_id.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    assert_eq!(
        started_tasks,
        HashSet::from(["left".to_string(), "right".to_string()])
    );
    assert_eq!(completed_tasks, HashSet::from(["left".to_string()]));
    assert_eq!(waiting_tasks, HashSet::from(["right".to_string()]));
}

#[tokio::test]
async fn test_workflow_executor_demand_multiple_returns_redundant_requested_target_outputs() {
    let graph = make_linear_graph();
    let event_sink = Arc::new(NullEventSink);
    let executor_impl = CountingExecutor::new();
    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

    let outputs = workflow_executor
        .demand_multiple(&["b".to_string(), "c".to_string()], &executor_impl)
        .await
        .expect("incremental demand succeeds");

    assert_eq!(executor_impl.count(), 3);
    assert!(outputs.contains_key("b"));
    assert!(outputs.contains_key("c"));
    assert_eq!(outputs["b"]["out"]["task"], serde_json::json!("b"));
    assert_eq!(outputs["c"]["out"]["task"], serde_json::json!("c"));
}

#[tokio::test]
async fn test_workflow_executor_demand_multiple_stops_after_failed_batch() {
    let graph = make_shared_dependency_graph();
    let event_sink = Arc::new(NullEventSink);
    let executor_impl = FailingExecutor::new("b");
    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

    let error = workflow_executor
        .demand_multiple(&["b".to_string(), "c".to_string()], &executor_impl)
        .await
        .expect_err("first batch should fail");

    assert!(
        matches!(error, NodeEngineError::ExecutionFailed(message) if message.contains("forced failure at b"))
    );
    assert_eq!(
        executor_impl.executed_tasks(),
        vec!["a".to_string(), "b".to_string()]
    );
}

#[tokio::test]
async fn test_workflow_executor_demand_multiple_stops_after_waiting_batch() {
    let graph = make_shared_dependency_graph();
    let event_sink = Arc::new(NullEventSink);
    let executor_impl = WaitingExecutor::new("b");
    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

    let error = workflow_executor
        .demand_multiple(&["b".to_string(), "c".to_string()], &executor_impl)
        .await
        .expect_err("first batch should pause execution");

    assert!(matches!(
        error,
        NodeEngineError::WaitingForInput {
            task_id,
            prompt: Some(prompt)
        } if task_id == "b" && prompt == "waiting at b"
    ));
    assert_eq!(
        executor_impl.executed_tasks(),
        vec!["a".to_string(), "b".to_string()]
    );
}
