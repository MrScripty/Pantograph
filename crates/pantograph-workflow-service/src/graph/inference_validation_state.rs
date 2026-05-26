use std::collections::HashMap;

use pantograph_inference_interface_contracts::{
    DependencyEnvironmentActionIntent, DependencyEnvironmentActionIntentResult,
    DependencyEnvironmentActionIntentStatus, DraftGraphValidationSummary, InferenceDiagnosticCode,
    InferenceDiagnosticSeverity, InferenceInterfaceDiagnostic,
    ValidatedDependencyEnvironmentActionIntent, WorkflowGraphRevision,
};
use tokio::sync::RwLock;

#[derive(Debug)]
pub(crate) struct CurrentInferenceValidationStateStore {
    summaries: RwLock<HashMap<CurrentInferenceValidationStateKey, DraftGraphValidationSummary>>,
}

impl Default for CurrentInferenceValidationStateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CurrentInferenceValidationStateStore {
    pub(crate) fn new() -> Self {
        Self {
            summaries: RwLock::new(HashMap::new()),
        }
    }

    pub(crate) async fn resolve_dependency_environment_action_intent(
        &self,
        request: DependencyEnvironmentActionIntentStateRequest,
    ) -> DependencyEnvironmentActionIntentResult {
        let intent = request.intent.into_inner();
        if request.current_graph_revision != intent.graph_revision {
            return blocked_dependency_environment_action_result(
                &intent,
                InferenceDiagnosticCode::GraphRevisionMismatch,
                "Dependency environment action was requested for a stale graph revision.",
                Some(
                    "Refresh graph validation for the current graph before resolving dependencies.",
                ),
            );
        }

        if !request.target_node_exists {
            return blocked_dependency_environment_action_result(
                &intent,
                InferenceDiagnosticCode::TargetNodeMissing,
                "Dependency environment action target node does not exist in the current graph.",
                None,
            );
        }

        let key = CurrentInferenceValidationStateKey {
            graph_session_id: intent.graph_session_id.clone(),
            graph_revision: intent.graph_revision.clone(),
        };
        let summaries = self.summaries.read().await;
        if !summaries.contains_key(&key) {
            return blocked_dependency_environment_action_result(
                &intent,
                InferenceDiagnosticCode::ValidationSummaryMissing,
                "Inference validation has not completed for this graph revision.",
                Some("Run descriptor validation before resolving dependency environments."),
            );
        }

        blocked_dependency_environment_action_result(
            &intent,
            InferenceDiagnosticCode::DependencyRequirementsMissing,
            "Dependency environment request derivation requires dependency requirements for this graph revision.",
            Some("Resolve dependency requirements from the current validation state before checking or installing environments."),
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DependencyEnvironmentActionIntentStateRequest {
    pub intent: ValidatedDependencyEnvironmentActionIntent,
    pub current_graph_revision: WorkflowGraphRevision,
    pub target_node_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CurrentInferenceValidationStateKey {
    graph_session_id: pantograph_inference_interface_contracts::WorkflowGraphSessionId,
    graph_revision: WorkflowGraphRevision,
}

fn blocked_dependency_environment_action_result(
    intent: &DependencyEnvironmentActionIntent,
    code: InferenceDiagnosticCode,
    message: &str,
    hint: Option<&str>,
) -> DependencyEnvironmentActionIntentResult {
    DependencyEnvironmentActionIntentResult {
        contract_version: intent.contract_version,
        graph_session_id: intent.graph_session_id.clone(),
        graph_revision: intent.graph_revision.clone(),
        validation_session_id: intent.validation_session_id.clone(),
        target_node_id: intent.target_node_id.clone(),
        action: intent.action,
        status: DependencyEnvironmentActionIntentStatus::Blocked,
        diagnostics: vec![InferenceInterfaceDiagnostic {
            severity: InferenceDiagnosticSeverity::Error,
            code,
            message: message.to_string(),
            hint: hint.map(str::to_string),
            port_id: None,
        }],
    }
}

#[cfg(test)]
mod tests {
    use pantograph_inference_interface_contracts::{
        DependencyEnvironmentAction, InferenceDiagnosticCode,
    };

    use super::*;

    #[tokio::test]
    async fn action_intent_state_rejects_stale_graph_revision() {
        let store = CurrentInferenceValidationStateStore::new();

        let result = store
            .resolve_dependency_environment_action_intent(state_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "bbbbbbbbbbbbbbbb",
                "dependency-node-1",
                true,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::GraphRevisionMismatch
        );
    }

    #[tokio::test]
    async fn action_intent_state_rejects_missing_target_node() {
        let store = CurrentInferenceValidationStateStore::new();

        let result = store
            .resolve_dependency_environment_action_intent(state_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "dependency-node-1",
                false,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::TargetNodeMissing
        );
    }

    #[tokio::test]
    async fn action_intent_state_requires_current_summary() {
        let store = CurrentInferenceValidationStateStore::new();

        let result = store
            .resolve_dependency_environment_action_intent(state_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "dependency-node-1",
                true,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::ValidationSummaryMissing
        );
    }

    fn state_request(
        graph_session_id: &str,
        intent_revision: &str,
        current_revision: &str,
        target_node_id: &str,
        target_node_exists: bool,
    ) -> DependencyEnvironmentActionIntentStateRequest {
        DependencyEnvironmentActionIntentStateRequest {
            intent: ValidatedDependencyEnvironmentActionIntent::try_from(
                DependencyEnvironmentActionIntent {
                    contract_version: 1,
                    graph_session_id: graph_session_id.parse().expect("valid graph session id"),
                    graph_revision: intent_revision.parse().expect("valid graph revision"),
                    validation_session_id: None,
                    target_node_id: target_node_id.parse().expect("valid target node id"),
                    action: DependencyEnvironmentAction::Resolve,
                },
            )
            .expect("valid action intent"),
            current_graph_revision: current_revision.parse().expect("valid current revision"),
            target_node_exists,
        }
    }
}
