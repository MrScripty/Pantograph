use std::collections::BTreeMap;

use pantograph_workflow_service::{
    WorkflowTimingExpectationComparison, WorkflowTraceGraphTimingExpectations,
    WorkflowTraceNodeTimingExpectation,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsTimingExpectation {
    pub comparison: WorkflowTimingExpectationComparison,
    pub sample_count: usize,
    #[serde(default)]
    pub current_duration_ms: Option<u64>,
    #[serde(default)]
    pub median_duration_ms: Option<u64>,
    #[serde(default)]
    pub typical_min_duration_ms: Option<u64>,
    #[serde(default)]
    pub typical_max_duration_ms: Option<u64>,
}

impl From<&pantograph_workflow_service::WorkflowTimingExpectation>
    for DiagnosticsTimingExpectation
{
    fn from(expectation: &pantograph_workflow_service::WorkflowTimingExpectation) -> Self {
        Self {
            comparison: expectation.comparison,
            sample_count: expectation.sample_count,
            current_duration_ms: expectation.current_duration_ms,
            median_duration_ms: expectation.median_duration_ms,
            typical_min_duration_ms: expectation.typical_min_duration_ms,
            typical_max_duration_ms: expectation.typical_max_duration_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsWorkflowTimingHistory {
    pub workflow_id: String,
    #[serde(default)]
    pub graph_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing_expectation: Option<DiagnosticsTimingExpectation>,
    pub nodes: BTreeMap<String, DiagnosticsWorkflowNodeTimingHistory>,
}

impl From<&WorkflowTraceGraphTimingExpectations> for DiagnosticsWorkflowTimingHistory {
    fn from(expectations: &WorkflowTraceGraphTimingExpectations) -> Self {
        Self {
            workflow_id: expectations.workflow_id.clone(),
            graph_fingerprint: expectations.graph_fingerprint.clone(),
            timing_expectation: expectations
                .timing_expectation
                .as_ref()
                .map(DiagnosticsTimingExpectation::from),
            nodes: expectations
                .nodes
                .iter()
                .map(|node| {
                    (
                        node.node_id.clone(),
                        DiagnosticsWorkflowNodeTimingHistory::from(node),
                    )
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsWorkflowNodeTimingHistory {
    pub node_id: String,
    #[serde(default)]
    pub node_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing_expectation: Option<DiagnosticsTimingExpectation>,
}

impl From<&WorkflowTraceNodeTimingExpectation> for DiagnosticsWorkflowNodeTimingHistory {
    fn from(expectation: &WorkflowTraceNodeTimingExpectation) -> Self {
        Self {
            node_id: expectation.node_id.clone(),
            node_type: expectation.node_type.clone(),
            timing_expectation: expectation
                .timing_expectation
                .as_ref()
                .map(DiagnosticsTimingExpectation::from),
        }
    }
}
