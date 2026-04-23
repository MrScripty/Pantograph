use super::*;

impl TauriTaskExecutor {
    /// Execute a RAG search task
    pub(super) async fn execute_rag_search(
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

        let rag_backend = self.rag_backend.as_ref().ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "rag-search node requires a configured RAG backend".to_string(),
            )
        })?;
        let docs = rag_backend
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
}
