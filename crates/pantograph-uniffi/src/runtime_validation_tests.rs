use std::path::Path;
use std::sync::Arc;

use pantograph_workflow_service::WorkflowErrorCode;
use serde_json::{json, Value};

use super::runtime_tests::{create_temp_root, workflow_error_envelope};
use super::{FfiEmbeddedRuntimeConfig, FfiPantographRuntime};

async fn open_runtime(root: &Path) -> Arc<FfiPantographRuntime> {
    FfiPantographRuntime::new(
        FfiEmbeddedRuntimeConfig {
            app_data_dir: root.join("app-data").to_string_lossy().into_owned(),
            project_root: root.to_string_lossy().into_owned(),
            workflow_roots: Vec::new(),
            max_loaded_sessions: None,
        },
        None,
    )
    .await
    .expect("open runtime")
}

fn parse(response: String) -> Value {
    serde_json::from_str(&response).expect("public response JSON")
}

async fn refresh_saved_graph(
    runtime: &FfiPantographRuntime,
    root: &Path,
    workflow_id: &str,
) -> (Value, Value) {
    let file = parse(
        runtime
            .workflow_graph_load(
                json!({
                    "path": root.join(".pantograph/workflows").join(format!("{workflow_id}.json"))
                })
                .to_string(),
            )
            .expect("load saved graph"),
    );
    let edit = parse(
        runtime
            .workflow_graph_create_edit_session(
                json!({
                    "workflow_id": workflow_id,
                    "graph": file["graph"],
                })
                .to_string(),
            )
            .await
            .expect("create edit session"),
    );
    let validation = parse(
        runtime
            .workflow_graph_refresh_current_validation_summary(
                json!({
                    "graph_session_id": edit["session_id"],
                    "graph_revision": edit["graph_revision"],
                })
                .to_string(),
            )
            .await
            .expect("refresh owner validation"),
    );
    assert_eq!(validation["summary"]["state"], "current");
    assert_eq!(validation["summary"]["submit_gate"]["allowed"], true);
    assert!(validation["summary"]["validation_session_id"]
        .as_str()
        .is_some());
    (edit, validation)
}

fn publication_request(workflow_id: &str, edit: &Value, validation: &Value) -> Value {
    json!({
        "workflow_id": workflow_id,
        "workflow_semantic_version": "0.1.0",
        "graph_session_id": edit["session_id"],
        "validation_session_id": validation["summary"]["validation_session_id"],
    })
}

pub(super) async fn publish_saved_workflow(
    runtime: &FfiPantographRuntime,
    root: &Path,
    workflow_id: &str,
) -> Value {
    let (edit, validation) = refresh_saved_graph(runtime, root, workflow_id).await;
    let published = parse(
        runtime
            .publish_graph_session_executable_validation_snapshot(
                publication_request(workflow_id, &edit, &validation).to_string(),
            )
            .await
            .expect("publish owner-derived executable snapshot"),
    );
    assert_eq!(published["workflow_id"], workflow_id);
    assert_eq!(
        published["validation_session_id"],
        validation["summary"]["validation_session_id"]
    );
    assert!(published["validation_snapshot_id"]
        .as_str()
        .is_some_and(|id| !id.is_empty()));
    runtime
        .workflow_graph_close_edit_session(json!({"session_id": edit["session_id"]}).to_string())
        .await
        .expect("close published edit session");
    published
}

async fn run_text(
    runtime: &FfiPantographRuntime,
    workflow_id: &str,
) -> Result<String, crate::FfiError> {
    let session = parse(
        runtime
            .workflow_create_session(
                json!({
                    "workflow_id": workflow_id,
                    "keep_alive": false,
                })
                .to_string(),
            )
            .await
            .expect("create execution session"),
    );
    let result = runtime.workflow_run_session(json!({
        "session_id": session["session_id"],
        "workflow_semantic_version": "0.1.0",
        "inputs": [{"node_id": "text-input-1", "port_id": "text", "value": "published text"}],
        "output_targets": [{"node_id": "text-output-1", "port_id": "text"}],
    }).to_string()).await;
    runtime
        .workflow_close_session(json!({"session_id": session["session_id"]}).to_string())
        .await
        .expect("close execution session");
    result
}

fn assert_missing_publication(error: crate::FfiError) {
    let envelope = workflow_error_envelope(error);
    assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
    assert_eq!(
        envelope.message,
        "saved executable validation snapshot was not found for workflow version"
    );
    let diagnostics = envelope.diagnostics.expect("run admission diagnostics");
    assert!(diagnostics
        .workflow_run_id
        .as_deref()
        .is_some_and(|id| !id.is_empty()));
}

#[tokio::test]
async fn public_validation_publication_is_required_again_after_runtime_reopen() {
    let workflow_id = "uniffi-publication-reopen";
    let root = create_temp_root(workflow_id);
    let runtime = open_runtime(&root).await;
    assert_missing_publication(
        run_text(&runtime, workflow_id)
            .await
            .expect_err("missing publication"),
    );
    publish_saved_workflow(&runtime, &root, workflow_id).await;
    let response = parse(
        run_text(&runtime, workflow_id)
            .await
            .expect("published text runs"),
    );
    assert_eq!(response["outputs"][0]["value"], "published text");
    runtime.shutdown().await.expect("runtime shutdown");
    drop(runtime);

    let reopened = open_runtime(&root).await;
    assert_missing_publication(
        run_text(&reopened, workflow_id)
            .await
            .expect_err("ephemeral snapshot is absent"),
    );
    publish_saved_workflow(&reopened, &root, workflow_id).await;
    let response = parse(
        run_text(&reopened, workflow_id)
            .await
            .expect("republished text runs"),
    );
    assert_eq!(response["outputs"][0]["value"], "published text");
    reopened.shutdown().await.expect("runtime shutdown");
    drop(reopened);
    std::fs::remove_dir_all(root).expect("remove fixture");
}

#[tokio::test]
async fn public_publication_rejects_stale_validation_and_caller_supplied_proof() {
    let workflow_id = "uniffi-publication-stale";
    let root = create_temp_root(workflow_id);
    let runtime = open_runtime(&root).await;
    let (edit, validation) = refresh_saved_graph(&runtime, &root, workflow_id).await;
    let request = publication_request(workflow_id, &edit, &validation);
    let mut forged_request = request.clone();
    forged_request["validation_publication"] = json!({"status": "executable"});
    let error = runtime
        .publish_graph_session_executable_validation_snapshot(forged_request.to_string())
        .await
        .expect_err("caller proof is not accepted");
    let envelope = workflow_error_envelope(error);
    assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
    assert!(envelope.message.contains("unknown field"));

    let changed = parse(
        runtime
            .workflow_graph_update_node_data(
                json!({
                    "session_id": edit["session_id"],
                    "node_id": "text-input-1",
                    "data": {"text": "changed after validation"},
                })
                .to_string(),
            )
            .await
            .expect("mutate graph"),
    );
    assert_ne!(changed["graph_revision"], edit["graph_revision"]);
    let error = runtime
        .publish_graph_session_executable_validation_snapshot(request.to_string())
        .await
        .expect_err("stale validation cannot be published");
    let envelope = workflow_error_envelope(error);
    assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
    assert!(!envelope.message.is_empty());
    assert_missing_publication(
        run_text(&runtime, workflow_id)
            .await
            .expect_err("rejected publication stores no executable snapshot"),
    );
    runtime.shutdown().await.expect("runtime shutdown");
    drop(runtime);
    std::fs::remove_dir_all(root).expect("remove fixture");
}
