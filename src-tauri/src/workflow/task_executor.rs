//! Tauri-specific task executor for host-dependent node types.
//!
//! Only handles node types that require Tauri-specific resources
//! (e.g. RagManager). All other nodes are handled by
//! `CoreTaskExecutor` via `CompositeTaskExecutor`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use node_engine::{
    core_executor::resolve_node_type, extension_keys, Context, DependencyState, ExecutorExtensions,
    ModelDependencyRequest, ModelDependencyResolver, NodeEngineError, Result, TaskExecutor,
};
use tokio::sync::RwLock;

use crate::agent::rag::RagManager;
use crate::workflow::python_runtime::{
    ProcessPythonRuntimeAdapter, PythonNodeExecutionRequest, PythonRuntimeAdapter,
};

/// Tauri-specific task executor that handles only host-dependent nodes.
///
/// Currently handles:
/// - `rag-search`: requires `RagManager` (Tauri-managed state)
///
/// All other node types should be handled by `CoreTaskExecutor` via
/// `CompositeTaskExecutor`. Unknown types return the sentinel error
/// that `CompositeTaskExecutor` uses for fallthrough.
pub struct TauriTaskExecutor {
    /// RAG manager for document search
    rag_manager: Arc<RwLock<RagManager>>,
    /// Host adapter for python-backed nodes (pytorch/audio).
    python_runtime: Arc<dyn PythonRuntimeAdapter>,
}

impl TauriTaskExecutor {
    /// Create a new Tauri-specific task executor.
    pub fn new(rag_manager: Arc<RwLock<RagManager>>) -> Self {
        Self::with_python_runtime(rag_manager, Arc::new(ProcessPythonRuntimeAdapter))
    }

    /// Create a task executor with a custom python runtime adapter.
    pub fn with_python_runtime(
        rag_manager: Arc<RwLock<RagManager>>,
        python_runtime: Arc<dyn PythonRuntimeAdapter>,
    ) -> Self {
        Self {
            rag_manager,
            python_runtime,
        }
    }

    /// Execute a RAG search task
    async fn execute_rag_search(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let query = inputs
            .get("query")
            .and_then(|q| q.as_str())
            .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing query input".to_string()))?;

        let limit = inputs
            .get("limit")
            .and_then(|l| l.as_f64())
            .map(|l| l as usize)
            .unwrap_or(5);

        let rag_manager = self.rag_manager.read().await;
        let docs = rag_manager
            .search_as_docs(query, limit)
            .await
            .map_err(|e| NodeEngineError::ExecutionFailed(format!("RAG search failed: {}", e)))?;

        // Build context string
        let context_str = docs
            .iter()
            .map(|doc| format!("## {}\n{}", doc.title, doc.content))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        let mut outputs = HashMap::new();
        outputs.insert(
            "documents".to_string(),
            serde_json::to_value(&docs).unwrap(),
        );
        outputs.insert("context".to_string(), serde_json::json!(context_str));
        Ok(outputs)
    }

