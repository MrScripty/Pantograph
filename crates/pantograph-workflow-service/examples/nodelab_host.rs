//! Isolated stdio host for NodeLab authoring qualification. No runtime code loader.
//! A verifier supplies the loopback HTTP transport; EOF drops the temporary store.
use async_trait::async_trait;
use node_engine::{Context, ExecutorExtensions, NodeExecutor};
use pantograph_node_contracts::{
    NodeAuthoringMetadata, NodeCategory, NodeExecutionSemantics, NodeTypeContract, PortContract,
    PortRequirement, PortValueType,
};
use pantograph_workflow_service::{
    custom_nodes::{authoring::CustomNodeDraftRequest, CustomNodeCatalog, TrustedCustomNode},
    *,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{BufRead, Write},
    sync::Arc,
};

struct Uppercase;
#[async_trait]
impl NodeExecutor for Uppercase {
    async fn execute(
        &self,
        _: &str,
        inputs: HashMap<String, Value>,
        _: &Context,
        _: &ExecutorExtensions,
    ) -> node_engine::Result<HashMap<String, Value>> {
        Ok(HashMap::from([(
            "output".into(),
            json!(inputs["input"]
                .as_str()
                .ok_or_else(|| node_engine::NodeEngineError::ExecutionFailed(
                    "string input required".into()
                ))?
                .to_uppercase()),
        )]))
    }
}
fn catalog() -> Result<Arc<CustomNodeCatalog>, Box<dyn std::error::Error>> {
    let mut output = PortContract::output("output".parse()?, "Output", PortValueType::String);
    output.requirement = PortRequirement::Required;
    let contract = NodeTypeContract {
        node_type: "host-uppercase".parse()?,
        category: NodeCategory::Processing,
        label: "Host uppercase".into(),
        description: "Pure string transform compiled into this isolated Rust host".into(),
        inputs: vec![PortContract::input(
            "input".parse()?,
            "Input",
            PortValueType::String,
            PortRequirement::Required,
        )],
        outputs: vec![output],
        execution_semantics: NodeExecutionSemantics::Batch,
        capability_requirements: vec![],
        inference_tasks: vec![],
        authoring: NodeAuthoringMetadata::default(),
        contract_version: Some("1.0.0".into()),
        contract_digest: None,
    };
    let digest = |text: &str| format!("blake3:{}", blake3::hash(text.as_bytes()));
    Ok(Arc::new(CustomNodeCatalog::try_new(vec![
        TrustedCustomNode::admit(
            contract,
            digest("nodelab_host.rs uppercase v1"),
            digest("isolated Rust authoring host v1"),
            Arc::new(Uppercase),
        )?,
    ])?))
}
fn decode<T: DeserializeOwned>(value: Value) -> Result<T, WorkflowServiceError> {
    serde_json::from_value(value).map_err(|e| WorkflowServiceError::InvalidRequest(e.to_string()))
}
fn encode(value: impl Serialize) -> Result<Value, WorkflowServiceError> {
    serde_json::to_value(value).map_err(|e| WorkflowServiceError::Internal(e.to_string()))
}
fn string(args: &Value, name: &str) -> Result<String, WorkflowServiceError> {
    args[name]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| WorkflowServiceError::InvalidRequest(format!("{name} is required")))
}
async fn dispatch(
    service: &WorkflowService,
    store: &FileSystemWorkflowGraphStore,
    command: &str,
    args: Value,
) -> Result<Value, WorkflowServiceError> {
    let edit = || -> Result<WorkflowGraphEditSessionGraphRequest, WorkflowServiceError> {
        Ok(WorkflowGraphEditSessionGraphRequest {
            session_id: string(&args, "executionId")?,
        })
    };
    match command {
        "get_custom_node_authoring_catalog" => encode(service.custom_node_authoring_catalog()),
        "create_custom_node_draft" => encode(
            service
                .create_custom_node_draft(decode::<CustomNodeDraftRequest>(
                    args["request"].clone(),
                )?)
                .await?,
        ),
        "get_node_lab_draft" => encode(
            service
                .workflow_graph_get_edit_session_graph(edit()?)
                .await?,
        ),
        "create_workflow_execution_session" => encode(
            service
                .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                    graph: decode(args["graph"].clone())?,
                    workflow_id: args["workflowId"].as_str().map(str::to_owned),
                })
                .await?,
        ),
        "update_node_data" => encode(
            service
                .workflow_graph_update_node_data(WorkflowGraphUpdateNodeDataRequest {
                    session_id: string(&args, "executionId")?,
                    node_id: string(&args, "nodeId")?,
                    data: args["data"].clone(),
                })
                .await?,
        ),
        "save_workflow" => encode(
            service
                .workflow_graph_save(
                    store,
                    WorkflowGraphSaveRequest {
                        name: string(&args, "name")?,
                        graph: decode(args["graph"].clone())?,
                    },
                )?
                .path,
        ),
        "load_workflow" => encode(service.workflow_graph_load(
            store,
            WorkflowGraphLoadRequest {
                path: string(&args, "path")?,
            },
        )?),
        "remove_execution" => {
            service
                .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                    session_id: edit()?.session_id,
                })
                .await?;
            Ok(Value::Null)
        }
        _ => Err(WorkflowServiceError::CapabilityViolation(
            "unsupported authoring host command".into(),
        )),
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let store = FileSystemWorkflowGraphStore::new(directory.path());
    let service = if std::env::args().any(|arg| arg == "--empty-catalog") {
        WorkflowService::new()
    } else {
        WorkflowService::with_trusted_custom_nodes(catalog()?)
    };
    for line in std::io::stdin().lock().lines() {
        let result = match serde_json::from_str::<Value>(&line?) {
            Ok(request) => match request["command"].as_str() {
                Some(command) => dispatch(&service, &store, command, request["args"].clone()).await,
                None => Err(WorkflowServiceError::InvalidRequest(
                    "command is required".into(),
                )),
            },
            Err(error) => Err(WorkflowServiceError::InvalidRequest(error.to_string())),
        };
        let response = match result {
            Ok(value) => json!({"ok": true, "value": value}),
            Err(error) => {
                json!({"ok": false, "error": serde_json::from_str::<Value>(&error.to_envelope_json())?})
            }
        };
        println!("{}", serde_json::to_string(&response)?);
        std::io::stdout().flush()?;
    }
    Ok(())
}
