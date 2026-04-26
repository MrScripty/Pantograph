use super::super::types::InsertNodePositionHint;
use super::*;
use crate::graph::types::{ConnectionAnchor, GraphNode, Position};
use crate::graph::{
    WorkflowGraphDeleteSelectionRequest, WorkflowGraphEditSessionGraphRequest,
    WorkflowGraphRemoveEdgesRequest,
};
use crate::{
    WorkflowExecutionSessionQueueItemStatus, WorkflowGraphRemoveNodeRequest,
    WorkflowGraphUpdateNodeDataRequest, WorkflowGraphUpdateNodePositionRequest,
};

fn sample_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "text-input".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({
                    "label": "Text Input",
                    "text": "hello",
                    "definition": {
                        "node_type": "text-input"
                    }
                }),
            },
            GraphNode {
                id: "text-output".to_string(),
                node_type: "text-output".to_string(),
                position: Position { x: 120.0, y: 0.0 },
                data: serde_json::json!({
                    "label": "Text Output"
                }),
            },
        ],
        edges: vec![GraphEdge {
            id: "text-input-text-text-output-text".to_string(),
            source: "text-input".to_string(),
            source_handle: "text".to_string(),
            target: "text-output".to_string(),
            target_handle: "text".to_string(),
        }],
        derived_graph: None,
    }
}

fn disconnected_graph() -> WorkflowGraph {
    let mut graph = sample_graph();
    graph.edges.clear();
    graph
}

fn branching_graph() -> WorkflowGraph {
    let mut graph = sample_graph();
    graph.nodes.push(GraphNode {
        id: "text-copy".to_string(),
        node_type: "text-output".to_string(),
        position: Position { x: 120.0, y: 80.0 },
        data: serde_json::json!({
            "label": "Text Copy"
        }),
    });
    graph.edges.push(GraphEdge {
        id: "text-input-text-text-copy-text".to_string(),
        source: "text-input".to_string(),
        source_handle: "text".to_string(),
        target: "text-copy".to_string(),
        target_handle: "text".to_string(),
    });
    graph
}

#[tokio::test]
async fn create_session_returns_backend_owned_edit_kind() {
    let store = GraphSessionStore::new();

    let session = store.create_session(sample_graph(), None).await;

    assert_eq!(session.session_kind, WorkflowExecutionSessionKind::Edit);
    assert!(!session.session_id.is_empty());
    assert!(!session.graph_revision.is_empty());
}

#[tokio::test]
async fn scheduler_snapshot_preserves_source_workflow_id_for_loaded_edit_session() {
    let store = GraphSessionStore::new();

    let session = store
        .create_session(sample_graph(), Some("saved-flow".to_string()))
        .await;

    let snapshot = store
        .get_scheduler_snapshot(&session.session_id)
        .await
        .expect("scheduler snapshot");

    assert_eq!(snapshot.workflow_id, None);
    assert_eq!(snapshot.session.session_id, session.session_id);
    assert_eq!(snapshot.session.workflow_id, "saved-flow");
}

#[tokio::test]
async fn scheduler_snapshot_tracks_running_edit_session_queue_item() {
    let store = GraphSessionStore::new();

    let session = store.create_session(sample_graph(), None).await;
    store
        .mark_running(&session.session_id, "run-1")
        .await
        .expect("mark running");

    let running_snapshot = store
        .get_scheduler_snapshot(&session.session_id)
        .await
        .expect("running scheduler snapshot");

    assert_eq!(running_snapshot.session.queued_runs, 1);
    assert_eq!(running_snapshot.items.len(), 1);
    assert_eq!(running_snapshot.items[0].workflow_run_id, "run-1");
    assert_eq!(running_snapshot.workflow_run_id.as_deref(), Some("run-1"));
    assert_eq!(
        running_snapshot.items[0].status,
        WorkflowExecutionSessionQueueItemStatus::Running
    );

    store
        .finish_run(&session.session_id)
        .await
        .expect("finish run");
    let finished_snapshot = store
        .get_scheduler_snapshot(&session.session_id)
        .await
        .expect("finished scheduler snapshot");

    assert_eq!(finished_snapshot.session.queued_runs, 0);
    assert_eq!(finished_snapshot.session.run_count, 1);
    assert!(finished_snapshot.items.is_empty());
}

