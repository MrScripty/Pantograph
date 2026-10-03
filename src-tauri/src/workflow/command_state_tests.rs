use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::{App, Builder, WebviewWindow, WebviewWindowBuilder};
use tokio::sync::{mpsc, RwLock};

use super::*;
use crate::workflow::commands;

struct NoSpawn;

#[async_trait]
impl inference::ProcessSpawner for NoSpawn {
    async fn spawn_sidecar(
        &self,
        _: &str,
        _: &[&str],
    ) -> Result<(mpsc::Receiver<inference::ProcessEvent>, Box<dyn inference::ProcessHandle>), String> {
        panic!("state extraction must not spawn a backend")
    }

    fn app_data_dir(&self) -> Result<PathBuf, String> {
        panic!("state extraction must not resolve runtime paths")
    }

    fn binaries_dir(&self) -> Result<PathBuf, String> {
        panic!("state extraction must not resolve runtime paths")
    }
}

struct ManagedStates {
    gateway: SharedGateway,
    runtime_registry: SharedRuntimeRegistry,
    extensions: SharedExtensions,
    rag_manager: SharedRagManager,
    workflow_service: SharedWorkflowService,
    diagnostics_store: SharedWorkflowDiagnosticsStore,
    registry: SharedNodeRegistry,
}

impl ManagedStates {
    fn new(registry: SharedNodeRegistry) -> Self {
        let mut extensions = node_engine::ExecutorExtensions::new();
        extensions.set("state-fixture-marker", "managed extension".to_string());
        Self {
            gateway: Arc::new(crate::llm::InferenceGateway::new(Arc::new(NoSpawn))),
            runtime_registry: Arc::new(pantograph_runtime_registry::RuntimeRegistry::new()),
            extensions: Arc::new(RwLock::new(extensions)),
            rag_manager: crate::agent::rag::create_rag_manager(PathBuf::from("unused-state-fixture")),
            workflow_service: Arc::new(pantograph_workflow_service::WorkflowService::new()),
            diagnostics_store: Arc::new(crate::workflow::WorkflowDiagnosticsStore::default()),
            registry,
        }
    }

    fn run_builder(&self, prefix: usize) -> Builder<MockRuntime> {
        let mut builder = mock_builder();
        if prefix > 0 {
            builder = builder.manage(self.gateway.clone());
        }
        if prefix > 1 {
            builder = builder.manage(self.runtime_registry.clone());
        }
        if prefix > 2 {
            builder = builder.manage(self.extensions.clone());
        }
        if prefix > 3 {
            builder = builder.manage(self.rag_manager.clone());
        }
        if prefix > 4 {
            builder = builder.manage(self.workflow_service.clone());
        }
        if prefix > 5 {
            builder = builder.manage(self.diagnostics_store.clone());
        }
        builder
    }

    fn query_builder(&self, prefix: usize) -> Builder<MockRuntime> {
        let mut builder = mock_builder();
        if prefix > 0 {
            builder = builder.manage(self.registry.clone());
        }
        if prefix > 1 {
            builder = builder.manage(self.extensions.clone());
        }
        if prefix > 2 {
            builder = builder.manage(self.workflow_service.clone());
        }
        builder
    }
}

fn identity<T>(value: &Arc<T>) -> String {
    format!("{:p}", Arc::as_ptr(value))
}

#[tauri::command]
fn inspect_run_state(state: WorkflowRunCommandState<'_>) -> Vec<String> {
    vec![
        identity(state.gateway.inner()),
        identity(state.runtime_registry.inner()),
        identity(state.extensions.inner()),
        identity(state.rag_manager.inner()),
        identity(state.workflow_service.inner()),
        identity(state.diagnostics_store.inner()),
    ]
}

#[tauri::command]
fn inspect_query_state(state: PortOptionsCommandState<'_>) -> Vec<String> {
    vec![
        identity(state.registry.inner()),
        identity(state.extensions.inner()),
        identity(state.workflow_service.inner()),
    ]
}

fn build(builder: Builder<MockRuntime>) -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let app = builder
        .invoke_handler(tauri::generate_handler![
            inspect_run_state,
            inspect_query_state,
            commands::query_port_options,
        ])
        .build(mock_context(noop_assets()))
        .expect("mock app");
    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("mock webview");
    (app, webview)
}

