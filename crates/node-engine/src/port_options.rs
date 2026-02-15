//! Port options query system for dynamic value selection.
//!
//! Nodes can register a `PortOptionsProvider` for specific ports, enabling
//! hosts to query available values at configuration time. For example, the
//! `puma-lib` node registers a provider for its `model_path` port that
//! returns available models from the pumas-library.
//!
//! # Registration
//!
//! Providers are registered via `inventory` at link time, using the same
//! pattern as `DescriptorFn`:
//!
//! ```ignore
//! inventory::submit!(node_engine::PortQueryFn {
//!     node_type: "my-node",
//!     port_id: "my_port",
//!     provider: || Box::new(MyOptionsProvider),
//! });
//! ```
//!
//! # Querying
//!
//! Hosts call `NodeRegistry::query_port_options()` with the node type,
//! port id, query parameters, and `ExecutorExtensions` (for accessing
//! runtime dependencies like `PumasApi`).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::extensions::ExecutorExtensions;

/// A selectable option for a port value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortOption {
    /// The value to store when this option is selected (e.g., a file path).
    pub value: serde_json::Value,
    /// Human-readable display label.
    pub label: String,
    /// Optional description or extra context.
    pub description: Option<String>,
    /// Optional structured metadata (e.g., model type, tags).
    pub metadata: Option<serde_json::Value>,
}

/// Query parameters for fetching port options.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortOptionsQuery {
    /// Optional search/filter string.
    pub search: Option<String>,
    /// Maximum number of results to return.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
}

/// Result of a port options query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortOptionsResult {
    /// Available options.
    pub options: Vec<PortOption>,
    /// Total number of matching options (may exceed `options.len()` if paginated).
    pub total_count: usize,
    /// Whether this provider supports server-side search filtering.
    pub searchable: bool,
}

/// Trait for providing dynamic options for a port.
///
/// Implementations are stateless — runtime dependencies come from
/// `ExecutorExtensions` (e.g., `PumasApi` accessed via `extension_keys::PUMAS_API`).
#[async_trait]
pub trait PortOptionsProvider: Send + Sync {
    /// Query available options for this port.
    async fn query_options(
        &self,
        query: &PortOptionsQuery,
        extensions: &ExecutorExtensions,
    ) -> Result<PortOptionsResult>;
}

/// Link-time registration of a port options provider.
///
/// Uses the same function-pointer pattern as `DescriptorFn`:
/// the `provider` field is a const function pointer that creates a
/// `Box<dyn PortOptionsProvider>` at runtime.
///
/// # Example
///
/// ```ignore
/// inventory::submit!(node_engine::PortQueryFn {
///     node_type: "puma-lib",
///     port_id: "model_path",
///     provider: || Box::new(PumaLibOptionsProvider),
/// });
/// ```
pub struct PortQueryFn {
    /// The node type this provider belongs to.
    pub node_type: &'static str,
    /// The port id this provider serves options for.
    pub port_id: &'static str,
    /// Factory function that creates the provider instance.
    pub provider: fn() -> Box<dyn PortOptionsProvider>,
}

inventory::collect!(PortQueryFn);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_option_serialization() {
        let option = PortOption {
            value: serde_json::json!("/path/to/model.gguf"),
            label: "Llama 3.2 7B".to_string(),
            description: Some("llm | gguf, quantized".to_string()),
            metadata: Some(serde_json::json!({"model_type": "llm"})),
        };

        let json = serde_json::to_value(&option).unwrap();
        assert_eq!(json["label"], "Llama 3.2 7B");
        assert_eq!(json["value"], "/path/to/model.gguf");
        assert!(json["description"].is_string());
    }

    #[test]
    fn test_port_options_query_default() {
        let query = PortOptionsQuery::default();
        assert!(query.search.is_none());
        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
    }

    #[test]
    fn test_port_options_result_serialization() {
        let result = PortOptionsResult {
            options: vec![PortOption {
                value: serde_json::json!("test"),
                label: "Test".to_string(),
                description: None,
                metadata: None,
            }],
            total_count: 1,
            searchable: true,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["totalCount"], 1);
        assert_eq!(json["searchable"], true);
        assert_eq!(json["options"].as_array().unwrap().len(), 1);
    }
}
