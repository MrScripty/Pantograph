//! Public authoring/persistence regression for the desktop JSON Filter editor.
//! Uses synthetic structured input and actual core extraction, without inference.

use std::collections::HashMap;

use node_engine::{execute_core_task_once, NodeEngineSingleTaskRequest};
use pantograph_workflow_service::graph::{
    FileSystemWorkflowGraphStore, GraphNode, GraphSessionStore, NodeRegistry, Position,
    WorkflowGraph, WorkflowGraphEditSessionGraphRequest, WorkflowGraphStore,
    WorkflowGraphUpdateNodeDataRequest,
};
use serde_json::{json, Value};

fn filter_graph(path: Value) -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![GraphNode {
            id: "extract".into(),
            node_type: "json-filter".into(),
            position: Position::default(),
            data: json!({"path": path, "label": "Prompt from structured output", "unrelated": {"retain": true}}),
        }],
        ..WorkflowGraph::default()
    }
}

async fn extract(graph: &WorkflowGraph, value: Value) -> (Value, Value) {
    let node = graph.find_node("extract").unwrap();
    let request = NodeEngineSingleTaskRequest::try_new(
        &node.id,
        &node.node_type,
        HashMap::from([("json".into(), value), ("_data".into(), node.data.clone())]),
    )
    .unwrap();
    let result = execute_core_task_once(request).await.unwrap();
    (
        result.outputs()["value"].clone(),
        result.outputs()["found"].clone(),
    )
}

#[tokio::test]
async fn path_only_edit_survives_real_file_reopen_undo_redo_and_extracts_exact_nested_text() {
    let directory = tempfile::tempdir().unwrap();
    let sessions = GraphSessionStore::new();
    let session = sessions
        .create_session(filter_graph(json!("original")), None)
        .await;
    let path = "results[0].document.text";
    let changed = sessions
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: session.session_id.clone(),
            node_id: "extract".into(),
            data: json!({"path": path}),
        })
        .await
        .unwrap();
    assert_ne!(changed.graph_revision, session.graph_revision);
    assert_eq!(
        changed.graph.find_node("extract").unwrap().data["unrelated"],
        json!({"retain": true})
    );
    let store = FileSystemWorkflowGraphStore::new(directory.path());
    let file = store
        .save_workflow("nested-prompt".into(), changed.graph)
        .unwrap();
    sessions.close_session(&session.session_id).await.unwrap();
    drop(sessions);
    drop(store);

    // Reopen with entirely new persistence/session owners, never an in-memory
    // serialization roundtrip pretending to be file persistence.
    let cold_store = FileSystemWorkflowGraphStore::new(directory.path());
    let cold = cold_store.load_workflow(file.clone()).unwrap();
    assert_eq!(cold.graph.find_node("extract").unwrap().data["path"], path);
    let registry = NodeRegistry::new();
    let definition = registry.get_definition("json-filter").unwrap();
    assert_eq!(definition.inputs[0].id, "json");
    assert_eq!(
        definition
            .outputs
            .iter()
            .map(|port| port.id.as_str())
            .collect::<Vec<_>>(),
        vec!["value", "found"]
    );
    let text = "  exact generated prompt\n<literal text> 🐾  ";
    assert_eq!(
        extract(
            &cold.graph,
            json!({"results": [{"document": {"text": text}}]})
        )
        .await,
        (json!(text), json!(true))
    );
    assert_eq!(
        extract(&cold.graph, json!({"results": []})).await,
        (Value::Null, json!(false))
    );

    let reopened = GraphSessionStore::new();
    let edit = reopened.create_session(cold.graph, None).await;
    let replace = reopened
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: edit.session_id.clone(),
            node_id: "extract".into(),
            data: json!({"path": ""}),
        })
        .await
        .unwrap();
    let structured = json!({"zero": 0, "false": false, "null": null, "items": [1, 2]});
    assert_eq!(
        extract(&replace.graph, structured.clone()).await,
        (structured, json!(true))
    );
    let undo = reopened
        .undo(WorkflowGraphEditSessionGraphRequest {
            session_id: edit.session_id.clone(),
        })
        .await
        .unwrap();
    assert_eq!(undo.graph.find_node("extract").unwrap().data["path"], path);
    let redo = reopened
        .redo(WorkflowGraphEditSessionGraphRequest {
            session_id: edit.session_id.clone(),
        })
        .await
        .unwrap();
    assert_eq!(redo.graph.find_node("extract").unwrap().data["path"], "");
    cold_store
        .save_workflow("nested-prompt".into(), redo.graph)
        .unwrap();
    assert_eq!(
        FileSystemWorkflowGraphStore::new(directory.path())
            .load_workflow(file)
            .unwrap()
            .graph
            .find_node("extract")
            .unwrap()
            .data["path"],
        ""
    );
    reopened.close_session(&edit.session_id).await.unwrap();
}

#[tokio::test]
async fn exact_path_bytes_and_malformed_saved_type_are_preserved_until_an_explicit_repair() {
    let directory = tempfile::tempdir().unwrap();
    let store = FileSystemWorkflowGraphStore::new(directory.path());
    let path = "  literal.field\n";
    let file = store
        .save_workflow("exact-path".into(), filter_graph(json!(path)))
        .unwrap();
    let loaded = FileSystemWorkflowGraphStore::new(directory.path())
        .load_workflow(file)
        .unwrap();
    assert_eq!(
        loaded.graph.find_node("extract").unwrap().data["path"],
        path
    );
    assert_eq!(
        extract(&loaded.graph, json!({"  literal": {"field\n": 42}})).await,
        (json!(42), json!(true))
    );

    let file = store
        .save_workflow("invalid-path".into(), filter_graph(json!(42)))
        .unwrap();
    let loaded = store.load_workflow(file).unwrap();
    assert_eq!(loaded.graph.find_node("extract").unwrap().data["path"], 42);
    let sessions = GraphSessionStore::new();
    let session = sessions.create_session(loaded.graph, None).await;
    let repaired = sessions
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: session.session_id.clone(),
            node_id: "extract".into(),
            data: json!({"path": "items[0]"}),
        })
        .await
        .unwrap();
    assert_eq!(
        extract(&repaired.graph, json!({"items": [false]})).await,
        (json!(false), json!(true))
    );
    sessions.close_session(&session.session_id).await.unwrap();
}