#[tokio::test]
async fn update_node_data_merges_patch_into_existing_data() {
    let store = GraphSessionStore::new();
    let session = store.create_session(sample_graph(), None).await;

    let response = store
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: session.session_id.clone(),
            node_id: "text-input".to_string(),
            data: serde_json::json!({
                "text": "updated",
                "placeholder": "Prompt"
            }),
        })
        .await
        .expect("update node data");

    let node = response
        .graph
        .find_node("text-input")
        .expect("text-input node");
    assert_eq!(node.data["text"], "updated");
    assert_eq!(node.data["placeholder"], "Prompt");
    assert_eq!(node.data["label"], "Text Input");
    assert!(node.data.get("definition").is_some());
    assert!(matches!(
        response.workflow_event.as_ref(),
        Some(node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            ..
        }) if workflow_id == &session.session_id
            && execution_id == &session.session_id
            && dirty_tasks == &vec!["text-input".to_string(), "text-output".to_string()]
    ));
    let memory_impact = response
        .workflow_execution_session_state
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert!(!memory_impact.fallback_to_full_invalidation);
    assert_eq!(memory_impact.node_decisions.len(), 2);
    assert!(matches!(
        memory_impact.node_decisions.as_slice(),
        [
            node_engine::NodeMemoryCompatibilitySnapshot {
                node_id,
                compatibility,
                reason: Some(reason),
            },
            node_engine::NodeMemoryCompatibilitySnapshot {
                node_id: dependent_node_id,
                compatibility: dependent_compatibility,
                reason: Some(dependent_reason),
            }
        ] if node_id == "text-input"
            && *compatibility == node_engine::NodeMemoryCompatibility::PreserveWithInputRefresh
            && reason == "node_data_changed"
            && dependent_node_id == "text-output"
            && *dependent_compatibility
                == node_engine::NodeMemoryCompatibility::PreserveWithInputRefresh
            && dependent_reason == "upstream_dependency_changed"
    ));
    assert!(matches!(
        response.workflow_event.as_ref(),
        Some(node_engine::WorkflowEvent::GraphModified {
            memory_impact: Some(memory_impact),
            ..
        }) if memory_impact.node_decisions.len() == 2
    ));
}

#[tokio::test]
async fn update_node_position_updates_session_graph() {
    let store = GraphSessionStore::new();
    let session = store.create_session(sample_graph(), None).await;

    let response = store
        .update_node_position(WorkflowGraphUpdateNodePositionRequest {
            session_id: session.session_id.clone(),
            node_id: "text-output".to_string(),
            position: Position { x: 320.0, y: 48.0 },
        })
        .await
        .expect("update node position");

    let node = response
        .graph
        .find_node("text-output")
        .expect("text-output node");
    assert_eq!(node.position, Position { x: 320.0, y: 48.0 });
    assert!(matches!(
        response.workflow_event,
        Some(node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            ..
        }) if workflow_id == session.session_id
            && execution_id == session.session_id
            && dirty_tasks.is_empty()
    ));
    assert_eq!(
        response
            .workflow_execution_session_state
            .expect("workflow execution session state")
            .memory_impact,
        None
    );
}

#[tokio::test]
async fn remove_node_prunes_attached_edges() {
    let store = GraphSessionStore::new();
    let session = store.create_session(sample_graph(), None).await;

    let response = store
        .remove_node(WorkflowGraphRemoveNodeRequest {
            session_id: session.session_id.clone(),
            node_id: "text-output".to_string(),
        })
        .await
        .expect("remove node");

    assert!(response.graph.find_node("text-output").is_none());
    assert!(response.graph.edges.is_empty());
    assert!(matches!(
        response.workflow_event.as_ref(),
        Some(node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            ..
        }) if workflow_id == &session.session_id
            && execution_id == &session.session_id
            && dirty_tasks == &vec!["text-output".to_string()]
    ));
    let memory_impact = response
        .workflow_execution_session_state
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert_eq!(memory_impact.node_decisions.len(), 1);
    assert_eq!(
        memory_impact.node_decisions[0].compatibility,
        node_engine::NodeMemoryCompatibility::DropOnIdentityChange
    );
    assert_eq!(
        memory_impact.node_decisions[0].reason.as_deref(),
        Some("node_removed")
    );
}

