//! Real owner and filesystem qualification; no browser, scheduler or inference.
use node_engine::{execute_core_task_once, NodeEngineSingleTaskRequest};
use pantograph_workflow_service::graph::{
    FileSystemWorkflowGraphStore, GraphEdge, GraphNode, GraphSessionStore, NodeGroup, Position,
    WorkflowGraph, WorkflowGraphCreateGroupRequest, WorkflowGraphEditSessionGraphRequest,
    WorkflowGraphStore, WorkflowGraphUngroupRequest, WorkflowGraphUpdateGroupNodeDataRequest,
};
use serde_json::{json, Value};
use std::collections::HashMap;

fn node(id: &str) -> GraphNode {
    GraphNode {
        id: id.into(),
        node_type: "json-filter".into(),
        position: Position::default(),
        data: json!({"path":"old", "label":id, "opaque":{"keep":true}}),
    }
}
fn graph() -> WorkflowGraph {
    let group = NodeGroup {
        id: "group".into(),
        name: "Extraction".into(),
        nodes: vec![node("extract"), node("sibling")],
        edges: vec![edge("inside", "extract", "sibling")],
        exposed_inputs: vec![],
        exposed_outputs: vec![],
        position: Position::default(),
        collapsed: true,
        description: Some("keep description".into()),
        color: None,
    };
    let mut value = serde_json::to_value(group).unwrap();
    value["opaque_group"] = json!({"preserve": "future extension"});
    value["nodes"][0]["opaque_node"] = json!([1, 2, 3]);
    value["edges"][0]["opaque_edge"] = json!(true);
    WorkflowGraph {
        nodes: vec![GraphNode {
            id: "group".into(),
            node_type: "node-group".into(),
            position: Position::default(),
            data: json!({"group":value,"label":"Extraction","isGroup":true,"opaque_wrapper":42}),
        }],
        ..WorkflowGraph::default()
    }
}
fn edge(id: &str, source: &str, target: &str) -> GraphEdge {
    GraphEdge {
        id: id.into(),
        source: source.into(),
        target: target.into(),
        source_handle: "value".into(),
        target_handle: "json".into(),
    }
}
fn request(
    session_id: &str,
    graph: &WorkflowGraph,
    path: &str,
) -> WorkflowGraphUpdateGroupNodeDataRequest {
    let internal = &graph.find_node("group").unwrap().data["group"]["nodes"][0];
    WorkflowGraphUpdateGroupNodeDataRequest {
        session_id: session_id.into(),
        group_id: "group".into(),
        node_id: "extract".into(),
        expected_node_type: "json-filter".into(),
        expected_node_data: internal["data"].clone(),
        data: json!({"path":path}),
    }
}

