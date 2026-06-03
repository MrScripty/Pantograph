//! Port options query system for dynamic value selection.
//!
//! Nodes can register a `PortOptionsProvider` for specific ports, enabling
//! hosts to query available values at configuration time. For example, the
//! `puma-lib` node registers a provider for its `pumas_model_ref` port that
//! returns path-free model references from the Pumas library.
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
use thiserror::Error;

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
    /// Whether this option should be shown but not selectable.
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
    /// Typed unavailable state for disabled options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_state: Option<PortOptionAvailabilityState>,
    /// Stable reason code for disabled/unavailable display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason_code: Option<String>,
    /// Human-readable disabled/unavailable reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
}

/// Port-option availability state projected from backend capability facts.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PortOptionAvailabilityState {
    /// The option can be selected.
    Available,
    /// The option is supported but not installed locally.
    NotInstalled,
    /// The option is reserved/planned but execution is not implemented.
    NotImplemented,
    /// The option cannot run on the current platform.
    UnsupportedPlatform,
    /// The option needs a package/runtime dependency that is not ready.
    MissingDependency,
    /// Product or host policy disables this option.
    DisabledByPolicy,
    /// Required model/package facts were not available.
    MissingModelFacts,
    /// The option requires runtime support that is not available.
    RequiresRuntimeCapability,
    /// The option requires model support that is not available.
    RequiresModelCapability,
}

/// Error returned when a port-option context identifier is invalid.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PortOptionsContextIdError {
    /// Context identifiers must not be blank.
    #[error("port options context id must not be blank")]
    Blank,
    /// Context identifiers are bounded to keep interop payloads small.
    #[error("port options context id must be at most {max} bytes, got {actual}")]
    TooLong { max: usize, actual: usize },
    /// Context identifiers must be displayable, single-line boundary values.
    #[error("port options context id must not contain control characters")]
    ContainsControlCharacter,
}

/// Validated identifier carried in provider query context.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PortOptionsContextId(String);

impl PortOptionsContextId {
    /// Maximum serialized byte length for a single context identifier.
    pub const MAX_LEN: usize = 512;