#[tokio::test]
async fn remove_edges_removes_multiple_edges_with_one_undo_snapshot() {
    let store = GraphSessionStore::new();
    let session = store.create_session(branching_graph(), None).await;

    let response = store
        .remove_edges(WorkflowGraphRemoveEdgesRequest {
            session_id: session.session_id.clone(),
            edge_ids: vec![
                "text-input-text-text-output-text".to_string(),
                "text-input-text-text-copy-text".to_string(),
            ],
        })
        .await
        .expect("remove edges");

    assert!(response.graph.edges.is_empty());
    let undo_state = store
        .get_undo_redo_state(&session.session_id)
        .await
        .expect("undo state");
    assert_eq!(undo_state.undo_count, 1);

    let undo_response = store
        .undo(WorkflowGraphEditSessionGraphRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("undo remove edges");
    assert_eq!(undo_response.graph.edges.len(), 2);
}

#[tokio::test]
async fn delete_selection_removes_mixed_selection_with_one_undo_snapshot() {
    let store = GraphSessionStore::new();
    let session = store.create_session(branching_graph(), None).await;

    let response = store
        .delete_selection(WorkflowGraphDeleteSelectionRequest {
            session_id: session.session_id.clone(),
            node_ids: vec!["text-copy".to_string()],
            edge_ids: vec!["text-input-text-text-output-text".to_string()],
        })
        .await
        .expect("delete selection");

    assert!(response.graph.find_node("text-copy").is_none());
    assert!(response.graph.edges.is_empty());
    let undo_state = store
        .get_undo_redo_state(&session.session_id)
        .await
        .expect("undo state");
    assert_eq!(undo_state.undo_count, 1);

    let undo_response = store
        .undo(WorkflowGraphEditSessionGraphRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("undo delete selection");
    assert!(undo_response.graph.find_node("text-copy").is_some());
    assert_eq!(undo_response.graph.edges.len(), 2);
}

#[tokio::test]
async fn undo_response_carries_backend_owned_graph_modified_event() {
    let store = GraphSessionStore::new();
    let session = store.create_session(sample_graph(), None).await;

    store
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: session.session_id.clone(),
            node_id: "text-input".to_string(),
            data: serde_json::json!({
                "text": "updated"
            }),
        })
        .await
        .expect("update node data");

    let response = store
        .undo(WorkflowGraphEditSessionGraphRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("undo graph edit");

    assert!(matches!(
        response.workflow_event,
        Some(node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            ..
        }) if workflow_id == session.session_id
            && execution_id == session.session_id
            && dirty_tasks == vec!["text-input".to_string(), "text-output".to_string()]
    ));
}

#[tokio::test]
async fn get_session_graph_replays_last_memory_impact_until_a_non_invalidating_edit_clears_it() {
    let store = GraphSessionStore::new();
    let session = store.create_session(sample_graph(), None).await;

    store
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: session.session_id.clone(),
            node_id: "text-input".to_string(),
            data: serde_json::json!({
                "text": "updated"
            }),
        })
        .await
        .expect("update node data");

    let after_data_edit = store
        .get_session_graph(&session.session_id)
        .await
        .expect("get session graph after data edit");
    let memory_impact = after_data_edit
        .workflow_execution_session_state
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert_eq!(memory_impact.node_decisions.len(), 2);
    assert!(!memory_impact.fallback_to_full_invalidation);

    store
        .update_node_position(WorkflowGraphUpdateNodePositionRequest {
            session_id: session.session_id.clone(),
            node_id: "text-output".to_string(),
            position: Position { x: 240.0, y: 32.0 },
        })
        .await
        .expect("update node position");

    let after_position_edit = store
        .get_session_graph(&session.session_id)
        .await
        .expect("get session graph after position edit");
    assert_eq!(
        after_position_edit
            .workflow_execution_session_state
            .expect("workflow execution session state")
            .memory_impact,
        None
    );
}

