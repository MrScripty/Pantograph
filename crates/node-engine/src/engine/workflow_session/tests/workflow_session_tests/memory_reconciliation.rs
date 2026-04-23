use super::*;
use crate::{NodeMemoryIdentity, NodeMemoryStatus};

#[tokio::test]
async fn rerun_can_consume_prior_node_memory_from_the_bound_workflow_session() {
    let executor = WorkflowExecutor::new("exec-1", single_node_graph(), Arc::new(NullEventSink));
    let task_executor = MemoryConsumingTaskExecutor::new();
    bind_workflow_session(&executor, "session-1").await;

    let first_run = executor
        .demand(&"memory".to_string(), &task_executor)
        .await
        .expect("run first memory demand");
    assert_eq!(
        first_run.get("value"),
        Some(&serde_json::json!({
            "sequence": 1,
            "previous_output": null,
            "memory_status": null,
        }))
    );

    executor.mark_modified(&"memory".to_string()).await;
    let second_run = executor
        .demand(&"memory".to_string(), &task_executor)
        .await
        .expect("run second memory demand");
    assert_eq!(
        second_run.get("value"),
        Some(&serde_json::json!({
            "sequence": 2,
            "previous_output": {
                "out": {
                    "sequence": 1,
                    "previous_output": null,
                    "memory_status": null,
                },
                "value": {
                    "sequence": 1,
                    "previous_output": null,
                    "memory_status": null,
                }
            },
            "memory_status": "ready",
        }))
    );
}

#[tokio::test]
async fn workflow_session_helpers_reconcile_recorded_node_memory() {
    let executor = WorkflowExecutor::new("exec-1", linear_graph(), Arc::new(NullEventSink));
    bind_workflow_session(&executor, "session-1").await;
    record_workflow_session_node_memory(
        &executor,
        NodeMemorySnapshot {
            identity: NodeMemoryIdentity {
                session_id: "session-1".to_string(),
                node_id: "b".to_string(),
                node_type: "process".to_string(),
                schema_version: Some("v1".to_string()),
            },
            status: NodeMemoryStatus::Ready,
            input_fingerprint: Some("fp-b".to_string()),
            output_snapshot: Some(serde_json::json!({ "out": "b" })),
            private_state: None,
            indirect_state_reference: None,
            inspection_metadata: None,
        },
    )
    .await;

    reconcile_workflow_session_node_memory(
        &executor,
        "session-1",
        &GraphMemoryImpactSummary {
            node_decisions: vec![NodeMemoryCompatibilitySnapshot {
                node_id: "b".to_string(),
                compatibility: NodeMemoryCompatibility::PreserveWithInputRefresh,
                reason: Some("upstream_dependency_changed".to_string()),
            }],
            fallback_to_full_invalidation: false,
        },
    )
    .await;

    let snapshots = workflow_session_node_memory_snapshots(&executor, "session-1").await;
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].identity.node_id, "b");
    assert_eq!(snapshots[0].status, NodeMemoryStatus::Invalidated);
}
