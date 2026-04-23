use super::*;

#[tokio::test]
async fn test_workflow_executor_demand() {
    let graph = make_linear_graph();
    let event_sink = Arc::new(VecEventSink::new());
    let executor_impl = CountingExecutor::new();

    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink.clone());

    let result = workflow_executor
        .demand(&"c".to_string(), &executor_impl)
        .await;

    assert!(result.is_ok());
    assert_eq!(executor_impl.count(), 3);
}

#[tokio::test]
async fn test_workflow_executor_update_node() {
    let graph = make_linear_graph();
    let event_sink = Arc::new(NullEventSink);
    let executor_impl = CountingExecutor::new();

    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

    // First demand
    let _ = workflow_executor
        .demand(&"c".to_string(), &executor_impl)
        .await;
    let count_after_first = executor_impl.count();
    assert_eq!(count_after_first, 3);

    // Verify caching works - demand again without modification
    let _ = workflow_executor
        .demand(&"c".to_string(), &executor_impl)
        .await;
    assert_eq!(executor_impl.count(), 3); // No additional executions

    // Update node 'a' data - this marks it as modified
    workflow_executor
        .update_node_data(&"a".to_string(), serde_json::json!({"new": "data"}))
        .await
        .unwrap();

    // Demand again - should recompute the chain
    let _ = workflow_executor
        .demand(&"c".to_string(), &executor_impl)
        .await;

    // After marking 'a' as modified, we expect recomputation of the entire chain
    // because b depends on a, and c depends on b
    let count_after_update = executor_impl.count();
    assert!(
        count_after_update > count_after_first,
        "Expected recomputation after update: got {} executions (expected > {})",
        count_after_update,
        count_after_first
    );
}

#[tokio::test]
async fn test_workflow_executor_mark_modified_emits_graph_modified_with_dirty_subgraph() {
    let graph = make_linear_graph();
    let event_sink = Arc::new(VecEventSink::new());
    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink.clone());

    workflow_executor.mark_modified(&"b".to_string()).await;

    let events = event_sink.events();
    let graph_modified = events
        .iter()
        .find(|event| matches!(event, WorkflowEvent::GraphModified { .. }))
        .expect("graph modified event");

    match graph_modified {
        WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            ..
        } => {
            assert_eq!(workflow_id, "test");
            assert_eq!(execution_id, "exec_1");
            assert_eq!(dirty_tasks, &vec!["b".to_string(), "c".to_string()]);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn test_workflow_executor_demand_multiple_emits_incremental_execution_started() {
    let graph = make_linear_graph();
    let event_sink = Arc::new(VecEventSink::new());
    let executor_impl = CountingExecutor::new();
    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink.clone());

    let _ = workflow_executor
        .demand_multiple(&["b".to_string(), "c".to_string()], &executor_impl)
        .await
        .expect("incremental demand succeeds");

    let events = event_sink.events();
    let incremental_started = events
        .iter()
        .find(|event| matches!(event, WorkflowEvent::IncrementalExecutionStarted { .. }))
        .expect("incremental execution event");

    match incremental_started {
        WorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            execution_id,
            tasks,
            ..
        } => {
            assert_eq!(workflow_id, "test");
            assert_eq!(execution_id, "exec_1");
            assert_eq!(tasks, &vec!["b".to_string(), "c".to_string()]);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn test_workflow_executor_demand_multiple_emits_task_lifecycle_for_parallel_roots() {
    let graph = make_parallel_roots_graph();
    let event_sink = Arc::new(VecEventSink::new());
    let executor_impl = YieldingExecutor::new();
    let workflow_executor = WorkflowExecutor::new("exec_parallel", graph, event_sink.clone());

    let outputs = workflow_executor
        .demand_multiple(&["left".to_string(), "right".to_string()], &executor_impl)
        .await
        .expect("parallel incremental demand succeeds");

    assert_eq!(executor_impl.max_in_flight(), 2);
    assert_eq!(outputs.len(), 2);

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
    assert_eq!(
        completed_tasks,
        HashSet::from(["left".to_string(), "right".to_string()])
    );
}
