use std::sync::Arc;

use tauri::State;

use super::commands::SharedExtensions;

async fn require_pumas_api(
    extensions: &State<'_, SharedExtensions>,
) -> Result<Arc<pumas_library::PumasApi>, String> {
    let ext = extensions.read().await;
    ext.get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
        .cloned()
        .ok_or_else(|| "Pumas API not available in executor extensions".to_string())
}

pub async fn list_models_needing_review(
    extensions: State<'_, SharedExtensions>,
    filter: Option<pumas_library::model_library::ModelReviewFilter>,
) -> Result<Vec<pumas_library::model_library::ModelReviewItem>, String> {
    let api = require_pumas_api(&extensions).await?;
    api.list_models_needing_review(filter)
        .await
        .map_err(|e| e.to_string())
}

pub async fn submit_model_review(
    extensions: State<'_, SharedExtensions>,
    model_id: String,
    patch: serde_json::Value,
    reviewer: String,
    reason: Option<String>,
) -> Result<pumas_library::model_library::SubmitModelReviewResult, String> {
    let api = require_pumas_api(&extensions).await?;
    api.submit_model_review(&model_id, patch, &reviewer, reason.as_deref())
        .await
        .map_err(|e| e.to_string())
}

pub async fn reset_model_review(
    extensions: State<'_, SharedExtensions>,
    model_id: String,
    reviewer: String,
    reason: Option<String>,
) -> Result<bool, String> {
    let api = require_pumas_api(&extensions).await?;
    api.reset_model_review(&model_id, &reviewer, reason.as_deref())
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_effective_model_metadata(
    extensions: State<'_, SharedExtensions>,
    model_id: String,
) -> Result<Option<pumas_library::models::ModelMetadata>, String> {
    let api = require_pumas_api(&extensions).await?;
    api.get_effective_model_metadata(&model_id)
        .await
        .map_err(|e| e.to_string())
}