#[tokio::test]
async fn scoped_edit_preserves_opaque_group_tree_and_undo_redo() {
    let sessions = GraphSessionStore::new();
    let session = sessions.create_session(graph(), None).await;
    let before = sessions
        .get_session_graph(&session.session_id)
        .await
        .unwrap();
    let mut expected = before.graph.clone();
    expected.find_node_mut("group").unwrap().data["group"]["nodes"][0]["data"]["path"] =
        json!("results[0].text");
    let changed = sessions
        .update_group_node_data(request(
            &session.session_id,
            &before.graph,
            "results[0].text",
        ))
        .await
        .unwrap();
    assert_eq!(changed.graph.nodes, expected.nodes);
    assert_eq!(changed.graph.edges, expected.edges);
    assert_ne!(changed.graph_revision, before.graph_revision);
    let undone = sessions
        .undo(WorkflowGraphEditSessionGraphRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .unwrap();
    assert_eq!(undone.graph, before.graph);
    let redone = sessions
        .redo(WorkflowGraphEditSessionGraphRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .unwrap();
    assert_eq!(redone.graph, changed.graph);
}

#[tokio::test]
async fn public_group_create_edit_save_reopen_ungroup_and_core_extract() {
    let sessions = GraphSessionStore::new();
    let initial = WorkflowGraph {
        nodes: vec![
            node("before"),
            node("extract"),
            node("sibling"),
            node("after"),
        ],
        edges: vec![
            edge("in-boundary", "before", "extract"),
            edge("internal", "extract", "sibling"),
            edge("out-boundary", "sibling", "after"),
        ],
        ..WorkflowGraph::default()
    };
    let session = sessions.create_session(initial.clone(), None).await;
    let grouped = sessions
        .create_group(WorkflowGraphCreateGroupRequest {
            session_id: session.session_id.clone(),
            name: "Extraction".into(),
            selected_node_ids: vec!["extract".into(), "sibling".into()],
        })
        .await
        .unwrap();
    let wrapper = grouped
        .graph
        .nodes
        .iter()
        .find(|n| n.node_type == "node-group")
        .unwrap();
    let group_id = wrapper.id.clone();
    let before_group = wrapper.data["group"].clone();
    let expected_data = before_group["nodes"][0]["data"].clone();
    let path = "results[0].text";
    let changed = sessions
        .update_group_node_data(WorkflowGraphUpdateGroupNodeDataRequest {
            session_id: session.session_id.clone(),
            group_id: group_id.clone(),
            node_id: "extract".into(),
            expected_node_type: "json-filter".into(),
            expected_node_data: expected_data,
            data: json!({"path":path}),
        })
        .await
        .unwrap();
    let mut expected_group = before_group;
    expected_group["nodes"][0]["data"]["path"] = json!(path);
    assert_eq!(
        changed.graph.find_node(&group_id).unwrap().data["group"],
        expected_group
    );
    assert_eq!(changed.graph.edges, grouped.graph.edges);
    let directory = tempfile::tempdir().unwrap();
    let store = FileSystemWorkflowGraphStore::new(directory.path());
    let file = store
        .save_workflow("group-path".into(), changed.graph.clone())
        .unwrap();
    sessions.close_session(&session.session_id).await.unwrap();
    drop(sessions);
    drop(store);
    let cold_store = FileSystemWorkflowGraphStore::new(directory.path());
    let cold = cold_store.load_workflow(file).unwrap();
    assert_eq!(cold.graph, changed.graph);
    let cold_sessions = GraphSessionStore::new();
    let reopened = cold_sessions.create_session(cold.graph, None).await;
    let flat = cold_sessions
        .ungroup(WorkflowGraphUngroupRequest {
            session_id: reopened.session_id,
            group_id,
        })
        .await
        .unwrap()
        .graph;
    // Existing ungroup restores the exact original node/edge/handle identities.
    for original in &initial.nodes {
        let mut expected = original.clone();
        if expected.id == "extract" {
            expected.data["path"] = json!(path);
        }
        assert_eq!(flat.find_node(&original.id).unwrap(), &expected);
    }
    for original in &initial.edges {
        assert!(flat.edges.contains(original));
    }
    assert_eq!(flat.edges.len(), initial.edges.len());
    let target = flat.find_node("extract").unwrap();
    let text = "  exact prompt\nUnicode 🐾  ";
    let result = execute_core_task_once(
        NodeEngineSingleTaskRequest::try_new(
            &target.id,
            &target.node_type,
            HashMap::from([
                ("json".into(), json!({"results":[{"text":text}]})),
                ("_data".into(), target.data.clone()),
            ]),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(result.outputs()["value"], json!(text));
    assert_eq!(result.outputs()["found"], json!(true));
}

#[tokio::test]
async fn stale_target_data_and_type_refuse_without_graph_or_undo_changes() {
    let sessions = GraphSessionStore::new();
    let session = sessions.create_session(graph(), None).await;
    let before = sessions
        .get_session_graph(&session.session_id)
        .await
        .unwrap();
    let stale = request(&session.session_id, &before.graph, "stale");
    sessions
        .update_group_node_data(request(&session.session_id, &before.graph, "newer"))
        .await
        .unwrap();
    let current = sessions
        .get_session_graph(&session.session_id)
        .await
        .unwrap();
    let undo = sessions
        .get_undo_redo_state(&session.session_id)
        .await
        .unwrap();
    assert!(sessions.update_group_node_data(stale).await.is_err());
    let mut wrong_type = request(&session.session_id, &current.graph, "wrong");
    wrong_type.expected_node_type = "text-input".into();
    assert!(sessions.update_group_node_data(wrong_type).await.is_err());
    assert_eq!(
        sessions
            .get_session_graph(&session.session_id)
            .await
            .unwrap()
            .graph,
        current.graph
    );
    assert_eq!(
        sessions
            .get_undo_redo_state(&session.session_id)
            .await
            .unwrap(),
        undo
    );
}

#[tokio::test]
async fn unrelated_sibling_changes_are_preserved_by_scoped_guard() {
    let sessions = GraphSessionStore::new();
    let session = sessions.create_session(graph(), None).await;
    let before = sessions
        .get_session_graph(&session.session_id)
        .await
        .unwrap();
    let edit = request(&session.session_id, &before.graph, "edited");
    let sibling = &before.graph.find_node("group").unwrap().data["group"]["nodes"][1];
    sessions
        .update_group_node_data(WorkflowGraphUpdateGroupNodeDataRequest {
            session_id: session.session_id.clone(),
            group_id: "group".into(),
            node_id: "sibling".into(),
            expected_node_type: "json-filter".into(),
            expected_node_data: sibling["data"].clone(),
            data: json!({"path":"sibling concurrent"}),
        })
        .await
        .unwrap();
    let changed = sessions.update_group_node_data(edit).await.unwrap();
    assert_eq!(
        changed.graph.find_node("group").unwrap().data["group"]["nodes"][1]["data"]["path"],
        "sibling concurrent"
    );
}

#[tokio::test]
async fn malformed_ambiguous_missing_and_nested_targets_are_refused() {
    let mut cases = Vec::new();
    let mut g = graph();
    g.nodes[0].node_type = "json-filter".into();
    cases.push(g);
    let mut g = graph();
    g.nodes[0].data["group"]["id"] = json!("other");
    cases.push(g);
    let mut g = graph();
    let target = g.nodes[0].data["group"]["nodes"][0].clone();
    g.nodes[0].data["group"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(target);
    cases.push(g);
    let mut g = graph();
    g.nodes[0].data["group"]["nodes"][0]["id"] = json!("missing");
    cases.push(g);
    let mut g = graph();
    g.nodes[0].data["group"]["nodes"][0]["node_type"] = json!("node-group");
    cases.push(g);
    let mut g = graph();
    g.nodes[0].data["group"]["edges"] = Value::Null;
    cases.push(g);
    let mut g = graph();
    g.nodes.push(g.nodes[0].clone());
    cases.push(g);
    for g in cases {
        let sessions = GraphSessionStore::new();
        let session = sessions.create_session(g, None).await;
        let before = sessions
            .get_session_graph(&session.session_id)
            .await
            .unwrap();
        let undo = sessions
            .get_undo_redo_state(&session.session_id)
            .await
            .unwrap();
        assert!(sessions
            .update_group_node_data(request(&session.session_id, &before.graph, "refused"))
            .await
            .is_err());
        assert_eq!(
            sessions
                .get_session_graph(&session.session_id)
                .await
                .unwrap()
                .graph,
            before.graph
        );
        assert_eq!(
            sessions
                .get_undo_redo_state(&session.session_id)
                .await
                .unwrap(),
            undo
        );
    }
}

#[tokio::test]
async fn absent_null_and_non_object_saved_data_are_repaired_only_by_explicit_patch() {
    for data in [
        None,
        Some(Value::Null),
        Some(json!(42)),
        Some(json!([1, 2])),
    ] {
        let mut g = graph();
        let raw = g.nodes[0].data["group"]["nodes"][0]
            .as_object_mut()
            .unwrap();
        if let Some(data) = data {
            raw.insert("data".into(), data);
        } else {
            raw.remove("data");
        }
        let sessions = GraphSessionStore::new();
        let session = sessions.create_session(g, None).await;
        let before = sessions
            .get_session_graph(&session.session_id)
            .await
            .unwrap();
        let changed = sessions
            .update_group_node_data(request(&session.session_id, &before.graph, ""))
            .await
            .unwrap();
        assert_eq!(
            changed.graph.find_node("group").unwrap().data["group"]["nodes"][0]["data"],
            json!({"path":""})
        );
        assert_eq!(
            sessions
                .undo(WorkflowGraphEditSessionGraphRequest {
                    session_id: session.session_id
                })
                .await
                .unwrap()
                .graph,
            before.graph
        );
    }
}

#[tokio::test]
async fn sequence_shaped_group_or_inner_record_refuses_without_panicking_or_undo() {
    let mut group_sequence = graph();
    group_sequence.nodes[0].data["group"] =
        json!(["group","Extraction",[],[],[],[],{"x":0,"y":0},true,null,null]);
    let mut node_sequence = graph();
    node_sequence.nodes[0].data["group"]["nodes"][0] =
        json!(["extract","json-filter",{"x":0,"y":0},{"path":"old"}]);
    for g in [group_sequence, node_sequence] {
        let sessions = GraphSessionStore::new();
        let session = sessions.create_session(g, None).await;
        let before = sessions
            .get_session_graph(&session.session_id)
            .await
            .unwrap();
        let undo = sessions
            .get_undo_redo_state(&session.session_id)
            .await
            .unwrap();
        let request = WorkflowGraphUpdateGroupNodeDataRequest {
            session_id: session.session_id.clone(),
            group_id: "group".into(),
            node_id: "extract".into(),
            expected_node_type: "json-filter".into(),
            expected_node_data: json!({"path":"old"}),
            data: json!({"path":"new"}),
        };
        assert!(sessions.update_group_node_data(request).await.is_err());
        assert_eq!(
            sessions
                .get_session_graph(&session.session_id)
                .await
                .unwrap()
                .graph,
            before.graph
        );
        assert_eq!(
            sessions
                .get_undo_redo_state(&session.session_id)
                .await
                .unwrap(),
            undo
        );
    }
}
