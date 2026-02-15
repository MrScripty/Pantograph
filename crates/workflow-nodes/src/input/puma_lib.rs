//! Puma-Lib Node
//!
//! This module registers a stub node descriptor for `puma-lib` so that
//! `register_builtins()` discovers the node via `inventory`. Actual execution
//! is handled by the host application through the callback bridge — the host
//! provides the model file path from its local pumas-core library.
//!
//! When the `model-library` feature is enabled, this module also registers
//! a `PortOptionsProvider` for the `model_path` port, enabling hosts to
//! query available models from the pumas-library.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata};

const PORT_MODEL_PATH: &str = "model_path";

/// Stub task for the puma-lib node.
///
/// The node is discoverable by all consumers (including puma-bot NIF) but
/// always fails at runtime — the host must intercept execution via the
/// callback bridge and supply the model file path itself.
#[derive(Clone)]
pub struct PumaLibTask {
    task_id: String,
}

impl PumaLibTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for PumaLibTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "puma-lib".to_string(),
            category: NodeCategory::Input,
            label: "Puma-Lib".to_string(),
            description: "Provides AI model file path".to_string(),
            inputs: vec![],
            outputs: vec![PortMetadata::optional(
                PORT_MODEL_PATH,
                "Model Path",
                PortDataType::String,
            )],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(PumaLibTask::descriptor));

// ---------------------------------------------------------------------------
// Port options provider (model-library feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "model-library")]
mod options_provider {
    use async_trait::async_trait;
    use node_engine::{
        extension_keys, ExecutorExtensions, NodeEngineError, PortOption, PortOptionsProvider,
        PortOptionsQuery, PortOptionsResult,
    };
    use std::sync::Arc;

    /// Provides available models from pumas-library for the `model_path` port.
    pub struct PumaLibOptionsProvider;

    #[async_trait]
    impl PortOptionsProvider for PumaLibOptionsProvider {
        async fn query_options(
            &self,
            query: &PortOptionsQuery,
            extensions: &ExecutorExtensions,
        ) -> node_engine::Result<PortOptionsResult> {
            let api = extensions
                .get::<Arc<pumas_library::PumasApi>>(extension_keys::PUMAS_API)
                .ok_or_else(|| {
                    NodeEngineError::ExecutionFailed(
                        "Model library not available".to_string(),
                    )
                })?;

            let records = if let Some(ref search) = query.search {
                let result = api
                    .search_models(search, query.limit.unwrap_or(50), query.offset.unwrap_or(0))
                    .await
                    .map_err(|e| NodeEngineError::ExecutionFailed(e.to_string()))?;
                result.models
            } else {
                api.list_models()
                    .await
                    .map_err(|e| NodeEngineError::ExecutionFailed(e.to_string()))?
            };

            let options: Vec<PortOption> = records
                .iter()
                .map(|m| PortOption {
                    value: serde_json::json!(m.path),
                    label: m.official_name.clone(),
                    description: Some(format!("{} | {}", m.model_type, m.tags.join(", "))),
                    metadata: Some(serde_json::json!({
                        "id": m.id,
                        "model_type": m.model_type,
                        "cleaned_name": m.cleaned_name,
                    })),
                })
                .collect();

            let total = options.len();
            Ok(PortOptionsResult {
                options,
                total_count: total,
                searchable: true,
            })
        }
    }
}

#[cfg(feature = "model-library")]
inventory::submit!(node_engine::PortQueryFn {
    node_type: "puma-lib",
    port_id: "model_path",
    provider: || Box::new(options_provider::PumaLibOptionsProvider),
});

#[async_trait]
impl Task for PumaLibTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "puma-lib requires host-specific execution via the callback bridge".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = PumaLibTask::descriptor();
        assert_eq!(meta.node_type, "puma-lib");
    }

    #[test]
    fn test_descriptor_has_correct_ports() {
        let meta = PumaLibTask::descriptor();

        assert!(meta.inputs.is_empty());
        assert_eq!(meta.outputs.len(), 1);

        let port = &meta.outputs[0];
        assert_eq!(port.id, "model_path");
        assert_eq!(port.data_type, PortDataType::String);
        assert!(!port.required);
    }

    #[tokio::test]
    async fn test_run_returns_error() {
        let task = PumaLibTask::new("test");
        let context = Context::new();

        let result = task.run(context).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("callback bridge"),
            "expected callback bridge message, got: {err}"
        );
    }
}