fn invoke(webview: &WebviewWindow<MockRuntime>, command: &str, body: Value) -> Result<Value, Value> {
    tauri::test::get_ipc_response(webview, tauri::webview::InvokeRequest {
        cmd: command.to_string(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: if cfg!(any(windows, target_os = "android")) {
            "http://tauri.localhost"
        } else {
            "tauri://localhost"
        }.parse().expect("mock IPC URL"),
        body: tauri::ipc::InvokeBody::Json(body),
        headers: Default::default(),
        invoke_key: tauri::test::INVOKE_KEY.to_string(),
    }).map(|body| body.deserialize().expect("JSON response"))
}

fn missing_state(command: &str, key: &str) -> Value {
    json!(format!("state not managed for field `{key}` on command `{command}`. You must call `.manage()` before using this command"))
}

#[test]
fn run_bundle_preserves_managed_identity_without_payload_state() {
    let states = ManagedStates::new(Arc::new(node_engine::NodeRegistry::new()));
    let (_app, webview) = build(states.run_builder(6));
    let actual = invoke(&webview, "inspect_run_state", json!({"state": "ignored client input"})).expect("managed state");
    assert_eq!(actual, json!([
        identity(&states.gateway), identity(&states.runtime_registry), identity(&states.extensions),
        identity(&states.rag_manager), identity(&states.workflow_service), identity(&states.diagnostics_store),
    ]));
}

#[test]
fn run_bundle_preserves_missing_state_order_and_original_camel_case_keys() {
    let states = ManagedStates::new(Arc::new(node_engine::NodeRegistry::new()));
    for (prefix, key) in ["gateway", "runtimeRegistry", "extensions", "ragManager", "workflowService", "diagnosticsStore"].iter().enumerate() {
        let (_app, webview) = build(states.run_builder(prefix));
        assert_eq!(invoke(&webview, "inspect_run_state", json!({})), Err(missing_state("inspect_run_state", key)));
    }
}

#[test]
fn query_bundle_preserves_managed_identity_without_payload_state() {
    let states = ManagedStates::new(Arc::new(node_engine::NodeRegistry::new()));
    let (_app, webview) = build(states.query_builder(3));
    let actual = invoke(&webview, "inspect_query_state", json!({})).expect("managed state");
    assert_eq!(actual, json!([identity(&states.registry), identity(&states.extensions), identity(&states.workflow_service)]));
}

#[test]
fn query_command_preserves_missing_state_order_and_original_keys() {
    let states = ManagedStates::new(Arc::new(node_engine::NodeRegistry::new()));
    for (prefix, key) in ["registry", "extensions", "workflowService"].iter().enumerate() {
        let (_app, webview) = build(states.query_builder(prefix));
        assert_eq!(invoke(&webview, "query_port_options", json!({"nodeType":"fixture", "portId":"choice"})), Err(missing_state("query_port_options", key)));
    }
}

struct EchoOptionsProvider(Arc<AtomicUsize>);

#[async_trait]
impl node_engine::PortOptionsProvider for EchoOptionsProvider {
    async fn query_options(
        &self,
        query: &node_engine::PortOptionsQuery,
        extensions: &node_engine::ExecutorExtensions,
    ) -> node_engine::Result<node_engine::PortOptionsResult> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(node_engine::PortOptionsResult {
            options: Vec::new(), total_count: 0, searchable: true,
            metadata: Some(json!({"query":query,"marker":extensions.get::<String>("state-fixture-marker")})),
        })
    }
}

#[test]
fn query_command_preserves_flat_required_optional_and_context_payloads() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = node_engine::NodeRegistry::new();
    registry.register_port_provider("fixture", "choice", Box::new(EchoOptionsProvider(calls.clone())));
    let states = ManagedStates::new(Arc::new(registry));
    let (_app, webview) = build(states.query_builder(3));
    let context = json!({"targetNodeId":"target-1", "taskKind":"embedding"});
    let actual = invoke(&webview, "query_port_options", json!({
        "nodeType":"fixture", "portId":"choice", "search":"needle", "limit":7, "offset":3,
        "context":context, "state":"must not deserialize", "registry":"must not replace managed state",
    })).expect("original flat command payload");
    assert_eq!(actual, json!({"options":[],"totalCount":0,"searchable":true,"metadata":{
        "query":{"search":"needle","limit":7,"offset":3,"context":context},"marker":"managed extension",
    }}));
    let minimal = invoke(&webview, "query_port_options", json!({"nodeType":"fixture", "portId":"choice"})).expect("original optional defaults");
    assert_eq!(minimal["metadata"]["query"], json!({"search":null,"limit":null,"offset":null}));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
