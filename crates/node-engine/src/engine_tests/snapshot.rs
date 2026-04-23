use super::*;

#[tokio::test]
async fn test_workflow_executor_snapshot() {
    let graph = make_linear_graph();
    let event_sink = Arc::new(NullEventSink);

    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

    // Get initial snapshot
    let snapshot = workflow_executor.get_graph_snapshot().await;
    assert_eq!(snapshot.nodes.len(), 3);

    // Add a new node
    workflow_executor
        .add_node(GraphNode {
            id: "d".to_string(),
            node_type: "new".to_string(),
            data: serde_json::Value::Null,
            position: (300.0, 0.0),
        })
        .await;

    // Verify node was added
    let updated = workflow_executor.get_graph_snapshot().await;
    assert_eq!(updated.nodes.len(), 4);

    // Restore original snapshot
    workflow_executor.restore_graph_snapshot(snapshot).await;

    // Verify restoration
    let restored = workflow_executor.get_graph_snapshot().await;
    assert_eq!(restored.nodes.len(), 3);
}
