use crate::error::{NodeEngineError, Result};

pub(crate) const OLLAMA_RETIRED_MESSAGE: &str = "Ollama is no longer supported as a first-party Pantograph inference backend. Migrate this saved workflow node to the canonical inference node with a Pumas model reference.";

pub(crate) async fn execute_ollama_inference(
    _inputs: &std::collections::HashMap<String, serde_json::Value>,
) -> Result<std::collections::HashMap<String, serde_json::Value>> {
    Err(NodeEngineError::ExecutionFailed(
        OLLAMA_RETIRED_MESSAGE.to_string(),
    ))
}
