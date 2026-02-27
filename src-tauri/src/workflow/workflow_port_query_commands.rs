use tauri::State;

use super::commands::{SharedExtensions, SharedNodeRegistry};

pub async fn query_port_options(
    registry: State<'_, SharedNodeRegistry>,
    extensions: State<'_, SharedExtensions>,
    node_type: String,
    port_id: String,
    search: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<node_engine::PortOptionsResult, String> {
    let ext = extensions.read().await;
    let query = node_engine::PortOptionsQuery {
        search,
        limit,
        offset,
    };
    registry
        .query_port_options(&node_type, &port_id, &query, &ext)
        .await
        .map_err(|e| e.to_string())
}

pub fn get_queryable_ports(registry: State<'_, SharedNodeRegistry>) -> Vec<(String, String)> {
    registry
        .queryable_ports()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p.to_string()))
        .collect()
}
