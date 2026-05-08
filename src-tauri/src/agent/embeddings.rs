//! Embedding client helpers for RAG
//!
//! Provides utilities for the host RAG embedding adapter used by desktop
//! documentation indexing and retrieval.

use rig::providers::openai;

/// Create the host RAG embedding client used by desktop documentation search.
///
/// This adapter is intentionally scoped to RAG indexing/query embedding calls.
/// Workflow inference must use canonical typed inference contracts instead.
pub(crate) fn create_rag_embedding_client(base_url: &str) -> Result<openai::Client, String> {
    log::debug!("Creating RAG embedding client for base_url: {}", base_url);
    let base_url_with_v1 = rag_embedding_base_url(base_url)?;
    let result = openai::Client::builder()
        .api_key("local")
        .base_url(&base_url_with_v1)
        .build()
        .map_err(|e| format!("Failed to create RAG embedding client: {}", e));

    if result.is_ok() {
        log::debug!("RAG embedding client created successfully");
    }

    result
}

fn rag_embedding_base_url(base_url: &str) -> Result<String, String> {
    let base_url = base_url.trim();
    if base_url.is_empty() {
        return Err("RAG embedding base URL cannot be empty".to_string());
    }
    if base_url.ends_with("/v1") {
        Ok(base_url.to_string())
    } else {
        Ok(format!("{}/v1", base_url.trim_end_matches('/')))
    }
}

/// Check if an embedding server is available at the given URL
///
/// Attempts to connect to the server's health endpoint to verify availability.
///
/// # Arguments
/// * `base_url` - Base URL of the embedding server
///
/// # Returns
/// `true` if the server responds successfully, `false` otherwise
pub async fn check_embedding_server(base_url: &str) -> bool {
    log::debug!("Checking embedding server at base URL: {}", base_url);
    let base = base_url.trim_end_matches('/');

    // Try /health endpoint first (llama.cpp standard)
    let health_url = format!("{}/health", base);
    log::debug!("Trying /health endpoint: {}", health_url);
    if let Ok(resp) = reqwest::get(&health_url).await {
        log::debug!("/health response status: {}", resp.status());
        if resp.status().is_success() {
            log::debug!("/health check passed");
            return true;
        }
    }
    log::debug!("/health endpoint failed or not available");

    // Fall back to /v1/models endpoint (OpenAI standard)
    let models_url = format!("{}/v1/models", base);
    log::debug!("Trying /v1/models endpoint: {}", models_url);
    match reqwest::get(&models_url).await {
        Ok(resp) => {
            log::debug!("/v1/models response status: {}", resp.status());
            if resp.status().is_success() {
                log::debug!("/v1/models check passed");
                return true;
            }
        }
        Err(e) => {
            log::error!("/v1/models request failed: {:?}", e);
        }
    }
    log::debug!("All health check endpoints failed");

    false
}

/// Get the embedding model name from a server
///
/// Queries the /v1/models endpoint to get the available model name.
/// Falls back to "default" if the query fails.
pub async fn get_embedding_model_name(base_url: &str) -> String {
    log::debug!("Getting embedding model name from: {}", base_url);
    let base = base_url.trim_end_matches('/');
    let models_url = format!("{}/v1/models", base);

    #[derive(serde::Deserialize)]
    struct ModelsResponse {
        data: Vec<ModelInfo>,
    }

    #[derive(serde::Deserialize)]
    struct ModelInfo {
        id: String,
    }

    if let Ok(resp) = reqwest::get(&models_url).await {
        if let Ok(models) = resp.json::<ModelsResponse>().await {
            if let Some(model) = models.data.first() {
                let model_name = model.id.clone();
                log::info!("Detected embedding model: {}", model_name);
                return model_name;
            }
        }
    }

    log::warn!("Could not detect embedding model name, using 'default'");
    "default".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rag_embedding_base_url_adds_v1() {
        assert_eq!(
            rag_embedding_base_url("http://localhost:8081").as_deref(),
            Ok("http://localhost:8081/v1")
        );
    }

    #[test]
    fn rag_embedding_base_url_preserves_v1() {
        assert_eq!(
            rag_embedding_base_url("http://localhost:8081/v1").as_deref(),
            Ok("http://localhost:8081/v1")
        );
    }

    #[test]
    fn rag_embedding_base_url_rejects_blank_url() {
        let error = rag_embedding_base_url(" ").expect_err("blank URL should fail");

        assert!(error.contains("cannot be empty"));
    }
}