    fn collect_model_ref_env_ids(inputs: &HashMap<String, serde_json::Value>) -> Vec<String> {
        let model_ref = inputs
            .get("model_ref")
            .or_else(|| inputs.get("_data").and_then(|v| v.get("model_ref")));
        let Some(bindings) = model_ref
            .and_then(|v| {
                v.get("dependency_bindings")
                    .or_else(|| v.get("dependencyBindings"))
            })
            .and_then(|v| v.as_array())
        else {
            return Vec::new();
        };

        let mut out = Vec::new();
        for binding in bindings {
            let env_id = binding
                .get("env_id")
                .and_then(|v| v.as_str())
                .or_else(|| binding.get("envId").and_then(|v| v.as_str()));
            if let Some(env_id) = env_id {
                let trimmed = env_id.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    fn read_optional_input_string(
        inputs: &HashMap<String, serde_json::Value>,
        key: &str,
    ) -> Option<String> {
        inputs
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                inputs
                    .get("_data")
                    .and_then(|d| d.get(key))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
    }

    fn read_optional_input_value(
        inputs: &HashMap<String, serde_json::Value>,
        key: &str,
    ) -> Option<serde_json::Value> {
        inputs
            .get(key)
            .cloned()
            .or_else(|| inputs.get("_data").and_then(|d| d.get(key)).cloned())
    }

    fn read_optional_input_string_aliases(
        inputs: &HashMap<String, serde_json::Value>,
        aliases: &[&str],
    ) -> Option<String> {
        aliases
            .iter()
            .find_map(|key| Self::read_optional_input_string(inputs, key))
    }

    fn read_optional_input_value_aliases(
        inputs: &HashMap<String, serde_json::Value>,
        aliases: &[&str],
    ) -> Option<serde_json::Value> {
        aliases
            .iter()
            .find_map(|key| Self::read_optional_input_value(inputs, key))
    }

    fn read_input_selected_binding_ids(inputs: &HashMap<String, serde_json::Value>) -> Vec<String> {
        let Some(raw) = Self::read_optional_input_value_aliases(
            inputs,
            &["selected_binding_ids", "selectedBindingIds"],
        ) else {
            return Vec::new();
        };

        raw.as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .filter(|s| !s.trim().is_empty())
            .collect()
    }

    fn infer_task_type_primary(node_type: &str, inputs: &HashMap<String, serde_json::Value>) -> String {
        if let Some(task) =
            Self::read_optional_input_string_aliases(inputs, &["task_type_primary", "taskTypePrimary"])
        {
            if !task.trim().is_empty() {
                return task;
            }
        }

        let model_type = Self::read_optional_input_string_aliases(inputs, &["model_type", "modelType"])
            .unwrap_or_default()
            .to_lowercase();

        if node_type == "audio-generation" || model_type == "audio" {
            return "text-to-audio".to_string();
        }

        match model_type.as_str() {
            "diffusion" => "text-to-image".to_string(),
            "vision" => "image-to-text".to_string(),
            "embedding" => "feature-extraction".to_string(),
            _ => "text-generation".to_string(),
        }
    }

    fn infer_backend_key(node_type: &str) -> String {
        match node_type {
            "audio-generation" => "stable_audio".to_string(),
            "pytorch-inference" => "pytorch".to_string(),
            _ => "pytorch".to_string(),
        }
    }

    fn build_model_dependency_request(
        node_type: &str,
        model_path: &str,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> ModelDependencyRequest {
        let backend_key = Self::read_optional_input_string_aliases(inputs, &["backend_key", "backendKey"])
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| Self::infer_backend_key(node_type));

        let task_type_primary = Self::infer_task_type_primary(node_type, inputs);

        ModelDependencyRequest {
            node_type: node_type.to_string(),
            model_path: model_path.to_string(),
            model_id: Self::read_optional_input_string_aliases(inputs, &["model_id", "modelId"]),
            model_type: Self::read_optional_input_string_aliases(inputs, &["model_type", "modelType"]),
            task_type_primary: Some(task_type_primary),
            backend_key: Some(backend_key),
            platform_context: Self::read_optional_input_value_aliases(
                inputs,
                &["platform_context", "platformContext"],
            ),
            selected_binding_ids: Self::read_input_selected_binding_ids(inputs),
        }
    }

    async fn enforce_dependency_preflight(
        &self,
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
        extensions: &ExecutorExtensions,
    ) -> Result<Option<node_engine::ModelRefV2>> {
        if node_type != "pytorch-inference" && node_type != "audio-generation" {
            return Ok(None);
        }

        let Some(resolver) =
            extensions.get::<Arc<dyn ModelDependencyResolver>>(extension_keys::MODEL_DEPENDENCY_RESOLVER)
        else {
            return Err(NodeEngineError::ExecutionFailed(
                "Dependency preflight blocked execution: dependency resolver is not configured"
                    .to_string(),
            ));
        };

        let model_path = inputs
            .get("model_path")
            .and_then(|m| m.as_str())
            .ok_or_else(|| {
                NodeEngineError::ExecutionFailed(
                    "Missing model_path input. Connect a Puma-Lib node.".to_string(),
                )
            })?;

        let request = Self::build_model_dependency_request(node_type, model_path, inputs);
        let plan = resolver
            .resolve_model_dependency_plan(request.clone())
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Dependency preflight plan resolution failed for '{}': {}",
                    node_type, e
                ))
            })?;

