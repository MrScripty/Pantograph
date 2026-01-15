//! Embedding client helpers for RAG
//!
//! Provides utilities for creating OpenAI-compatible embedding clients
//! that can connect to local llama.cpp servers running embedding models.

use rig::providers::openai;

/// Create an OpenAI-compatible client for embeddings
///
/// This client can connect to a llama.cpp server running an embedding model
/// (e.g., Qwen3-Embedding-0.6B) at a custom base URL.
///
/// # Arguments
/// * `base_url` - Base URL of the embedding server (e.g., "http://127.0.0.1:8081")
///
/// # Returns
/// An OpenAI client configured for the embedding server
pub fn create_embedding_client(base_url: &str) -> Result<openai::Client, String> {
    log::debug!("Creating embedding client for base_url: {}", base_url);
    // RIG expects the base URL to include /v1 suffix for proper endpoint routing
    let base_url_with_v1 = if base_url.ends_with("/v1") {
        base_url.to_string()
    } else {
        format!("{}/v1", base_url.trim_end_matches('/'))
    };

    let result = openai::Client::builder()
        .api_key("local") // Local servers typically don't require auth
        .base_url(&base_url_with_v1)
        .build()
        .map_err(|e| format!("Failed to create embedding client: {}", e));

    if result.is_ok() {
        log::debug!("Embedding client created successfully");
    }

    result
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
    fn test_create_embedding_client_adds_v1() {
        let result = create_embedding_client("http://localhost:8081");
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_embedding_client_preserves_v1() {
        let result = create_embedding_client("http://localhost:8081/v1");
        assert!(result.is_ok());
    }
}
