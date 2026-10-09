//! Captured group facts use the authored semantic revision, never a lowered ID.
use pantograph_inference_interface_contracts::{
    DraftGraphEnqueueDisabledReason, DraftGraphValidationSessionId, DraftGraphValidationStatus,
    InferenceDiagnosticSeverity, InferenceInterfaceDiagnostic, WorkflowGraphRevision,
    WorkflowGraphSessionId,
};
use serde::{Deserialize, Serialize};

use super::{
    preflight_groups, NodeRegistry, WorkflowGraph, WorkflowGraphInferenceValidationEventPayload,
    WorkflowGraphInferenceValidationSession, WorkflowGroupFailureFact,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowGroupPreflight {
    pub graph_session_id: WorkflowGraphSessionId,
    pub graph_revision: WorkflowGraphRevision,
    pub validation_session_id: DraftGraphValidationSessionId,
    pub group_count: usize,
    /// The shared lowerer deterministically reports its first blocking refusal.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<WorkflowGroupFailureFact>,
}

impl WorkflowGroupPreflight {
    pub(crate) fn capture(
        graph: &WorkflowGraph,
        graph_session_id: WorkflowGraphSessionId,
        graph_revision: WorkflowGraphRevision,
        validation_session_id: DraftGraphValidationSessionId,
    ) -> Option<Self> {
        let group_count = graph
            .nodes
            .iter()
            .filter(|n| n.node_type == "node-group")
            .count();
        if group_count == 0 {
            return None;
        }
        let failures = preflight_groups(graph, &NodeRegistry::new())
            .err()
            .map(|error| vec![*error.failure])
            .unwrap_or_default();
        Some(Self {
            graph_session_id,
            graph_revision,
            validation_session_id,
            group_count,
            failures,
        })
    }

    pub(crate) fn apply_to_session(&self, session: &mut WorkflowGraphInferenceValidationSession) {
        if self.failures.is_empty() {
            return;
        }
        session.summary.status = DraftGraphValidationStatus::Blocked;
        session.summary.executable = false;
        if !session
            .summary
            .enqueue_disabled_reasons
            .contains(&DraftGraphEnqueueDisabledReason::BlockingDiagnostics)
        {
            session
                .summary
                .enqueue_disabled_reasons
                .insert(0, DraftGraphEnqueueDisabledReason::BlockingDiagnostics);
        }
        session.summary.diagnostics_count = session
            .summary
            .diagnostics_count
            .saturating_add(self.failures.len() as u32);
        session.summary.blocking_diagnostics_count = session
            .summary
            .blocking_diagnostics_count
            .saturating_add(self.failures.len() as u32);
        for event in &mut session.events {
            if let WorkflowGraphInferenceValidationEventPayload::Summary(summary) =
                &mut event.payload
            {
                *summary = session.summary.clone();
            }
        }
    }

    pub(crate) fn diagnostics(&self) -> impl Iterator<Item = InferenceInterfaceDiagnostic> + '_ {
        self.failures.iter().map(|f| InferenceInterfaceDiagnostic {
            severity: InferenceDiagnosticSeverity::Error,
            code: f.code,
            message: f.message.clone(),
            hint: Some(f.repair_hint.clone()),
            port_id: None,
        })
    }
}
