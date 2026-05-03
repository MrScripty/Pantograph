use std::collections::HashMap;

use crate::error::{NodeEngineError, Result};

pub(crate) fn parse_reranker_documents(value: &serde_json::Value) -> Result<Vec<String>> {
    let items = if let Some(items) = value.as_array() {
        items
    } else {
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker documents input must be a JSON array".to_string(),
        ));
    };

    let mut documents = Vec::with_capacity(items.len());
    for item in items {
        if let Some(text) = item.as_str() {
            if !text.trim().is_empty() {
                documents.push(text.to_string());
            }
            continue;
        }
        if let Some(text) = item
            .get("text")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("content").and_then(|v| v.as_str()))
            .or_else(|| item.get("document").and_then(|v| v.as_str()))
        {
            if !text.trim().is_empty() {
                documents.push(text.to_string());
            }
            continue;
        }
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker documents must be strings or objects with text/content/document fields"
                .to_string(),
        ));
    }

    if documents.is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker documents input cannot be empty".to_string(),
        ));
    }

    Ok(documents)
}

pub(crate) fn parse_reranker_documents_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<Vec<String>> {
    if let Some(value) = inputs.get("documents") {
        return parse_reranker_documents(value);
    }

    if let Some(raw) = inputs
        .get("documents_json")
        .and_then(|value| value.as_str())
    {
        let parsed: serde_json::Value = serde_json::from_str(raw).map_err(|e| {
            NodeEngineError::ExecutionFailed(format!(
                "Reranker documents_json must be valid JSON: {}",
                e
            ))
        })?;
        return parse_reranker_documents(&parsed);
    }

    Err(NodeEngineError::ExecutionFailed(
        "Missing documents input".to_string(),
    ))
}
