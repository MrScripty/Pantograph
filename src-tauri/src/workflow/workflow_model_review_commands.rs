use std::sync::Arc;
use tauri::State;
use workflow_nodes::setup::{PumasSelectorAccess, PUMAS_SELECTOR_ACCESS};

use super::commands::SharedExtensions;

async fn require_pumas_selector_access(
    extensions: &State<'_, SharedExtensions>,
) -> Result<Arc<PumasSelectorAccess>, String> {
    let ext = extensions.read().await;
    ext.get::<Arc<PumasSelectorAccess>>(PUMAS_SELECTOR_ACCESS)
        .cloned()
        .ok_or_else(|| "Pumas selector access not available in executor extensions".to_string())
}

pub async fn list_models_needing_review(
    extensions: State<'_, SharedExtensions>,
    filter: Option<pumas_library::model_library::ModelReviewFilter>,
) -> Result<Vec<pumas_library::model_library::ModelReviewItem>, String> {
    let selector_access = require_pumas_selector_access(&extensions).await?;
    selector_access
        .list_models_needing_review(filter)
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
    let selector_access = require_pumas_selector_access(&extensions).await?;
    selector_access
        .submit_model_review(&model_id, patch, &reviewer, reason.as_deref())
        .await
        .map_err(|e| e.to_string())
}

pub async fn reset_model_review(
    extensions: State<'_, SharedExtensions>,
    model_id: String,
    reviewer: String,
    reason: Option<String>,
) -> Result<bool, String> {
    let selector_access = require_pumas_selector_access(&extensions).await?;
    selector_access
        .reset_model_review(&model_id, &reviewer, reason.as_deref())
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_effective_model_metadata(
    extensions: State<'_, SharedExtensions>,
    model_id: String,
) -> Result<Option<pumas_library::models::ModelMetadata>, String> {
    let selector_access = require_pumas_selector_access(&extensions).await?;
    selector_access
        .effective_model_metadata(&model_id)
        .await
        .map_err(|e| e.to_string())
}
