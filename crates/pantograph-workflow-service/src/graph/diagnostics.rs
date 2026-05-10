use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

const MAX_DIAGNOSTIC_MESSAGE_CHARS: usize = 512;
const MAX_DIAGNOSTIC_DETAIL_FIELDS: usize = 8;
const MAX_DIAGNOSTIC_DETAIL_KEY_CHARS: usize = 64;
const MAX_DIAGNOSTIC_DETAIL_VALUE_CHARS: usize = 256;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowGraphDiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowGraphDiagnosticScope {
    Graph,
    Node,
    Edge,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowGraphDiagnosticCode {
    DuplicateNodeId,
    DuplicateEdgeId,
    UnknownNodeType,
    RetiredNodeType,
    InvalidNodeId,
    InvalidNodeType,
    InvalidDynamicDefinition,
    MissingEdgeSourceNode,
    MissingEdgeTargetNode,
    SelfConnection,
    MissingSourceContract,
    MissingTargetContract,
    MissingSourceOutput,
    MissingTargetInput,
    TargetInputCapacityReached,
    IncompatiblePortTypes,
    CompatibilityCheckFailed,
    CycleDetected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphDiagnostic {
    pub code: WorkflowGraphDiagnosticCode,
    pub severity: WorkflowGraphDiagnosticSeverity,
    pub scope: WorkflowGraphDiagnosticScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_type: Option<String>,
    pub message: String,
    pub blocking_submission: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub details: BTreeMap<String, String>,
}

impl WorkflowGraphDiagnostic {
    pub(crate) fn node(
        code: WorkflowGraphDiagnosticCode,
        severity: WorkflowGraphDiagnosticSeverity,
        node_id: impl AsRef<str>,
        node_type: impl AsRef<str>,
        message: impl AsRef<str>,
        blocking_submission: bool,
    ) -> Self {
        Self::new(
            code,
            severity,
            WorkflowGraphDiagnosticScope::Node,
            Some(bounded_text(
                node_id.as_ref(),
                MAX_DIAGNOSTIC_DETAIL_VALUE_CHARS,
            )),
            Some(bounded_text(
                node_type.as_ref(),
                MAX_DIAGNOSTIC_DETAIL_VALUE_CHARS,
            )),
            message,
            blocking_submission,
        )
    }

    pub(crate) fn edge(
        code: WorkflowGraphDiagnosticCode,
        severity: WorkflowGraphDiagnosticSeverity,
        edge_id: impl AsRef<str>,
        message: impl AsRef<str>,
        blocking_submission: bool,
    ) -> Self {
        Self::new(
            code,
            severity,
            WorkflowGraphDiagnosticScope::Edge,
            None,
            None,
            message,
            blocking_submission,
        )
        .with_detail("edge_id", edge_id)
    }

    fn new(
        code: WorkflowGraphDiagnosticCode,
        severity: WorkflowGraphDiagnosticSeverity,
        scope: WorkflowGraphDiagnosticScope,
        node_id: Option<String>,
        node_type: Option<String>,
        message: impl AsRef<str>,
        blocking_submission: bool,
    ) -> Self {
        Self {
            code,
            severity,
            scope,
            node_id,
            node_type,
            message: bounded_text(message.as_ref(), MAX_DIAGNOSTIC_MESSAGE_CHARS),
            blocking_submission,
            details: BTreeMap::new(),
        }
    }

    pub(crate) fn with_detail(mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        if self.details.len() >= MAX_DIAGNOSTIC_DETAIL_FIELDS {
            return self;
        }

        let key = bounded_text(key.as_ref().trim(), MAX_DIAGNOSTIC_DETAIL_KEY_CHARS);
        if key.is_empty() {
            return self;
        }
        let value = bounded_text(value.as_ref(), MAX_DIAGNOSTIC_DETAIL_VALUE_CHARS);
        self.details.insert(key, value);
        self
    }
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    let mut bounded = value.chars().take(keep).collect::<String>();
    bounded.push_str("...");
    bounded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_round_trip_preserves_wire_shape() {
        let diagnostic = WorkflowGraphDiagnostic::node(
            WorkflowGraphDiagnosticCode::RetiredNodeType,
            WorkflowGraphDiagnosticSeverity::Error,
            "diffusion",
            "diffusion-inference",
            "retired direct diffusion node",
            true,
        )
        .with_detail("replacement_node_type", "llm-inference");

        let encoded = serde_json::to_value(&diagnostic).expect("encode diagnostic");

        assert_eq!(encoded["code"], serde_json::json!("retired_node_type"));
        assert_eq!(encoded["severity"], serde_json::json!("error"));
        assert_eq!(encoded["scope"], serde_json::json!("node"));
        assert_eq!(encoded["node_id"], serde_json::json!("diffusion"));
        assert_eq!(
            encoded["details"]["replacement_node_type"],
            serde_json::json!("llm-inference")
        );

        let decoded: WorkflowGraphDiagnostic =
            serde_json::from_value(encoded).expect("decode diagnostic");
        assert_eq!(decoded, diagnostic);
    }

    #[test]
    fn diagnostic_details_are_bounded_before_serializing() {
        let long_value = "x".repeat(MAX_DIAGNOSTIC_DETAIL_VALUE_CHARS + 20);
        let diagnostic = WorkflowGraphDiagnostic::edge(
            WorkflowGraphDiagnosticCode::CompatibilityCheckFailed,
            WorkflowGraphDiagnosticSeverity::Error,
            "edge",
            "x".repeat(MAX_DIAGNOSTIC_MESSAGE_CHARS + 20),
            true,
        )
        .with_detail("a".repeat(MAX_DIAGNOSTIC_DETAIL_KEY_CHARS + 20), long_value)
        .with_detail("field_1", "1")
        .with_detail("field_2", "2")
        .with_detail("field_3", "3")
        .with_detail("field_4", "4")
        .with_detail("field_5", "5")
        .with_detail("field_6", "6")
        .with_detail("field_7", "7")
        .with_detail("field_8", "8");

        assert_eq!(
            diagnostic.message.chars().count(),
            MAX_DIAGNOSTIC_MESSAGE_CHARS
        );
        assert_eq!(diagnostic.details.len(), MAX_DIAGNOSTIC_DETAIL_FIELDS);
        assert!(diagnostic
            .details
            .keys()
            .all(|key| key.chars().count() <= MAX_DIAGNOSTIC_DETAIL_KEY_CHARS));
        assert!(diagnostic
            .details
            .values()
            .all(|value| value.chars().count() <= MAX_DIAGNOSTIC_DETAIL_VALUE_CHARS));
    }
}
