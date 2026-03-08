//! Reranker Task — Stub Descriptor
//!
//! Provides metadata so that `register_builtins()` discovers the `reranker`
//! node type. Actual execution is delegated to the host application via the
//! node-engine executor.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_MODEL_PATH: &str = "model_path";
const PORT_QUERY: &str = "query";
const PORT_DOCUMENTS: &str = "documents";
const PORT_TOP_K: &str = "top_k";
const PORT_RETURN_DOCUMENTS: &str = "return_documents";
const PORT_RESULTS: &str = "results";
const PORT_SCORES: &str = "scores";
const PORT_TOP_DOCUMENT: &str = "top_document";
const PORT_TOP_SCORE: &str = "top_score";
const PORT_MODEL_REF: &str = "model_ref";

#[derive(Clone)]
pub struct RerankerTask {
    task_id: String,
}

impl RerankerTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for RerankerTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "reranker".to_string(),
            category: NodeCategory::Processing,
            label: "LlamaCpp Reranker".to_string(),
            description: "Rank candidate documents with a GGUF reranker via llama.cpp"
                .to_string(),
            inputs: vec![
                PortMetadata::required(PORT_MODEL_PATH, "Model Path", PortDataType::String),
                PortMetadata::required(PORT_QUERY, "Query", PortDataType::String),
                PortMetadata::required(PORT_DOCUMENTS, "Documents", PortDataType::Json),
                PortMetadata::optional(PORT_TOP_K, "Top K", PortDataType::Number),
                PortMetadata::optional(
                    PORT_RETURN_DOCUMENTS,
                    "Return Documents",
                    PortDataType::Boolean,
                ),
                PortMetadata::optional(
                    "inference_settings",
                    "Inference Settings",
                    PortDataType::Json,
                ),
            ],
            outputs: vec![
                PortMetadata::required(PORT_RESULTS, "Results", PortDataType::Json),
                PortMetadata::optional(PORT_SCORES, "Scores", PortDataType::Json),
                PortMetadata::optional(PORT_TOP_DOCUMENT, "Top Document", PortDataType::String),
                PortMetadata::optional(PORT_TOP_SCORE, "Top Score", PortDataType::Number),
                PortMetadata::optional(PORT_MODEL_PATH, "Model Path", PortDataType::String),
                PortMetadata::optional(PORT_MODEL_REF, "Model Reference", PortDataType::Json),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(RerankerTask::descriptor));

#[async_trait]
impl Task for RerankerTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "reranker requires host-specific execution via the callback bridge".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = RerankerTask::descriptor();

        assert_eq!(meta.node_type, "reranker");
        assert_eq!(meta.execution_mode, ExecutionMode::Reactive);
    }

    #[test]
    fn test_descriptor_has_expected_ports() {
        let meta = RerankerTask::descriptor();

        assert_eq!(meta.inputs.len(), 6);
        assert!(meta.inputs.iter().any(|p| p.id == "model_path"));
        assert!(meta.inputs.iter().any(|p| p.id == "query"));
        assert!(meta.inputs.iter().any(|p| p.id == "documents"));
        assert!(meta.inputs.iter().any(|p| p.id == "top_k"));
        assert!(meta.inputs.iter().any(|p| p.id == "return_documents"));
        assert!(meta.inputs.iter().any(|p| p.id == "inference_settings"));

        assert_eq!(meta.outputs.len(), 6);
        assert!(meta.outputs.iter().any(|p| p.id == "results"));
        assert!(meta.outputs.iter().any(|p| p.id == "scores"));
        assert!(meta.outputs.iter().any(|p| p.id == "top_document"));
        assert!(meta.outputs.iter().any(|p| p.id == "top_score"));
        assert!(meta.outputs.iter().any(|p| p.id == "model_path"));
        assert!(meta.outputs.iter().any(|p| p.id == "model_ref"));
    }

    #[tokio::test]
    async fn test_run_returns_error() {
        let task = RerankerTask::new("test-reranker");
        let context = Context::new();

        let result = task.run(context).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("callback bridge"),
            "error should mention callback bridge, got: {err}"
        );
    }
}