        let status = resolver.check_dependencies(request.clone()).await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!(
                "Dependency preflight check failed for '{}': {}",
                node_type, e
            ))
        })?;

        if status.state != DependencyState::Ready {
            let payload = serde_json::json!({
                "kind": "dependency_preflight",
                "node_type": node_type,
                "model_path": model_path,
                "plan_state": plan.state,
                "plan_code": plan.code,
                "plan_message": plan.message,
                "review_reasons": plan.review_reasons,
                "required_binding_ids": plan.required_binding_ids,
                "selected_binding_ids": plan.selected_binding_ids,
                "state": status.state,
                "code": status.code,
                "bindings": status.bindings,
                "message": status.message,
            });
            return Err(NodeEngineError::ExecutionFailed(format!(
                "Dependency preflight blocked execution: {}",
                payload
            )));
        }

        let resolved = resolver
            .resolve_model_ref(request, Some(plan))
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Dependency preflight failed to resolve model_ref: {}",
                    e
                ))
            })?;
        if let Some(ref model_ref) = resolved {
            model_ref
                .validate()
                .map_err(NodeEngineError::ExecutionFailed)?;
        }

        Ok(resolved)
    }

    async fn execute_python_node(
        &self,
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut runtime_inputs = inputs.clone();
        if let Some(model_ref) = self
            .enforce_dependency_preflight(node_type, inputs, extensions)
            .await?
        {
            let value = serde_json::to_value(model_ref).map_err(|err| {
                NodeEngineError::ExecutionFailed(format!(
                    "Failed to serialize resolved model_ref for python runtime adapter: {}",
                    err
                ))
            })?;
            runtime_inputs.insert("model_ref".to_string(), value);
        }

        let request = PythonNodeExecutionRequest {
            node_type: node_type.to_string(),
            inputs: runtime_inputs.clone(),
            env_ids: Self::collect_model_ref_env_ids(&runtime_inputs),
        };
        self.python_runtime
            .execute_node(request)
            .await
            .map_err(NodeEngineError::ExecutionFailed)
    }
}