#[tokio::test]
async fn insert_node_on_edge_replaces_original_edge_in_session_graph() {
    let store = GraphSessionStore::new();
    let session = store.create_session(sample_graph(), None).await;
    let session_id = session.session_id.clone();

    let response = store
        .insert_node_on_edge(WorkflowGraphInsertNodeOnEdgeRequest {
            session_id: session_id.clone(),
            edge_id: "text-input-text-text-output-text".to_string(),
            node_type: "llm-inference".to_string(),
            graph_revision: session.graph_revision,
            position_hint: InsertNodePositionHint {
                position: Position { x: 80.0, y: 24.0 },
            },
        })
        .await
        .expect("insert node on edge");

    assert!(response.accepted);
    let graph = response.graph.expect("updated graph");
    assert_eq!(graph.edges.len(), 2);
    assert!(
        graph
            .edges
            .iter()
            .all(|edge| edge.id != "text-input-text-text-output-text")
    );
    let inserted_node_id = response.inserted_node_id.expect("inserted node id");
    assert!(graph.find_node(&inserted_node_id).is_some());
    assert!(matches!(
        response.workflow_event,
        Some(node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            ..
        }) if workflow_id == session_id && execution_id == session_id
    ));
    let response_memory_impact = response
        .workflow_execution_session_state
        .clone()
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert!(
        response_memory_impact
            .node_decisions
            .iter()
            .any(|decision| decision.node_id == inserted_node_id)
    );

    let snapshot = store
        .get_session_graph(&session.session_id)
        .await
        .expect("get session graph after insert");
    let memory_impact = snapshot
        .workflow_execution_session_state
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert!(!memory_impact.node_decisions.is_empty());
    assert!(
        memory_impact
            .node_decisions
            .iter()
            .any(|decision| decision.node_id == inserted_node_id)
    );
}

#[tokio::test]
async fn connect_persists_memory_impact_for_later_session_snapshot() {
    let store = GraphSessionStore::new();
    let session = store.create_session(disconnected_graph(), None).await;

    let response = store
        .connect(WorkflowGraphConnectRequest {
            session_id: session.session_id.clone(),
            graph_revision: session.graph_revision,
            source_anchor: ConnectionAnchor {
                node_id: "text-input".to_string(),
                port_id: "text".to_string(),
            },
            target_anchor: ConnectionAnchor {
                node_id: "text-output".to_string(),
                port_id: "text".to_string(),
            },
        })
        .await
        .expect("connect nodes");
    assert!(response.accepted);
    assert!(matches!(
        response.workflow_event.as_ref(),
        Some(node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            ..
        }) if workflow_id == &session.session_id
            && execution_id == &session.session_id
            && dirty_tasks == &vec!["text-output".to_string()]
    ));
    assert!(matches!(
        response.workflow_event.as_ref(),
        Some(node_engine::WorkflowEvent::GraphModified {
            memory_impact: Some(memory_impact),
            ..
        }) if !memory_impact.node_decisions.is_empty()
    ));
    let response_memory_impact = response
        .workflow_execution_session_state
        .clone()
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert_eq!(response_memory_impact.node_decisions.len(), 1);
    assert_eq!(
        response_memory_impact.node_decisions[0].node_id,
        "text-output"
    );

    let snapshot = store
        .get_session_graph(&session.session_id)
        .await
        .expect("get session graph after connect");
    let memory_impact = snapshot
        .workflow_execution_session_state
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert_eq!(memory_impact.node_decisions.len(), 1);
    assert_eq!(memory_impact.node_decisions[0].node_id, "text-output");
    assert_eq!(
        memory_impact.node_decisions[0].compatibility,
        node_engine::NodeMemoryCompatibility::PreserveWithInputRefresh
    );
    assert_eq!(
        memory_impact.node_decisions[0].reason.as_deref(),
        Some("edge_topology_changed")
    );
}
