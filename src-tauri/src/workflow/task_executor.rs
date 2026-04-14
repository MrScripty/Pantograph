use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use node_engine::{Context, ExecutorExtensions, Result, TaskExecutor};

use crate::agent::rag::SharedRagManager;

pub use pantograph_embedded_runtime::task_executor::PythonRuntimeExecutionRecorder;
use pantograph_embedded_runtime::task_executor::TauriTaskExecutor as EmbeddedTaskExecutor;
use pantograph_embedded_runtime::{PythonRuntimeAdapter, RagBackend, RagDocument};

struct TauriRagBackend {
    rag_manager: SharedRagManager,
}

#[async_trait]
impl RagBackend for TauriRagBackend {
    async fn search_as_docs(
        &self,
        query: &str,
        limit: usize,
    ) -> std::result::Result<Vec<RagDocument>, String> {
        let guard = self.rag_manager.read().await;
        let docs = guard
            .search_as_docs(query, limit)
            .await
            .map_err(|err| err.to_string())?;
        Ok(docs
            .into_iter()
            .map(|doc| RagDocument {
                id: doc.id,
                title: doc.title,
                section: doc.section,
                summary: doc.summary,
                content: doc.content,
            })
            .collect())
    }
}

pub struct TauriTaskExecutor {
    inner: EmbeddedTaskExecutor,
}

impl TauriTaskExecutor {
    pub fn new(rag_manager: SharedRagManager) -> Self {
        Self::with_python_runtime(
            rag_manager,
            Arc::new(pantograph_embedded_runtime::ProcessPythonRuntimeAdapter),
        )
    }

    pub fn with_python_runtime(
        rag_manager: SharedRagManager,
        python_runtime: Arc<dyn PythonRuntimeAdapter>,
    ) -> Self {
        Self {
            inner: EmbeddedTaskExecutor::with_python_runtime(
                Some(Arc::new(TauriRagBackend { rag_manager }) as Arc<dyn RagBackend>),
                python_runtime,
            ),
        }
    }
}

#[async_trait]
impl TaskExecutor for TauriTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        context: &Context,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        self.inner
            .execute_task(task_id, inputs, context, extensions)
            .await
    }
}