    /// Build a validated context identifier.
    pub fn new(value: impl Into<String>) -> std::result::Result<Self, PortOptionsContextIdError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(PortOptionsContextIdError::Blank);
        }
        if trimmed.len() > Self::MAX_LEN {
            return Err(PortOptionsContextIdError::TooLong {
                max: Self::MAX_LEN,
                actual: trimmed.len(),
            });
        }
        if trimmed.chars().any(char::is_control) {
            return Err(PortOptionsContextIdError::ContainsControlCharacter);
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Return the validated identifier string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for PortOptionsContextId {
    type Error = PortOptionsContextIdError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<PortOptionsContextId> for String {
    fn from(value: PortOptionsContextId) -> Self {
        value.0
    }
}

/// Optional fact context used by model/runtime-dependent option providers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortOptionsQueryContext {
    /// Target graph node requesting options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_node_id: Option<PortOptionsContextId>,
    /// Canonical task kind for the target port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_kind: Option<PortOptionsContextId>,
    /// Stable selected model reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_model_ref: Option<PortOptionsContextId>,
    /// Package-facts summary/update cursor used for cache invalidation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_facts_summary_cursor: Option<PortOptionsContextId>,
    /// Optional graph-authored scheduler runtime requirement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_runtime_id: Option<PortOptionsContextId>,
    /// Optional graph-authored scheduler device requirement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_device_id: Option<PortOptionsContextId>,
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
    /// Optional provider context for model/runtime-dependent option lists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<PortOptionsQueryContext>,
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
    /// Optional structured result metadata, such as snapshot cursors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
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
///     port_id: "pumas_model_ref",
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

fn is_false(value: &bool) -> bool {
    !*value
}

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
            disabled: false,
            unavailable_state: None,
            unavailable_reason_code: None,
            unavailable_reason: None,
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
        assert!(query.context.is_none());
    }

    #[test]
    fn test_port_options_query_context_serializes_stable_refs() {
        let query = PortOptionsQuery {
            search: Some("euler".to_string()),
            limit: Some(10),
            offset: None,
            context: Some(PortOptionsQueryContext {
                target_node_id: Some(PortOptionsContextId::new("node-image-1").unwrap()),
                task_kind: Some(PortOptionsContextId::new("image_generation").unwrap()),
                selected_model_ref: Some(
                    PortOptionsContextId::new("pumas://models/diffusion/tiny").unwrap(),
                ),
                package_facts_summary_cursor: Some(
                    PortOptionsContextId::new("model-library-updates:42").unwrap(),
                ),
                requested_runtime_id: Some(PortOptionsContextId::new("pytorch").unwrap()),
                requested_device_id: Some(PortOptionsContextId::new("cuda:0").unwrap()),
            }),
        };

        let json = serde_json::to_value(&query).unwrap();
        assert_eq!(json["context"]["targetNodeId"], "node-image-1");
        assert_eq!(json["context"]["taskKind"], "image_generation");
        assert_eq!(
            json["context"]["selectedModelRef"],
            "pumas://models/diffusion/tiny"
        );
        assert_eq!(
            json["context"]["packageFactsSummaryCursor"],
            "model-library-updates:42"
        );
        assert_eq!(json["context"]["requestedRuntimeId"], "pytorch");
        assert_eq!(json["context"]["requestedDeviceId"], "cuda:0");
        assert!(json["context"]["backendId"].is_null());
        assert!(json["context"]["runtimeVariantId"].is_null());

        let round_trip: PortOptionsQuery = serde_json::from_value(json).unwrap();
        let context = round_trip.context.expect("context should round trip");
        assert_eq!(
            context
                .selected_model_ref
                .as_ref()
                .map(PortOptionsContextId::as_str),
            Some("pumas://models/diffusion/tiny")
        );
    }

    #[test]
    fn test_port_options_query_context_rejects_blank_ids() {
        let error = serde_json::from_value::<PortOptionsQuery>(serde_json::json!({
            "context": {
                "targetNodeId": "  "
            }
        }))
        .expect_err("blank context identifiers should fail deserialization");

        assert!(error
            .to_string()
            .contains("port options context id must not be blank"));
    }

    #[test]
    fn test_port_options_result_serialization() {
        let result = PortOptionsResult {
            options: vec![PortOption {
                value: serde_json::json!("test"),
                label: "Test".to_string(),
                description: None,
                metadata: None,
                disabled: false,
                unavailable_state: None,
                unavailable_reason_code: None,
                unavailable_reason: None,
            }],
            total_count: 1,
            searchable: true,
            metadata: Some(serde_json::json!({"cursor": "model-library-updates:1"})),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["totalCount"], 1);
        assert_eq!(json["searchable"], true);
        assert_eq!(json["metadata"]["cursor"], "model-library-updates:1");
        assert_eq!(json["options"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_port_option_disabled_state_serializes_append_only_fields() {
        let option = PortOption {
            value: serde_json::json!("euler_discrete"),
            label: "Euler Discrete".to_string(),
            description: None,
            metadata: None,
            disabled: true,
            unavailable_state: Some(PortOptionAvailabilityState::RequiresRuntimeCapability),
            unavailable_reason_code: Some("scheduler_not_supported".to_string()),
            unavailable_reason: Some("Selected runtime does not expose this scheduler".to_string()),
        };

        let json = serde_json::to_value(&option).unwrap();
        assert_eq!(json["disabled"], true);
        assert_eq!(json["unavailableState"], "requires_runtime_capability");
        assert_eq!(json["unavailableReasonCode"], "scheduler_not_supported");
        assert_eq!(
            json["unavailableReason"],
            "Selected runtime does not expose this scheduler"
        );

        let round_trip: PortOption = serde_json::from_value(json).unwrap();
        assert!(round_trip.disabled);
        assert_eq!(
            round_trip.unavailable_state,
            Some(PortOptionAvailabilityState::RequiresRuntimeCapability)
        );
    }

    #[test]
    fn test_port_option_disabled_fields_default_for_legacy_payloads() {
        let option: PortOption = serde_json::from_value(serde_json::json!({
            "value": "euler_discrete",
            "label": "Euler Discrete",
            "description": null,
            "metadata": null
        }))
        .unwrap();

        assert!(!option.disabled);
        assert!(option.unavailable_state.is_none());
        assert!(option.unavailable_reason_code.is_none());
        assert!(option.unavailable_reason.is_none());
    }
}
