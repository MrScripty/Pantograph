use tauri::State;

use super::commands::{SharedExtensions, SharedNodeRegistry, SharedWorkflowService};

pub async fn query_port_options(
    registry: State<'_, SharedNodeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    node_type: String,
    port_id: String,
    search: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<node_engine::PortOptionsResult, String> {
    let ext = extensions.read().await;
    let query = node_engine::PortOptionsQuery {
        search: search.clone(),
        limit,
        offset,
    };
    let result = registry
        .query_port_options(&node_type, &port_id, &query, &ext)
        .await
        .map_err(|e| e.to_string())?;

    record_pumas_port_options_audit(&workflow_service, &node_type, &port_id, search.as_deref());

    Ok(result)
}

pub fn get_queryable_ports(registry: State<'_, SharedNodeRegistry>) -> Vec<(String, String)> {
    registry
        .queryable_ports()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p.to_string()))
        .collect()
}

fn record_pumas_port_options_audit(
    workflow_service: &SharedWorkflowService,
    node_type: &str,
    port_id: &str,
    search: Option<&str>,
) {
    if node_type != "puma-lib" || port_id != "model_path" {
        return;
    }

    let operation = if search.is_some_and(|value| !value.trim().is_empty()) {
        pantograph_workflow_service::LibraryAssetOperation::Search
    } else {
        pantograph_workflow_service::LibraryAssetOperation::Access
    };
    if let Err(error) = workflow_service.workflow_library_asset_access_record(
        pantograph_workflow_service::WorkflowLibraryAssetAccessRecordRequest {
            asset_id: "pumas://models".to_string(),
            operation,
            cache_status: Some(pantograph_workflow_service::LibraryAssetCacheStatus::Unknown),
            network_bytes: None,
            source_instance_id: Some("puma-lib-port-options".to_string()),
        },
    ) {
        log::warn!("Failed to record Puma-Lib port options audit event: {error}");
    }
}