#[async_trait]
impl TaskExecutor for TauriTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        extensions: &node_engine::ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let node_type = resolve_node_type(task_id, &inputs);

        match node_type.as_str() {
            "rag-search" => self.execute_rag_search(&inputs).await,
            "pytorch-inference" | "audio-generation" => {
                self.execute_python_node(&node_type, &inputs, extensions).await
            }
            _ => {
                // Signal to CompositeTaskExecutor that this node type
                // requires host-specific executor (i.e., fall through to core)
                Err(NodeEngineError::ExecutionFailed(format!(
                    "Node type '{}' requires host-specific executor",
                    node_type
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;

    use node_engine::{
        extension_keys, DependencyState, ExecutorExtensions, ModelDependencyBinding,
        ModelDependencyInstallResult, ModelDependencyPlan, ModelDependencyRequest,
        ModelDependencyResolver, ModelDependencyStatus, ModelRefV2,
    };

    #[derive(Clone)]
    struct StubDependencyResolver {
        plan: ModelDependencyPlan,
        status: ModelDependencyStatus,
        model_ref: Option<ModelRefV2>,
    }

    #[async_trait]
    impl ModelDependencyResolver for StubDependencyResolver {
        async fn resolve_model_dependency_plan(
            &self,
            _request: ModelDependencyRequest,
        ) -> std::result::Result<ModelDependencyPlan, String> {
            Ok(self.plan.clone())
        }

        async fn check_dependencies(
            &self,
            _request: ModelDependencyRequest,
        ) -> std::result::Result<ModelDependencyStatus, String> {
            Ok(self.status.clone())
        }

        async fn install_dependencies(
            &self,
            _request: ModelDependencyRequest,
        ) -> std::result::Result<ModelDependencyInstallResult, String> {
            Err("install not used in task-executor tests".to_string())
        }

        async fn resolve_model_ref(
            &self,
            _request: ModelDependencyRequest,
            _plan: Option<ModelDependencyPlan>,
        ) -> std::result::Result<Option<ModelRefV2>, String> {
            Ok(self.model_ref.clone())
        }
    }

    struct RecordingPythonAdapter {
        requests: Arc<Mutex<Vec<PythonNodeExecutionRequest>>>,
        response: HashMap<String, serde_json::Value>,
    }

    #[async_trait]
    impl PythonRuntimeAdapter for RecordingPythonAdapter {
        async fn execute_node(
            &self,
            request: PythonNodeExecutionRequest,
        ) -> std::result::Result<HashMap<String, serde_json::Value>, String> {
            self.requests.lock().expect("recording lock").push(request);
            Ok(self.response.clone())
        }
    }

    fn test_executor(
        adapter: Arc<dyn PythonRuntimeAdapter>,
        resolver: Arc<dyn ModelDependencyResolver>,
    ) -> (TauriTaskExecutor, ExecutorExtensions) {
        let rag_manager = Arc::new(RwLock::new(RagManager::new(PathBuf::from(
            "/tmp/pantograph-task-executor-tests",
        ))));
        let executor = TauriTaskExecutor::with_python_runtime(rag_manager, adapter);

        let mut extensions = ExecutorExtensions::new();
        extensions.set(extension_keys::MODEL_DEPENDENCY_RESOLVER, resolver);
        (executor, extensions)
    }

    fn make_plan(state: DependencyState, code: Option<&str>) -> ModelDependencyPlan {
        ModelDependencyPlan {
            state,
            code: code.map(|s| s.to_string()),
            message: code.map(|s| format!("state={}", s)),
            review_reasons: Vec::new(),
            plan_id: Some("plan-test".to_string()),
            bindings: Vec::new(),
            selected_binding_ids: Vec::new(),
            required_binding_ids: Vec::new(),
            missing_pins: Vec::new(),
        }
    }

    fn make_status(state: DependencyState, code: Option<&str>) -> ModelDependencyStatus {
        ModelDependencyStatus {
            state,
            code: code.map(|s| s.to_string()),
            message: code.map(|s| format!("status={}", s)),
            review_reasons: Vec::new(),
            plan_id: Some("plan-test".to_string()),
            bindings: Vec::new(),
            checked_at: None,
            missing_pins: Vec::new(),
        }
    }

    #[tokio::test]
    async fn python_nodes_block_when_dependency_preflight_is_not_ready() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests: requests.clone(),
            response: HashMap::new(),
        });
        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            plan: make_plan(DependencyState::ManualInterventionRequired, Some("unpinned_dependency")),
            status: make_status(
                DependencyState::ManualInterventionRequired,
                Some("unpinned_dependency"),
            ),
            model_ref: None,
        });
        let (executor, extensions) = test_executor(adapter, resolver);

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/model-not-ready"),
        );
        inputs.insert("prompt".to_string(), serde_json::json!("hello"));

        let err = executor
            .execute_task(
                "pytorch-inference-1",
                inputs,
                &Context::new(),
                &extensions,
            )
            .await
            .expect_err("preflight should block non-ready dependency state");

        match err {
            NodeEngineError::ExecutionFailed(message) => {
                assert!(message.contains("Dependency preflight blocked execution"));
                assert!(message.contains("unpinned_dependency"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
        assert_eq!(requests.lock().expect("recording lock").len(), 0);
    }

    #[tokio::test]
    async fn python_nodes_receive_resolved_model_ref_and_env_ids_after_preflight() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let mut adapter_response = HashMap::new();
        adapter_response.insert("response".to_string(), serde_json::json!("ok"));
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests: requests.clone(),
            response: adapter_response,
        });

        let resolved_model_ref = ModelRefV2 {
            contract_version: 2,
            engine: "pytorch".to_string(),
            model_id: "model-a".to_string(),
            model_path: "/tmp/model-ready".to_string(),
            task_type_primary: "text-generation".to_string(),
            dependency_bindings: vec![ModelDependencyBinding {
                binding_id: "binding-a".to_string(),
                profile_id: "profile-a".to_string(),
                profile_version: 1,
                profile_hash: Some("hash".to_string()),
                binding_kind: "required".to_string(),
                backend_key: Some("pytorch".to_string()),
                platform_selector: Some("linux-x86_64".to_string()),
                env_id: "venv:test".to_string(),
                pin_summary: None,
                required_pins: Vec::new(),
                missing_pins: Vec::new(),
            }],
            dependency_plan_id: Some("plan-test".to_string()),
        };

        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            plan: make_plan(DependencyState::Ready, None),
            status: make_status(DependencyState::Ready, None),
            model_ref: Some(resolved_model_ref),
        });
        let (executor, extensions) = test_executor(adapter, resolver);

        let mut inputs = HashMap::new();
        inputs.insert("model_path".to_string(), serde_json::json!("/tmp/model-ready"));
        inputs.insert("prompt".to_string(), serde_json::json!("hello"));

        let outputs = executor
            .execute_task(
                "pytorch-inference-1",
                inputs,
                &Context::new(),
                &extensions,
            )
            .await
            .expect("ready preflight should allow adapter execution");
        assert_eq!(outputs.get("response"), Some(&serde_json::json!("ok")));

        let recorded = requests.lock().expect("recording lock");
        assert_eq!(recorded.len(), 1);
        let request = &recorded[0];
        assert_eq!(request.node_type, "pytorch-inference");
        assert_eq!(request.env_ids, vec!["venv:test".to_string()]);
        assert!(request.inputs.contains_key("model_ref"));
    }
}
