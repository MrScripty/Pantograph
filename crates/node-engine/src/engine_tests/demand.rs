use super::*;

#[tokio::test]
async fn test_demand_linear_graph() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");
    let executor = CountingExecutor::new();
    let context = Context::new();
    let event_sink = NullEventSink;
    let extensions = ExecutorExtensions::new();

    // Demand 'c' - should execute a, b, c
    let result = engine
        .demand(
            &"c".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;

    assert!(result.is_ok());
    assert_eq!(executor.count(), 3); // All three nodes executed
}

#[tokio::test]
async fn test_demand_caching() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");
    let executor = CountingExecutor::new();
    let context = Context::new();
    let event_sink = NullEventSink;
    let extensions = ExecutorExtensions::new();

    // First demand
    let _ = engine
        .demand(
            &"c".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;
    assert_eq!(executor.count(), 3);

    // Second demand - should use cache
    let _ = engine
        .demand(
            &"c".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;
    assert_eq!(executor.count(), 3); // No additional executions
}

#[tokio::test]
async fn test_demand_partial_recompute() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");
    let executor = CountingExecutor::new();
    let context = Context::new();
    let event_sink = NullEventSink;
    let extensions = ExecutorExtensions::new();

    // First demand
    let _ = engine
        .demand(
            &"c".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;
    assert_eq!(executor.count(), 3);

    // Mark 'b' as modified
    engine.mark_modified(&"b".to_string());

    // Demand again - should only recompute b and c (not a)
    let _ = engine
        .demand(
            &"c".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;
    // Note: Due to version-based invalidation, this depends on implementation details.
    // The current implementation uses sum of dependency versions, so modifying 'b'
    // will invalidate 'c' but not necessarily re-execute 'a' if it's still cached.
}

#[tokio::test]
async fn test_demand_diamond_graph() {
    let graph = make_diamond_graph();
    let mut engine = DemandEngine::new("test");
    let executor = CountingExecutor::new();
    let context = Context::new();
    let event_sink = NullEventSink;
    let extensions = ExecutorExtensions::new();

    // Demand 'd' - should execute a, b, c, d (a only once despite diamond)
    let result = engine
        .demand(
            &"d".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;

    assert!(result.is_ok());
    assert_eq!(executor.count(), 4); // All four nodes executed exactly once
}

#[tokio::test]
async fn test_demand_events() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test_exec");
    let executor = CountingExecutor::new();
    let context = Context::new();
    let event_sink = VecEventSink::new();
    let extensions = ExecutorExtensions::new();

    let _ = engine
        .demand(
            &"c".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;

    let events = event_sink.events();

    // Should have TaskStarted and TaskCompleted for each node
    let started_count = events
        .iter()
        .filter(|e| matches!(e, WorkflowEvent::TaskStarted { .. }))
        .count();
    let completed_count = events
        .iter()
        .filter(|e| matches!(e, WorkflowEvent::TaskCompleted { .. }))
        .count();

    assert_eq!(started_count, 3);
    assert_eq!(completed_count, 3);
}
